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
    ToggleHelp,
}

/// Which shortcut a key press means. `shift` is only consulted for `?`.
pub fn shortcut_for(key: Key, shift: bool) -> Option<Shortcut> {
    match key {
        Key::Space => Some(Shortcut::PlayStop),
        Key::ArrowUp => Some(Shortcut::TempoUp),
        Key::ArrowDown => Some(Shortcut::TempoDown),
        Key::ArrowRight => Some(Shortcut::NextPattern),
        Key::ArrowLeft => Some(Shortcut::PrevPattern),
        Key::F1 => Some(Shortcut::ToggleHelp),
        Key::Questionmark if shift => Some(Shortcut::ToggleHelp),
        _ => None,
    }
}

/// Carry out a shortcut. Returns true when shared state changed.
///
/// Pattern moves push the new pattern into `SharedState`, exactly as the panel's pattern switch does.
/// `ToggleHelp` changes no shared state — the panel owns whether the card is showing — so it reports false.
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
            bank.select((bank.current() + 1) % TOTAL_PATTERNS);
            state.set_pattern(Arc::new(*bank.current_pattern()));
            true
        }
        Shortcut::PrevPattern => {
            bank.select((bank.current() + TOTAL_PATTERNS - 1) % TOTAL_PATTERNS);
            state.set_pattern(Arc::new(*bank.current_pattern()));
            true
        }
        Shortcut::ToggleHelp => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_toggles_the_transport() {
        let s = SharedState::new();
        let mut bank = PatternBank::factory();
        assert_eq!(
            shortcut_for(Key::Space, false),
            Some(Shortcut::PlayStop)
        );
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
    fn help_is_reachable_by_f1_and_by_question_mark() {
        assert_eq!(shortcut_for(Key::F1, false), Some(Shortcut::ToggleHelp));
        assert_eq!(
            shortcut_for(Key::Questionmark, true),
            Some(Shortcut::ToggleHelp)
        );
        assert_eq!(shortcut_for(Key::B, false), None);
    }
}
