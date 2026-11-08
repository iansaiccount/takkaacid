//! The seven tone knobs, as one catalogue.
//!
//! A knob's identity used to live in six places at once — the position array, the publish `match`, the label special
//! case, the tooltip mapping, the learn mapping and the saved struct — each a `match` on `0..=6` with a `_` fallback, so
//! reordering a knob in one of them produced a silent or mislabelled knob instead of a compile error. This module is now
//! the only place that knows what a knob is called, which atomic it drives, and whether it is a dial or a switch.

use crate::midi::cc::CcTarget;
use crate::state::SharedState;

use super::tips;

/// One tone knob of the panel. Drawn left to right in [`Knob::ALL`] order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Knob {
    /// Not a dial: a four-face switch over the wave catalogue, snapped to its detents.
    Waveform,
    Tuning,
    Cutoff,
    Resonance,
    EnvMod,
    Decay,
    Accent,
}

/// What sort of control a knob is. A dial follows the hand continuously; a switch lands on the nearest detent and is
/// read back from the shared state only when the hand is off it.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    Dial,
    Switch,
}

impl Knob {
    /// Every knob, in the order the panel draws them.
    pub const ALL: [Knob; 7] = [
        Knob::Waveform,
        Knob::Tuning,
        Knob::Cutoff,
        Knob::Resonance,
        Knob::EnvMod,
        Knob::Decay,
        Knob::Accent,
    ];

    /// Where this knob sits in [`Self::ALL`], and therefore in `PanelState::knobs`.
    pub fn index(self) -> usize {
        match self {
            Knob::Waveform => 0,
            Knob::Tuning => 1,
            Knob::Cutoff => 2,
            Knob::Resonance => 3,
            Knob::EnvMod => 4,
            Knob::Decay => 5,
            Knob::Accent => 6,
        }
    }

    pub fn kind(self) -> Kind {
        match self {
            Knob::Waveform => Kind::Switch,
            Knob::Tuning
            | Knob::Cutoff
            | Knob::Resonance
            | Knob::EnvMod
            | Knob::Decay
            | Knob::Accent => Kind::Dial,
        }
    }

    /// The silkscreen caption. The waveform's names itself from the wave catalogue, hence the shared state.
    pub fn label(self, shared: &SharedState) -> &'static str {
        match self {
            Knob::Waveform => shared.waveform().name(),
            Knob::Tuning => "TUNING",
            Knob::Cutoff => "CUT OFF",
            Knob::Resonance => "RESONANCE",
            Knob::EnvMod => "ENV MOD",
            Knob::Decay => "DECAY",
            Knob::Accent => "ACCENT",
        }
    }

    /// This knob's hover help.
    pub fn tip(self) -> tips::Control {
        match self {
            Knob::Waveform => tips::Control::Waveform,
            Knob::Tuning => tips::Control::Tuning,
            Knob::Cutoff => tips::Control::Cutoff,
            Knob::Resonance => tips::Control::Resonance,
            Knob::EnvMod => tips::Control::EnvMod,
            Knob::Decay => tips::Control::Decay,
            Knob::Accent => tips::Control::Accent,
        }
    }

    /// The MIDI-learn target this knob teaches. Every tone knob is learnable, so there is no `None` case to forget.
    pub const fn learn_target(self) -> CcTarget {
        match self {
            Knob::Waveform => CcTarget::Waveform,
            Knob::Tuning => CcTarget::Tuning,
            Knob::Cutoff => CcTarget::Cutoff,
            Knob::Resonance => CcTarget::Resonance,
            Knob::EnvMod => CcTarget::EnvMod,
            Knob::Decay => CcTarget::Decay,
            Knob::Accent => CcTarget::Accent,
        }
    }

    /// What the knob shows: the shared value, as a control position. The waveform's position comes from the switch.
    pub fn read(self, shared: &SharedState) -> f32 {
        match self {
            Knob::Waveform => super::waveform_position(shared.waveform()),
            Knob::Tuning => shared.tuning.get(),
            Knob::Cutoff => shared.cutoff.get(),
            Knob::Resonance => shared.resonance.get(),
            Knob::EnvMod => shared.env_mod.get(),
            Knob::Decay => shared.decay.get(),
            Knob::Accent => shared.accent.get(),
        }
    }

    /// Publish a position to the atomic this knob owns. The waveform is a switch, so its position is snapped first.
    pub fn publish(self, shared: &SharedState, position: f32) {
        match self {
            Knob::Waveform => shared.set_waveform(super::waveform_from_position(position)),
            Knob::Tuning => shared.tuning.set(position),
            Knob::Cutoff => shared.cutoff.set(position),
            Knob::Resonance => shared.resonance.set(position),
            Knob::EnvMod => shared.env_mod.set(position),
            Knob::Decay => shared.decay.set(position),
            Knob::Accent => shared.accent.set(position),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use acid_core::oscillator::Waveform;

    #[test]
    fn every_knob_is_in_all_once_and_in_index_order() {
        assert_eq!(Knob::ALL.len(), 7);
        for (position, knob) in Knob::ALL.iter().enumerate() {
            assert_eq!(knob.index(), position, "{knob:?} has the wrong slot");
            assert_eq!(
                Knob::ALL.iter().filter(|other| *other == knob).count(),
                1,
                "{knob:?} is listed twice"
            );
        }
    }

    #[test]
    fn each_knob_is_named_by_its_own_caption_tip_and_learn_target() {
        let shared = SharedState::new();
        // The silkscreen says exactly this; a caption typo used to fail nothing, because the tip table holds its own
        // strings. The switch is captioned by the wave it is on.
        assert_eq!(
            Knob::ALL.map(|knob| knob.label(&shared)),
            [
                shared.waveform().name(),
                "TUNING",
                "CUT OFF",
                "RESONANCE",
                "ENV MOD",
                "DECAY",
                "ACCENT"
            ]
        );
        for knob in Knob::ALL {
            assert!(knob.label(&shared).is_ascii(), "{knob:?} is not ASCII");
            // A permuted arm is the failure no round trip can see — `publish` and `read` would both follow it — so the two
            // lookups are pinned to the knob's own name. Every knob's tip and learn target are named after it by design.
            assert_eq!(
                format!("{:?}", knob.tip()),
                format!("{knob:?}"),
                "{knob:?} wears another control's tip"
            );
            assert_eq!(
                format!("{:?}", knob.learn_target()),
                format!("{knob:?}"),
                "{knob:?} teaches another control's slot"
            );
        }
    }

    #[test]
    fn publishing_a_knob_moves_exactly_that_ctrl_and_reading_it_returns_it() {
        // The round trip is the contract: if a knob's publish and read ever disagree, the face lies about the sound.
        for knob in Knob::ALL {
            let shared = SharedState::new();
            let before: Vec<f32> = Knob::ALL.iter().map(|other| other.read(&shared)).collect();
            let wanted = if knob.kind() == Kind::Switch {
                1.0
            } else {
                0.75
            };
            knob.publish(&shared, wanted);
            assert!(
                (knob.read(&shared) - wanted).abs() < 1e-6,
                "{knob:?} does not read back what it published: {}",
                knob.read(&shared)
            );
            for (index, other) in Knob::ALL.iter().enumerate() {
                if *other != knob {
                    assert!(
                        (other.read(&shared) - before[index]).abs() < 1e-6,
                        "{knob:?} moved {other:?} as well"
                    );
                }
            }
        }
    }

    #[test]
    fn the_waveform_is_the_only_switch() {
        assert_eq!(Knob::Waveform.kind(), Kind::Switch);
        for knob in Knob::ALL.iter().filter(|knob| **knob != Knob::Waveform) {
            assert_eq!(knob.kind(), Kind::Dial, "{knob:?} should be a dial");
        }
        // A switch snaps to the nearest face, and an exact tie rounds up: 0.4 is nearer the second face, 0.5 is the tie
        // between the second and third.
        let shared = SharedState::new();
        Knob::Waveform.publish(&shared, 0.4);
        assert_eq!(shared.waveform(), Waveform::Square);
        Knob::Waveform.publish(&shared, 0.5);
        assert_eq!(shared.waveform(), Waveform::Sine);
    }

    #[test]
    fn every_knob_teaches_a_different_controller_slot() {
        let mut targets: Vec<CcTarget> = Knob::ALL.iter().map(|knob| knob.learn_target()).collect();
        let count = targets.len();
        targets.sort_by_key(|target| format!("{target:?}"));
        targets.dedup();
        assert_eq!(targets.len(), count, "two knobs teach the same target");
    }
}
