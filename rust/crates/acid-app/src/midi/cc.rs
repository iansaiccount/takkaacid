//! Continuous-controller mapping: eleven CCs move the panel's knobs, the tempo, the pattern and the swing.
//!
//! Every write lands in the same atomics the mouse writes, so a controller and the panel can never fight
//! over a value — the last one to arrive wins, and the panel redraws from the atomic.

use std::sync::atomic::Ordering;

use acid_core::bank::PATTERNS_PER_BANK;
use acid_core::timing::{MAX_BPM, MIN_BPM, clamp_bpm};

use crate::knobs::Knob;
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
    LEARNABLE
        .iter()
        .enumerate()
        .map(|(slot, target)| (DEFAULT_CC[slot], *target))
        .collect()
}

/// The controls MIDI learn can teach, in `SharedState::cc_map` slot order: the seven tone knobs in the catalogue's own
/// drawing order, then the four teachable controls that are not in the knob row.
///
/// Derived rather than written out a second time, so a knob added to `Knob::ALL` takes the next learn slot instead of
/// being silently unteachable — the failure this replaces was a list that had to be kept in step by hand.
pub const LEARNABLE: [CcTarget; 11] = learnable();

/// The four teachable controls outside the knob row, in slot order: the volume knob, the two readouts and the shuffle.
const OUTSIDE_THE_ROW: [CcTarget; 4] = [
    CcTarget::Volume,
    CcTarget::Tempo,
    CcTarget::Pattern,
    CcTarget::Swing,
];

const fn learnable() -> [CcTarget; 11] {
    let mut slots = [CcTarget::Waveform; 11];
    let mut slot = 0;
    while slot < Knob::ALL.len() {
        slots[slot] = Knob::ALL[slot].learn_target();
        slot += 1;
    }
    let mut outside = 0;
    while outside < OUTSIDE_THE_ROW.len() {
        slots[slot] = OUTSIDE_THE_ROW[outside];
        slot += 1;
        outside += 1;
    }
    slots
}

/// The controller a control ships with: one each, keyed on the control rather than on the slot it sits in, so a catalogue
/// that moves cannot hand one control's fader to its neighbour.
///
/// The waveform gets CC 20 and the pattern selector CC 21, both undefined controllers no common box sends by accident, so
/// a fresh session never has a fader fighting the mouse. Tempo keeps 72 and swing keeps 73 — the controllers they had while
/// they were fixed — so a rig that already works keeps working.
pub const fn factory_cc(target: CcTarget) -> u8 {
    match target {
        CcTarget::Waveform => 20,
        CcTarget::Tuning => 3,
        CcTarget::Cutoff => 74,
        CcTarget::Resonance => 71,
        CcTarget::EnvMod => 76,
        CcTarget::Decay => 75,
        CcTarget::Accent => 5,
        CcTarget::Volume => 7,
        CcTarget::Tempo => 72,
        CcTarget::Pattern => 21,
        CcTarget::Swing => 73,
    }
}

/// The factory pairing for the learnable controls: one controller each.
pub const DEFAULT_CC: [u8; LEARNABLE.len()] = factory_pairing();

const fn factory_pairing() -> [u8; LEARNABLE.len()] {
    let mut ccs = [0u8; LEARNABLE.len()];
    let mut slot = 0;
    while slot < LEARNABLE.len() {
        ccs[slot] = factory_cc(LEARNABLE[slot]);
        slot += 1;
    }
    ccs
}
/// Which slot a target occupies in `SharedState::cc_map`, or `None` for the fixed ones.
pub fn slot(target: CcTarget) -> Option<usize> {
    LEARNABLE.iter().position(|candidate| *candidate == target)
}

/// The target a controller is wired to, from the one learned table. One controller drives one control, because `set_cc`
/// clears it from every other slot.
pub fn cc_target(state: &SharedState, cc: u8) -> Option<CcTarget> {
    (0..LEARNABLE.len())
        .find(|slot| state.cc_map[*slot].load(Ordering::Relaxed) == i32::from(cc))
        .map(|slot| LEARNABLE[slot])
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

/// How close a controller has to be to the knob to count as already on it: two and a half of its 128 steps.
pub const PICKUP_TOLERANCE: f32 = 0.02;

/// Whether a control is a dial the controller has to reach first, or a stepper it names outright.
///
/// A dial and a fader are two positions of one value, and they are almost never in the same place: without this, the
/// first message after a song loads slams the knob to wherever the fader happened to be. A stepper (the waveform) is not a
/// position but a choice — a fader pointed at TRIANGLE means TRIANGLE.
///
/// The pattern selector looks like a stepper but is played like a dial: a fader parked on pattern 8 must not jump the
/// selection the moment it is touched, so it picks up the way the TEMPO readout does.
pub fn takes_pickup(target: CcTarget) -> bool {
    !matches!(target, CcTarget::Waveform)
}

/// Whether a controller at `sent` may turn a knob at `current`.
///
/// `last` is what the controller sent before, `None` until it sends its first value — an `Option` rather than a
/// sentinel, because `0.0` is a real controller position.
pub fn takeover(last: Option<f32>, engaged: bool, sent: f32, current: f32) -> bool {
    if engaged {
        // The knob still holds what the controller last wrote, so it follows. If the hand (or a loaded song) has moved
        // it since, the controller has lost the knob and has to come back around to it.
        return last.is_some_and(|last| (current - last).abs() <= PICKUP_TOLERANCE);
    }
    match last {
        // First touch: only a controller that is already on the knob may turn it.
        None => (sent - current).abs() <= PICKUP_TOLERANCE,
        // The travel crossed the knob's value: the two were on opposite sides of it last time.
        Some(last) => {
            let before = last - current;
            let now = sent - current;
            before * now <= 0.0 && (sent - last).abs() > 0.0
        }
    }
}

/// Which pattern of the bank a controller value points at: 128 steps over eight equal zones, sixteen each.
fn pattern_number(value: u8) -> usize {
    let zone = f32::from(value) / 128.0 * PATTERNS_PER_BANK as f32;
    (zone as usize).min(PATTERNS_PER_BANK - 1)
}

/// Where the pattern selector's knob sits, as a control position: the middle of the zone the playing pattern occupies.
/// Reading a fader and asking whether it has reached the pattern are the same mapping, so they are one function.
fn pattern_zone(index: usize) -> f32 {
    (pattern_number_of(index) as f32 + 0.5) / PATTERNS_PER_BANK as f32
}

/// The pattern's number inside its bank, from an absolute index in the 64.
fn pattern_number_of(index: usize) -> usize {
    index % PATTERNS_PER_BANK
}
/// Where a control's knob currently sits, as a position rather than its own units.
fn knob_value(state: &SharedState, target: CcTarget) -> Option<f32> {
    match target {
        CcTarget::Tuning => Some(state.tuning.get()),
        CcTarget::Cutoff => Some(state.cutoff.get()),
        CcTarget::Resonance => Some(state.resonance.get()),
        CcTarget::EnvMod => Some(state.env_mod.get()),
        CcTarget::Decay => Some(state.decay.get()),
        CcTarget::Accent => Some(state.accent.get()),
        CcTarget::Volume => Some(state.volume.get()),
        // The tempo fader spans the whole range, so its "position" is where the current tempo sits in that range.
        CcTarget::Tempo => Some((state.tempo_bpm.get() - MIN_BPM) / (MAX_BPM - MIN_BPM)),
        CcTarget::Swing => Some(state.swing.get()),
        // The pattern selector's position is the middle of the zone the playing pattern occupies — the same mapping
        // `apply_cc` reads a fader with, so "has the fader reached it" means one thing on both sides.
        CcTarget::Pattern => Some(pattern_zone(state.pattern_index())),
        // Not a dial: no pickup, and no position to compare against.
        CcTarget::Waveform => None,
    }
}

/// Move whatever `cc` is wired to. Returns false when the controller is unmapped.
///
/// A mapped controller that has not yet reached its knob returns true: the message was understood and deliberately
/// ignored, which is not the same thing as an unmapped controller.
pub fn apply_cc(state: &SharedState, cc: u8, value: u8) -> bool {
    // While the panel is listening, the next controller is remembered rather than applied: a knob jumping under
    // the hand that is teaching it would be the wrong answer to "move a control".
    if state.learn_active.load(Ordering::Relaxed) {
        state.learn_cc.store(i32::from(cc), Ordering::Relaxed);
        return false;
    }
    let knob = cc_value(value);
    let target = match cc_target(state, cc) {
        Some(target) => target,
        None => return false,
    };
    // A dial has to be reached before it may be turned. The controller and the knob are two positions of one value, so
    // a fader parked elsewhere is not an instruction to jump — it is a fader that has not met the knob yet.
    //
    // What a controller "writes" is the position it puts the control at, which is not the raw travel for a stepper: a
    // pattern fader writes the middle of the zone it landed in, so staying inside the zone keeps it engaged.
    let written = match target {
        CcTarget::Pattern => pattern_zone(pattern_number(value)),
        _ => knob,
    };
    if takes_pickup(target)
        && let Some(slot) = slot(target)
        && let Some(current) = knob_value(state, target)
    {
        let pickup = &state.pickup[slot];
        let engaged = takeover(pickup.last(), pickup.engaged(), written, current);
        pickup.record(written, engaged);
        if !engaged {
            return true;
        }
    }
    match target {
        // A switch, not a dial: the controller's travel is snapped to the nearest of the four faces, which is the same
        // rule the knob follows when the hand lets go.
        CcTarget::Waveform => {
            state.set_waveform(acid_core::oscillator::Waveform::from_position(knob))
        }
        CcTarget::Tuning => state.tuning.set(knob),
        CcTarget::Cutoff => state.cutoff.set(knob),
        CcTarget::Resonance => state.resonance.set(knob),
        CcTarget::EnvMod => state.env_mod.set(knob),
        CcTarget::Decay => state.decay.set(knob),
        CcTarget::Accent => state.accent.set(knob),
        CcTarget::Volume => state.volume.set(knob),
        CcTarget::Tempo => {
            state
                .tempo_bpm
                .set(clamp_bpm(MIN_BPM + knob * (MAX_BPM - MIN_BPM)));
        }
        // The pattern's zones are equal: 128 controller steps over eight patterns is sixteen each, so a fader rests in
        // the middle of a pattern rather than on a hair-trigger boundary. The MIDI thread may not move the bank, so it
        // leaves a request and `main` drains it when the bar allows.
        CcTarget::Pattern => {
            let number = pattern_number(value);
            let bank = state.pattern_index() / PATTERNS_PER_BANK;
            state.pattern_request.store(
                (bank * PATTERNS_PER_BANK + number) as i32,
                Ordering::Relaxed,
            );
        }
        CcTarget::Swing => state.swing.set(knob.clamp(0.0, 1.0)),
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

    /// Bring a controller onto the knob it drives, the way a hand does: sweep from the bottom past the knob's value.
    fn reach(state: &SharedState, cc: u8) {
        assert!(apply_cc(state, cc, 0));
        assert!(apply_cc(state, cc, 127), "the sweep should reach the knob");
    }

    #[test]
    fn every_mapped_cc_moves_its_own_knob() {
        let s = SharedState::new();
        reach(&s, 74);
        assert!((s.cutoff.get() - 1.0).abs() < 1e-6);
        reach(&s, 71);
        assert!(apply_cc(&s, 71, 0));
        assert_eq!(s.resonance.get(), 0.0);
        reach(&s, 75);
        assert!(apply_cc(&s, 75, 64));
        assert!((s.decay.get() - 64.0 / 127.0).abs() < 1e-6);
        reach(&s, 7);
        assert!(apply_cc(&s, 7, 32));
        assert!((s.volume.get() - 32.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn tempo_cc_spans_the_whole_range() {
        let s = SharedState::new();
        reach(&s, 72);
        assert_eq!(s.tempo_bpm.get(), MAX_BPM);

        apply_cc(&s, 72, 0);
        assert_eq!(s.tempo_bpm.get(), MIN_BPM);
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
        assert_eq!(mapped().len(), LEARNABLE.len());
    }

    #[test]
    fn swing_cc_is_clamped_to_the_shuffle_range() {
        let s = SharedState::new();
        reach(&s, 73);
        assert!((s.swing.get() - 1.0).abs() < 1e-6);
        apply_cc(&s, 73, 0);
        assert_eq!(s.swing.get(), 0.0);
    }

    #[test]
    fn a_learned_controller_replaces_the_factory_one_and_never_doubles() {
        let s = SharedState::new();
        assert_eq!(cc_for(&s, CcTarget::Cutoff), Some(74));

        set_cc(&s, CcTarget::Cutoff, 21);
        assert_eq!(cc_for(&s, CcTarget::Cutoff), Some(21));
        assert_eq!(cc_target(&s, 21), Some(CcTarget::Cutoff));
        assert_eq!(cc_target(&s, 74), None, "the old controller is free again");
        reach(&s, 21);
        assert_eq!(s.cutoff.get(), 1.0);
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
        // The fader has to reach the pattern that is playing before it may move it: 24 is inside pattern 1's zone, which
        // is where this fader is parked, so the first message takes over.
        assert!(apply_cc(&s, 21, 24));
        assert_eq!(s.take_pattern_request(), Some(bank_start + 1));

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
            // `main` applies the request, and that is what moves the knob the fader is holding.
            s.set_pattern_index(bank_start + number);
        }
    }

    #[test]
    fn a_pattern_fader_has_to_reach_the_pattern_before_it_moves_it() {
        // A fader parked at the far end of its travel is not an instruction to jump to the last pattern: it has to meet
        // the pattern that is playing first, like every other dial in the instrument.
        let s = SharedState::new();
        s.set_pattern_index(0);

        assert!(apply_cc(&s, 21, 127));
        assert_eq!(
            s.take_pattern_request(),
            None,
            "the fader has not found the playing pattern yet"
        );

        // Sweeping down across the pattern it is on takes over — the crossing is what counts, not the landing.
        assert!(apply_cc(&s, 21, 0));
        assert_eq!(
            s.take_pattern_request(),
            Some(0),
            "the sweep reached the pattern"
        );

        // From there it follows, so a sweep across the travel asks for each pattern in turn.
        apply_cc(&s, 21, 127);
        assert_eq!(s.take_pattern_request(), Some(7));
    }

    #[test]
    fn a_pattern_controller_leaves_nothing_else_behind() {
        // The MIDI thread may not move the bank, so the whole effect is one request; nothing else may budge.
        let s = SharedState::new();
        let before = s.pattern();
        // Find the pattern that is playing first (A1), then ask for another one.
        assert!(apply_cc(&s, 21, 0));
        assert_eq!(s.take_pattern_request(), Some(0), "it found A1");
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

        // A low fader must not drag a running tempo down; sweeping up to it takes over.
        assert!(apply_cc(&s, 22, 0));
        assert!(
            s.tempo_bpm.get() > MIN_BPM,
            "a fader at the bottom has not reached the tempo yet"
        );
        reach(&s, 22);
        assert_eq!(s.tempo_bpm.get(), MAX_BPM);
    }

    #[test]
    fn the_shuffle_is_a_learnable_slot_now() {
        assert_eq!(slot(CcTarget::Swing), Some(10), "the shuffle has a slot");
        assert_eq!(slot(CcTarget::Pattern), Some(9));
        assert_eq!(slot(CcTarget::Waveform), Some(0));

        // The factory pairing keeps 73, so a rig that already drives the shuffle keeps driving it.
        let s = SharedState::new();
        assert_eq!(cc_for(&s, CcTarget::Swing), Some(73));
        assert_eq!(cc_target(&s, 73), Some(CcTarget::Swing));

        // And it can be taught elsewhere, which frees 73 like any other controller.
        set_cc(&s, CcTarget::Swing, 40);
        assert_eq!(cc_for(&s, CcTarget::Swing), Some(40));
        assert_eq!(cc_target(&s, 73), None);
        assert_eq!(
            cc_target(&s, 40),
            Some(CcTarget::Swing),
            "one controller drives one control"
        );
    }

    #[test]
    fn a_shuffle_fader_has_to_reach_the_knob() {
        // Swing is a dial now, so it gets the same pickup rule as the rest: a fader parked at the bottom must not flatten
        // a groove that is already pushing.
        let s = SharedState::new();
        s.swing.set(0.6);
        assert!(apply_cc(&s, 73, 0));
        assert_eq!(s.swing.get(), 0.6, "the fader has not found the knob yet");

        apply_cc(&s, 73, 127);
        assert_eq!(s.swing.get(), 1.0, "the sweep took over");
        apply_cc(&s, 73, 0);
        assert_eq!(s.swing.get(), 0.0, "and then follows");
    }

    #[test]
    fn the_learn_map_is_still_the_one_every_saved_rig_is_indexed_by() {
        // `SharedState::cc_map` is saved by slot, so this order and this pairing are a compatibility contract: a reorder
        // would silently rebind every existing rig's faders on upgrade. Both are pinned as literals — comparing them to
        // the catalogue that now builds them would only say the derivation is the derivation.
        let slots: Vec<String> = LEARNABLE
            .iter()
            .map(|target| format!("{target:?}"))
            .collect();
        assert_eq!(
            slots,
            [
                "Waveform",
                "Tuning",
                "Cutoff",
                "Resonance",
                "EnvMod",
                "Decay",
                "Accent",
                "Volume",
                "Tempo",
                "Pattern",
                "Swing"
            ],
            "the learn slots moved: a saved cc_map would arrive pointing at other controls"
        );
        assert_eq!(
            DEFAULT_CC,
            [20, 3, 74, 71, 76, 75, 5, 7, 72, 21, 73],
            "the factory controllers moved"
        );
    }

    #[test]
    fn a_control_keeps_its_factory_controller_when_the_learn_map_moves() {
        // The pairing is keyed on the control, not on the position it happens to sit in: 74 is CUT OFF whoever is third.
        // A catalogue that moves must move the controller with the control, not hand CUT OFF's fader to its neighbour.
        assert_eq!(factory_cc(CcTarget::Cutoff), 74);
        assert_eq!(factory_cc(CcTarget::Waveform), 20, "a rig's wave fader");
        assert_eq!(factory_cc(CcTarget::Swing), 73, "the shuffle keeps 73");
        for (slot, target) in LEARNABLE.iter().enumerate() {
            assert_eq!(
                DEFAULT_CC[slot],
                factory_cc(*target),
                "{target:?} lost its controller"
            );
        }
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
    fn a_far_controller_does_not_move_the_knob() {
        // The complaint this exists for: a song is loaded with the cutoff open, and a fader parked at the bottom must
        // not slam it shut just because it twitched.
        let s = SharedState::new();
        s.cutoff.set(0.8);
        assert!(
            apply_cc(&s, 74, 0),
            "the controller is mapped, it just is not there yet"
        );
        assert_eq!(s.cutoff.get(), 0.8, "the knob stayed where the song put it");
        assert!(apply_cc(&s, 74, 40));
        assert_eq!(s.cutoff.get(), 0.8, "still on its way up");
    }

    #[test]
    fn a_controller_that_reaches_the_knob_takes_over_and_then_follows() {
        let s = SharedState::new();
        s.cutoff.set(0.5);
        apply_cc(&s, 74, 0);
        apply_cc(&s, 74, 63); // 0.496: one step short of the knob
        assert_eq!(s.cutoff.get(), 0.5, "not reached yet");
        apply_cc(&s, 74, 64); // crossed
        assert!(
            (s.cutoff.get() - 64.0 / 127.0).abs() < 1e-6,
            "the crossing is where it takes over"
        );

        // From here the knob follows, including small moves back down.
        apply_cc(&s, 74, 60);
        assert!((s.cutoff.get() - 60.0 / 127.0).abs() < 1e-6);
    }

    #[test]
    fn a_hand_on_the_knob_takes_it_back() {
        let s = SharedState::new();
        reach(&s, 74);
        assert!((s.cutoff.get() - 1.0).abs() < 1e-6);

        // The hand drags the knob somewhere else, or a song load moves it.
        s.cutoff.set(0.3);
        apply_cc(&s, 74, 127);
        assert_eq!(s.cutoff.get(), 0.3, "the controller lost the knob");

        // It has to come back around to it.
        apply_cc(&s, 74, 0);
        apply_cc(&s, 74, 64);
        assert!(
            (s.cutoff.get() - 64.0 / 127.0).abs() < 1e-6,
            "and can take it again"
        );
    }

    #[test]
    fn the_switches_take_the_value_they_are_pointed_at() {
        // A stepper is not a position: a fader pointed at TRIANGLE means TRIANGLE, first message or not.
        use acid_core::oscillator::Waveform;

        let s = SharedState::new();
        s.set_waveform(Waveform::Saw);
        apply_cc(&s, 20, 127);
        assert_eq!(s.waveform(), Waveform::Triangle);

        // The pattern selector is not in this class: it looks like a stepper but is played like a dial, so it has to be
        // reached first — `a_pattern_fader_has_to_reach_the_pattern_before_it_moves_it` is the test for that.
    }

    #[test]
    fn takeover_wants_a_crossing_or_a_near_miss() {
        // A first touch has to land on the knob; a sweep has to pass it; an engaged controller follows a knob that has
        // not moved, and loses one that has.
        assert!(!takeover(None, false, 1.0, 0.2), "a first touch far away");
        assert!(
            takeover(None, false, 0.21, 0.2),
            "a first touch on the knob"
        );
        assert!(!takeover(Some(0.0), false, 0.1, 0.2), "short of the knob");
        assert!(takeover(Some(0.1), false, 0.3, 0.2), "swept past it");
        assert!(takeover(Some(0.3), false, 0.1, 0.2), "swept back past it");
        assert!(takeover(Some(0.2), true, 0.9, 0.2), "engaged and holding");
        assert!(
            !takeover(Some(0.2), true, 0.9, 0.5),
            "engaged, but the hand moved it"
        );
    }
}
