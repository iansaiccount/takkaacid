//! Step mode and note placement: a note typed on a keyboard writes the selected step, and step mode walks on.

use acid_app::state::SharedState;
use acid_app::ui::layout::layout;
use acid_app::ui::switches::{self, Switch};
use acid_app::ui::{PanelState, draw_panel};
use acid_core::pattern::Step;
use acid_core::pitch::Pitch;
use eframe::egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};
use std::sync::atomic::Ordering;

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn run_on(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![button(true)]);
    run_on(ctx, panel, shared, vec![button(false)]);
}

/// A key press the panel sees as one fresh press.
fn press(key: Key, shift: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift,
            ..Modifiers::default()
        },
    }
}

/// STEP is the eighth switches cell.
fn step_centre() -> Pos2 {
    switches::cell(layout(screen()).switches, Switch::Step).center()
}

fn lit_shared() -> std::sync::Arc<SharedState> {
    let shared = SharedState::new();
    for index in 0..16 {
        shared.set_step(index, Step::note(Pitch::C, 0));
    }
    shared
}

#[test]
fn shift_and_a_note_writes_the_selected_step() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 4,
        ..PanelState::default()
    };

    // `D` is E: the note lands on step 5 and the selection stays where it was.
    run_on(&ctx, &mut panel, &shared, vec![press(Key::D, true)]);

    let step = shared.pattern().steps[4];
    assert!(step.on);
    assert_eq!(step.pitch, Pitch::E);
    assert_eq!(panel.selected, 4, "shift does not walk the steps");
    assert!(
        !shared.midi_gate.load(Ordering::Relaxed),
        "an edit must not also sound the voice"
    );
}

#[test]
fn step_mode_writes_the_note_and_walks_on() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();

    click(&ctx, &mut panel, &shared, step_centre());
    assert!(panel.step_mode, "STEP turns step mode on");

    // A, then W: C then C sharp, onto steps 1 and 2.
    run_on(&ctx, &mut panel, &shared, vec![press(Key::A, false)]);
    run_on(&ctx, &mut panel, &shared, vec![press(Key::W, false)]);

    let steps = shared.pattern().steps;
    assert_eq!(steps[0].pitch, Pitch::C);
    assert_eq!(steps[1].pitch, Pitch::CSharp);
    assert_eq!(panel.selected, 2, "the selection walked with the notes");
}

#[test]
fn step_mode_wraps_after_the_last_step() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 15,
        step_mode: true,
        ..PanelState::default()
    };

    run_on(&ctx, &mut panel, &shared, vec![press(Key::A, false)]);

    assert_eq!(shared.pattern().steps[15].pitch, Pitch::C);
    assert_eq!(panel.selected, 0, "after step 16 comes step 1");
}

#[test]
fn a_note_without_shift_or_step_mode_only_sounds() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 2,
        ..PanelState::default()
    };
    let before = shared.pattern().steps;

    run_on(&ctx, &mut panel, &shared, vec![press(Key::A, false)]);

    assert_eq!(shared.pattern().steps, before, "nothing was written");
    assert!(
        shared.midi_gate.load(Ordering::Relaxed),
        "the note was auditioned instead"
    );
    assert_eq!(panel.selected, 2);
}

#[test]
fn a_midi_note_writes_the_step_when_the_panel_is_editing() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 8,
        step_mode: true,
        ..PanelState::default()
    };

    // MIDI 36 is three semitones above the panel's zero (33), so it is E flat.
    shared.step_note.store(36, Ordering::Relaxed);
    shared.step_note_seq.fetch_add(1, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![]);

    let step = shared.pattern().steps[8];
    assert_eq!(step.pitch, Pitch::DSharp, "MIDI 36 is E flat on this panel");
    assert_eq!(panel.selected, 9, "and step mode walked on");

    // The same counter must not place the note twice.
    let after = shared.pattern().steps;
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(shared.pattern().steps, after);
}

/// Publish a MIDI note the way the MIDI thread does: the velocity first (it becomes the accent), then the note number
/// and the counter that wakes the panel.
fn midi_note(shared: &SharedState, note: i32, velocity: u8) {
    // The real predicate, so a change to the accent threshold is felt here rather than silently duplicated.
    shared.midi_accent.store(
        acid_app::midi::message::is_accent(velocity),
        Ordering::Relaxed,
    );
    shared.step_note.store(note, Ordering::Relaxed);
    shared.step_note_seq.fetch_add(1, Ordering::Relaxed);
}

#[test]
fn a_hard_midi_note_accents_the_step_and_a_soft_one_does_not() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        step_mode: true,
        ..PanelState::default()
    };

    midi_note(&shared, 36, 110);
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert!(
        shared.pattern().steps[0].accent,
        "a velocity at the accent threshold accents the step it lands on"
    );

    midi_note(&shared, 36, 30);
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert!(
        !shared.pattern().steps[1].accent,
        "a soft note leaves the next step unaccented"
    );
}

#[test]
fn backspace_rests_the_selected_step_and_walks_in_step_mode() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 3,
        step_mode: true,
        ..PanelState::default()
    };
    shared.set_step(3, Step::note(Pitch::G, 0));

    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![press(Key::Backspace, false)],
    );

    assert!(!shared.pattern().steps[3].on, "the step became a rest");
    assert_eq!(
        shared.pattern().steps[3].pitch,
        Pitch::G,
        "the rest keeps the pitch it had"
    );
    assert_eq!(panel.selected, 4, "step mode walked on");
}

#[test]
fn shift_and_backspace_rests_the_step_without_walking() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 5,
        ..PanelState::default()
    };

    run_on(&ctx, &mut panel, &shared, vec![press(Key::Backspace, true)]);

    assert!(!shared.pattern().steps[5].on);
    assert_eq!(panel.selected, 5, "no step mode, no walk");
}

#[test]
fn backspace_alone_is_not_ours() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 2,
        ..PanelState::default()
    };
    let before = shared.pattern().steps;

    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![press(Key::Backspace, false)],
    );

    assert_eq!(shared.pattern().steps, before, "nothing was written");
    assert_eq!(panel.selected, 2);
    assert!(!panel.dirty, "and nothing was marked for saving");
}
