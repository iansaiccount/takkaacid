//! The playhead on both faces: the 303's LEDs over the step keys, and the drum grid's washed column.
//!
//! One file because the round's claim is that the two faces agree. Both draw the step that **sounds**, which is
//! `sequencer::sounding_step(position, reversed)` — so under `REVERSE` the light walks back along the keys while the notes stay
//! where they are written, and a marker on the clock would be pointing at the one step that is not sounding.

use acid_app::device::{DeviceKind, Scheme, Transport};
use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, step_cells};
use acid_app::ui::{PanelState, draw_panel, drum_panel, strip::StripCell, theme};
use acid_core::drums::Drum;
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

/// The wash's colour, from the one definition the face paints with — so this test and the grid cannot drift apart over a
/// number, the way `the_selected_row_is_plated` keeps its own copy of the row band's alpha.
fn wash(scheme: Scheme) -> Color32 {
    drum_panel::playhead_fill(theme::palette(scheme))
}

/// Where a fill sits in the paint order, by the rect it was drawn on.
fn fill_index_at(filled: &[(Rect, Color32)], rect: Rect) -> Option<usize> {
    filled.iter().position(|(painted, _)| {
        (painted.left() - rect.left()).abs() < 0.5
            && (painted.top() - rect.top()).abs() < 0.5
            && (painted.width() - rect.width()).abs() < 0.5
            && (painted.height() - rect.height()).abs() < 0.5
    })
}

/// The drum grid's playhead: the column the line is on, washed in the lamp. Read back from the painted shapes, so "there is a
/// playhead" means "something was drawn", not "a flag is set".
#[test]
fn the_wash_is_on_the_column_that_sounds() {
    let l = layout(screen());
    let ctx = Context::default();

    let painted = |step: usize, reverse_active: bool, scheme: Scheme| {
        let shared = SharedState::new();
        running_at(&shared, step, reverse_active);
        let mut panel = panel_showing(DeviceKind::Drums);
        panel.scheme = scheme;
        fills(&paint(&ctx, &mut panel, &shared))
    };

    // Forwards: the wash is on the step the clock is on.
    let filled = painted(3, false, Scheme::Black);
    assert_eq!(
        fill_index_at(&filled, drum_panel::column_rect(&l, 3)).map(|at| filled[at].1),
        Some(wash(Scheme::Black)),
        "the playing column is washed in the lamp"
    );
    assert!(
        fill_index_at(&filled, drum_panel::column_rect(&l, 12)).is_none(),
        "and no other column is"
    );

    // Backwards: the line reads the bar from its other end, so the wash is on the step that sounds.
    let filled = painted(3, true, Scheme::Black);
    assert_eq!(
        fill_index_at(&filled, drum_panel::column_rect(&l, 12)).map(|at| filled[at].1),
        Some(wash(Scheme::Black)),
        "reversed, the wash follows the sound"
    );
    assert!(
        fill_index_at(&filled, drum_panel::column_rect(&l, 3)).is_none(),
        "and is not left on the step the clock is on"
    );

    // The palette decides the ink: the same frame on the other scheme washes in its own lamp.
    let filled = painted(3, false, Scheme::ChoirPastel);
    assert_eq!(
        fill_index_at(&filled, drum_panel::column_rect(&l, 3)).map(|at| filled[at].1),
        Some(wash(Scheme::ChoirPastel)),
        "the light scheme washes in its own lamp"
    );
    assert_ne!(
        wash(Scheme::Black),
        wash(Scheme::ChoirPastel),
        "a hard-coded wash would pass both assertions above; this is what says it does not"
    );
}

/// A silence has no playhead: the 303's LEDs go out and the grid's column goes dark. **Both** silences — the guide says
/// "stopped or paused", and a gate of `transport != Stopped` would leave the page lying while a stop-only test stayed green.
#[test]
fn a_stopped_or_paused_machine_has_no_playhead() {
    let l = layout(screen());
    let ctx = Context::default();

    for transport in [Transport::Stopped, Transport::Paused] {
        let shared = SharedState::new();
        running_at(&shared, 3, false);
        shared.set_transport(transport);
        let mut panel = panel_showing(DeviceKind::Drums);
        let filled = fills(&paint(&ctx, &mut panel, &shared));

        for step in 0..STEPS_PER_PATTERN {
            assert!(
                fill_index_at(&filled, drum_panel::column_rect(&l, step)).is_none(),
                "column {step} is washed on a {transport:?} machine"
            );
        }
    }
}

/// The reason the wash was chosen over recolouring the cells: a column struck on every row must still show the line, and the
/// strikes must still read through it. The paint *order* matters as much as the colour — a wash drawn before the cells is in
/// the shape list and invisible on screen, so a test that only asked "is there a fill at that rect" would pass a face with no
/// playhead on it at all.
#[test]
fn the_wash_reads_over_a_column_of_strikes() {
    let l = layout(screen());
    let ctx = Context::default();
    let shared = SharedState::new();
    running_at(&shared, 3, true);
    let sounding = STEPS_PER_PATTERN - 1 - 3;
    for drum in Drum::ALL {
        let mut step = shared.pattern().steps[sounding];
        step.hit(drum, false);
        shared.set_step(sounding, step);
    }

    let mut panel = panel_showing(DeviceKind::Drums);
    let filled = fills(&paint(&ctx, &mut panel, &shared));
    let at = fill_index_at(&filled, drum_panel::column_rect(&l, sounding))
        .unwrap_or_else(|| panic!("the sounding column carries no wash, so there is no playhead"));
    assert_eq!(
        filled[at].1,
        wash(Scheme::Black),
        "the wash is the lamp's own colour"
    );

    for drum in Drum::ALL {
        let cell = drum_panel::cell_rect(&l, drum, sounding);
        let painted = fill_index_at(&filled, cell)
            .unwrap_or_else(|| panic!("{drum:?}: the struck cell was not painted"));
        assert_eq!(
            filled[painted].1,
            theme::BLACK.silkscreen_dim,
            "{drum:?}: the strike keeps its own colour under the wash"
        );
        assert!(
            at > painted,
            "{drum:?}: the wash must be painted after the cell, or it is lit under an opaque square"
        );
    }
}
