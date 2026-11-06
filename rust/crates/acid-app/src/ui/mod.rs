// The palette and layout constants are the panel's vocabulary: they are introduced here and consumed
// from Phase 06 onwards, so the dead-code lint is waived for the module rather than per constant.
#[allow(dead_code)]
pub mod theme;
pub mod layout;
pub mod panel;
pub mod knob;
pub mod step_key;
pub mod display;

use std::sync::Arc;

use acid_core::pattern::{Pattern, PatternId, STEPS_PER_PATTERN};
use acid_core::timing::DEFAULT_BPM;

use crate::state::SharedState;

/// Everything the panel needs in order to draw itself.
///
/// Phase 07 replaces the interior with reads from `SharedState`; the fields and the `draw_panel` signature
/// are what the wiring depends on, so they are named now.
pub struct PanelState {
    /// The seven knob positions, left to right, in knob domain.
    pub knobs: [f32; 7],
    /// The pattern to draw in the step row.
    pub pattern: Arc<Pattern>,
    /// Which slot that pattern came from, shown in the pattern display.
    pub pattern_id: PatternId,
    /// Tempo in BPM, shown in the tempo display.
    pub tempo: f32,
    /// Transport state: drives the position LED.
    pub playing: bool,
    /// Step the transport is on, `0..16`.
    pub position: usize,
}

impl Default for PanelState {
    fn default() -> Self {
        Self {
            knobs: [0.5; 7],
            pattern: Arc::new(Pattern::empty()),
            pattern_id: PatternId { bank: 0, number: 0 },
            tempo: DEFAULT_BPM,
            playing: false,
            position: 0,
        }
    }
}

/// Paint the whole panel into `ui`: face, displays, knobs, step keys, switches and transport legends.
///
/// Painting uses absolute rects from `layout`, so the panel lands in the same place whatever the Ui's own
/// cursor is doing. Returns nothing: Phase 07 wires the controls to the engine.
pub fn draw_panel(state: &mut PanelState, shared: &SharedState, ui: &mut egui::Ui) {
    let window = ui.max_rect();
    let l = layout::layout(window);
    let s = layout::scale(window);
    // The pattern is whatever the UI thread last published — one refcount bump, no allocation.
    state.pattern = shared.pattern();
    state.playing = shared.is_playing();
    panel::paint_panel(ui.painter(), &l);

    display::paint_display(
        ui.painter(),
        panel::pattern_display_rect(l.top),
        &display::format_pattern(state.pattern_id),
        s,
    );
    display::paint_display(
        ui.painter(),
        panel::tempo_display_rect(l.top),
        &display::format_tempo(state.tempo),
        s,
    );

    for (i, cell) in l.knobs.cells.iter().enumerate() {
        let diameter = cell.width().min(cell.height() * 0.78);
        let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(cell.shrink(2.0 * s)));
        cell_ui.set_clip_rect(*cell);
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("knob", i)),
            theme::KNOB_LABELS[i],
            &mut state.knobs[i],
            diameter,
        );
    }

    let keys = layout::step_cells(l.steps);
    for (i, cell) in keys.iter().enumerate() {
        let step = state.pattern.steps[i % STEPS_PER_PATTERN];
        let view = step_key::step_view(&step, i, state.position, state.playing);
        if step_key::step_key(ui, i, &view, *cell).clicked() {
            // Phase 08 Task 38 refines the gesture; for now a click toggles the note on and off.
            let mut edited = step;
            edited.on = !edited.on;
            shared.set_step(i % STEPS_PER_PATTERN, edited);
            state.pattern = shared.pattern();
        }
    }

    // Switches are engraved legends for now; Phase 07 (Task 33) turns them into controls.
    for (label, cell) in ["BANK", "PATTERN", "MODE"]
        .iter()
        .zip(layout::knob_cells(l.switches, 3))
    {
        paint_switch(ui.painter(), cell, s, label);
    }
    // The transport row keeps its faces blank: the app owns the one working button until Phase 07.
    for cell in layout::knob_cells(l.transport, 4) {
        paint_switch(ui.painter(), cell, s, "");
    }
}

/// A static button face: raised, engraved, with a silkscreen caption when `label` is not empty.
fn paint_switch(painter: &egui::Painter, rect: egui::Rect, scale: f32, label: &str) {
    painter.rect_filled(rect, 4.0 * scale, theme::PANEL_RAISED);
    painter.rect_stroke(
        rect,
        4.0 * scale,
        theme::silkscreen_stroke(scale),
        egui::StrokeKind::Inside,
    );
    if !label.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            theme::legend(12.0 * scale),
            theme::SILKSCREEN,
        );
    }
}
