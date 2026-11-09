//! Keyboard shortcuts: transport, tempo and pattern stepping, plus the way into the help card.
//!
//! Every shortcut is a pure mapping plus a function that changes shared state, so what a key does is testable
//! without a window.

use std::sync::Arc;

use acid_core::bank::{PatternBank, TOTAL_PATTERNS};
use acid_core::timing::clamp_bpm;
use eframe::egui::Key;

use crate::state::SharedState;

/// A keyboard shortcut the panel understands.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Shortcut {
    PlayStop,
    TempoUp,
    TempoDown,
    NextPattern,
    PrevPattern,
    ToggleSettings,
    ToggleHelp,
}

/// Which shortcut a key press means. Shift picks the pattern out of the arrows; the tempo nudges stay unshifted,
/// because shift+up/down belongs to the note editor.
pub fn shortcut_for(key: Key, shift: bool) -> Option<Shortcut> {
    match (key, shift) {
        (Key::Space, _) => Some(Shortcut::PlayStop),
        (Key::ArrowUp, false) => Some(Shortcut::TempoUp),
        (Key::ArrowDown, false) => Some(Shortcut::TempoDown),
        (Key::ArrowRight, true) => Some(Shortcut::NextPattern),
        (Key::ArrowLeft, true) => Some(Shortcut::PrevPattern),
        (Key::F1, _) => Some(Shortcut::ToggleHelp),
        (Key::F2, _) => Some(Shortcut::ToggleSettings),
        (Key::Questionmark, true) => Some(Shortcut::ToggleHelp),
        _ => None,
    }
}

/// Copy the live pattern back into the slot it came from. `SharedState` holds the pattern being edited and the
/// bank holds the saved copies, so without this a pattern loses every edit the moment the selection moves.
fn stash_current(state: &SharedState, bank: &mut PatternBank) {
    let current = bank.current();
    bank.replace(current, *state.pattern());
}

/// Carry out a shortcut. Returns true when shared state changed.
///
/// Pattern moves push the new pattern into `SharedState`, exactly as the panel's pattern switch does.
/// `ToggleHelp` changes no shared state — the panel owns whether the card is showing — so it reports false.
/// Move the bank to `index` and hand the engine that pattern.
///
/// One path for the keyboard, the panel and a learned controller: the slot being edited is copied back into the bank
/// first, so a controller cannot leave the bank and the engine disagreeing about what is playing.
pub fn select_pattern(state: &SharedState, bank: &mut PatternBank, index: usize) {
    stash_current(state, bank);
    bank.select(index % TOTAL_PATTERNS);
    state.set_pattern(Arc::new(*bank.current_pattern()));
    state.set_pattern_index(bank.current());
}

pub fn apply(shortcut: Shortcut, state: &SharedState, bank: &mut PatternBank) -> bool {
    match shortcut {
        Shortcut::PlayStop => {
            state.set_playing(!state.is_playing());
            true
        }
        Shortcut::TempoUp => {
            state.tempo_bpm.set(clamp_bpm(state.tempo_bpm.get() + 1.0));
            true
        }
        Shortcut::TempoDown => {
            state.tempo_bpm.set(clamp_bpm(state.tempo_bpm.get() - 1.0));
            true
        }
        Shortcut::NextPattern => {
            select_pattern(state, bank, (bank.current() + 1) % TOTAL_PATTERNS);
            true
        }
        Shortcut::PrevPattern => {
            select_pattern(
                state,
                bank,
                (bank.current() + TOTAL_PATTERNS - 1) % TOTAL_PATTERNS,
            );
            true
        }
        // Both cards are panel state, so `handle` toggles them; nothing here touches the shared state.
        Shortcut::ToggleHelp | Shortcut::ToggleSettings => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::pattern::Step;
    use acid_core::pitch::Pitch;

    #[test]
    fn space_toggles_the_transport() {
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        assert_eq!(shortcut_for(Key::Space, false), Some(Shortcut::PlayStop));
        apply(Shortcut::PlayStop, &s, &mut bank);
        assert!(s.playing.load(std::sync::atomic::Ordering::Relaxed));
        apply(Shortcut::PlayStop, &s, &mut bank);
        assert!(!s.playing.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn arrows_move_tempo_in_whole_bpm_and_clamp() {
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        s.tempo_bpm.set(130.0);
        apply(Shortcut::TempoUp, &s, &mut bank);
        assert_eq!(s.tempo_bpm.get(), 131.0);
        s.tempo_bpm.set(acid_core::timing::MAX_BPM);
        apply(Shortcut::TempoUp, &s, &mut bank);
        assert_eq!(s.tempo_bpm.get(), acid_core::timing::MAX_BPM);
    }

    #[test]
    fn pattern_selection_wraps_at_both_ends() {
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        bank.select(0);
        apply(Shortcut::PrevPattern, &s, &mut bank);
        assert_eq!(bank.current(), 63);
        apply(Shortcut::NextPattern, &s, &mut bank);
        assert_eq!(bank.current(), 0);
    }

    #[test]
    fn switching_away_and_back_keeps_the_edits_you_made() {
        // The bank holds the saved copies; `SharedState` holds the live pattern. Moving the selection used to
        // re-publish the stale bank copy, so the pattern lost every edit on the way out.
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        bank.select(0);
        s.set_pattern(Arc::new(*bank.current_pattern()));
        s.set_step(0, Step::note(Pitch::E, 1));

        apply(Shortcut::NextPattern, &s, &mut bank);
        apply(Shortcut::PrevPattern, &s, &mut bank);

        let back = s.pattern();
        assert_eq!(
            (back.steps[0].pitch, back.steps[0].octave_offset),
            (Pitch::E, 1)
        );
    }

    #[test]
    fn a_pattern_you_switch_to_is_its_own_and_not_the_one_you_left() {
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        bank.select(0);
        s.set_pattern(Arc::new(*bank.current_pattern()));
        let untouched = *bank.get(1);
        s.set_step(0, Step::note(Pitch::E, 1));

        apply(Shortcut::NextPattern, &s, &mut bank);

        assert_eq!(
            *s.pattern(),
            untouched,
            "the edit must not leak into the pattern you moved to"
        );
        assert_eq!(
            bank.get(0).steps[0].pitch,
            Pitch::E,
            "and must be kept in the pattern you left"
        );
    }

    #[test]
    fn help_is_reachable_by_f1_and_by_question_mark() {
        assert_eq!(shortcut_for(Key::F1, false), Some(Shortcut::ToggleHelp));
        assert_eq!(
            shortcut_for(Key::Questionmark, true),
            Some(Shortcut::ToggleHelp)
        );
        assert_eq!(shortcut_for(Key::B, false), None);
    }
}
