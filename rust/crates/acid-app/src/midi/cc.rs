//! Continuous-controller mapping: ten CCs move the panel's knobs, the tempo and the swing.
//!
//! Every write lands in the same atomics the mouse writes, so a controller and the panel can never fight
//! over a value — the last one to arrive wins, and the panel redraws from the atomic.

use std::sync::atomic::Ordering;

use acid_core::timing::{MAX_BPM, MIN_BPM, clamp_bpm};

use crate::state::SharedState;

/// What a continuous controller is wired to.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CcTarget {
    /// The oscillator's wave. A controller has 128 steps and the switch has four faces, so `apply_cc` snaps the value to
    /// the nearest detent — the same rule the knob follows when the hand lets go.
    Waveform,
    Tuning,
    Cutoff,
    Resonance,
    EnvMod,
    Decay,
    Accent,
    Volume,
    /// Which pattern of the current bank is playing. A controller has 128 steps and a bank has eight patterns, so the
    /// value is snapped onto the eight, like the waveform's four.
    Pattern,
    Tempo,
    Swing,
}

/// Every controller the instrument listens to as it ships: the learnable pairing, then the fixed one.
///
/// Built from the catalogues rather than written out again, so a learned control can never be missing from the help
/// overlay's list, and the list cannot claim a controller nothing answers to.
pub fn mapped() -> Vec<(u8, CcTarget)> {
    let learnable = LEARNABLE
        .iter()
        .enumerate()
        .map(|(slot, target)| (DEFAULT_CC[slot], *target));
    learnable.chain(FIXED_CC).collect()
}

/// The controls MIDI learn can teach, in `SharedState::cc_map` slot order.
pub const LEARNABLE: [CcTarget; 10] = [
    CcTarget::Waveform,
    CcTarget::Tuning,
    CcTarget::Cutoff,
    CcTarget::Resonance,
    CcTarget::EnvMod,
    CcTarget::Decay,
    CcTarget::Accent,
    CcTarget::Volume,
    CcTarget::Tempo,
    CcTarget::Pattern,
];

/// The factory pairing for the learnable controls: one controller each.
///
/// The waveform gets CC 20 and the pattern selector CC 21, both undefined controllers no common box sends by accident,
/// so a fresh session never has a fader fighting the mouse. Tempo keeps 72 — the controller it had while it was fixed —
/// so a rig that already works keeps working.
pub const DEFAULT_CC: [u8; 10] = [20, 3, 74, 71, 76, 75, 5, 7, 72, 21];

/// Swing is not learnable: it keeps its fixed controller, so a performance never loses the shuffle.
pub const FIXED_CC: [(u8, CcTarget); 1] = [(73, CcTarget::Swing)];

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
        // A switch, not a dial: the controller's travel is snapped to the nearest of the four faces, which is the same
        // rule the knob follows when the hand lets go.
        Some(CcTarget::Waveform) => {
            state.set_waveform(acid_core::oscillator::Waveform::from_position(knob))
        }
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
        // The pattern is a stepper as well, but its zones are equal: 128 controller steps over eight patterns is sixteen
        // each, so a fader rests in the middle of a pattern rather than on a hair-trigger boundary. The MIDI thread may
        // not move the bank, so it leaves a request and `main` drains it.
        Some(CcTarget::Pattern) => {
            use acid_core::bank::PATTERNS_PER_BANK;

            let zone = f32::from(value) / 128.0 * PATTERNS_PER_BANK as f32;
            let number = (zone as usize).min(PATTERNS_PER_BANK - 1);
            let bank = state.pattern_index() / PATTERNS_PER_BANK;
            let wanted = bank * PATTERNS_PER_BANK + number;
            state
                .pattern_request
                .store(wanted as i32, Ordering::Relaxed);
        }
        Some(CcTarget::Swing) => state.swing.set(knob.clamp(0.0, 1.0)),
        None => return false,
    }
    true
}

/// Every mapping, one line per controller, for the help overlay.
pub fn mapping_lines() -> Vec<String> {
    mapped()
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
        assert_eq!(mapped().len(), LEARNABLE.len() + FIXED_CC.len());
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
    fn a_waveform_controller_snaps_to_the_nearest_detent() {
        // A controller has 128 steps and the switch has four faces. Floored travel would give the last wave a single
        // step of its own at the very top; snapped travel divides the range evenly.
        use acid_core::oscillator::Waveform;

        let s = SharedState::new();
        let faces = [
            (0, Waveform::Saw),
            (42, Waveform::Square),
            (85, Waveform::Sine),
            (127, Waveform::Triangle),
        ];
        for (value, expected) in faces {
            assert!(
                apply_cc(&s, 20, value),
                "CC 20 is the factory waveform fader"
            );
            assert_eq!(
                s.waveform(),
                expected,
                "CC {value} should reach {expected:?}"
            );
        }
    }

    #[test]
    fn the_waveform_can_be_taught_and_the_taught_controller_drives_it() {
        use acid_core::oscillator::Waveform;

        let s = SharedState::new();
        set_cc(&s, CcTarget::Waveform, 64);
        assert_eq!(cc_for(&s, CcTarget::Waveform), Some(64));
        assert_eq!(cc_target(&s, 64), Some(CcTarget::Waveform));
        assert_eq!(
            cc_target(&s, 20),
            None,
            "the factory controller is free again"
        );

        assert!(apply_cc(&s, 64, 127));
        assert_eq!(s.waveform(), Waveform::Triangle);
        assert!(apply_cc(&s, 64, 0));
        assert_eq!(s.waveform(), Waveform::Saw);
    }

    #[test]
    fn the_pattern_controller_snaps_onto_the_eight_of_the_current_bank() {
        // 128 controller steps over eight patterns: sixteen each, so the middle of the travel always lands somewhere
        // deliberate. The request is absolute, inside the bank that is playing.
        let s = SharedState::new();
        s.set_pattern_index(9); // bank 1, pattern 1 (zero-based: bank 1 starts at 8)
        let bank_start = 8;

        for (value, number) in [(0, 0), (15, 0), (16, 1), (31, 1), (127, 7)] {
            assert!(
                apply_cc(&s, 21, value),
                "CC 21 is the factory pattern fader"
            );
            assert_eq!(
                s.take_pattern_request(),
                Some(bank_start + number),
                "CC {value} should ask for pattern {number} of the bank"
            );
        }
        assert_eq!(s.take_pattern_request(), None, "a request is taken once");
    }

    #[test]
    fn a_pattern_controller_leaves_nothing_else_behind() {
        // The MIDI thread may not move the bank, so the whole effect is one request; nothing else may budge.
        let s = SharedState::new();
        let before = s.pattern();
        assert!(apply_cc(&s, 21, 60));
        assert_eq!(
            *s.pattern(),
            *before,
            "the pattern itself is the panel's job"
        );
        assert_eq!(s.take_pattern_request(), Some(3));
    }

    #[test]
    fn the_tempo_can_be_taught_and_still_spans_the_range() {
        let s = SharedState::new();
        assert_eq!(slot(CcTarget::Tempo), Some(8), "tempo is learnable now");

        set_cc(&s, CcTarget::Tempo, 22);
        assert_eq!(cc_for(&s, CcTarget::Tempo), Some(22));
        assert_eq!(
            cc_target(&s, 72),
            None,
            "the factory controller is free again"
        );

        apply_cc(&s, 22, 0);
        assert_eq!(s.tempo_bpm.get(), MIN_BPM);
        apply_cc(&s, 22, 127);
        assert_eq!(s.tempo_bpm.get(), MAX_BPM);
    }

    #[test]
    fn swing_keeps_its_fixed_controller() {
        assert_eq!(slot(CcTarget::Swing), None, "swing stays out of the map");
        assert_eq!(slot(CcTarget::Waveform), Some(0), "the switch is learnable");
        assert_eq!(slot(CcTarget::Volume), Some(7));
        assert_eq!(
            slot(CcTarget::Pattern),
            Some(9),
            "the pattern selector has the last slot"
        );
        // The fixed controller still answers after every learned slot is emptied.
        let s = SharedState::new();
        for slot in 0..LEARNABLE.len() {
            s.cc_map[slot].store(-1, std::sync::atomic::Ordering::Relaxed);
        }
        assert_eq!(cc_target(&s, 73), Some(CcTarget::Swing));
        assert_eq!(
            cc_target(&s, 72),
            None,
            "tempo was learnable, so its empty slot frees 72"
        );
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
}
