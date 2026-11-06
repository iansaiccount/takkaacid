//! Bounded retries for the audio stream, so a lost device never hangs or panics the app.
//!
//! Pure logic: the app drains stream errors into [`Supervisor::on_error`] and acts on the [`Action`].

use std::time::Duration;

/// How many reopen attempts before the app gives up and shows a failure.
pub const MAX_RETRIES: u32 = 5;

/// Delay between reopen attempts.
pub const RETRY_DELAY: Duration = Duration::from_millis(250);

/// What the app should do after an error or a success.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Action {
    /// Wait `RETRY_DELAY` and try again.
    Retry,
    /// Stop trying; report the failure.
    GiveUp,
}

/// Counts consecutive stream failures and decides when to stop.
#[derive(Debug, Default)]
pub struct Supervisor {
    attempts: u32,
}

impl Supervisor {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a stream error and get the next step.
    pub fn on_error(&mut self) -> Action {
        self.attempts += 1;
        if self.attempts > MAX_RETRIES {
            Action::GiveUp
        } else {
            Action::Retry
        }
    }

    /// A stream is running: forget past failures.
    pub fn on_success(&mut self) {
        self.attempts = 0;
    }

    pub fn attempts(&self) -> u32 {
        self.attempts
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_five_times_then_gives_up() {
        let mut s = Supervisor::new();
        for _ in 0..MAX_RETRIES {
            assert_eq!(s.on_error(), Action::Retry);
        }
        assert_eq!(s.on_error(), Action::GiveUp);
    }

    #[test]
    fn a_success_resets_the_counter() {
        let mut s = Supervisor::new();
        assert_eq!(s.on_error(), Action::Retry);
        s.on_success();
        assert_eq!(s.attempts(), 0);
        assert_eq!(s.on_error(), Action::Retry);
    }
}
