// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
#[allow(dead_code)]
pub mod theme;
pub mod layout;
pub mod panel;
pub mod knob;
pub mod step_key;
