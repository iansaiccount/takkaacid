//! Turning MIDI bytes into events, and events into shared state.
//!
//! Parsing is total: any byte sequence yields an event or `None`, never a panic. Handling only ever writes
//! atomics, because it runs on the MIDI thread.

use std::sync::atomic::Ordering;

use acid_core::pitch::midi_to_hz;

use crate::state::SharedState;

/// Velocity at or above this counts as an accent.
pub const ACCENT_VELOCITY: u8 = 100;

/// What an incoming MIDI message means.
///
/// The channel travels with every channel message, because routing is per device: a device may be told to listen to one
/// channel and not the others, so the channel is part of what the message *is*, not something the reader can throw away.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    ControlChange { channel: u8, cc: u8, value: u8 },
    Clock,
    Start,
    Stop,
    Continue,
}

impl MidiEvent {
    /// The channel this arrived on, `0..=15`, or `None` for the four real-time bytes.
    pub fn channel(&self) -> Option<u8> {
        match *self {
            MidiEvent::NoteOn { channel, .. }
            | MidiEvent::NoteOff { channel, .. }
            | MidiEvent::ControlChange { channel, .. } => Some(channel),
            MidiEvent::Clock | MidiEvent::Start | MidiEvent::Stop | MidiEvent::Continue => None,
        }
    }

    /// Whether this message can start or stop a note. The routing asks this, because a device may be told to answer
    /// controllers and not to play.
    pub fn carries_notes(&self) -> bool {
        match *self {
            MidiEvent::NoteOn { .. } | MidiEvent::NoteOff { .. } => true,
            MidiEvent::ControlChange { .. }
            | MidiEvent::Clock
            | MidiEvent::Start
            | MidiEvent::Stop
            | MidiEvent::Continue => false,
        }
    }
}

/// Decode one MIDI message. Anything that is not a channel message we act on, or one of the four transport
/// bytes, is `None` — including running status, sysex and the other real-time bytes.
pub fn parse(bytes: &[u8]) -> Option<MidiEvent> {
    match *bytes {
        [0xF8] => Some(MidiEvent::Clock),
        [0xFA] => Some(MidiEvent::Start),
        [0xFB] => Some(MidiEvent::Continue),
        [0xFC] => Some(MidiEvent::Stop),
        [status, note, velocity] if status & 0xF0 == 0x90 => Some(MidiEvent::NoteOn {
            channel: status & 0x0F,
            note,
            velocity,
        }),
        [status, note, _] if status & 0xF0 == 0x80 => Some(MidiEvent::NoteOff {
            channel: status & 0x0F,
            note,
        }),
        [status, cc, value] if status & 0xF0 == 0xB0 => Some(MidiEvent::ControlChange {
            channel: status & 0x0F,
            cc,
            value,
        }),
        _ => None,
    }
}

/// A hard hit: velocity at or above [`ACCENT_VELOCITY`].
pub fn is_accent(velocity: u8) -> bool {
    velocity >= ACCENT_VELOCITY
}

/// The frequency a keyboard note plays.
///
/// Exactly the note's own frequency, so the audition and the step it writes agree: the panel stores a note as the pair
/// whose frequency is the same, so playing a key and hearing it back from the sequencer must not differ by an octave.
/// (This used to transpose down by one, which made the audition disagree with the step it was about to write.)
pub fn note_hz(note: u8) -> f32 {
    midi_to_hz(f32::from(note))
}

/// The concert name of a MIDI note, `C4` for 60.
///
/// This is what a keyboard's own labels mean, so it is what a player compares against: the app's own names come from its
/// instrument register (`--midi-map` prints both side by side).
pub fn concert_name(note: u8) -> String {
    const NAMES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = i32::from(note) / 12 - 1;
    format!("{}{octave}", NAMES[usize::from(note) % 12])
}

/// Apply an event to one device's shared state. Returns true when something changed, so callers can skip work.
///
/// Only atomics are written: this runs on the MIDI thread. The channel is not this function's business — the routing has
/// already decided that this device is the one being addressed.
pub fn handle(event: MidiEvent, state: &SharedState) -> bool {
    match event {
        // A note-on with zero velocity is the running-status way of saying note off.
        MidiEvent::NoteOn { velocity: 0, .. } | MidiEvent::NoteOff { .. } => {
            state.midi_gate.store(false, Ordering::Relaxed);
            true
        }
        MidiEvent::NoteOn { note, velocity, .. } => {
            crate::trace::note("midi in", || format!("note {note} velocity {velocity}"));
            state.midi_hz.set(note_hz(note));
            state
                .midi_velocity
                .store(u32::from(velocity), Ordering::Relaxed);
            state
                .midi_accent
                .store(is_accent(velocity), Ordering::Relaxed);
            // `Release` here and `Acquire` on the read: the frequency and velocity written above must be visible before
            // the gate that says "play this note". Without it the audio thread can start the previous note's pitch.
            state.midi_gate.store(true, Ordering::Release);
            // The panel decides whether this is an audition or a note for the selected step (step mode, or shift
            // held): the MIDI thread only says that a note arrived, which one, and how hard it was struck.
            state.notes.push(note, velocity);
            true
        }
        MidiEvent::ControlChange { .. }
        | MidiEvent::Clock
        | MidiEvent::Start
        | MidiEvent::Stop
        | MidiEvent::Continue => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concert_names_are_the_ones_printed_on_a_keyboard() {
        assert_eq!(concert_name(60), "C4", "middle C");
        assert_eq!(concert_name(69), "A4", "the tuning reference");
        assert_eq!(concert_name(36), "C2");
        assert_eq!(concert_name(61), "C#4");
        assert_eq!(concert_name(72), "C5");
        assert_eq!(concert_name(0), "C-1");
    }

    #[test]
    fn a_message_carries_the_channel_it_arrived_on() {
        assert_eq!(
            parse(&[0x92, 60, 100]),
            Some(MidiEvent::NoteOn {
                channel: 2,
                note: 60,
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x8F, 36, 0]),
            Some(MidiEvent::NoteOff {
                channel: 15,
                note: 36
            })
        );
        assert_eq!(
            parse(&[0xB0, 74, 127]),
            Some(MidiEvent::ControlChange {
                channel: 0,
                cc: 74,
                value: 127
            })
        );
        assert_eq!(
            parse(&[0xF8]).and_then(|event| event.channel()),
            None,
            "clock has no channel"
        );
    }

    #[test]
    fn only_the_two_note_messages_can_carry_notes() {
        assert!(
            MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 1
            }
            .carries_notes()
        );
        assert!(
            MidiEvent::NoteOff {
                channel: 0,
                note: 60
            }
            .carries_notes()
        );
        assert!(
            !MidiEvent::ControlChange {
                channel: 0,
                cc: 1,
                value: 1
            }
            .carries_notes()
        );
        assert!(!MidiEvent::Clock.carries_notes());
    }

    #[test]
    fn note_on_and_off_parse_on_any_channel() {
        assert_eq!(
            parse(&[0x90, 60, 100]),
            Some(MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x9F, 36, 1]),
            Some(MidiEvent::NoteOn {
                channel: 15,
                note: 36,
                velocity: 1
            })
        );
        assert_eq!(
            parse(&[0x80, 60, 0]),
            Some(MidiEvent::NoteOff {
                channel: 0,
                note: 60
            })
        );
        assert_eq!(
            parse(&[0x8A, 61, 64]),
            Some(MidiEvent::NoteOff {
                channel: 10,
                note: 61
            })
        );
    }

    #[test]
    fn control_clock_and_transport_parse() {
        assert_eq!(
            parse(&[0xB0, 74, 127]),
            Some(MidiEvent::ControlChange {
                channel: 0,
                cc: 74,
                value: 127
            })
        );
        assert_eq!(parse(&[0xF8]), Some(MidiEvent::Clock));
        assert_eq!(parse(&[0xFA]), Some(MidiEvent::Start));
        assert_eq!(parse(&[0xFC]), Some(MidiEvent::Stop));
        assert_eq!(parse(&[0xFE]), None, "active sensing is not interesting");
    }

    #[test]
    fn garbage_never_panics() {
        for a in 0u8..=255 {
            for b in 0u8..=255 {
                let _ = parse(&[a]);
                let _ = parse(&[a, b]);
                let _ = parse(&[a, b, 64]);
                let _ = parse(&[a, 0x90, b, 64]);
            }
        }
    }

    #[test]
    fn velocity_accent_threshold_and_note_mapping() {
        assert!(!is_accent(99));
        assert!(is_accent(100));
        assert!(is_accent(127));
        assert!((note_hz(69) - 440.0).abs() < 0.5, "A4 plays A4");

        // What you hear is what gets written: inside the panel's own range, a note auditions at exactly the frequency of
        // the step it writes (outside it the octave clamps, which is the panel's 3-octave window, not a tone shift).
        // The panel carries three octaves, MIDI 21..=56; above that the octave folds, so the fold is asserted separately.
        for note in 21..=56u8 {
            let (pitch, octave) = crate::ui::edit::from_midi_note(note);
            let step = acid_core::pitch::step_hz(pitch, octave);
            assert!(
                (step - note_hz(note)).abs() < 0.5,
                "note {note} auditions at {} Hz but writes {} Hz",
                note_hz(note),
                step
            );
        }
    }

    #[test]
    fn a_note_event_reaches_the_shared_state() {
        let s = SharedState::new();
        assert!(handle(
            MidiEvent::NoteOn {
                channel: 3,
                note: 45,
                velocity: 120
            },
            &s
        ));
        assert!(s.midi_gate.load(Ordering::Relaxed));
        assert!(s.midi_accent.load(Ordering::Relaxed));
        assert!((s.midi_hz.get() - note_hz(45)).abs() < 0.01);
        assert!(handle(
            MidiEvent::NoteOff {
                channel: 3,
                note: 45
            },
            &s
        ));
        assert!(!s.midi_gate.load(Ordering::Relaxed));
    }

    #[test]
    fn the_channel_does_not_change_what_a_note_does_to_one_device() {
        let s = SharedState::new();
        // A device has already been told this message is for it, so the channel is not its business.
        assert!(handle(
            MidiEvent::NoteOn {
                channel: 15,
                note: 50,
                velocity: 100
            },
            &s
        ));
        assert!(s.midi_gate.load(Ordering::Relaxed));
    }

    #[test]
    fn note_on_with_zero_velocity_is_a_note_off() {
        let s = SharedState::new();
        handle(
            MidiEvent::NoteOn {
                channel: 0,
                note: 50,
                velocity: 100,
            },
            &s,
        );
        assert!(handle(
            MidiEvent::NoteOn {
                channel: 0,
                note: 50,
                velocity: 0
            },
            &s
        ));
        assert!(!s.midi_gate.load(Ordering::Relaxed));
    }
}
