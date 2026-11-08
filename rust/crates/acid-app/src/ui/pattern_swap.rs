//! When a pattern a controller asked for may take effect.
//!
//! The MIDI thread may not move the bank, so a learned pattern controller leaves an absolute request and `main` holds it.
//! A pattern is a bar long: applying the request the moment it arrives restarts the bar on every message, so cycling
//! through the patterns with a knob would chop the groove instead of riding it. The swap therefore waits for the
//! transport to come round.

/// Whether a held pattern request may be applied this frame.
///
/// `position` and `last_position` are the transport's step, `0..16`, as published for the panel's LEDs. A request is due
/// at the start of a pattern, which is step 0 — or when the step number went backwards, because a very short bar can be
/// crossed between two frames. Stopped, there is no bar to wait for, so the request is due at once.
pub fn swap_now(playing: bool, position: usize, last_position: usize) -> bool {
    !playing || position == 0 || position < last_position
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_swap_waits_for_the_bar() {
        // Stopped: nothing is playing, so there is nothing to interrupt.
        assert!(swap_now(false, 7, 7));
        // The bar has come round.
        assert!(swap_now(true, 0, 15));
        // ... or a short bar was crossed between two frames, so the step went backwards.
        assert!(swap_now(true, 2, 9));
        // Mid-bar: the request is held.
        assert!(!swap_now(true, 5, 4));
        assert!(!swap_now(true, 15, 14));
    }
}
