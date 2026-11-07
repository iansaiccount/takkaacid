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
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MidiEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    ControlChange { cc: u8, value: u8 },
    Clock,
    Start,
    Stop,
    Continue,
}

/// Decode one MIDI message. Anything that is not a channel message we act on, or one of the four transport
/// bytes, is `None` — including running status, sysex and the other real-time bytes.
pub fn parse(bytes: &[u8]) -> Option<MidiEvent> {
    match *bytes {
        [0xF8] => Some(MidiEvent::Clock),
        [0xFA] => Some(MidiEvent::Start),
        [0xFB] => Some(MidiEvent::Continue),
        [0xFC] => Some(MidiEvent::Stop),
        [status, note, velocity] if status & 0xF0 == 0x90 => {
            Some(MidiEvent::NoteOn { note, velocity })
        }
        [status, note, _] if status & 0xF0 == 0x80 => Some(MidiEvent::NoteOff { note }),
        [status, cc, value] if status & 0xF0 == 0xB0 => {
            Some(MidiEvent::ControlChange { cc, value })
        }
        _ => None,
    }
}

/// A hard hit: velocity at or above [`ACCENT_VELOCITY`].
pub fn is_accent(velocity: u8) -> bool {
    velocity >= ACCENT_VELOCITY
}

/// The frequency a keyboard note plays, an octave below its concert pitch so the panel's home register and
/// the keyboard's middle C agree.
pub fn note_hz(note: u8) -> f32 {
    midi_to_hz(f32::from(note) - 12.0)
}

/// Apply an event to the shared state. Returns true when something changed, so callers can skip work.
///
/// Only atomics are written: this runs on the MIDI thread. Clock, transport and continuous controllers are
/// other tasks' business for now, so they report "no change".
pub fn handle(event: MidiEvent, state: &SharedState) -> bool {
    match event {
        // A note-on with zero velocity is the running-status way of saying note off.
        MidiEvent::NoteOn { velocity: 0, .. } | MidiEvent::NoteOff { .. } => {
            state.midi_gate.store(false, Ordering::Relaxed);
            true
        }
        MidiEvent::NoteOn { note, velocity } => {
            state.midi_hz.set(note_hz(note));
            state
                .midi_accent
                .store(is_accent(velocity), Ordering::Relaxed);
            state.midi_gate.store(true, Ordering::Relaxed);
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
    fn note_on_and_off_parse_on_any_channel() {
        assert_eq!(
            parse(&[0x90, 60, 100]),
            Some(MidiEvent::NoteOn {
                note: 60,
                velocity: 100
            })
        );
        assert_eq!(
            parse(&[0x9F, 36, 1]),
            Some(MidiEvent::NoteOn {
                note: 36,
                velocity: 1
            })
        );
        assert_eq!(parse(&[0x80, 60, 0]), Some(MidiEvent::NoteOff { note: 60 }));
        assert_eq!(
            parse(&[0x8A, 61, 64]),
            Some(MidiEvent::NoteOff { note: 61 })
        );
    }

    #[test]
    fn control_clock_and_transport_parse() {
        assert_eq!(
            parse(&[0xB0, 74, 127]),
            Some(MidiEvent::ControlChange { cc: 74, value: 127 })
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
        assert!((note_hz(69) - 220.0).abs() < 0.5, "A4 should play as A3");
    }

    #[test]
    fn a_note_event_reaches_the_shared_state() {
        let s = SharedState::new();
        assert!(handle(
            MidiEvent::NoteOn {
                note: 45,
                velocity: 120
            },
            &s
        ));
        assert!(s.midi_gate.load(Ordering::Relaxed));
        assert!(s.midi_accent.load(Ordering::Relaxed));
        assert!((s.midi_hz.get() - note_hz(45)).abs() < 0.01);
        assert!(handle(MidiEvent::NoteOff { note: 45 }, &s));
        assert!(!s.midi_gate.load(Ordering::Relaxed));
    }

    #[test]
    fn note_on_with_zero_velocity_is_a_note_off() {
        let s = SharedState::new();
        handle(
            MidiEvent::NoteOn {
                note: 50,
                velocity: 100,
            },
            &s,
        );
        assert!(handle(
            MidiEvent::NoteOn {
                note: 50,
                velocity: 0
            },
            &s
        ));
        assert!(!s.midi_gate.load(Ordering::Relaxed));
    }
}
