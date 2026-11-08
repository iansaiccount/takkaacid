//! Continuous-controller mapping: nine CCs move the panel's knobs, the tempo and the swing.
//!
//! Every write lands in the same atomics the mouse writes, so a controller and the panel can never fight
//! over a value — the last one to arrive wins, and the panel redraws from the atomic.

use std::sync::atomic::Ordering;

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

/// The controls MIDI learn can teach, in `SharedState::cc_map` slot order.
pub const LEARNABLE: [CcTarget; 7] = [
    CcTarget::Tuning,
    CcTarget::Cutoff,
    CcTarget::Resonance,
    CcTarget::EnvMod,
    CcTarget::Decay,
    CcTarget::Accent,
    CcTarget::Volume,
];

/// The factory pairing for the learnable knobs: one controller each.
pub const DEFAULT_CC: [u8; 7] = [3, 74, 71, 76, 75, 5, 7];

/// Tempo and swing are not learnable: they keep their fixed controllers, so a performance never loses the clock.
pub const FIXED_CC: [(u8, CcTarget); 2] = [(72, CcTarget::Tempo), (73, CcTarget::Swing)];

/// Which slot a target occupies in `SharedState::cc_map`, or `None` for the fixed ones.
pub fn slot(target: CcTarget) -> Option<usize> {
    LEARNABLE.iter().position(|candidate| *candidate == target)
}

/// The target a controller is wired to: the learned table first, the fixed pair after it. One controller drives
/// one control, because `set_cc` clears it from every other slot.
pub fn cc_target(state: &SharedState, cc: u8) -> Option<CcTarget> {
    let learned = (0..LEARNABLE.len())
        .find(|slot| state.cc_map[*slot].load(Ordering::Relaxed) == i32::from(cc));
    if let Some(slot) = learned {
        return Some(LEARNABLE[slot]);
    }
    FIXED_CC
        .iter()
        .find(|(number, _)| *number == cc)
        .map(|(_, target)| *target)
}

/// The controller a control currently listens to.
pub fn cc_for(state: &SharedState, target: CcTarget) -> Option<u8> {
    let slot = slot(target)?;
    match state.cc_map[slot].load(Ordering::Relaxed) {
        cc if cc >= 0 => Some(cc as u8),
        _ => None,
    }
}

/// Teach a control to listen to `cc`, clearing that controller from any other slot.
pub fn set_cc(state: &SharedState, target: CcTarget, cc: u8) {
    let Some(slot) = slot(target) else {
        return;
    };
    for other in 0..LEARNABLE.len() {
        if other != slot && state.cc_map[other].load(Ordering::Relaxed) == i32::from(cc) {
            state.cc_map[other].store(-1, Ordering::Relaxed);
        }
    }
    state.cc_map[slot].store(i32::from(cc), Ordering::Relaxed);
}

/// A controller value as a knob position, `0.0..=1.0`.
pub fn cc_value(value: u8) -> f32 {
    f32::from(value) / 127.0
}

/// Move whatever `cc` is wired to. Returns false when the controller is unmapped.
pub fn apply_cc(state: &SharedState, cc: u8, value: u8) -> bool {
    // While the panel is listening, the next controller is remembered rather than applied: a knob jumping under
    // the hand that is teaching it would be the wrong answer to "move a control".
    if state.learn_active.load(Ordering::Relaxed) {
        state.learn_cc.store(i32::from(cc), Ordering::Relaxed);
        return false;
    }
    let knob = cc_value(value);
    match cc_target(state, cc) {
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
        assert_eq!(cc_target(&s, 1), None);
        assert_eq!(cc_target(&s, 74), Some(CcTarget::Cutoff));
        assert_eq!(CC_MAP.len(), 9);
    }

    #[test]
    fn swing_cc_is_clamped_to_the_shuffle_range() {
        let s = SharedState::new();
        apply_cc(&s, 73, 127);
        assert!((s.swing.get() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_learned_controller_replaces_the_factory_one_and_never_doubles() {
        let s = SharedState::new();
        assert_eq!(cc_for(&s, CcTarget::Cutoff), Some(74));

        set_cc(&s, CcTarget::Cutoff, 21);
        assert_eq!(cc_for(&s, CcTarget::Cutoff), Some(21));
        assert_eq!(cc_target(&s, 21), Some(CcTarget::Cutoff));
        assert_eq!(cc_target(&s, 74), None, "the old controller is free again");
        assert!(apply_cc(&s, 21, 0));
        assert_eq!(s.cutoff.get(), 0.0);

        // Teaching a controller that another knob owns moves it, so one CC drives exactly one control.
        set_cc(&s, CcTarget::Resonance, 21);
        assert_eq!(cc_for(&s, CcTarget::Resonance), Some(21));
        assert_eq!(cc_for(&s, CcTarget::Cutoff), None);
        assert_eq!(cc_target(&s, 21), Some(CcTarget::Resonance));
    }

    #[test]
    fn listening_records_the_controller_instead_of_moving_a_knob() {
        let s = SharedState::new();
        let before = s.cutoff.get();
        s.learn_active
            .store(true, std::sync::atomic::Ordering::Relaxed);

        assert!(!apply_cc(&s, 22, 127), "nothing is applied while learning");
        assert_eq!(s.cutoff.get(), before, "the knob stayed put");
        assert_eq!(
            s.learn_cc.load(std::sync::atomic::Ordering::Relaxed),
            22,
            "the controller is remembered for the panel"
        );
    }

    #[test]
    fn tempo_and_swing_cannot_be_taught() {
        assert_eq!(slot(CcTarget::Tempo), None);
        assert_eq!(slot(CcTarget::Swing), None);
        assert_eq!(slot(CcTarget::Volume), Some(6));
        // The fixed pair still answers after the learned table is empty.
        let s = SharedState::new();
        for slot in 0..LEARNABLE.len() {
            s.cc_map[slot].store(-1, std::sync::atomic::Ordering::Relaxed);
        }
        assert_eq!(cc_target(&s, 72), Some(CcTarget::Tempo));
        assert_eq!(cc_target(&s, 73), Some(CcTarget::Swing));
    }
}
