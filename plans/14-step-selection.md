# Step Selection and Note Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give the panel a visible selected step, print every step's note on its keycap, and let the keyboard move the selection and transpose the selected note.

**Architecture:** Selection is one index in `PanelState` and the rules that read or move it are pure functions in a new `ui::selection` module, so gestures and shortcuts stay testable without a window. `draw_panel` owns the click that selects; `main.rs` owns the key events, because the note it edits lives in `SharedState`.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `acid-core` pattern model, `cargo test --workspace`.

**Spec:** `docs/superpowers/specs/2026-11-07-step-selection-design.md`

## Global Constraints

- Legends and labels are plain ASCII — the bundled font is missing several glyphs (`AGENTS.md`).
- No control face may be a font glyph; icons come from `ui::icons::load_icon` (`AGENTS.md`).
- Nothing in the audio callback may allocate or lock: `tests/no_alloc.rs` must stay green.
- Every step ends green: `cargo test --workspace` plus `cargo clippy --workspace --all-targets -- -D warnings`.
- Selection is `PanelState::selected: usize`, always in `0..16`, never persisted.
- Nudge range is the panel's existing `-12..=24` absolute semitones (`edit::from_absolute`).

## Review Focus

1. **A click while the help card is up must not select behind it** — the card is modal (Ruling 111); a click there closes the card and must leave the selection alone.
2. **Ctrl+click toggles, and `ctrl` is read as `modifiers.ctrl`** — on Linux `command` and `ctrl` coincide, but reading the wrong one makes the toggle dead on every platform that differs.
3. **Shift+arrow must not also fire its unshifted action** — shift+right moves the selection and must not change the pattern.
4. **Dragging a rest selects it and changes nothing else** — a rest carries a remembered pitch, and a drag must not resurrect it silently.
5. **The selection is clamped, not wrapped** — shift+left on step 1 stays on step 1; a wrap would silently jump to the far end mid-line.

---

### Task 1: Selection rules and transposition

**Files:**
- Create: `rust/crates/acid-app/src/ui/selection.rs`
- Modify: `rust/crates/acid-app/src/ui/edit.rs` (add `transpose`)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (module list, `PanelState::selected`)
- Test: `rust/crates/acid-app/src/ui/selection.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `acid_core::pattern::Step`, `acid_core::pitch::Pitch`, `edit::{apply_gesture, nudge_pitch, StepGesture}`.
- Produces:
  - `edit::transpose(pitch: Pitch, octave: i8, semitones: i8) -> (Pitch, i8)` — absolute-semitone move, canonical result, clamped to `-12..=24`.
  - `selection::move_selection(current: usize, delta: i8, count: usize) -> usize` — clamped to `0..count-1`.
  - `selection::nudge_step(step: Step, semitones: i8, last_pitch: Pitch, last_octave: i8) -> Step`.
  - `PanelState::selected: usize` (defaults to `0`).

- [ ] **Step 1: Write the failing tests**

In `ui/edit.rs`'s test module:

```rust
#[test]
fn transposition_respects_the_saved_octave() {
    assert_eq!(transpose(Pitch::C, 1, 1), (Pitch::CSharp, 1));
    assert_eq!(transpose(Pitch::C, 1, -12), (Pitch::C, 0));
    assert_eq!(transpose(Pitch::B, -1, 1), (Pitch::C, 0));
    // The clamp is the panel's, not the octave's: two octaves up is the ceiling.
    assert_eq!(transpose(Pitch::C, 1, 12), (Pitch::C, 1));
}
```

In `ui/selection.rs`:

```rust
#[test]
fn the_selection_moves_and_clamps_at_both_ends() {
    assert_eq!(move_selection(0, 1, 16), 1);
    assert_eq!(move_selection(15, 1, 16), 15);
    assert_eq!(move_selection(0, -1, 16), 0);
    assert_eq!(move_selection(3, -2, 16), 1);
}

#[test]
fn nudging_a_note_moves_it_a_semitone_and_keeps_its_flags() {
    let step = Step { accent: true, slide: true, ..Step::note(Pitch::C, 1) };
    let up = nudge_step(step, 1, Pitch::A, 0);
    assert_eq!((up.pitch, up.octave_offset), (Pitch::CSharp, 1));
    assert!(up.on && up.accent && up.slide);
}

#[test]
fn nudging_a_rest_takes_the_last_pitch_already_moved() {
    let rest = nudge_step(Step::rest(), 1, Pitch::A, 0);
    assert!(rest.on);
    assert_eq!((rest.pitch, rest.octave_offset), (Pitch::ASharp, 0));
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `cannot find function 'move_selection'` and `cannot find function 'transpose'`.

- [ ] **Step 3: Implement**

`edit::transpose` moves by absolute semitones and re-uses the existing private `from_absolute`, which already clamps and canonicalises. Rewrite `nudge_pitch` as `transpose(pitch, 0, semitones)` so the two cannot drift.

`selection::nudge_step` transposes the step's own `(pitch, octave_offset)` when it is on; when it is off it enables the step at the last pitch already transposed — the same shape as `apply_gesture`'s `enable` closure:

```rust
pub fn nudge_step(step: Step, semitones: i8, last_pitch: Pitch, last_octave: i8) -> Step {
    if step.on {
        let (pitch, octave_offset) = edit::transpose(step.pitch, step.octave_offset, semitones);
        Step { pitch, octave_offset, ..step }
    } else {
        let (pitch, octave_offset) = edit::transpose(last_pitch, last_octave, semitones);
        Step { on: true, pitch, octave_offset, ..step }
    }
}
```

Add `pub mod selection;` to `ui/mod.rs` and `pub selected: usize` to `PanelState` (default `0`).

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app selection`
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): selection rules and step transposition"
```

---

### Task 2: The keycap shows its note

**Files:**
- Modify: `rust/crates/acid-app/src/ui/step_key.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (pass the selection into `step_view`)

**Interfaces:**
- Consumes: `PanelState::selected`, `edit::pitch_name(pitch, octave) -> String`.
- Produces:
  - `step_key::step_label(step: &Step) -> String` — `"D#0"` for a note, `"--"` for a rest.
  - `StepView` gains `pub selected: bool`.
  - `step_view(step: &Step, index: usize, position: usize, playing: bool, selected: bool) -> StepView`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_keycap_names_its_note_and_a_rest_shows_dashes() {
    assert_eq!(step_label(&Step::note(Pitch::DSharp, 0)), "D#0");
    assert_eq!(step_label(&Step::note(Pitch::CHigh, 0)), "C1");
    assert_eq!(step_label(&Step::rest()), "--");
    // A rest keeps a remembered pitch; the label must still read as a rest.
    assert_eq!(step_label(&Step { on: false, ..Step::note(Pitch::G, 0) }), "--");
}

#[test]
fn the_view_carries_the_selection() {
    let step = Step::note(Pitch::C, 0);
    assert!(step_view(&step, 0, 0, false, true).selected);
    assert!(!step_view(&step, 0, 0, false, false).selected);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: FAIL — `cannot find function 'step_label'`, `StepView has no field named 'selected'`.

- [ ] **Step 3: Implement**

`step_label(step)` returns `"--"` when `!step.on`, otherwise `edit::pitch_name(step.pitch, step.octave_offset)`.

In `step_key`, replace the centred number with the note label at `keycap.center()` in `theme::legend(12.0 * s)` — a size up from the number it replaces, because a note is two or three glyphs and the panel's kerning is loose — and draw the step number small (`theme::legend(8.0 * s)`) at `keycap.right_bottom() - vec2(4.0 * s, 2.0 * s)`, right-and-bottom aligned.

When `view.selected`, stroke `rect` (the whole cell, not `keycap`) with `egui::Stroke::new((2.0 * s).max(1.0), theme::LED_ACCENT)` and `egui::StrokeKind::Inside`, after the keycap so the outline sits on top of every bar.

Update the three existing `step_view` calls in `ui/mod.rs` to pass `index == state.selected`.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: PASS — 5 tests (the three existing ones updated for the new argument, plus the two new).

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): keycaps print their note and outline the selected step"
```

---

### Task 3: A click selects, ctrl+click edits

**Files:**
- Modify: `rust/crates/acid-app/src/ui/edit.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (step loop: select on any click or drag start)
- Test: `rust/crates/acid-app/src/ui/edit.rs`, `rust/crates/acid-app/tests/selection_live.rs` (create)

**Interfaces:**
- Produces:
  - `StepGesture::Select` (new variant).
  - `gesture_from_input(left: bool, shift: bool, alt: bool, ctrl: bool, double: bool) -> Option<StepGesture>` — plain click gives `Select`, ctrl+click gives `Toggle`, double gives `Clear`, shift gives `Accent`, alt gives `Slide`.

- [ ] **Step 1: Write the failing tests**

In `ui/edit.rs`:

```rust
#[test]
fn a_plain_click_only_selects_and_ctrl_click_toggles() {
    assert_eq!(gesture_from_input(true, false, false, false, false), Some(StepGesture::Select));
    assert_eq!(gesture_from_input(true, false, false, true, false), Some(StepGesture::Toggle));
    assert_eq!(gesture_from_input(true, true, false, true, false), Some(StepGesture::Accent));
}

#[test]
fn selecting_leaves_the_step_untouched() {
    let step = Step { accent: true, ..Step::note(Pitch::E, 0) };
    assert_eq!(apply_gesture(step, StepGesture::Select, Pitch::C, 0), step);
    assert_eq!(apply_gesture(Step::rest(), StepGesture::Select, Pitch::C, 0), Step::rest());
}
```

In `tests/selection_live.rs` (new file; drive `draw_panel` through `Context::run_ui` with `RawInput` events, as `tests/dismiss_live.rs` does):

```rust
```rust
#[test]
fn a_click_selects_without_editing() {
    // Every step starts on; the click lands on step 5.
    // ... harness setup, then:
    assert_eq!(panel.selected, 5);
    assert!(shared.pattern().steps[5].on, "a plain click must not rest the step");
}

#[test]
fn ctrl_click_selects_and_toggles() {
    // Step 5 is on; ctrl+click both moves the border there and takes the step off.
    // ... harness setup, then:
    assert_eq!(panel.selected, 5, "a click always moves the selection");
    assert!(!shared.pattern().steps[5].on, "ctrl+click must toggle the step");
}

#[test]
fn dragging_a_rest_selects_it_and_leaves_it_off() {
    // Step 3 is a rest; a drag over it selects it and must not revive its remembered pitch.
    // ... harness setup, then:
    assert_eq!(panel.selected, 3);
    assert!(!shared.pattern().steps[3].on);
}
```

Fill the harness setup from `tests/dismiss_live.rs` (`run_on`, `moved`, `button`, plus a `button_with(at, pressed, modifiers)` that sets `modifiers.ctrl`); the assertions above are the whole test.

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `no variant named 'Select'`, and the live test fails to find the selection field's effect.

- [ ] **Step 3: Implement**

Add `Select` to `StepGesture`; in `apply_gesture`, return `step` unchanged for it. Add the `ctrl` parameter to `gesture_from_input`, ordered `double`, then `shift`, then `alt`, then `ctrl`, then plain `left` → `Select` — so a modifier still beats the plain click, exactly as before.

In `draw_panel`'s step loop, set `state.selected = index` whenever the response is clicked or drag-started — **every** click moves the selection, ctrl+click included — then apply the gesture only when it is not `Select`. The drag path keeps writing pitch as it does now, and because the selection moved, the drag also decides what the keyboard edits.

A rest responds to a drag by selecting and doing nothing else: it stays off, and its remembered pitch is untouched.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS. The existing gesture tests that asserted click → toggle now assert click → select; switch them to ctrl+click rather than changing what they prove.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): click selects a step, ctrl+click toggles it"
```

---

### Task 4: The keyboard moves and edits the selection

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs`
- Modify: `rust/crates/acid-app/src/main.rs` (key dispatch, pattern-change reset)
- Test: `rust/crates/acid-app/src/ui/selection.rs`, `rust/crates/acid-app/tests/selection_live.rs`

**Interfaces:**
- Consumes: `selection::{move_selection, nudge_step}`, `PanelState::{selected, last_pitch, last_octave}`, `SharedState::{pattern, set_step}`, `acid_core::pattern::Pattern::steps` (a public array field: `shared.pattern().steps[index]`).
- Produces:
  - `selection::EditKey { NoteUp, NoteDown, SelectPrev, SelectNext, ToggleSelected }`.
  - `selection::edit_key(key: egui::Key, shift: bool) -> Option<EditKey>`.
  - `selection::dispatch(key: egui::Key, shift: bool, panel: &mut PanelState, shared: &SharedState) -> bool` — applies the key and returns `true` when it consumed it.

- [ ] **Step 1: Write the failing tests**

In `ui/selection.rs`:

```rust
#[test]
fn shift_turns_the_arrows_into_editing_keys() {
    assert_eq!(edit_key(Key::ArrowRight, true), Some(EditKey::SelectNext));
    assert_eq!(edit_key(Key::ArrowLeft, true), Some(EditKey::SelectPrev));
    assert_eq!(edit_key(Key::ArrowUp, true), Some(EditKey::NoteUp));
    assert_eq!(edit_key(Key::ArrowDown, true), Some(EditKey::NoteDown));
    assert_eq!(edit_key(Key::Enter, false), Some(EditKey::ToggleSelected));
    // Unshifted arrows belong to tempo and pattern; they must not be swallowed here.
    assert_eq!(edit_key(Key::ArrowRight, false), None);
}
```

In `tests/selection_live.rs` (same harness as Task 3, plus a `key(at, key, modifiers)` helper that sends `Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }`):

```rust
#[test]
fn shift_right_moves_the_selection_and_leaves_the_pattern_alone() {
    // ... harness setup with a pattern in SharedState, then:
    assert_eq!(panel.selected, 1);
    assert_eq!(shared.pattern().steps, before, "shift+right must not touch the pattern");
}

#[test]
fn shift_up_raises_the_selected_note_one_semitone() {
    // ... harness setup with step 0 on C0, then:
    let step = shared.pattern().steps[0];
    assert_eq!((step.pitch, step.octave_offset), (Pitch::CSharp, 0));
}

#[test]
fn enter_toggles_the_selected_step() {
    // ... harness setup with step 0 on and accented, then:
    assert!(!shared.pattern().steps[0].on, "enter must rest the step");
    // a second enter re-enables it with its flags: the shared pattern's step 0 is on and accented again
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `cannot find function 'edit_key'` and `cannot find function 'dispatch'`.

- [ ] **Step 3: Implement**

`edit_key` maps shift+arrows to the four `EditKey`s and `Key::Enter` (with or without shift) to `ToggleSelected`; everything else is `None`.

`dispatch` reads the key, then:

- selection moves: `panel.selected = move_selection(panel.selected, ±1, STEPS_PER_PATTERN)`;
- note up/down: `let step = shared.pattern().steps[panel.selected];` then `shared.set_step(panel.selected, nudge_step(step, ±1, panel.last_pitch, panel.last_octave))`;
- toggle: `apply_gesture(step, StepGesture::Toggle, panel.last_pitch, panel.last_octave)` through `shared.set_step`.

In `main.rs`, add `egui::Key::Enter` to the collected presses and, inside the loop, dispatch to `selection::dispatch(key, shift, &mut self.panel, &self.state)` first and `continue` when it returns `true` — that is what keeps shift+right from also reaching `shortcuts::shortcut_for` (Review Focus 3). After `shortcuts::apply` changes the pattern, reset `self.panel.selected = 0`.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS, and `cargo clippy --workspace --all-targets -- -D warnings` clean.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): keyboard moves the selection and transposes its note"
```

---

## Review Focus → tests

| Review Focus item | Test | Task |
|---|---|---|
| Click while the card is up must not select | extend `tests/dismiss_live.rs`: card visible, click a step, `selected` unchanged | 3 |
| `ctrl` read from `modifiers.ctrl` | `a_plain_click_only_selects_and_ctrl_click_toggles` + the ctrl+click live test | 3 |
| Shift+arrow must not fire its unshifted action | `shift_right_moves_the_selection_and_leaves_the_pattern_alone` | 4 |
| Dragging a rest selects and changes nothing | live test: drag over an off step leaves it off with its pitch intact | 3 |
| Clamped, not wrapped | `the_selection_moves_and_clamps_at_both_ends` | 1 |

## Handoff

Read the spec before starting. Tasks 1 and 2 are independent of 3 and 4 in code, but not in behaviour: do them in
order so each task's tests can lean on the one before. `selection::dispatch` is the only place the keyboard edits
a note; if a later task needs to nudge a note from elsewhere, call it rather than opening a second path.
