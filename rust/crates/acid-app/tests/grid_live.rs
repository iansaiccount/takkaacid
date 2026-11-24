//! The `GRID` key and the well beside it: the pattern's grid on the face, what a press costs, and that a press mid-bar
//! neither stalls the clock nor double-triggers a step.
//!
//! The press is a **pattern edit**, not a transport request — the grid belongs to the pattern — so this file drives the
//! real `draw_panel` and then reads the same `SharedState` the audio thread reads.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{Scheme, Transport};
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, step_cells};
use acid_app::ui::perform::{self, Perform};
use acid_app::ui::strip::StripCell;
use acid_app::ui::transport::{self, Cell};
use acid_app::ui::{PanelState, draw_panel, theme};
use acid_core::pattern::{Grid, PATTERN_SLOTS, Pattern, Step};
use acid_core::pitch::Pitch;
use eframe::egui::{Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, theme::DESIGN_SIZE)
}

fn paint_at(
    ctx: &Context,
    window: Rect,
    panel: &mut PanelState,
    shared: &SharedState,
) -> eframe::egui::FullOutput {
    let input = RawInput {
        screen_rect: Some(window),
        events: Vec::new(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

fn paint(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> eframe::egui::FullOutput {
    paint_at(ctx, screen(), panel, shared)
}

/// A click: a settled pointer, a press and a release, because a widget must exist before it is hit.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(screen()),
            events: vec![Event::PointerMoved(at), button(true)],
            ..Default::default()
        },
        |ui| draw_panel(panel, shared, ui),
    );
    output.textures_delta.clear();
    let mut output = ctx.run_ui(
        RawInput {
            screen_rect: Some(screen()),
            events: vec![button(false)],
            ..Default::default()
        },
        |ui| draw_panel(panel, shared, ui),
    );
    output.textures_delta.clear();
}

fn notice(panel: &PanelState) -> String {
    panel
        .notice
        .as_ref()
        .map(|(said, _)| said.clone())
        .unwrap_or_default()
}

/// The `GRID` key and the well beside it, asked for by name from the row's catalogue.
fn grid_key() -> Pos2 {
    transport::cell(layout(screen()).transport, Cell::Grid).center()
}

/// The CLEAR key, from the performance row's catalogue.
fn clear_key() -> Pos2 {
    perform::cell(layout(screen()).perform.area, Perform::Clear).center()
}

/// The panel showing one acid device: `main` fills the strip from the rack, so this is the same mirror it would fill.
fn panel_showing() -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = vec![StripCell {
        name: "ACID 1".to_string(),
        number: 1,
        scheme: Scheme::Black,
        running: false,
        muted: false,
        kind: acid_app::device::DeviceKind::Acid,
    }];
    panel.strip.selected = 0;
    panel
}

/// The key cycles the catalogue mid-bar, the engine holds the same bar, the well's value is the pattern's, and a block of
/// audio a half-step long still advances the position by exactly one step — no stall, no double trigger.
#[test]
fn the_key_cycles_the_grid_mid_bar_and_the_engine_hears_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    shared.set_transport(Transport::Playing);
    let mut panel = panel_showing();
    // The well prints the grid the pattern is on: its caption is on the face before any press.
    let painted = paint(&ctx, &mut panel, &shared);
    assert!(
        said(&painted).iter().any(|text| text == "1/16"),
        "the well must print the pattern's grid: {:?}",
        said(&painted)
    );
    // Let the engine itself run to the middle of a bar: five sixteenths, position 5 of 16, and the clock knows it.
    let mut renderer = AudioRenderer::new(48_000.0, 2, Arc::clone(&shared), 2_048);
    let bpm = shared.tempo_bpm.get();
    let sixteenth =
        acid_core::timing::samples_per_step(bpm, 48_000.0, Grid::Sixteenth.steps_per_bar());
    let mut out = vec![0.0f32; (sixteenth * 5.5) as usize * 2];
    renderer.render_into(&mut out);
    let mid_bar = shared.position.load(Ordering::Relaxed);
    assert_eq!(mid_bar, 5, "five sixteenths in, the line is on step six");

    // An armed CLEAR was armed against the pattern as it was: a grid press may rest steps inside it.
    panel.clear_armed_at = Some(std::time::Instant::now());
    click(&ctx, &mut panel, &shared, grid_key());
    assert_eq!(
        panel.pattern.grid,
        Grid::ThirtySecond,
        "sixteenths cycle to thirty-seconds"
    );
    assert!(
        panel.clear_armed_at.is_none(),
        "a grid press must spend an armed CLEAR"
    );
    assert_eq!(
        shared.pattern().grid,
        Grid::ThirtySecond,
        "and the engine holds the same bar as the face"
    );
    assert!(panel.dirty, "a grid press is a change to save");
    assert_eq!(notice(&panel), "16 TO 32 STEPS");
    // The clock, mid-bar: a block half a thirty-second long crosses one step — never two (a double trigger) and never
    // none (a stall), which is what a rescaled step in flight could do.
    let half =
        (acid_core::timing::samples_per_step(bpm, 48_000.0, Grid::ThirtySecond.steps_per_bar())
            / 2.0) as usize;
    let mut out = vec![0.0f32; (half + 4) * 2];
    let before = shared.position.load(Ordering::Relaxed);
    renderer.render_into(&mut out);
    let after = shared.position.load(Ordering::Relaxed);
    assert_eq!(
        after,
        before + 1,
        "half a thirty-second advances exactly one step (was {before})"
    );
    let painted = paint(&ctx, &mut panel, &shared);
    assert!(
        said(&painted).iter().any(|text| text == "1/32"),
        "the well follows the pattern onto the new grid: {:?}",
        said(&painted)
    );

    // A second press: thirty-seconds cycle to eighth-note triplets, and nothing sounded past twelve in this empty bar.
    click(&ctx, &mut panel, &shared, grid_key());
    assert_eq!(panel.pattern.grid, Grid::EighthTriplet);
    assert_eq!(shared.pattern().grid, Grid::EighthTriplet);
    let painted = paint(&ctx, &mut panel, &shared);
    assert!(
        said(&painted).iter().any(|text| text == "1/8T"),
        "and the well says 1/8T: {:?}",
        said(&painted)
    );
    assert_eq!(notice(&panel), "32 TO 12 STEPS");
}

/// Narrowing the bar rests the steps it cannot hold, and the status line counts the ones that **sounded** — not the slots
/// that stopped being drawn.
#[test]
fn narrowing_the_bar_counts_the_sound_it_rests() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[5] = Step::note(Pitch::A, 1); // inside every bar
    steps[20] = Step::note(Pitch::C, 2); // past twelve
    steps[21] = Step::note(Pitch::D, 2); // past twelve
    steps[22] = Step::rest(); // outside the bar, but it is not sound
    shared.set_pattern(Arc::new(Pattern::from_steps_in(Grid::ThirtySecond, steps)));
    let mut panel = panel_showing();
    paint(&ctx, &mut panel, &shared);
    // The border is on step 21 of thirty-two; the press narrows the bar to twelve and must pull it in with it.
    panel.selected = 20;

    click(&ctx, &mut panel, &shared, grid_key());

    assert_eq!(panel.pattern.grid, Grid::EighthTriplet);
    assert_eq!(panel.pattern.len(), 12, "the row is twelve keys now");
    assert_eq!(notice(&panel), "32 TO 12 STEPS, 2 CLEARED");
    assert_eq!(
        panel.selected, 11,
        "the border is pulled into the arriving bar, not left off the row"
    );
    for index in [20, 21, 22] {
        assert!(
            !panel.pattern.steps[index].on,
            "step {} must be rested, not hidden",
            index + 1
        );
    }
    assert!(
        panel.pattern.steps[5].on,
        "the steps inside the bar are untouched"
    );
    assert!(
        !shared.pattern().steps[20].on,
        "and the engine's bar rests it too"
    );
}

/// `CLEAR` empties the bar without leaving it: a key a player meets in the first minute must not turn thirty-two keys
/// into sixteen.
#[test]
fn clear_keeps_the_grid() {
    let ctx = Context::default();
    let shared = SharedState::new();
    shared.set_pattern(Arc::new(Pattern::from_steps_in(
        Grid::ThirtySecond,
        [Step::note(Pitch::A, 1); PATTERN_SLOTS],
    )));
    let mut panel = panel_showing();
    paint(&ctx, &mut panel, &shared);

    // Two clicks: the first arms, the second clears.
    click(&ctx, &mut panel, &shared, clear_key());
    click(&ctx, &mut panel, &shared, clear_key());

    assert_eq!(
        shared.pattern().grid,
        Grid::ThirtySecond,
        "the grid survives"
    );
    assert_eq!(shared.pattern().len(), 32, "and so does the length");
    assert_eq!(panel.pattern.len(), 32, "and the row the face draws");
    assert!(
        shared.pattern().iter().all(|step| !step.on),
        "the bar is empty"
    );
}

/// The note has to fit a thirty-second key. Both the text scale and the cell width are linear in the panel, so the ratio
/// is what must hold at every size; this checks the design size and the smallest window the panel draws (`scale` clamps at
/// 0.65, so the panel is 637 × 436).
#[test]
fn the_note_fits_a_thirty_second_key() {
    let shared = SharedState::new();
    let step = Step::note(Pitch::CSharp, 2);
    shared.set_pattern(Arc::new(Pattern::from_steps_in(
        Grid::ThirtySecond,
        [step; PATTERN_SLOTS],
    )));
    for window in [
        theme::DESIGN_SIZE,
        vec2(theme::DESIGN_SIZE.x * 0.65, theme::DESIGN_SIZE.y * 0.65),
    ] {
        let ctx = Context::default();
        let rect = Rect::from_min_size(Pos2::ZERO, window);
        let l = layout(rect);
        let cells = step_cells(l.steps, Grid::ThirtySecond.steps_per_bar());
        assert_eq!(cells.len(), 32);
        for cell in &cells {
            assert!(
                l.steps.contains_rect(*cell),
                "a key escapes the band: {cell:?}"
            );
        }

        let mut panel = panel_showing();
        let painted = paint_at(&ctx, rect, &mut panel, &shared);

        // Read it back from the picture: every label the face drew inside the step band must be narrower than the key
        // it was drawn in — the note and the corner number alike.
        let mut labels = 0;
        for (text, at) in texts(&painted) {
            let Some(cell) = cells.iter().find(|cell| cell.contains(at)) else {
                continue;
            };
            labels += 1;
            assert!(
                text < cell.width(),
                "a {text:.1} px label does not fit a {:.1} px key at {window:?}",
                cell.width()
            );
        }
        assert_eq!(
            labels,
            Grid::ThirtySecond.steps_per_bar() * 2,
            "every key drew its note and its number"
        );
    }
}

/// Every label in the frame: its width and where it was drawn, in paint order.
fn texts(output: &eframe::egui::FullOutput) -> Vec<(f32, Pos2)> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<(f32, Pos2)>) {
        match shape {
            eframe::egui::Shape::Text(text) => out.push((text.galley.rect.width(), text.pos)),
            eframe::egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// Every word the frame printed, in paint order.
fn said(output: &eframe::egui::FullOutput) -> Vec<String> {
    fn walk(shape: &eframe::egui::Shape, out: &mut Vec<String>) {
        match shape {
            eframe::egui::Shape::Text(text) => out.push(text.galley.text().to_owned()),
            eframe::egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// The slide look-ahead is `(i + 1) % len`: a slide on the last step of a twelve-step bar glides into the bar's first
/// note, so the keycap must draw its amber cue on step 12 exactly as it does anywhere else in the bar.
#[test]
fn a_slide_on_the_last_step_reads_its_next_step_across_the_wrap() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    steps[0] = Step::note(Pitch::C, 1);
    steps[11] = Step {
        slide: true,
        ..Step::note(Pitch::A, 1)
    };
    let sliding = Pattern::from_steps_in(Grid::EighthTriplet, steps);

    let shapes = |pattern: Pattern| {
        shared.set_pattern(Arc::new(pattern));
        let mut panel = panel_showing();
        paint(&ctx, &mut panel, &shared).shapes.len()
    };

    let with_slide = shapes(sliding);
    // The same bar with the slide cleared: the only difference on the face is the cue — an amber strip and two lines.
    let mut cleared = sliding;
    cleared.steps[11].slide = false;
    let without = shapes(cleared);
    assert!(
        with_slide >= without + 3,
        "the slide on step 12 drew no cue: {with_slide} vs {without} shapes"
    );
}
