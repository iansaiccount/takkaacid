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

/// The pattern a malformed bank reads when it has no slots at all.
///
/// `normalise` guarantees this cannot happen for a file that went through `sanitize`, but `get` is on the audio path and
/// a panic there is worse than an empty pattern, so the fallback exists rather than an `unwrap`.
static EMPTY: std::sync::LazyLock<Pattern> = std::sync::LazyLock::new(Pattern::empty);

/// All 64 patterns plus which one is selected.
#[derive(Clone, Debug, PartialEq)]
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

    /// The bank the app ships with: bank `A` holds the factory phrase, `B`–`H` are empty.
    pub fn factory() -> Self {
        let mut bank = Self::new_empty();
        for (index, pattern) in crate::factory::factory_patterns().iter().enumerate() {
            bank.replace(index, *pattern);
        }
        bank
    }

    /// Repair a bank that does not hold what the box promises: exactly sixty-four slots and a selection that exists.
    ///
    /// A save file is JSON a human may edit, and a truncated one can hold fewer patterns than the catalogue. Every
    /// accessor assumes the full count, so the repair belongs here rather than in each reader — and it is idempotent, so
    /// running it on every load costs nothing.
    pub fn normalise(&mut self) {
        // Keep the selection on a pattern that exists: a file naming slot 5 while holding one pattern should open on the
        // one it has, not on an empty slot the player never wrote.
        let existing = self.patterns.len().saturating_sub(1);
        self.patterns.resize(TOTAL_PATTERNS, Pattern::empty());
        self.current = self.current.min(existing).min(TOTAL_PATTERNS - 1);
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

    /// How many patterns the bank actually holds. `normalise` makes this agree with the catalogue; a bank straight out
    /// of a hand-edited file may not, which is why the count is worth asking for.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
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
    ///
    /// The clamp is against the *stored* length as well as the catalogue, because a hand-edited save file can hold fewer
    /// than sixty-four patterns; `normalise` repairs that on load, and this keeps a malformed bank from panicking before
    /// anyone gets the chance.
    pub fn get(&self, index: usize) -> &Pattern {
        let last = self
            .patterns
            .len()
            .saturating_sub(1)
            .min(TOTAL_PATTERNS - 1);
        self.patterns.get(index.min(last)).unwrap_or(&EMPTY)
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

    /// Apply `f` to every pattern in the bank, keeping the selection.
    ///
    /// This is how a bank written before the lattice moved is brought forward: every pattern is the same music, spelled
    /// on the new lattice.
    pub fn map_patterns(&mut self, mut f: impl FnMut(&Pattern) -> Pattern) {
        for slot in &mut self.patterns {
            *slot = f(slot);
        }
    }
}
