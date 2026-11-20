//! The drum machine's face: a grid of sixteen voices against the sixteen steps, the selected voice's dials, the filter key
//! and `HIT` — drawn on the panel in place of the 303's rows.
//!
//! Every test here drives the real `draw_panel` through `RawInput` frames, because the grid's behaviour *is* its geometry:
//! a click lands where the pointer is, and the dials turn the voice the label selected.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use acid_app::device::{DeviceKind, Scheme};
use acid_app::knobs::Knob;
use acid_app::state::SharedState;
use acid_app::ui::dispatch::{self, Handled, KeyPress};
use acid_app::ui::drum_panel::{self, Dial, DrumKey};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::shortcuts::Shortcut;
use acid_app::ui::strip::StripCell;
use acid_app::ui::theme::{self, DESIGN_SIZE};
use acid_app::ui::{PanelState, draw_panel};
use acid_core::bank::PatternBank;
use acid_core::drums::{Drum, FilterKind, KitVoice};
use acid_core::pattern::{STEPS_PER_PATTERN, Step};
use acid_core::pitch::Pitch;
use eframe::egui::{
    self, Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape,
    vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, DESIGN_SIZE)
}

fn run(
    ctx: &Context,
    screen_rect: Rect,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(screen_rect),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// A frame with nothing happening — the warm-up every first hit needs, and a way to paint a state.
fn paint(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> FullOutput {
    run(ctx, screen(), panel, shared, vec![])
}

fn press(at: Pos2, modifiers: Modifiers) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    }
}

fn release(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    }
}

fn key_press(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// The same press, flagged as the repeat a held key sends: `dispatch::presses` lets an arrow's repeat through, and a held
/// `shift+down` is meant to keep walking the grid.
fn key_repeat(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: true,
        modifiers,
    }
}

fn key_release(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::default(),
    }
}

/// One key press, through the two calls `main` makes: the frame's events become presses (`dispatch::presses`), and each
/// press is resolved (`dispatch::handle`). The shortcuts and the step editor do not run inside `draw_panel`, so this is the
/// seam a test presses — the same one `tests/settings_live.rs` uses.
fn voice_press(
    panel: &mut PanelState,
    shared: &SharedState,
    bank: &mut PatternBank,
    key: Key,
    shift: bool,
) -> Handled {
    let mut handled = Handled::Nothing;
    let kind = panel.showing_kind();
    for press in dispatch::presses(
        &[key_press(
            key,
            Modifiers {
                shift,
                ..Modifiers::default()
            },
        )],
        kind,
    ) {
        handled = dispatch::handle(press, panel, shared, bank);
    }
    handled
}

/// A settled pointer, a press and a release — three frames, because a widget must exist before it is hit.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2, mods: Modifiers) {
    run(
        ctx,
        screen(),
        panel,
        shared,
        vec![Event::PointerMoved(at), Event::ModifiersChanged(mods)],
    );
    run(ctx, screen(), panel, shared, vec![press(at, mods)]);
    run(ctx, screen(), panel, shared, vec![release(at)]);
}

/// The panel showing one drum device: `main` fills the strip from the rack, so this is the same mirror it would fill.
fn drum_panel_state() -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = vec![StripCell {
        name: "DRUMS 1".to_string(),
        number: 1,
        scheme: Scheme::Black,
        running: false,
        muted: false,
        kind: DeviceKind::Drums,
    }];
    panel.strip.selected = 0;
    panel
}

/// The same panel showing an acid device, for the rows a drum face must not draw.
fn acid_panel_state() -> PanelState {
    let mut panel = drum_panel_state();
    panel.strip.cells[0].kind = DeviceKind::Acid;
    panel
}

/// Every filled rectangle and its colour: the face, the wells, the grid's squares, the plate.
fn fills(output: &FullOutput) -> Vec<(Rect, egui::Color32)> {
    fn walk(shape: &Shape, out: &mut Vec<(Rect, egui::Color32)>) {
        match shape {
            Shape::Rect(rect) if rect.fill != egui::Color32::TRANSPARENT => {
                out.push((rect.rect, rect.fill));
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

/// Every stroked rectangle — a beat frame, a key's border — so a test can ask where one was drawn.
fn strokes(output: &FullOutput) -> Vec<Rect> {
    fn walk(shape: &Shape, out: &mut Vec<Rect>) {
        match shape {
            Shape::Rect(rect) if rect.fill == egui::Color32::TRANSPARENT => out.push(rect.rect),
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

/// Whether a stroked rect is the one painted on `rect` (a shape may shrink by a pixel or two).
fn framed(stroked: &[Rect], rect: Rect) -> bool {
    stroked.iter().any(|painted| {
        (painted.center() - rect.center()).length() < 1.5
            && (painted.width() - rect.width()).abs() < 4.0
            && (painted.height() - rect.height()).abs() < 4.0
    })
}

/// Every line of text the face drew, with the colour it was drawn in.
fn texts(output: &FullOutput) -> Vec<(String, egui::Color32)> {
    fn walk(shape: &Shape, out: &mut Vec<(String, egui::Color32)>) {
        match shape {
            Shape::Text(text) => out.push((text.galley.text().to_string(), text.fallback_color)),
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

/// The fill painted exactly on `rect`, if the frame painted one there.
fn fill_at(filled: &[(Rect, egui::Color32)], rect: Rect) -> Option<egui::Color32> {
    filled
        .iter()
        .find(|(painted, _)| {
            (painted.left() - rect.left()).abs() < 0.5
                && (painted.top() - rect.top()).abs() < 0.5
                && (painted.width() - rect.width()).abs() < 0.5
                && (painted.height() - rect.height()).abs() < 0.5
        })
        .map(|(_, colour)| *colour)
}

fn said(output: &FullOutput) -> Vec<String> {
    texts(output).into_iter().map(|(text, _)| text).collect()
}

#[test]
fn the_face_names_the_kit_it_plays() {
    // A drum machine is born on the `Init` kit, which has no samples: a face that never says the kit's name reads as a broken
    // machine, and the card's `KIT` row is a card away. Both read the state's kit, so the face and the card cannot disagree.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();

    let painted = said(&paint(&ctx, &mut panel, &shared));
    assert!(
        painted.iter().any(|text| text == "KIT Init"),
        "a fresh drum machine does not say which kit it has: {painted:?}"
    );

    let kit = acid_app::kits::factory_kits(48_000.0)
        .into_iter()
        .next()
        .expect("the first factory kit");
    let name = kit.name.clone();
    shared.adopt_kit(kit);
    let painted = said(&paint(&ctx, &mut panel, &shared));
    assert!(
        painted.iter().any(|text| *text == format!("KIT {name}")),
        "the face does not name the kit it plays: {painted:?}"
    );
}

#[test]
fn the_grid_has_sixteen_labelled_rows_and_sixteen_cells() {
    // The grid is the catalogue's shape, not a second sixteen written down beside it.
    assert_eq!(Drum::ALL.len(), 16);
    assert_eq!(STEPS_PER_PATTERN, 16);

    let l = layout(screen());
    let grid = drum_panel::grid_area(&l);
    let mut rows: Vec<Rect> = Vec::new();
    for drum in Drum::ALL {
        let row = drum_panel::row_band(&l, drum);
        assert!(grid.contains_rect(row), "{drum:?}'s row escapes the grid");
        let label = drum_panel::label_rect(&l, drum);
        assert!(row.contains_rect(label), "{drum:?}'s label escapes its row");
        assert!(
            label.right() <= drum_panel::cell_rect(&l, drum, 0).left() + 0.01,
            "{drum:?}'s label overlaps the first cell"
        );
        for step in 0..STEPS_PER_PATTERN {
            let cell = drum_panel::cell_rect(&l, drum, step);
            assert!(
                row.contains_rect(cell),
                "{drum:?}'s step {step} escapes its row"
            );
            assert!(cell.width() > 0.0 && cell.height() > 0.0);
        }
        if let Some(previous) = rows.last() {
            assert!(
                previous.bottom() <= row.top() + 0.01,
                "two rows share a line"
            );
        }
        rows.push(row);
    }

    // And the frame paints every label: the gutter is the only thing naming a voice. The selected voice's name is also the
    // caption over the dials, so it appears twice — a count, not `any`, or a dropped gutter label would hide behind it.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let painted = said(&paint(&ctx, &mut panel, &shared));
    for drum in Drum::ALL {
        let wanted = if drum == panel.drum.selected { 2 } else { 1 };
        let count = painted.iter().filter(|text| *text == drum.label()).count();
        assert_eq!(
            count, wanted,
            "{drum:?} is labelled {count} times, not {wanted}: {painted:?}"
        );
    }
}

#[test]
fn a_click_arms_a_voice_on_a_step() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::cell_rect(&l, Drum::Snare, 4).center(),
        Modifiers::default(),
    );

    let step = shared.pattern().steps[4];
    assert!(step.is_hit(Drum::Snare), "the voice is on the step");
    assert!(step.on, "a drum step sounds because a voice does");
    assert!(panel.dirty, "the pattern is in the save file");
}

#[test]
fn a_shift_click_accents_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::cell_rect(&l, Drum::Kick, 2).center(),
        shift,
    );

    let step = shared.pattern().steps[2];
    assert!(step.is_hit(Drum::Kick));
    assert!(step.is_accented(Drum::Kick), "shift is the accent gesture");
    assert!(
        !shared.pattern().steps[1].is_hit(Drum::Kick),
        "no neighbour moved"
    );
}

#[test]
fn clicking_a_voices_label_selects_without_sounding_it() {
    // The label selects the voice for the dials, and that is all: a selection is not a hit, and `HIT` is how a voice is
    // heard. (It used to audition, added when a machine could arrive with no kit and one click was the only way to find out
    // which row was silent; the kit arrives now, and the player hears an unsolicited hit on every row they click.)
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    shared.take_audition();

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::label_rect(&l, Drum::Snare).center(),
        Modifiers::default(),
    );

    assert_eq!(panel.drum.selected, Drum::Snare, "the label still selects");
    assert_eq!(
        shared.take_audition(),
        None,
        "a selection is not a hit; `HIT` plays the selected voice"
    );

    // A `SHIFT`-click is still the mute: a mute is not a sound, and neither is a selection.
    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::label_rect(&l, Drum::Clap).center(),
        Modifiers::SHIFT,
    );
    assert_eq!(shared.take_audition(), None, "a mute is not a preview");
    assert_eq!(panel.drum.selected, Drum::Snare, "a mute does not select");
}

#[test]
fn clicking_a_label_selects_that_voice_for_the_knobs() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    // A value the Kick does not have, so the dials can only be reading the selected voice.
    let mut kit = (*shared.kit()).clone();
    kit.voices[Drum::Clap.index()].cutoff = 0.25;
    shared.set_kit(Arc::new(kit));

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::label_rect(&l, Drum::Clap).center(),
        Modifiers::default(),
    );

    assert_eq!(
        panel.drum.selected,
        Drum::Clap,
        "the label selects the voice"
    );
    let kit = shared.kit();
    let voice = &kit.voices[Drum::Clap.index()];
    for knob in Knob::ALL {
        let dial = Dial::for_knob(knob);
        assert!(
            (panel.knobs[knob.index()] - dial.read(voice)).abs() < 1e-6,
            "{dial:?} does not show the clap's value"
        );
    }
    assert!(
        (panel.knobs[Knob::EnvMod.index()] - 0.25).abs() < 1e-6,
        "CUT OFF is the clap's own cutoff, at the 303's ENV MOD slot"
    );

    // The caption over the dials says the same thing the row does.
    let painted = said(&paint(&ctx, &mut panel, &shared));
    assert!(
        painted.iter().filter(|text| *text == "CLAP").count() >= 2,
        "the voice is named over the knobs as well as on its row: {painted:?}"
    );
}

#[test]
fn a_shift_click_on_a_label_mutes_the_row_and_the_mask() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    let shift = Modifiers {
        shift: true,
        ..Modifiers::default()
    };
    assert_eq!(shared.drum_mute.load(Ordering::Relaxed), 0);
    // An accented clap on step 1, so the row has an accent to lose.
    let mut struck = shared.pattern().steps[0];
    struck.hit(Drum::Clap, true);
    shared.set_step(0, struck);
    let cell = drum_panel::cell_rect(&l, Drum::Clap, 0);
    let accent = theme::palette(Scheme::Black).led_accent;
    let dim = theme::palette(Scheme::Black).silkscreen_dim;
    assert_eq!(
        fill_at(&fills(&paint(&ctx, &mut panel, &shared)), cell),
        Some(accent),
        "an accented strike is the accent before the mute"
    );

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::label_rect(&l, Drum::Clap).center(),
        shift,
    );

    let bit = 1u32 << Drum::Clap.index();
    assert_eq!(
        shared.drum_mute.load(Ordering::Relaxed) & bit,
        bit,
        "the live mute is the state's mask"
    );
    assert_eq!(panel.drum.selected, Drum::Kick, "a mute does not select");
    assert!(
        !shared.kit().voices[Drum::Clap.index()].mute,
        "the kit's own mute is its starting mask, not the panel's mute"
    );

    // The row says it: a muted label prints dim, and its accented strike keeps its square but loses the accent.
    let output = paint(&ctx, &mut panel, &shared);
    let painted = texts(&output);
    let label = painted
        .iter()
        .find(|(text, _)| text == "CLAP")
        .unwrap_or_else(|| panic!("the row is labelled: {painted:?}"));
    assert_eq!(
        label.1,
        theme::palette(Scheme::Black).silkscreen_dim,
        "a muted voice's label is dim"
    );
    assert_eq!(
        fill_at(&fills(&output), cell),
        Some(dim),
        "a muted row keeps its strike readable and loses the accent"
    );
}

#[test]
fn the_face_paints_from_the_palette() {
    let shared = SharedState::new();
    let l = layout(screen());
    let accent_cell = drum_panel::cell_rect(&l, Drum::Kick, 0);
    shared.set_step(0, {
        let mut step = shared.pattern().steps[0];
        step.hit(Drum::Kick, true);
        step
    });

    let painted = |scheme: Scheme| {
        let mut panel = drum_panel_state();
        panel.scheme = scheme;
        let ctx = Context::default();
        let output = paint(&ctx, &mut panel, &shared);
        let colour = fill_at(&fills(&output), accent_cell)
            .unwrap_or_else(|| panic!("the accented cell was not painted in {scheme:?}"));
        (colour, theme::palette(scheme).led_accent)
    };

    let (dark, dark_accent) = painted(Scheme::Black);
    let (light, light_accent) = painted(Scheme::ChoirPastel);
    assert_eq!(dark, dark_accent, "the strike uses the face's accent");
    assert_eq!(light, light_accent, "and the light scheme's own");
    assert_ne!(
        dark, light,
        "the same frame under two schemes must paint different ink"
    );
}

#[test]
fn the_selected_row_is_plated() {
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    panel.drum.selected = Drum::Kick;
    let l = layout(screen());
    let ctx = Context::default();
    let output = paint(&ctx, &mut panel, &shared);
    let painted = fills(&output);
    let accent = theme::palette(Scheme::Black).led_accent;

    assert_eq!(
        fill_at(&painted, drum_panel::label_rect(&l, Drum::Kick)),
        Some(accent),
        "the selected voice's label is a lit plate"
    );
    assert_ne!(
        fill_at(&painted, drum_panel::label_rect(&l, Drum::Snare)),
        Some(accent),
        "and no other row's label is"
    );
    // The row itself carries a faint band of the same accent, derived from the palette at a fraction of its opacity.
    let band = egui::Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36);
    assert_eq!(
        fill_at(&painted, drum_panel::row_band(&l, Drum::Kick)),
        Some(band),
        "the selected row is banded"
    );
    assert_ne!(
        fill_at(&painted, drum_panel::row_band(&l, Drum::Snare)),
        Some(band),
        "and no other row is"
    );
}

/// The grid's beat frames: every fourth column outlined, so a player finds the beat without counting squares.
#[test]
fn every_fourth_column_is_framed() {
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    let ctx = Context::default();
    let stroked = strokes(&paint(&ctx, &mut panel, &shared));

    assert!(
        framed(&stroked, drum_panel::cell_rect(&l, Drum::Kick, 0)),
        "the downbeat is framed"
    );
    assert!(
        framed(&stroked, drum_panel::cell_rect(&l, Drum::Snare, 4)),
        "and every fourth column on every row is"
    );
    assert!(
        !framed(&stroked, drum_panel::cell_rect(&l, Drum::Kick, 1)),
        "an off-beat cell is not framed"
    );
}

#[test]
fn a_click_lands_on_the_step_under_the_pointer_while_the_line_runs() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());

    // One painted frame, so the panel's mirror is a frame behind what follows.
    let _ = paint(&ctx, &mut panel, &shared);
    // The line is on step 8, and the engine has just put a kick on step 4 — the step the pointer is over. The panel's
    // mirror still says step 4 is empty, which is what makes this the input class the happy path never reaches.
    shared.position.store(7, Ordering::Relaxed);
    let mut engine_step = shared.pattern().steps[3];
    engine_step.hit(Drum::Kick, false);
    shared.set_step(3, engine_step);

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::cell_rect(&l, Drum::Snare, 3).center(),
        Modifiers::default(),
    );

    let pattern = shared.pattern();
    assert!(
        pattern.steps[3].is_hit(Drum::Snare),
        "the hit landed on the step under the pointer"
    );
    assert!(
        pattern.steps[3].is_hit(Drum::Kick),
        "the write started from the pattern as it is, not the stale mirror"
    );
    assert!(
        !pattern.steps[7].is_hit(Drum::Snare),
        "nothing was written where the chase had moved to"
    );
}

#[test]
fn the_grid_stays_hittable_at_both_scale_extremes() {
    for factor in [0.65f32, 1.6] {
        let window = Rect::from_min_size(Pos2::ZERO, DESIGN_SIZE * factor);
        assert!(
            (scale(window) - factor).abs() < 1e-5,
            "{factor}x is not the panel's own clamp"
        );
        let l = layout(window);
        let grid = drum_panel::grid_area(&l);
        assert!(grid.width() > 0.0 && grid.height() > 0.0);
        let mut rows: Vec<Rect> = Vec::new();
        for drum in Drum::ALL {
            let row = drum_panel::row_band(&l, drum);
            assert!(
                grid.contains_rect(row),
                "{factor}x: {drum:?}'s row escapes the grid"
            );
            let label = drum_panel::label_rect(&l, drum);
            assert!(
                row.contains_rect(label),
                "{factor}x: {drum:?}'s label escapes its row"
            );
            let first = drum_panel::cell_rect(&l, drum, 0);
            assert!(
                label.right() <= first.left() + 0.01
                    || label.top() >= first.bottom()
                    || label.bottom() <= first.top(),
                "{factor}x: {drum:?}'s label overlaps the cell beside it"
            );
            for step in 0..STEPS_PER_PATTERN {
                let cell = drum_panel::cell_rect(&l, drum, step);
                assert!(
                    row.contains_rect(cell),
                    "{factor}x: {drum:?}'s step {step} escapes its row"
                );
                assert!(
                    cell.width() > 8.0 && cell.height() > 8.0,
                    "{factor}x: {drum:?}'s step {step} is too small to hit: {cell:?}"
                );
            }
            if let Some(previous) = rows.last() {
                assert!(
                    previous.bottom() <= row.top() + 0.01,
                    "{factor}x: two rows overlap"
                );
                for step in 0..STEPS_PER_PATTERN {
                    assert!(
                        !label.intersects(drum_panel::cell_rect(
                            &l,
                            Drum::ALL[rows.len() - 1],
                            step
                        )),
                        "{factor}x: a label sits on the row above's cells"
                    );
                }
            }
            rows.push(row);
        }
    }
}

#[test]
fn turning_attack_edits_the_selected_voice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    // Every one of the 303's atomics, so a dial that published through its slot instead of the kit cannot pass by writing a
    // different one than the test happened to watch.
    let atomics: Vec<f32> = Knob::ALL.iter().map(|knob| knob.read(&shared)).collect();
    let before = shared.kit().voices[Drum::Kick.index()].attack_ms;

    let attack = drum_panel::dial_cells(&l)[Knob::Waveform.index()].center();
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack)],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![press(attack, Modifiers::default())],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack - vec2(0.0, 18.0))],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![release(attack - vec2(0.0, 18.0))],
    );

    let voice = shared.kit().voices[Drum::Kick.index()].clone();
    assert!(
        voice.attack_ms > before,
        "dragging ATTACK up must lengthen the hit: {} -> {}",
        before,
        voice.attack_ms
    );
    for (knob, was) in Knob::ALL.iter().zip(atomics) {
        assert!(
            (knob.read(&shared) - was).abs() < 1e-6,
            "the drum's ATTACK wrote the 303's {knob:?}: {was} -> {}",
            knob.read(&shared)
        );
    }
}

#[test]
fn a_learn_in_progress_turns_no_drum_dial() {
    // `LEARN` is on every face and asks the player to click a control. The drum face has no `CcTarget` for a voice this
    // phase, so there is nothing there to teach — but the dial must also not *turn*: on a factory kit an edit copies the
    // kit under a new name, and a hand that meant to learn must not end up with `808 (2)` and an unsaved session.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let before = shared.kit().voices[Drum::Kick.index()].clone();
    let name = shared.kit().name.clone();
    let attack = drum_panel::dial_cells(&layout(screen()))[Knob::Waveform.index()].center();
    // The first frame settles which device the editor is on, and a device arriving cancels any learn (`draw_panel`), so the
    // `LEARN` is started on the frame after it — which is the order a player's hands arrive in too.
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack)],
    );
    panel.learn = acid_app::ui::learn::Step::Choosing;
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![press(attack, Modifiers::default())],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack - vec2(0.0, 24.0))],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![release(attack - vec2(0.0, 24.0))],
    );
    // The filter key is the other voice edit on that face, and it copies a factory kit for the same reason.
    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::filter_cell(&layout(screen()), FilterKind::Band).center(),
        Modifiers::default(),
    );
    assert_eq!(
        shared.kit().voices[Drum::Kick.index()].filter,
        before.filter,
        "a learn in progress switched the voice's filter"
    );

    // ...and the drum `SHUFFLE`, which *can* be taught: it picks the controller instead of turning the swing.
    let swing = shared.swing.get();
    let shuffle = drum_panel::key_cell(&layout(screen()), DrumKey::Shuffle).center();
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(shuffle)],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![press(shuffle, Modifiers::default())],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(shuffle - vec2(0.0, 24.0))],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![release(shuffle - vec2(0.0, 24.0))],
    );
    assert_eq!(
        shared.swing.get(),
        swing,
        "a learn in progress turned the drum SHUFFLE"
    );
    let after = shared.kit().voices[Drum::Kick.index()].clone();
    assert_eq!(
        (after.attack_ms, after.decay_ms, after.cutoff),
        (before.attack_ms, before.decay_ms, before.cutoff),
        "a learn in progress turned the voice"
    );
    assert_eq!(
        shared.kit().name,
        name,
        "a learn in progress copied the kit"
    );
    assert!(
        !panel.dirty,
        "a learn in progress marked the session unsaved"
    );
}

#[test]
fn the_filter_key_sets_the_voices_filter() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::filter_cell(&l, FilterKind::Band).center(),
        Modifiers::default(),
    );
    assert_eq!(
        shared.kit().voices[Drum::Kick.index()].filter,
        FilterKind::Band
    );

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::filter_cell(&l, FilterKind::High).center(),
        Modifiers::default(),
    );
    assert_eq!(
        shared.kit().voices[Drum::Kick.index()].filter,
        FilterKind::High
    );
    // The key prints its three faces, each on its own segment.
    let painted = said(&paint(&ctx, &mut panel, &shared));
    for kind in FilterKind::ALL {
        assert!(
            painted.iter().any(|text| text == kind.label()),
            "the {kind:?} segment is not printed: {painted:?}"
        );
    }
}

#[test]
fn the_hit_key_auditions_the_selected_voice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    panel.drum.selected = Drum::Snare;
    let l = layout(screen());

    click(
        &ctx,
        &mut panel,
        &shared,
        drum_panel::key_cell(&l, DrumKey::Hit).center(),
        Modifiers::default(),
    );

    assert_eq!(
        shared.take_audition(),
        Some(Drum::Snare),
        "HIT asks for the voice the label selected"
    );
}

#[test]
fn a_drum_device_does_not_draw_the_303_rows() {
    let ctx = Context::default();
    let shared = SharedState::new();
    shared.cutoff.set(0.2);
    let mut panel = drum_panel_state();

    let painted = said(&paint(&ctx, &mut panel, &shared));
    for gone in ["OCT -", "OCT +", "RANDOM", "CLEAR", "ENV MOD", "ACCENT"] {
        assert!(
            !painted.iter().any(|text| text == gone),
            "{gone} is a 303-only control and must be gone, not inert: {painted:?}"
        );
    }

    // The step row would paint sixteen keycaps, each a note or `--`: a drum face has no step row at all.
    assert!(
        !painted.iter().any(|text| text == "--"),
        "the 303's step row is on the drum face: {painted:?}"
    );

    // The 303's own atomics do not reach a drum face: the dial at the 303's CUT OFF slot shows the voice's sustain.
    assert!(
        (panel.knobs[Knob::Cutoff.index()] - shared.kit().voices[Drum::Kick.index()].sustain).abs()
            < 1e-6,
        "a drum face reads the kit, not the 303's atomics"
    );

    // The acid face still draws them, so this is a branch and not a removal.
    let mut acid = acid_panel_state();
    let painted = said(&paint(&ctx, &mut acid, &shared));
    assert!(painted.iter().any(|text| text == "ENV MOD"));
    assert!(painted.iter().any(|text| text == "RANDOM"));
    assert_eq!(
        painted.iter().filter(|text| *text == "--").count(),
        STEPS_PER_PATTERN,
        "and its own step row: one keycap per step"
    );
}

/// The face's own reading of a voice: the dial positions are the engine's fields, mapped one way and back.
#[test]
fn every_dial_reads_and_writes_its_own_field() {
    for dial in Knob::ALL.map(Dial::for_knob) {
        let mut voice = KitVoice::default();
        let before = dial.read(&voice);
        let others: Vec<f32> = Knob::ALL
            .map(Dial::for_knob)
            .iter()
            .filter(|other| **other != dial)
            .map(|other| other.read(&voice))
            .collect();
        assert!(
            (0.0..=1.0).contains(&before),
            "{dial:?} reads outside the knob's travel: {before}"
        );
        dial.write(&mut voice, 0.8);
        assert!(
            (dial.read(&voice) - 0.8).abs() < 1e-4,
            "{dial:?} does not read back what it wrote: {}",
            dial.read(&voice)
        );
        // One dial, one field: nothing else in the voice moved.
        for (other, was) in Knob::ALL
            .map(Dial::for_knob)
            .iter()
            .filter(|other| **other != dial)
            .zip(others)
        {
            assert!(
                (other.read(&voice) - was).abs() < 1e-6,
                "{dial:?} moved {other:?}: {was} -> {}",
                other.read(&voice)
            );
        }
    }
}

/// The dial order is a face decision: each 303 slot is repurposed, left to right, as the spec's mockup draws them.
#[test]
fn the_seven_dials_are_labelled_in_the_order_the_face_draws_them() {
    assert_eq!(
        Knob::ALL.map(|knob| Dial::for_knob(knob).label()),
        [
            "ATTACK",
            "DECAY",
            "SUSTAIN",
            "RELEASE",
            "CUT OFF",
            "RESONANCE",
            "TUNE"
        ],
        "the 303's slots, relabelled left to right"
    );
    // One dial per 303 slot, taken from the catalogue: a second `Dial::ALL` is the list this face deliberately does not keep.
    for dial in Knob::ALL.map(Dial::for_knob) {
        assert!(dial.label().is_ascii(), "{dial:?} is not ASCII");
    }
}

/// `SHUFFLE` is the device's swing, not the voice's: the same atomic the 303's performance row writes, so both machines hear
/// one shuffle.
#[test]
fn the_shuffle_dial_writes_the_devices_swing() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let l = layout(screen());
    let shuffle = drum_panel::key_cell(&l, DrumKey::Shuffle).center();
    assert_eq!(shared.swing.get(), 0.0);

    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(shuffle)],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![press(shuffle, Modifiers::default())],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![Event::PointerMoved(shuffle - vec2(0.0, 18.0))],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![release(shuffle - vec2(0.0, 18.0))],
    );

    assert!(
        shared.swing.get() > 0.05,
        "the drum face's SHUFFLE moves the device's swing: {}",
        shared.swing.get()
    );
}

/// A drum device answers no 303 editor key and places no 303 note — the grid is its editor.
///
/// Trap 4 of the handover: `CLEAR`, `RANDOM`, the octave keys and the step shortcuts all run per device in `main`, and the
/// note paths run inside `draw_panel`, so removing the rows alone would leave every one of them writing 303 fields into a
/// drum pattern.
#[test]
fn a_drum_device_ignores_the_keyboard_and_the_note_queue() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    // One painted frame first: the first frame that shows a device loads its editor slot, and that load resets `step_mode`
    // to the slot's default. A player's STEP click lands on an established frame; this does the same.
    let before = shared.pattern();
    paint(&ctx, &mut panel, &shared);
    // Step mode is the loudest setting for the step editor: with it on, an acid face writes a step for `A` and places a
    // MIDI note.
    panel.step_mode = true;

    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![key_press(Key::A, Modifiers::default())],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![key_release(Key::A)],
    );
    run(
        &ctx,
        screen(),
        &mut panel,
        &shared,
        vec![key_press(Key::Enter, Modifiers::default())],
    );
    assert!(shared.notes.push(60, 100), "the queue has room");
    run(&ctx, screen(), &mut panel, &shared, vec![]);

    assert_eq!(
        shared.pattern().steps,
        before.steps,
        "a drum face's keyboard and MIDI notes write nothing"
    );
    assert!(
        shared.notes.is_empty(),
        "the note queue is still drained, so nothing lands later"
    );
    assert!(
        !shared.midi_gate.load(Ordering::Relaxed),
        "and no audition was sounded"
    );

    // The dispatcher refuses the step editor for a drum device, and still answers the transport.
    let mut bank = PatternBank::new_empty();
    let enter = KeyPress {
        key: Key::Enter,
        shift: false,
        ctrl: false,
    };
    assert_eq!(
        dispatch::handle(enter, &mut panel, &shared, &mut bank),
        Handled::Nothing,
        "a drum face has no step editor, so `Enter` is no face's key and nothing takes it from the frame"
    );
    let space = KeyPress {
        key: Key::Space,
        shift: false,
        ctrl: false,
    };
    assert_eq!(
        dispatch::handle(space, &mut panel, &shared, &mut bank),
        Handled::Shortcut(Shortcut::PlayStop),
        "the transport shortcuts are the drum machine's too"
    );
    assert_eq!(shared.pattern().steps, before.steps);
}

#[test]
fn shift_up_and_down_walk_the_selected_voice() {
    // The grid is the drum face's editor, so the arrows walk its rows the way they walk the 303's steps: `shift+down` is
    // the row below, `shift+up` the row above. A walk is a selection and nothing more — no voice sounds as it passes, because
    // `HIT` is how a voice is heard.
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let mut bank = PatternBank::new_empty();
    let pattern = shared.pattern();
    shared.take_audition();

    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowDown, true),
        Handled::Selection
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Snare,
        "down the grid is the next voice"
    );
    assert_eq!(
        shared.take_audition(),
        None,
        "the walk itself makes no sound; `HIT` plays the voice"
    );

    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowUp, true),
        Handled::Selection
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Kick,
        "up the grid is the previous one"
    );
    assert_eq!(shared.take_audition(), None, "and going back is silent too");

    // Both ends clamp, the rule the 303's step border keeps: a walk past the first row or the last is the same selection,
    // not a jump to the other end of the grid.
    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowUp, true),
        Handled::Selection
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Kick,
        "the top row is the top row"
    );

    // A held key repeats, and the walk is meant to carry on: `dispatch::presses` lets an arrow's repeat through, so holding
    // `shift+down` walks row after row, silently.
    let mut repeated = Handled::Nothing;
    for press in dispatch::presses(
        &[key_repeat(Key::ArrowDown, Modifiers::SHIFT)],
        panel.showing_kind(),
    ) {
        repeated = dispatch::handle(press, &mut panel, &shared, &mut bank);
    }
    assert_eq!(repeated, Handled::Selection, "a repeat is still a walk");
    assert_eq!(panel.drum.selected, Drum::Snare, "and it walks on");
    assert_eq!(shared.take_audition(), None, "a held key sounds nothing");
    for _ in 0..Drum::ALL.len() + 4 {
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowDown, true);
    }
    assert_eq!(
        panel.drum.selected,
        Drum::Shaker,
        "and the bottom row is the bottom"
    );

    // A selection is not an edit, and the plain arrows still belong to the tempo.
    assert_eq!(shared.pattern(), pattern, "no step changed");
    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowUp, false),
        Handled::Shortcut(Shortcut::TempoUp),
        "unshifted arrows are still the tempo's, and they moved no voice"
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Shaker,
        "and the tempo's arrow walked no voice"
    );
}

#[test]
fn a_card_refuses_the_voice_keys() {
    // A card over the face is modal for the keyboard as well as the pointer — the rule `selection::dispatch` keeps for the
    // step editor — so a walk aimed at a grid nobody can see lands nowhere and sounds nothing.
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let mut bank = PatternBank::new_empty();
    panel.help_visible = true;
    shared.take_audition();

    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowDown, true),
        Handled::Refused,
        "the help card is up"
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Kick,
        "a refused press moves nothing"
    );
    assert_eq!(shared.take_audition(), None, "and sounds nothing");
}

#[test]
fn the_voice_keys_leave_the_acid_note_alone() {
    // The drum face owns `shift+up`/`shift+down` for its rows and the 303 owns the same two keys for the selected note. One
    // mapping must not swallow the other: this is the pairing that proves the drum arm is the drum face's only.
    let shared = SharedState::new();
    shared.set_step(0, Step::note(Pitch::E, 0));
    let mut panel = acid_panel_state();
    let mut bank = PatternBank::new_empty();

    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::ArrowDown, true),
        Handled::Selection
    );
    let nudged = shared.pattern().steps[0];
    assert!(nudged.on, "the nudge leaves the step sounding");
    assert_eq!(
        (nudged.pitch, nudged.octave_offset),
        (Pitch::DSharp, 0),
        "shift+down on an acid face still moves the note down a semitone"
    );
    assert_eq!(
        panel.drum.selected,
        Drum::Kick,
        "and it moved no drum voice"
    );
}

#[test]
fn h_strikes_the_selected_voice() {
    // `HIT` is a click; `h` is the same hit from the keyboard, so a kit can be walked with the arrows and heard with one key.
    let shared = SharedState::new();
    let mut panel = drum_panel_state();
    let mut bank = PatternBank::new_empty();
    panel.drum.selected = Drum::Snare;
    shared.take_audition();

    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::H, false),
        Handled::Hit,
        "the hit key answers as a hit"
    );
    assert_eq!(
        shared.take_audition(),
        Some(Drum::Snare),
        "and asks for the voice the dials are on"
    );
    assert_eq!(panel.drum.selected, Drum::Snare, "a hit is not a selection");

    // A card is modal for selections and edits, not for auditions: the 303's letters already play through one.
    panel.help_visible = true;
    assert_eq!(
        voice_press(&mut panel, &shared, &mut bank, Key::H, false),
        Handled::Hit
    );
    assert_eq!(shared.take_audition(), Some(Drum::Snare));

    // On an acid panel `h` is a note, and the press is not the dispatcher's to act on: it has to reach `play_computer_keys`.
    let mut acid = acid_panel_state();
    let acid_shared = SharedState::new();
    acid_shared.take_audition();
    assert_eq!(
        voice_press(&mut acid, &acid_shared, &mut bank, Key::H, false),
        Handled::Nothing,
        "`h` is one of the 303's notes"
    );
    assert_eq!(
        acid_shared.take_audition(),
        None,
        "and it struck no drum voice"
    );
}
