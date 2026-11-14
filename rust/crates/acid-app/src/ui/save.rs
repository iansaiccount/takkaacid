//! What the panel says about the save file: one word, and the colour that makes it read at a glance.

use eframe::egui::Color32;

use super::theme;

/// The indicator's word and colour. Unsaved is the one that has to catch the eye, so it takes `led_accent` — the palette's
/// loud colour, chosen to outshout the dim legend (`silkscreen_dim`) that SAVED is written in, in both schemes.
pub fn state_label(dirty: bool, p: &theme::Palette) -> (&'static str, Color32) {
    if dirty {
        ("UNSAVED", p.led_accent)
    } else {
        ("SAVED", p.silkscreen_dim)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_says_unsaved_only_while_something_is_dirty() {
        assert_eq!(state_label(true, &theme::BLACK).0, "UNSAVED");
        assert_eq!(state_label(false, &theme::BLACK).0, "SAVED");
        assert_ne!(
            state_label(true, &theme::BLACK).1,
            state_label(false, &theme::BLACK).1,
            "the two must not look alike"
        );
    }
}
