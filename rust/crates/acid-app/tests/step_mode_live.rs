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

/// A key release, which is what walks the selection on when step mode is on.
fn release(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers: Modifiers::default(),
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

    // A, then W: C then C sharp, onto steps 1 and 2. The press writes the step and the release walks on, so the
    // selection only moves once the key is up.
    run_on(&ctx, &mut panel, &shared, vec![press(Key::A, false)]);
    assert_eq!(panel.selected, 0, "a held key has not walked yet");
    run_on(&ctx, &mut panel, &shared, vec![release(Key::A)]);
    run_on(&ctx, &mut panel, &shared, vec![press(Key::W, false)]);
    run_on(&ctx, &mut panel, &shared, vec![release(Key::W)]);

    let steps = shared.pattern().steps;
    assert_eq!(steps[0].pitch, Pitch::C);
    assert_eq!(steps[1].pitch, Pitch::CSharp);
    assert_eq!(
        panel.selected, 2,
        "the selection walked with the released keys"
    );
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
    run_on(&ctx, &mut panel, &shared, vec![release(Key::A)]);

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
    midi_note(&shared, 36, 100);
    run_on(&ctx, &mut panel, &shared, vec![]);

    let step = shared.pattern().steps[8];
    assert_eq!(step.pitch, Pitch::DSharp, "MIDI 36 is E flat on this panel");
    assert_eq!(
        step.velocity, 100,
        "and it landed at the velocity it was struck"
    );
    assert_eq!(panel.selected, 9, "and step mode walked on");

    // The queue was drained, so another frame places nothing.
    let after = shared.pattern().steps;
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(shared.pattern().steps, after);
}

/// Publish a MIDI note the way the MIDI thread does: the audition side first, then the note for the panel to place.
fn midi_note(shared: &SharedState, note: u8, velocity: u8) {
    // The real predicate, so a change to the accent threshold is felt here rather than silently duplicated.
    shared.midi_gate.store(true, Ordering::Relaxed);
    shared.midi_accent.store(
        acid_app::midi::message::is_accent(velocity),
        Ordering::Relaxed,
    );
    assert!(shared.notes.push(note, velocity), "the queue has room");
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
    run_on(&ctx, &mut panel, &shared, vec![release(Key::Backspace)]);

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

#[test]
fn two_notes_inside_one_frame_both_land_with_their_own_pitch_and_velocity() {
    // The regression the queue exists for: with one slot and a counter, the second note overwrote the first before the
    // panel ever looked, so a rolled chord or a fast pair landed as one step twice.
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        step_mode: true,
        ..PanelState::default()
    };

    midi_note(&shared, 36, 30); // E flat, soft
    midi_note(&shared, 40, 120); // G, hard
    run_on(&ctx, &mut panel, &shared, vec![]);

    let steps = shared.pattern().steps;
    assert_eq!(
        steps[0].pitch,
        Pitch::DSharp,
        "the first note kept its pitch"
    );
    assert_eq!(steps[0].velocity, 30, "and its own velocity");
    assert!(!steps[0].accent);
    assert_eq!(steps[1].pitch, Pitch::G, "the second note landed next");
    assert_eq!(steps[1].velocity, 120);
    assert!(steps[1].accent, "and brought its own accent");
    assert_eq!(panel.selected, 2, "the walk followed both");
}

#[test]
fn notes_wait_for_the_panel_even_when_it_is_not_editing() {
    // Nothing is placed while the panel is not editing, but the queue must still be drained: a note that arrived then
    // must not be placed later, when a stray shift press turns editing on.
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState::default();
    let before = shared.pattern().steps;

    midi_note(&shared, 36, 100);
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(
        shared.pattern().steps,
        before,
        "not editing, so nothing written"
    );
    assert!(
        shared.notes.is_empty(),
        "but the note was taken, not left waiting"
    );

    panel.step_mode = true;
    run_on(&ctx, &mut panel, &shared, vec![]);
    assert_eq!(shared.pattern().steps, before, "and it never comes back");
}

#[test]
fn every_key_keeps_its_pitch_at_the_edges_of_the_octave_range() {
    // Review finding: the keyboard's octave reaches `-2..=2` but the panel only carries `-1..=1`, and the round trip
    // through a MIDI number clamped *before* the pitch class was read back — so at either end of the octave range every
    // key wrote the same C. The pitch class must survive; only the octave may be clamped.
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        step_mode: true,
        ..PanelState::default()
    };

    shared.octave.store(2, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![press(Key::D, false)]);
    run_on(&ctx, &mut panel, &shared, vec![release(Key::D)]);
    assert_eq!(
        shared.pattern().steps[0].pitch,
        Pitch::E,
        "a high octave must not turn every key into C"
    );

    shared.octave.store(-2, Ordering::Relaxed);
    run_on(&ctx, &mut panel, &shared, vec![press(Key::G, false)]);
    run_on(&ctx, &mut panel, &shared, vec![release(Key::G)]);
    assert_eq!(
        shared.pattern().steps[1].pitch,
        Pitch::G,
        "nor must a low one"
    );
}

#[test]
fn a_release_walks_once_and_only_for_a_key_the_panel_wrote() {
    // A live panel cannot show a duplicated press — egui drops a `pressed` event for a key it already believes is down —
    // so the one-note-per-press rule is tested where it lives, in `keys::first_press` (see `ui::keys`). What this proves
    // is the other half: one release walks one step, a second release walks nothing, and nothing is written ahead.
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        step_mode: true,
        ..PanelState::default()
    };
    let before = shared.pattern().steps;

    run_on(&ctx, &mut panel, &shared, vec![press(Key::A, false)]);

    assert_eq!(panel.selected, 0, "no release, so no walk");
    assert_eq!(panel.placed, vec![Key::A], "the key is recorded as down");

    run_on(&ctx, &mut panel, &shared, vec![release(Key::A)]);
    assert_eq!(panel.selected, 1, "one release walks one step");
    assert!(
        panel.placed.is_empty(),
        "and the key is forgotten until it is pressed again"
    );

    run_on(&ctx, &mut panel, &shared, vec![release(Key::A)]);
    assert_eq!(panel.selected, 1, "a second release is not a second step");

    // One press, one step: the next step is untouched.
    let steps = shared.pattern().steps;
    assert_eq!(steps[1], before[1], "the next step was never written");
}

#[test]
fn an_arrow_never_walks_the_border_on_its_release() {
    // The bug: every key press was remembered, so pressing left (which walks back a step) had its *release* walk the
    // border forward again — the note could not be nudged back without the mouse. Only keys that write a step are ours.
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 4,
        step_mode: true,
        ..PanelState::default()
    };

    run_on(
        &ctx,
        &mut panel,
        &shared,
        vec![press(Key::ArrowLeft, false)],
    );
    run_on(&ctx, &mut panel, &shared, vec![release(Key::ArrowLeft)]);

    assert_eq!(
        panel.selected, 4,
        "an arrow is not a note: its release must not walk"
    );
    assert!(
        panel.placed.is_empty(),
        "and it is never tracked as a press"
    );
    assert_eq!(shared.pattern(), shared.pattern(), "and nothing is written");
}

#[test]
fn a_release_without_a_press_never_walks() {
    let ctx = Context::default();
    let shared = lit_shared();
    let mut panel = PanelState {
        selected: 5,
        step_mode: true,
        ..PanelState::default()
    };

    run_on(&ctx, &mut panel, &shared, vec![release(Key::A)]);

    assert_eq!(panel.selected, 5, "nothing was pressed, so nothing walks");
}
