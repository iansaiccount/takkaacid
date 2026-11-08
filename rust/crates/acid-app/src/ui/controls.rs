//! Small controls two panel rows both draw: a teachable target, and a key that lights.

use eframe::egui;

use crate::midi;
use crate::state::SharedState;

use super::{PanelState, learn, tips, transport};

/// A click on a control that can be taught, while a learn is choosing.
///
/// Guarded on `Choosing` rather than on the click alone: outside a learn these are the player's controls, and a stray
/// click on the tempo readout must not start a binding that then swallows the next controller they touch.
pub(super) fn learn_pick(
    ui: &mut egui::Ui,
    state: &mut PanelState,
    shared: &SharedState,
    id: &str,
    rect: egui::Rect,
    control: midi::cc::CcTarget,
) {
    if state.learn != learn::Step::Choosing {
        return;
    }
    let hit = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    if hit.clicked() {
        state.learn = learn::choose(control, shared);
    }
}
/// A panel button with its hover help attached: draw it, explain it, and say whether it was clicked.
///
/// One helper for every button, so the tip follows the control that was actually drawn — the CLEAR key reads `CLEAR?`
/// while it is armed, and its help still describes CLEAR.
pub(super) fn switch_key(
    ui: &mut egui::Ui,
    state: &PanelState,
    id: &str,
    rect: egui::Rect,
    label: &str,
    lit: bool,
    control: tips::Control,
) -> bool {
    let response = transport::transport_button(ui, id, rect, label, lit);
    let clicked = response.clicked();
    tips::hover(&response, state, control);
    clicked
}
