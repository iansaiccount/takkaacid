//! Which device hears which controller.
//!
//! The filter is the *only* scope: whether a message reaches a device is decided here, and what it drives once it is there
//! is the device's own learn map (`midi::cc`). One question per function, so a learned controller and a routed one cannot
//! disagree about who owns CC 74.

use std::sync::Arc;

use crate::device::BusState;
use crate::midi::gate::{self, NoteGate};
use crate::midi::message::{self, MidiEvent};
use crate::midi::tempo::TempoFollower;
use crate::state::AtomicF32;

/// Which of the opened ports may address a device.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Ports {
    /// Every port the app has open.
    All,
    /// These ports, by the name the system reports. An empty list means the device hears nothing, which is how a device
    /// that has just been added starts.
    Named(Vec<String>),
}

/// Which channel on those ports.
///
/// The store is `0..=15`; the device card shows `1..=16`.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Channel {
    All,
    One(u8),
}

/// Whether a device plays what it hears.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Role {
    /// Notes and controllers both.
    PlayAndControl,
    /// Controllers only: a box of knobs must not start a note just because a message arrived on its channel.
    ControlOnly,
}

/// What a device hears.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct InputFilter {
    pub ports: Ports,
    pub channel: Channel,
    pub role: Role,
}

impl InputFilter {
    /// The factory device: every port, every channel, notes and controllers. A player who never opens the device card cannot
    /// tell phase 2 happened.
    pub fn everything() -> Self {
        Self {
            ports: Ports::All,
            channel: Channel::All,
            role: Role::PlayAndControl,
        }
    }

    /// A device that has just been added: it hears nothing until it is given a port, so adding one never silently layers it
    /// under a keyboard that is already playing.
    pub fn silent() -> Self {
        Self {
            ports: Ports::Named(Vec::new()),
            ..Self::everything()
        }
    }

    /// Whether a message from `port` on `channel` is addressed to this device.
    ///
    /// The role is deliberately not consulted: a controller-only device still receives its knobs. The caller asks the role
    /// only about messages that carry notes.
    pub fn hears(&self, port: &str, channel: u8) -> bool {
        let port_ok = match &self.ports {
            Ports::All => true,
            Ports::Named(names) => names.iter().any(|name| name == port),
        };
        let channel_ok = match self.channel {
            Channel::All => true,
            Channel::One(only) => only == channel,
        };
        port_ok && channel_ok
    }
}

/// One open port's point of view: its name, its clock follower, its note gate, and the tempo cell it writes.
///
/// Built per connection and owned by that connection's handler, which runs on midir's thread: everything below writes atomics
/// or loads the published bus, and nothing locks or allocates.
pub struct Router {
    port: String,
    follow: TempoFollower,
    gate: NoteGate,
    tempo: Arc<AtomicF32>,
}

impl Router {
    pub fn new(port: impl Into<String>, tempo: Arc<AtomicF32>) -> Self {
        Self {
            port: port.into(),
            follow: TempoFollower::new(),
            gate: NoteGate::new(),
            tempo,
        }
    }

    pub fn port(&self) -> &str {
        &self.port
    }

    /// Route one message from this port to every device that hears it.
    pub fn on_message(&mut self, bus: &BusState, bytes: &[u8]) {
        // The raw bytes are the ground truth: they are what the keyboard actually sent, before the app reads anything into
        // them. Looking at them is how a player tells a transposed keyboard from a wrong mapping.
        crate::trace::note("midi raw", || {
            let hex: Vec<String> = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
            format!("{} ({} bytes)", hex.join(" "), bytes.len())
        });
        let Some(event) = message::parse(bytes) else {
            return;
        };
        match event {
            // A clock moves the whole rack, not one device: the follower writes the cell and `main` fans it out.
            MidiEvent::Clock => {
                if let Some(bpm) = self.follow.on_clock(std::time::Instant::now()) {
                    self.tempo.set(acid_core::timing::clamp_bpm(bpm));
                }
            }
            MidiEvent::ControlChange {
                channel, cc, value, ..
            } => {
                for (slot, filter) in bus.slots.iter().zip(&bus.routes) {
                    if filter.hears(&self.port, channel) {
                        crate::midi::cc::apply_cc(slot, cc, value);
                    }
                }
            }
            event if event.carries_notes() => {
                // One gate per port, shared by every device it reaches: a note already down is the same note delivered
                // twice, not a second press, and it must not write a second step.
                if !gate::passes(&mut self.gate, event) {
                    return;
                }
                for (slot, filter) in bus.slots.iter().zip(&bus.routes) {
                    if filter.role == Role::PlayAndControl
                        && filter.hears(&self.port, channel_of(event))
                    {
                        message::handle(event, slot);
                    }
                }
            }
            // Transport bytes are not routed yet: the transport is the rack's, and a MIDI transport has no rack to reach
            // from this thread. Parsed, dropped, and this round does not pretend otherwise.
            MidiEvent::Start | MidiEvent::Stop | MidiEvent::Continue => {}
            // Every channel message that carries notes is handled above, so the rest are not interesting here.
            MidiEvent::NoteOn { .. } | MidiEvent::NoteOff { .. } => {}
        }
    }
}

/// The channel a channel message arrived on. Only called for messages that have one.
fn channel_of(event: MidiEvent) -> u8 {
    event.channel().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_factory_device_hears_every_port_and_channel() {
        let filter = InputFilter::everything();
        for port in ["Launchkey MIDI", "Arturia KeyStep", "nanoKONTROL"] {
            for channel in 0..=15 {
                assert!(filter.hears(port, channel), "{port} ch{channel}");
            }
        }
    }

    #[test]
    fn a_new_device_hears_nothing_until_it_is_given_a_port() {
        let filter = InputFilter::silent();
        assert!(!filter.hears("Launchkey MIDI", 0));
        assert!(!filter.hears("Launchkey MIDI", 9));
    }

    #[test]
    fn a_named_port_and_a_named_channel_both_have_to_match() {
        let filter = InputFilter {
            ports: Ports::Named(vec!["nanoKONTROL".to_string()]),
            channel: Channel::One(3),
            role: Role::ControlOnly,
        };
        assert!(filter.hears("nanoKONTROL", 3));
        assert!(
            !filter.hears("nanoKONTROL", 2),
            "the channel is the one it was given"
        );
        assert!(
            !filter.hears("Launchkey MIDI", 3),
            "no other port may address it"
        );

        let any_channel = InputFilter {
            channel: Channel::All,
            ..filter
        };
        assert!(any_channel.hears("nanoKONTROL", 0));
        assert!(any_channel.hears("nanoKONTROL", 15));
    }

    #[test]
    fn a_controller_only_device_still_hears_but_may_not_play() {
        let filter = InputFilter {
            role: Role::ControlOnly,
            ..InputFilter::everything()
        };
        assert!(filter.hears("Launchkey MIDI", 0), "its knobs still arrive");
        assert_eq!(filter.role, Role::ControlOnly);
    }
}
