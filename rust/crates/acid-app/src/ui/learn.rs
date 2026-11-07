//! MIDI learn: press LEARN, click the knob you want to control, then move a control on the MIDI box.
//!
//! The panel owns the flow; the MIDI thread only records the first controller it sees while `learn_active` is set.
//! Nothing here touches the audio path: a learned CC writes the same atomics the mouse writes.

use std::sync::atomic::Ordering;

use crate::midi::cc::CcTarget;
use crate::state::SharedState;

/// Where the learn flow is.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Step {
    #[default]
    Idle,
    /// LEARN was pressed: the next knob click says which control to teach.
    Choosing,
    /// A control was picked: the next controller that arrives is bound to it.
    Listening(CcTarget),
}

/// The LEARN key's face — it says what it is waiting for.
pub fn label(step: Step) -> &'static str {
    match step {
        Step::Idle => "LEARN",
        Step::Choosing => "PICK",
        Step::Listening(_) => "MOVE",
    }
}

/// The one-line hint for the top strip, or `None` when nothing is being learned.
pub fn hint(step: Step) -> Option<&'static str> {
    match step {
        Step::Idle => None,
        Step::Choosing => Some("LEARN: click a knob"),
        Step::Listening(_) => Some("LEARN: move a MIDI control"),
    }
}

/// Stop listening. The shared flag must be cleared too, or the MIDI thread keeps swallowing controllers.
pub fn cancel(shared: &SharedState) {
    shared.learn_active.store(false, Ordering::Relaxed);
    shared.learn_cc.store(-1, Ordering::Relaxed);
}

/// Pick a control to teach: start listening on the MIDI side.
pub fn choose(target: CcTarget, shared: &SharedState) -> Step {
    shared.learn_cc.store(-1, Ordering::Relaxed);
    shared.learn_active.store(true, Ordering::Relaxed);
    Step::Listening(target)
}

/// The controller the MIDI thread has seen since listening began, if any.
pub fn heard(shared: &SharedState) -> Option<u8> {
    match shared.learn_cc.load(Ordering::Relaxed) {
        cc if cc >= 0 => Some(cc as u8),
        _ => None,
    }
}

/// The control a tone knob teaches. Index 0 is the waveform selector, which no controller drives; 1..=6 are the
/// knobs left to right. Volume is not in the row — it has its own corner of the panel.
pub fn control_for_knob(index: usize) -> Option<CcTarget> {
    match index {
        1 => Some(CcTarget::Tuning),
        2 => Some(CcTarget::Cutoff),
        3 => Some(CcTarget::Resonance),
        4 => Some(CcTarget::EnvMod),
        5 => Some(CcTarget::Decay),
        6 => Some(CcTarget::Accent),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_key_says_what_it_is_waiting_for() {
        assert_eq!(label(Step::Idle), "LEARN");
        assert_eq!(label(Step::Choosing), "PICK");
        assert_eq!(label(Step::Listening(CcTarget::Cutoff)), "MOVE");
        assert_eq!(hint(Step::Idle), None);
        assert!(hint(Step::Choosing).unwrap().contains("click a knob"));
        assert!(
            hint(Step::Listening(CcTarget::Volume))
                .unwrap()
                .contains("MIDI")
        );
    }

    #[test]
    fn choosing_starts_listening_and_cancelling_stops_it() {
        let shared = SharedState::new();
        let step = choose(CcTarget::Cutoff, &shared);
        assert_eq!(step, Step::Listening(CcTarget::Cutoff));
        assert!(
            shared
                .learn_active
                .load(std::sync::atomic::Ordering::Relaxed)
        );
        assert_eq!(heard(&shared), None, "nothing has arrived yet");

        cancel(&shared);
        assert!(
            !shared
                .learn_active
                .load(std::sync::atomic::Ordering::Relaxed)
        );
        assert_eq!(heard(&shared), None);
    }
}
