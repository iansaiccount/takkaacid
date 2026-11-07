//! Following an external 24-ppqn MIDI clock.
//!
//! The follower only believes a run of ticks that agree with each other: one stray tick moves nothing, and a
//! tempo is reported only once the stream has settled.

use std::time::Instant;

use acid_core::timing::clamp_bpm;

use crate::state::SharedState;

/// MIDI clock ticks per quarter note.
pub const PPQN: u32 = 24;

/// Shortest and longest tick interval worth believing: 2 ms is 1250 BPM, 200 ms is 12.5 BPM at 24 ppqn.
const MIN_INTERVAL_MS: f32 = 2.0;
const MAX_INTERVAL_MS: f32 = 200.0;
/// Ticks that must agree before a tempo is reported.
const CONFIDENCE: u32 = 8;
/// How close two ticks must be to count as agreeing, as a fraction of the current tempo.
const AGREEMENT: f32 = 0.05;
/// EMA weight for a newly accepted tempo.
const ALPHA: f32 = 0.2;

/// The tempo a tick interval implies, or `None` when the interval is outside the believable range.
pub fn bpm_from_interval(interval_ms: f32) -> Option<f32> {
    if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&interval_ms) {
        return None;
    }
    Some(60_000.0 / (interval_ms * PPQN as f32))
}

/// Watches the tick stream and reports a tempo once it has settled.
#[derive(Debug)]
pub struct TempoFollower {
    last: Option<Instant>,
    ema_bpm: Option<f32>,
    ticks: u32,
}

impl Default for TempoFollower {
    fn default() -> Self {
        Self::new()
    }
}

impl TempoFollower {
    pub fn new() -> Self {
        Self {
            last: None,
            ema_bpm: None,
            ticks: 0,
        }
    }

    /// Feed one clock tick. Returns the tempo when the stream is settled and this tick agreed with it.
    pub fn on_clock(&mut self, now: Instant) -> Option<f32> {
        let previous = self.last.replace(now)?;
        let interval_ms = now.duration_since(previous).as_secs_f32() * 1_000.0;
        let Some(candidate) = bpm_from_interval(interval_ms) else {
            // A tick that cannot be believed also breaks the run.
            self.ticks = self.ticks.saturating_sub(1);
            return self.ema_bpm;
        };

        match self.ema_bpm {
            Some(current) if (candidate - current).abs() / current > AGREEMENT => {
                // Disagrees: lose confidence, keep the tempo we had.
                self.ticks = self.ticks.saturating_sub(1);
                Some(current)
            }
            Some(current) => {
                self.ticks = self.ticks.saturating_add(1);
                if self.ticks < CONFIDENCE {
                    return Some(current);
                }
                let smoothed = current + ALPHA * (candidate - current);
                self.ema_bpm = Some(smoothed);
                Some(smoothed)
            }
            None => {
                self.ticks = self.ticks.saturating_add(1);
                if self.ticks < CONFIDENCE {
                    return None;
                }
                self.ema_bpm = Some(candidate);
                Some(candidate)
            }
        }
    }

    /// The tempo the stream has settled on, if any.
    pub fn bpm(&self) -> Option<f32> {
        self.ema_bpm
    }

    /// Forget the stream; the next tick only starts the clock again.
    pub fn reset(&mut self) {
        self.last = None;
        self.ema_bpm = None;
        self.ticks = 0;
    }
}

/// Adopt an external tempo: the panel shows it and the transport follows it.
pub fn follow_clock(state: &SharedState, bpm: f32) {
    state.tempo_bpm.set(clamp_bpm(bpm));
    state
        .external_clock
        .store(true, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn a_24_ppqn_stream_at_20_8333_ms_is_120_bpm() {
        let bpm = bpm_from_interval(60_000.0 / (120.0 * 24.0)).unwrap();
        assert!((bpm - 120.0).abs() < 0.01);
    }

    #[test]
    fn absurd_intervals_are_ignored() {
        assert_eq!(bpm_from_interval(0.5), None);
        assert_eq!(bpm_from_interval(5_000.0), None);
    }

    #[test]
    fn the_follower_settles_on_a_steady_clock() {
        let mut f = TempoFollower::new();
        let step = Duration::from_secs_f64(60.0 / (128.0 * 24.0));
        let start = Instant::now();
        let mut reported = None;
        for i in 0..48 {
            reported = f.on_clock(start + step * i).or(reported);
        }
        let bpm = reported.expect("a steady clock must be followed");
        assert!((bpm - 128.0).abs() < 1.5, "followed {bpm}");
    }

    #[test]
    fn a_single_stray_tick_does_not_change_tempo() {
        let mut f = TempoFollower::new();
        let step = Duration::from_secs_f64(60.0 / (130.0 * 24.0));
        let start = Instant::now();
        for i in 0..48 {
            f.on_clock(start + step * i);
        }
        let settled = f.bpm().unwrap();
        let stray = start + step * 49 + Duration::from_millis(15);
        let after = f.on_clock(stray);
        let after_bpm = after.unwrap_or(settled);
        assert!(
            (after_bpm - settled).abs() < 5.0,
            "{settled} -> {after_bpm}"
        );
    }

    #[test]
    fn following_writes_the_tempo_and_marks_the_clock_external() {
        let s = SharedState::new();
        follow_clock(&s, 124.0);
        assert!((s.tempo_bpm.get() - 124.0).abs() < 1e-6);
        assert!(s.external_clock.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn a_reset_forgets_the_stream() {
        let mut f = TempoFollower::new();
        let step = Duration::from_secs_f64(60.0 / (128.0 * 24.0));
        let start = Instant::now();
        for i in 0..48 {
            f.on_clock(start + step * i);
        }
        assert!(f.bpm().is_some());
        f.reset();
        assert_eq!(f.bpm(), None);
        assert_eq!(
            f.on_clock(start),
            None,
            "the first tick after a reset only starts the clock"
        );
    }
}
