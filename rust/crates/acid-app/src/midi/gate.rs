//! One note-on per press.
//!
//! A keyboard sends a note-on, then a note-off, then the next note-on. A second note-on for a note that is still down is
//! therefore not a new note — it is the same one delivered twice (a doubled connection, or a device that echoes). Left
//! alone it writes two steps for one key press, which is exactly what happened: the app held two open connections and
//! every note arrived twice.
//!
//! The duplicate refusal lives here, as plain data, because it is the kind of rule that is invisible until it is wrong.

use super::message::MidiEvent;

/// Which notes are currently down.
#[derive(Clone, Debug)]
pub struct NoteGate {
    down: [bool; 128],
}

impl Default for NoteGate {
    fn default() -> Self {
        Self::new()
    }
}

impl NoteGate {
    pub fn new() -> Self {
        Self { down: [false; 128] }
    }

    /// True when this note-on is a *new* note. A note already down is a duplicate.
    pub fn note_on(&mut self, note: u8) -> bool {
        let slot = &mut self.down[usize::from(note) % 128];
        if *slot {
            return false;
        }
        *slot = true;
        true
    }

    /// The note has ended, so the next note-on for it is new again.
    pub fn note_off(&mut self, note: u8) {
        self.down[usize::from(note) % 128] = false;
    }

    /// True while the note is down.
    pub fn is_down(&self, note: u8) -> bool {
        self.down[usize::from(note) % 128]
    }
}

/// Whether an event should reach the shared state.
///
/// Note bookkeeping always passes (the gate must see every end), and everything that is not a note always passes. A
/// duplicate note-on is refused and traced, so a run with `TAKKA_TRACE_INPUT=1` shows it.
pub fn passes(gate: &mut NoteGate, event: MidiEvent) -> bool {
    match event {
        MidiEvent::NoteOn { velocity: 0, note } => {
            gate.note_off(note);
            true
        }
        MidiEvent::NoteOff { note } => {
            gate.note_off(note);
            true
        }
        MidiEvent::NoteOn { note, velocity } => {
            if gate.note_on(note) {
                true
            } else {
                crate::trace::note("midi in", || {
                    format!("note {note} velocity {velocity} ignored: already down")
                });
                false
            }
        }
        MidiEvent::ControlChange { .. }
        | MidiEvent::Clock
        | MidiEvent::Start
        | MidiEvent::Stop
        | MidiEvent::Continue => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(note: u8) -> MidiEvent {
        MidiEvent::NoteOn {
            note,
            velocity: 100,
        }
    }

    #[test]
    fn a_note_that_is_already_down_is_not_a_second_note() {
        let mut gate = NoteGate::new();
        assert!(passes(&mut gate, note_on(64)), "the first press");
        assert!(
            !passes(&mut gate, note_on(64)),
            "the same note again, with no note-off between, is a duplicate"
        );
        assert!(!passes(&mut gate, note_on(64)), "and every further copy");

        assert!(passes(&mut gate, MidiEvent::NoteOff { note: 64 }));
        assert!(
            passes(&mut gate, note_on(64)),
            "released, so it is new again"
        );
    }

    #[test]
    fn a_note_on_with_no_velocity_is_a_release() {
        let mut gate = NoteGate::new();
        assert!(passes(&mut gate, note_on(64)));
        assert!(passes(
            &mut gate,
            MidiEvent::NoteOn {
                note: 64,
                velocity: 0
            }
        ));
        assert!(
            passes(&mut gate, note_on(64)),
            "the running-status note-off"
        );
    }

    #[test]
    fn different_notes_do_not_block_each_other() {
        let mut gate = NoteGate::new();
        for note in [60u8, 64, 67] {
            assert!(passes(&mut gate, note_on(note)), "note {note}");
            assert!(gate.is_down(note));
        }
        assert!(gate.is_down(60) && gate.is_down(64) && gate.is_down(67));
    }

    #[test]
    fn everything_that_is_not_a_note_passes_straight_through() {
        // Clock, transport and controllers must never be swallowed: the tempo follower needs every clock.
        let mut gate = NoteGate::new();
        for event in [
            MidiEvent::Clock,
            MidiEvent::Start,
            MidiEvent::Stop,
            MidiEvent::Continue,
            MidiEvent::ControlChange { cc: 74, value: 64 },
        ] {
            assert!(passes(&mut gate, event), "{event:?}");
        }
    }
}
