//! What the panel says about the save file: one word, and the colour that makes it read at a glance.

use eframe::egui::Color32;

use super::theme;

/// The indicator's word and colour. Unsaved is the one that has to catch the eye, so it takes the amber.
pub fn state_label(dirty: bool) -> (&'static str, Color32) {
    if dirty {
        ("UNSAVED", theme::LED_ACCENT)
    } else {
        ("SAVED", theme::SILKSCREEN_DIM)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_says_unsaved_only_while_something_is_dirty() {
        assert_eq!(state_label(true).0, "UNSAVED");
        assert_eq!(state_label(false).0, "SAVED");
        assert_ne!(
            state_label(true).1,
            state_label(false).1,
            "the two must not look alike"
        );
    }
}
