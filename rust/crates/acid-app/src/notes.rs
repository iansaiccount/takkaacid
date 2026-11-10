//! The hand-off for MIDI notes the panel has not placed yet.
//!
//! One slot plus a counter was enough to place one note, but not two: a second note in the same frame overwrote the
//! first before the panel ever saw it, and both steps took the later note's pitch and velocity. This is a small
//! single-producer/single-consumer ring instead — the MIDI thread only ever writes, the panel only ever reads, and
//! every note waits its turn.

use std::sync::atomic::{AtomicU32, Ordering};

/// How many notes can wait. Two would cover a frame; a phrase's worth costs 128 bytes and never drops anything a hand
/// can play inside one frame.
pub const NOTE_CAPACITY: usize = 32;

/// One waiting note: its number, and the velocity it was struck with.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WaitingNote {
    pub note: u8,
    pub velocity: u8,
}

/// A ring of waiting notes. Lock-free and allocation-free, so the MIDI thread may push from anywhere.
///
/// The indices are free-running counters masked into the ring: `NOTE_CAPACITY` divides 2^32, so the mask stays right
/// across a wrap and a wrap costs nothing to handle.
pub struct NoteQueue {
    slots: [AtomicU32; NOTE_CAPACITY],
    /// Where the MIDI thread writes. Only it stores this.
    head: AtomicU32,
    /// Where the panel reads. Only it stores this.
    tail: AtomicU32,
}

impl NoteQueue {
    pub fn new() -> Self {
        Self {
            // Not a `const` array of atomics: clippy is right that a named constant with interior mutability invites
            // the reader to believe it is immutable. A fresh array per queue is four bytes a slot.
            slots: std::array::from_fn(|_| AtomicU32::new(0)),
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
        }
    }

    /// Publish a note for the panel. `false` means the queue was full and the note was dropped: a full queue must
    /// never stall the MIDI thread, and a dropped note is better than a held-up one.
    pub fn push(&self, note: u8, velocity: u8) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);
        if head.wrapping_sub(tail) as usize >= NOTE_CAPACITY {
            return false;
        }
        self.slot(head)
            .store(pack(note, velocity), Ordering::Relaxed);
        self.head.store(head.wrapping_add(1), Ordering::Release);
        true
    }

    /// Take the oldest waiting note, or `None` once the panel has caught up.
    pub fn pop(&self) -> Option<WaitingNote> {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);
        if tail == head {
            return None;
        }
        let packed = self.slot(tail).load(Ordering::Relaxed);
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
        Some(unpack(packed))
    }

    /// How many notes are waiting.
    pub fn len(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        head.wrapping_sub(tail) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn slot(&self, index: u32) -> &AtomicU32 {
        &self.slots[(index as usize) % NOTE_CAPACITY]
    }
}

impl std::fmt::Debug for NoteQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoteQueue({} waiting)", self.len())
    }
}

impl Default for NoteQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// One word per note: velocity in the high byte, number in the low one.
fn pack(note: u8, velocity: u8) -> u32 {
    (u32::from(velocity) << 8) | u32::from(note)
}

fn unpack(packed: u32) -> WaitingNote {
    WaitingNote {
        note: (packed & 0xff) as u8,
        velocity: ((packed >> 8) & 0xff) as u8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_come_out_in_the_order_they_went_in_with_their_own_velocity() {
        let queue = NoteQueue::new();
        assert_eq!(queue.pop(), None, "an empty queue has nothing to give");

        assert!(queue.push(36, 110));
        assert!(queue.push(40, 20));
        assert_eq!(queue.len(), 2);

        assert_eq!(
            queue.pop(),
            Some(WaitingNote {
                note: 36,
                velocity: 110
            })
        );
        assert_eq!(
            queue.pop(),
            Some(WaitingNote {
                note: 40,
                velocity: 20
            })
        );
        assert_eq!(queue.pop(), None);
        assert!(queue.is_empty());
    }

    #[test]
    fn a_full_queue_drops_the_newest_note_instead_of_wrapping_onto_the_oldest() {
        let queue = NoteQueue::new();
        for index in 0..NOTE_CAPACITY {
            assert!(queue.push(index as u8, 100), "{index} should fit");
        }
        assert!(!queue.push(99, 100), "the queue is full");
        assert_eq!(queue.len(), NOTE_CAPACITY);

        // The notes that made it are untouched and still in order.
        for index in 0..NOTE_CAPACITY {
            assert_eq!(queue.pop().map(|w| w.note), Some(index as u8));
        }
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn the_ring_survives_more_traffic_than_it_has_slots() {
        // The indices are free-running counters, so most of the traffic is on the second lap and beyond; a masked
        // index that only happens to be right for the first lap would fail here.
        let queue = NoteQueue::new();
        for round in 0..10_000u32 {
            let note = (round % 128) as u8;
            assert!(queue.push(note, (round % 127) as u8 + 1));
            let waited = queue.pop().expect("what was just pushed");
            assert_eq!(waited.note, note, "round {round}");
        }
        assert!(queue.is_empty());
    }

    #[test]
    fn every_byte_survives_the_pack() {
        for note in [0u8, 1, 60, 127, 255] {
            for velocity in [0u8, 1, 64, 100, 127, 255] {
                assert_eq!(unpack(pack(note, velocity)), WaitingNote { note, velocity });
            }
        }
    }
}
