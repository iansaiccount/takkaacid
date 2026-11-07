//! Keeping MIDI alive: probe without blocking, retry every two seconds, and give up quietly after a while.
//!
//! The shape mirrors `audio::supervisor`: a probe never blocks the UI thread (it runs on a spawned thread and
//! reports through a channel), and a missing interface is a status line, never an error dialog.

use std::time::{Duration, Instant};

/// How long to wait between probes.
pub const MIDI_RETRY: Duration = Duration::from_secs(2);
/// How many failed probes before the app stops trying on its own.
pub const MIDI_MAX_RETRIES: u32 = 15;

/// What the supervisor wants done next.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum MidiAction {
    /// Try to open a port now. Reserved: the app opens once at startup and retries on `Wait`.
    Open,
    /// Wait for the retry interval.
    Wait,
    /// Stop probing; only a REPROBE click will try again.
    GiveUp,
}

/// Decides when to probe for a MIDI port again.
#[derive(Debug)]
pub struct MidiSupervisor {
    retries: u32,
    next_probe: Option<Instant>,
}

impl Default for MidiSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl MidiSupervisor {
    pub fn new() -> Self {
        Self {
            retries: 0,
            next_probe: None,
        }
    }

    /// Record a failed probe and say what to do next.
    pub fn on_error(&mut self, now: Instant) -> MidiAction {
        self.retries = self.retries.saturating_add(1);
        if self.retries >= MIDI_MAX_RETRIES {
            self.next_probe = None;
            return MidiAction::GiveUp;
        }
        self.next_probe = Some(now + MIDI_RETRY);
        MidiAction::Wait
    }

    /// A port opened: forget every failure, so the next unplug starts the count again.
    /// A port opened: forget every failure, and wait a full interval before looking again.
    ///
    /// Waiting is the point. `next_probe: None` means "never looked", so leaving it `None` after a success made
    /// `should_probe` true on the very next frame: the app opened a second connection while the first was still live, and
    /// both delivered every note — one key press, two steps.
    pub fn on_success(&mut self, now: Instant) {
        self.retries = 0;
        self.next_probe = Some(now + MIDI_RETRY);
    }

    /// True when a probe is due — immediately before the first attempt, then once per interval.
    pub fn should_probe(&self, now: Instant) -> bool {
        match self.next_probe {
            None => self.retries < MIDI_MAX_RETRIES,
            Some(due) => now >= due,
        }
    }

    /// Forget everything and probe on the next frame: the player picked a port, or asked to retry.
    ///
    /// This is the opposite of [`Self::on_success`], which waits: a chosen port should open now, while a port that just
    /// opened should not be opened again.
    pub fn probe_now(&mut self) {
        self.retries = 0;
        self.next_probe = None;
    }

    /// How many consecutive failures have been recorded.
    pub fn retries(&self) -> u32 {
        self.retries
    }
}

/// The one line the panel shows about MIDI.
pub fn status_line(connected: Option<&str>, retries: u32) -> String {
    match connected {
        Some(name) => format!("MIDI: {name}"),
        None if retries >= MIDI_MAX_RETRIES => "MIDI: none (press REPROBE)".to_string(),
        None => "MIDI: none (retrying)".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probing_waits_two_seconds_between_attempts() {
        let mut s = MidiSupervisor::new();
        let t0 = Instant::now();
        assert!(s.should_probe(t0), "the first probe is due at once");
        assert_eq!(s.on_error(t0), MidiAction::Wait);
        assert!(!s.should_probe(t0 + MIDI_RETRY / 2));
        assert!(s.should_probe(t0 + MIDI_RETRY));
    }

    #[test]
    fn a_port_that_just_opened_is_not_probed_again_on_the_next_frame() {
        // The bug behind "one key press, two steps": a success used to leave `next_probe: None`, which reads as "never
        // probed", so the app opened a second connection the very next frame while the first was still delivering notes.
        let mut s = MidiSupervisor::new();
        let t0 = Instant::now();
        assert!(s.should_probe(t0), "the first probe is due at once");

        s.on_success(t0);
        assert!(
            !s.should_probe(t0),
            "a connected port must not be probed again at once"
        );
        assert!(!s.should_probe(t0 + MIDI_RETRY / 2));
        assert!(
            s.should_probe(t0 + MIDI_RETRY),
            "but it is looked at again after the interval, so an unplugged device is noticed"
        );
    }

    #[test]
    fn asking_for_a_port_probes_at_once_instead_of_waiting() {
        // A chosen port is the opposite case: the player asked for it, so it opens now rather than after the interval.
        let mut s = MidiSupervisor::new();
        let t0 = Instant::now();
        s.on_success(t0);
        assert!(!s.should_probe(t0), "still inside the interval");

        s.probe_now();
        assert!(s.should_probe(t0), "a chosen port opens at once");
    }

    #[test]
    fn it_gives_up_after_fifteen_attempts_and_can_be_reset() {
        let mut s = MidiSupervisor::new();
        let t0 = Instant::now();
        let mut action = MidiAction::Wait;
        for i in 0..MIDI_MAX_RETRIES {
            action = s.on_error(t0 + MIDI_RETRY * (i + 1));
        }
        assert_eq!(action, MidiAction::GiveUp);
        assert!(
            !s.should_probe(t0 + MIDI_RETRY * 40),
            "given up means quiet"
        );
        s.on_success(t0 + MIDI_RETRY * 41);
        assert_eq!(s.on_error(t0 + MIDI_RETRY * 40), MidiAction::Wait);
    }

    #[test]
    fn status_lines_are_plain_ascii_and_name_the_state() {
        assert_eq!(
            status_line(Some("Launchkey MIDI"), 0),
            "MIDI: Launchkey MIDI"
        );
        assert!(status_line(None, 1).contains("retrying"));
        let given_up = status_line(None, MIDI_MAX_RETRIES);
        assert!(given_up.contains("none"));
        for line in [status_line(None, 0), given_up] {
            assert!(line.is_ascii(), "{line}");
        }
    }
}
