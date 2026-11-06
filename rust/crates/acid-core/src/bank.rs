//! The 64-pattern bank: eight banks of eight patterns, addressed like the hardware's `A1`..`H8`.
//!
//! Selection is separate from the pattern data, so switching while the sequencer runs only swaps which
//! `Pattern` the next step boundary reads — position, slide history and the clock are untouched.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::pattern::{Pattern, PatternId};

/// Banks in the box: `A`..`H`.
pub const BANKS: usize = 8;
/// Patterns in one bank.
pub const PATTERNS_PER_BANK: usize = 8;
/// Every pattern the bank holds.
pub const TOTAL_PATTERNS: usize = BANKS * PATTERNS_PER_BANK;

/// All 64 patterns plus which one is selected.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct PatternBank {
    patterns: Vec<Pattern>,
    current: usize,
}

impl PatternBank {
    /// Sixty-four empty patterns, the first one selected.
    pub fn new_empty() -> Self {
        Self {
            patterns: vec![Pattern::empty(); TOTAL_PATTERNS],
            current: 0,
        }
    }

    /// The hardware's name for a slot: `0 -> A1`, `63 -> H8`. Out-of-range indices wrap.
    pub fn id_of(index: usize) -> PatternId {
        let index = index % TOTAL_PATTERNS;
        PatternId {
            bank: (index / PATTERNS_PER_BANK) as u8,
            number: (index % PATTERNS_PER_BANK) as u8,
        }
    }

    /// The flat slot for a bank/pattern pair.
    pub fn index_of(id: PatternId) -> usize {
        id.bank as usize * PATTERNS_PER_BANK + id.number as usize
    }

    pub fn current(&self) -> usize {
        self.current
    }

    /// Select a pattern, clamped to the bank: a stray index lands on the last pattern rather than
    /// leaving the box with no pattern at all.
    pub fn select(&mut self, index: usize) {
        self.current = index.min(TOTAL_PATTERNS - 1);
    }

    /// Read a pattern. Like `select`, an out-of-range index reads the last slot instead of panicking —
    /// this is reachable from the UI and must never take the audio thread down.
    pub fn get(&self, index: usize) -> &Pattern {
        &self.patterns[index.min(TOTAL_PATTERNS - 1)]
    }

    pub fn current_pattern(&self) -> &Pattern {
        self.get(self.current)
    }

    /// Overwrite one slot. Out-of-range indices are ignored.
    pub fn replace(&mut self, index: usize, pattern: Pattern) {
        if let Some(slot) = self.patterns.get_mut(index) {
            *slot = pattern;
        }
    }
}
