//! The playhead on both faces: the 303's LEDs over the step keys, and the drum grid's washed column.
//!
//! One file because the round's claim is that the two faces agree. Both draw the step that **sounds**, which is
//! `sequencer::sounding_step(position, reversed)` — so under `REVERSE` the light walks back along the keys while the notes stay
//! where they are written, and a marker on the clock would be pointing at the one step that is not sounding.

use acid_app::device::{DeviceKind, Scheme, Transport};
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, step_cells};
use acid_app::ui::{PanelState, draw_panel, strip::StripCell, theme};
use acid_core::pattern::STEPS_PER_PATTERN;
use eframe::egui::{Color32, Context, FullOutput, Pos2, RawInput, Rect, Shape, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, acid_app::ui::theme::DESIGN_SIZE)
}

fn paint(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(screen()),
        events: Vec::new(),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// Every filled rectangle and circle, in paint order, with its colour. Order is the point: a wash drawn before the cells it
/// should sit over is in this list and invisible on screen.
fn fills(output: &FullOutput) -> Vec<(Rect, Color32)> {
    fn walk(shape: &Shape, out: &mut Vec<(Rect, Color32)>) {
        match shape {
            Shape::Rect(rect) if rect.fill != Color32::TRANSPARENT => {
                out.push((rect.rect, rect.fill))
            }
            Shape::Circle(circle) => out.push((
                Rect::from_center_size(
                    circle.center,
                    vec2(circle.radius * 2.0, circle.radius * 2.0),
                ),
                circle.fill,
            )),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// Which step keys have a lit LED, by cell index. The LED is the only filled lamp inside the step band, so this reads the
/// playhead out of the picture rather than out of a private field.
fn lit_step_keys(output: &FullOutput, band: Rect) -> Vec<usize> {
    let lamp = theme::BLACK.led_on;
    let cells = step_cells(band);
    fills(output)
        .into_iter()
        .filter(|(rect, colour)| *colour == lamp && band.contains(rect.center()))
        .filter_map(|(rect, _)| cells.iter().position(|cell| cell.contains(rect.center())))
        .collect()
}

fn panel_showing(kind: DeviceKind) -> PanelState {
    let mut panel = PanelState::default();
    // The name the rack gives a new device, from the kind's own catalogue caption: a second hand-kept list is the bug this
    // repo has already paid for twice.
    panel.strip.cells = vec![StripCell {
        name: format!("{} 1", kind.name()),
        number: 1,
        scheme: Scheme::Black,
        running: false,
        muted: false,
        kind,
    }];
    panel.strip.selected = 0;
    panel
}

/// A line running on `step`, with the direction the machine published as `reverse_active`. `reversed` — the *request* — is
/// deliberately left false: a face that reads it instead of the published direction lights the wrong step, and that is what
/// these tests are for.
fn running_at(shared: &SharedState, step: usize, reverse_active: bool) {
    shared.set_reversed(false);
    shared
        .reverse_active
        .store(reverse_active, std::sync::atomic::Ordering::Relaxed);
    shared
        .position
        .store(step, std::sync::atomic::Ordering::Relaxed);
    shared.set_transport(Transport::Playing);
}

/// The 303's playhead, mirrored. An empty pattern, so the only lit LED on the row is the one the transport put there.
#[test]
fn the_303_lights_the_step_that_sounds_when_the_bar_runs_backwards() {
    let l = layout(screen());
    let ctx = Context::default();

    let lit = |reverse_active: bool| {
        let shared = SharedState::new();
        running_at(&shared, 3, reverse_active);
        let mut panel = panel_showing(DeviceKind::Acid);
        lit_step_keys(&paint(&ctx, &mut panel, &shared), l.steps)
    };

    assert_eq!(lit(false), vec![3], "forwards, the light is on the sound");
    assert_eq!(
        lit(true),
        vec![STEPS_PER_PATTERN - 1 - 3],
        "reversed, the bar reads from its other end, so the light does too"
    );
}
