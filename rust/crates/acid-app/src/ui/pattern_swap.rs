//! When a pattern a controller asked for may take effect.
//!
//! The MIDI thread may not move the bank, so a learned pattern controller leaves an absolute request and `main` holds it.
//! A pattern is a bar long: applying the request the moment it arrives restarts the bar on every message, so cycling
//! through the patterns with a knob would chop the groove instead of riding it. The swap therefore waits for the
//! transport to come round.

use acid_core::bank::PatternBank;

use crate::state::SharedState;

use super::PanelState;

/// Whether a held pattern request may be applied this frame.
///
/// `position` and `last_position` are the transport's step, `0..16`, as published for the panel's LEDs. The swap has to
/// land *before* the next bar's first note, and the panel only sees a step after the audio thread has already triggered
/// it: step 0 is published after the old pattern played it. So the last step is the due one — swapping there means the
/// trigger that follows is the new pattern's first. A step that went backwards also counts, because a fast bar can be
/// crossed between two frames. Stopped, there is no bar to wait for.
pub fn swap_now(playing: bool, position: usize, last_position: usize, steps: usize) -> bool {
    !playing || position + 1 >= steps.max(1) || position < last_position
}

/// Adopt the bank's current pattern into the panel and the engine.
///
/// One path for a keyboard move, a learned controller, a song load and a device switch: the panel's copy, the display, the
/// engine's index and an armed CLEAR all belong to "which pattern is on screen", and letting four call sites each remember
/// that is how they drift apart.
///
/// The step the hand is on is *not* one of those things: this leaves `selected` alone, because a device switch must not
/// throw away the step its editor was on before the swap that puts it back (`draw_panel`'s `save_edit`/`load_edit`) has
/// run. A real pattern change is [`adopt`], which starts at the first step.
pub fn follow(panel: &mut PanelState, shared: &SharedState, bank: &PatternBank) {
    let index = bank.current();
    panel.pattern = shared.pattern();
    panel.pattern_id = PatternBank::id_of(index);
    shared.set_pattern_index(index);
    shared.clear_pattern_selected();
    super::clear::disarm(panel);
}

/// [`follow`] and start at the first step: what a *pattern* change does, never a device switch.
pub fn adopt(panel: &mut PanelState, shared: &SharedState, bank: &PatternBank) {
    follow(panel, shared, bank);
    panel.selected = 0;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_swap_waits_for_the_bar() {
        // Stopped: nothing is playing, so there is nothing to interrupt.
        assert!(swap_now(false, 7, 7, 16));
        // The bar has come round.
        assert!(
            swap_now(true, 15, 14, 16),
            "the last step: the next trigger is the new bar"
        );
        assert!(swap_now(true, 0, 15, 16));
        // ... or a short bar was crossed between two frames, so the step went backwards.
        assert!(swap_now(true, 2, 9, 16));
        // Mid-bar: the request is held.
        assert!(!swap_now(true, 5, 4, 16));
        assert!(!swap_now(true, 6, 5, 16));
    }
}
