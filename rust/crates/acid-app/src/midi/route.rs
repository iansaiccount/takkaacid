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
        self.on_message_at(bus, bytes, std::time::Instant::now());
    }

    /// The same, with the arrival time handed in rather than read.
    ///
    /// The clock follower reports a tempo only for a stream whose intervals agree, so a test has to choose those intervals;
    /// production always calls this with `Instant::now()`.
    pub fn on_message_at(&mut self, bus: &BusState, bytes: &[u8], now: std::time::Instant) {
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
            // A clock moves the whole rack, not one device: it writes the cell every device's state already holds, so the
            // engines read it at their next block and nothing has to be fanned out.
            MidiEvent::Clock => {
                if let Some(bpm) = self.follow.on_clock(now) {
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

    /// A bus of devices that each hold their own tempo — enough for the tests that never ask about tempo.
    fn bus_of(filters: Vec<InputFilter>) -> BusState {
        let slots: Vec<Arc<crate::state::SharedState>> = filters
            .iter()
            .map(|_| crate::state::SharedState::new())
            .collect();
        bus_with(slots, filters)
    }

    /// A bus of devices that all hold `tempo`, the way a rack builds them: one cell, so a tempo message is what every
    /// engine reads and the router's handle on it is the devices' own.
    fn bus_on(filters: Vec<InputFilter>, tempo: &Arc<AtomicF32>) -> BusState {
        let slots: Vec<Arc<crate::state::SharedState>> = filters
            .iter()
            .map(|_| crate::state::SharedState::with_tempo(Arc::clone(tempo)))
            .collect();
        bus_with(slots, filters)
    }

    fn bus_with(slots: Vec<Arc<crate::state::SharedState>>, routes: Vec<InputFilter>) -> BusState {
        BusState { slots, routes }
    }

    fn gate(slot: &crate::state::SharedState) -> bool {
        slot.midi_gate.load(std::sync::atomic::Ordering::Relaxed)
    }

    #[test]
    fn a_note_reaches_every_device_whose_filter_names_its_channel() {
        let everything = InputFilter::everything();
        let bus = bus_of(vec![
            everything.clone(),
            InputFilter {
                channel: Channel::One(2),
                ..everything
            },
        ]);
        let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
        router.on_message(&bus, &[0x92, 60, 100]); // channel 2, note 60

        assert_eq!(
            bus.slots[0]
                .midi_velocity
                .load(std::sync::atomic::Ordering::Relaxed),
            100
        );
        assert_eq!(
            bus.slots[1]
                .midi_velocity
                .load(std::sync::atomic::Ordering::Relaxed),
            100
        );
        assert!((bus.slots[0].midi_hz.get() - bus.slots[1].midi_hz.get()).abs() < 1e-3);
    }

    #[test]
    fn a_channel_filter_keeps_a_note_off_its_neighbours() {
        let everything = InputFilter::everything();
        let bus = bus_of(vec![
            InputFilter {
                channel: Channel::One(1),
                ..everything.clone()
            },
            everything,
        ]);
        let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
        router.on_message(&bus, &[0x92, 60, 100]); // channel 2

        assert!(!gate(&bus.slots[0]), "device 1 is not on channel 2");
        assert!(gate(&bus.slots[1]));
    }

    #[test]
    fn a_controller_only_device_gets_its_knobs_and_not_its_notes() {
        let controller = InputFilter {
            role: Role::ControlOnly,
            ..InputFilter::everything()
        };
        let keys = InputFilter::everything();
        let bus = bus_of(vec![controller, keys]);
        let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));

        router.on_message(&bus, &[0x90, 60, 100]); // a note
        assert!(!gate(&bus.slots[0]), "a box of knobs must not start a note");
        assert!(gate(&bus.slots[1]), "the playing device hears it");

        // Volume is a dial with a pickup: the first value only says where the fader is, and the second crosses the knob,
        // which is the moment it engages. Two messages, one arrived fader.
        router.on_message(&bus, &[0xB0, 7, 0]);
        router.on_message(&bus, &[0xB0, 7, 127]);
        assert!(
            (bus.slots[0].volume.get() - 1.0).abs() < 1e-3,
            "the controller-only device's fader still arrives: {}",
            bus.slots[0].volume.get()
        );
    }

    #[test]
    fn a_port_that_is_not_named_reaches_nothing() {
        let bus = bus_of(vec![InputFilter {
            ports: Ports::Named(vec!["nanoKONTROL".into()]),
            ..InputFilter::everything()
        }]);
        let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
        router.on_message(&bus, &[0x90, 60, 100]);
        assert!(!gate(&bus.slots[0]));
    }

    #[test]
    fn one_press_is_one_step_however_many_devices_hear_it() {
        // The gate is the port's, not the device's: a device that hears the same note-on twice is the same press, and a
        // second step would be written for a key the player pressed once.
        let bus = bus_of(vec![InputFilter::everything()]);
        let mut router = Router::new("Launchkey MIDI", Arc::new(AtomicF32::new(130.0)));
        router.on_message(&bus, &[0x90, 60, 100]);
        let first = bus.slots[0]
            .midi_velocity
            .load(std::sync::atomic::Ordering::Relaxed);
        router.on_message(&bus, &[0x90, 60, 100]);
        assert_eq!(
            bus.slots[0]
                .midi_velocity
                .load(std::sync::atomic::Ordering::Relaxed),
            first
        );
    }

    #[test]
    fn a_clock_stream_writes_the_rack_tempo_cell() {
        let tempo = Arc::new(AtomicF32::new(130.0));
        let bus = bus_on(vec![InputFilter::everything()], &tempo);
        let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));
        // A steady 120 bpm stream, at the times the clock would have arrived: the follower believes a run of agreeing ticks
        // rather than one interval, so a test must give it the run.
        let step = std::time::Duration::from_secs_f64(60.0 / (120.0 * 24.0));
        let start = std::time::Instant::now();
        for tick in 0..12 {
            router.on_message_at(&bus, &[0xF8], start + step * tick);
        }

        let named = tempo.get();
        assert!(
            (named - 120.0).abs() < 12.0,
            "the rack's cell should be near 120 bpm, was {named}"
        );
        assert_eq!(
            bus.slots[0].tempo_bpm.get(),
            named,
            "and the device's tempo is that same cell"
        );
    }

    /// The message whose meaning is the rack's: the fader is delivered to a device, but the tempo cell it writes is the one
    /// every device already holds, so nothing has to carry it anywhere afterwards.
    #[test]
    fn a_tempo_fader_moves_the_rack_not_one_device() {
        let tempo = Arc::new(AtomicF32::new(130.0));
        let bus = bus_on(vec![InputFilter::everything()], &tempo);
        let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));

        // CC 72 is the factory tempo controller, and it takes pickup: the fader is swept up to the knob before it may
        // turn it, so the first message only says where the hand is.
        router.on_message(&bus, &[0xB0, 72, 0]);
        assert_eq!(tempo.get(), 130.0, "a parked fader leaves the tempo alone");
        router.on_message(&bus, &[0xB0, 72, 127]);

        assert_eq!(
            tempo.get(),
            acid_core::timing::MAX_BPM,
            "the rack's cell moved"
        );
    }

    /// Two devices hear one fader: both write the same cell, so they agree by construction rather than by a fan-out.
    #[test]
    fn two_devices_hearing_one_tempo_fader_agree_on_the_tempo() {
        let tempo = Arc::new(AtomicF32::new(130.0));
        let bus = bus_on(
            vec![InputFilter::everything(), InputFilter::everything()],
            &tempo,
        );
        let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));

        router.on_message(&bus, &[0xB0, 72, 0]);
        router.on_message(&bus, &[0xB0, 72, 127]);

        assert_eq!(tempo.get(), acid_core::timing::MAX_BPM);
        for slot in &bus.slots {
            assert_eq!(
                slot.tempo_bpm.get(),
                acid_core::timing::MAX_BPM,
                "the device's tempo is the cell the fader wrote"
            );
        }
    }

    /// The tempo write is not a CC-72 special case: the target comes from the learn map, so a tempo learned onto a different
    /// controller writes the same cell.
    #[test]
    fn a_tempo_learned_onto_another_controller_still_reaches_the_rack() {
        use crate::midi::cc::{CcTarget, set_cc};

        let tempo = Arc::new(AtomicF32::new(130.0));
        let bus = bus_on(vec![InputFilter::everything()], &tempo);
        let mut router = Router::new("Launchkey MIDI", Arc::clone(&tempo));
        set_cc(&bus.slots[0], CcTarget::Tempo, 22);

        router.on_message(&bus, &[0xB0, 22, 0]);
        router.on_message(&bus, &[0xB0, 22, 127]);

        assert_eq!(tempo.get(), acid_core::timing::MAX_BPM);
    }

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
