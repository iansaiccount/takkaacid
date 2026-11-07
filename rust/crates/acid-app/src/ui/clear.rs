//! Clearing the pattern on screen: a destructive button with no undo, so it takes two clicks.
//!
//! The first click arms it and the button says so ("CLEAR?"); a second click inside the window empties the
//! pattern. Anything that edits the pattern disarms it, because a pending confirmation is about the pattern as
//! it was, not as it is now.

use std::time::{Duration, Instant};

use super::PanelState;

/// How long an armed CLEAR stays armed: long enough to be deliberate, short enough that a click a moment later
/// is a new decision rather than a confirmation of an old one.
pub const ARM_SECONDS: f32 = 3.0;

/// What a click on CLEAR does.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Click {
    /// The first click: the button is now armed.
    Armed,
    /// A second click inside the window: clear the pattern.
    Cleared,
}

/// Decide a click. Only a second click inside the window clears.
pub fn click(armed_at: Option<Instant>, now: Instant) -> Click {
    match armed_at {
        Some(at) if now.saturating_duration_since(at) <= Duration::from_secs_f32(ARM_SECONDS) => {
            Click::Cleared
        }
        _ => Click::Armed,
    }
}

/// Whether the button is showing its armed face right now.
pub fn is_armed(armed_at: Option<Instant>, now: Instant) -> bool {
    matches!(click(armed_at, now), Click::Cleared)
}

/// Anything that changes the pattern makes a pending CLEAR stale.
pub fn disarm(panel: &mut PanelState) {
    panel.clear_armed_at = None;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_click_arms_and_the_second_clears() {
        let now = Instant::now();
        assert_eq!(click(None, now), Click::Armed);
        assert!(!is_armed(None, now));
        assert_eq!(click(Some(now), now), Click::Cleared);
        assert!(is_armed(Some(now), now));
    }

    #[test]
    fn an_arm_from_a_while_ago_has_to_be_armed_again() {
        let now = Instant::now();
        let stale = now - Duration::from_secs_f32(ARM_SECONDS + 1.0);
        assert_eq!(click(Some(stale), now), Click::Armed);
        assert!(!is_armed(Some(stale), now));
    }

    #[test]
    fn disarming_forgets_the_pending_click() {
        let mut panel = PanelState {
            clear_armed_at: Some(Instant::now()),
            ..PanelState::default()
        };
        disarm(&mut panel);
        assert!(panel.clear_armed_at.is_none());
    }
}
