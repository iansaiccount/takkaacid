//! Continuous-controller mapping: nine CCs move the panel's knobs, the tempo and the swing.
//!
//! Every write lands in the same atomics the mouse writes, so a controller and the panel can never fight
//! over a value — the last one to arrive wins, and the panel redraws from the atomic.

use acid_core::timing::{MAX_BPM, MIN_BPM, clamp_bpm};

use crate::state::SharedState;

/// What a continuous controller is wired to.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CcTarget {
    Tuning,
    Cutoff,
    Resonance,
    EnvMod,
    Decay,
    Accent,
    Volume,
    Tempo,
    Swing,
}

/// The CCs this instrument listens to. Also printed by the help overlay.
pub const CC_MAP: [(u8, CcTarget); 9] = [
    (3, CcTarget::Tuning),
    (74, CcTarget::Cutoff),
    (71, CcTarget::Resonance),
    (76, CcTarget::EnvMod),
    (75, CcTarget::Decay),
    (5, CcTarget::Accent),
    (7, CcTarget::Volume),
    (72, CcTarget::Tempo),
    (73, CcTarget::Swing),
];

/// The target for a controller number, if we listen to it.
pub fn cc_target(cc: u8) -> Option<CcTarget> {
    CC_MAP
        .iter()
        .find(|(number, _)| *number == cc)
        .map(|(_, target)| *target)
}

/// A controller value as a knob position, `0.0..=1.0`.
pub fn cc_value(value: u8) -> f32 {
    f32::from(value) / 127.0
}

/// Move whatever `cc` is wired to. Returns false when the controller is unmapped.
pub fn apply_cc(state: &SharedState, cc: u8, value: u8) -> bool {
    let knob = cc_value(value);
    match cc_target(cc) {
        Some(CcTarget::Tuning) => state.tuning.set(knob),
        Some(CcTarget::Cutoff) => state.cutoff.set(knob),
        Some(CcTarget::Resonance) => state.resonance.set(knob),
        Some(CcTarget::EnvMod) => state.env_mod.set(knob),
        Some(CcTarget::Decay) => state.decay.set(knob),
        Some(CcTarget::Accent) => state.accent.set(knob),
        Some(CcTarget::Volume) => state.volume.set(knob),
        Some(CcTarget::Tempo) => {
            state
                .tempo_bpm
                .set(clamp_bpm(MIN_BPM + knob * (MAX_BPM - MIN_BPM)));
        }
        Some(CcTarget::Swing) => state.swing.set(knob.clamp(0.0, 1.0)),
        None => return false,
    }
    true
}

/// Every mapping, one line per controller, for the help overlay.
pub fn mapping_lines() -> Vec<String> {
    CC_MAP
        .iter()
        .map(|(cc, target)| format!("CC {cc}: {target:?}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_mapped_cc_moves_its_own_knob() {
        let s = SharedState::new();
        assert!(apply_cc(&s, 74, 127));
        assert!((s.cutoff.get() - 1.0).abs() < 1e-6);
        assert!(apply_cc(&s, 71, 0));
        assert_eq!(s.resonance.get(), 0.0);
        assert!(apply_cc(&s, 75, 64));
        assert!((s.decay.get() - 64.0 / 127.0).abs() < 1e-6);
        assert!(apply_cc(&s, 7, 32));
        assert!((s.volume.get() - 32.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn tempo_cc_spans_the_whole_range() {
        let s = SharedState::new();
        apply_cc(&s, 72, 0);
        assert_eq!(s.tempo_bpm.get(), MIN_BPM);
        apply_cc(&s, 72, 127);
        assert_eq!(s.tempo_bpm.get(), MAX_BPM);
        apply_cc(&s, 72, 64);
        let mid = s.tempo_bpm.get();
        assert!(mid > 150.0 && mid < 190.0, "midpoint was {mid}");
    }

    #[test]
    fn unmapped_cc_are_ignored_quietly() {
        let s = SharedState::new();
        assert!(!apply_cc(&s, 1, 100));
        assert_eq!(cc_target(1), None);
        assert_eq!(cc_target(74), Some(CcTarget::Cutoff));
        assert_eq!(CC_MAP.len(), 9);
    }

    #[test]
    fn swing_cc_is_clamped_to_the_shuffle_range() {
        let s = SharedState::new();
        apply_cc(&s, 73, 127);
        assert!((s.swing.get() - 1.0).abs() < 1e-6);
    }
}
