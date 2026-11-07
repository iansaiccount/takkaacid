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
4. **A drag on a rest brings the note in, and starts from the rest's own pitch** — ruled after review (2026-11-07): the drag keeps its pre-existing meaning (the spec's gesture table: "change pitch *and* select the step it started on"), so what a drag must never do is re-derive its base pitch from `last_pitch`; it starts from the rest's remembered `(pitch, octave)`.
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

- [x] **Step 1: Write the failing tests**

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

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `cannot find function 'move_selection'` and `cannot find function 'transpose'`.

- [x] **Step 3: Implement**

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

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app selection`
Expected: PASS (3 tests).

- [x] **Step 5: Commit**

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

- [x] **Step 1: Write the failing tests**

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

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: FAIL — `cannot find function 'step_label'`, `StepView has no field named 'selected'`.

- [x] **Step 3: Implement**

`step_label(step)` returns `"--"` when `!step.on`, otherwise `edit::pitch_name(step.pitch, step.octave_offset)`.

In `step_key`, replace the centred number with the note label at `keycap.center()` in `theme::legend(12.0 * s)` — a size up from the number it replaces, because a note is two or three glyphs and the panel's kerning is loose — and draw the step number small (`theme::legend(8.0 * s)`) at `keycap.right_bottom() - vec2(4.0 * s, 2.0 * s)`, right-and-bottom aligned.

When `view.selected`, stroke `rect` (the whole cell, not `keycap`) with `egui::Stroke::new((2.0 * s).max(1.0), theme::LED_ACCENT)` and `egui::StrokeKind::Inside`, after the keycap so the outline sits on top of every bar.

Update the three existing `step_view` calls in `ui/mod.rs` to pass `index == state.selected`.

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: PASS — 5 tests (the three existing ones updated for the new argument, plus the two new).

- [x] **Step 5: Commit**

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

- [x] **Step 1: Write the failing tests**

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
fn dragging_a_rest_selects_it_and_brings_it_in() {
    // Step 3 is a rest: the drag selects it and brings the note in, at the pitch the drag lands on measured
    // from the rest's own remembered pitch — never from `last_pitch`.
    // ... harness setup, then:
    assert_eq!(panel.selected, 3);
    assert!(shared.pattern().steps[3].on);
}
```

Fill the harness setup from `tests/dismiss_live.rs` (`run_on`, `moved`, `button`, plus a `button_with(at, pressed, modifiers)` that sets `modifiers.ctrl`); the assertions above are the whole test.

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `no variant named 'Select'`, and the live test fails to find the selection field's effect.

- [x] **Step 3: Implement**

Add `Select` to `StepGesture`; in `apply_gesture`, return `step` unchanged for it. Add the `ctrl` parameter to `gesture_from_input`, ordered `double`, then `shift`, then `alt`, then `ctrl`, then plain `left` → `Select` — so a modifier still beats the plain click, exactly as before.

In `draw_panel`'s step loop, set `state.selected = index` whenever the response is clicked or drag-started — **every** click moves the selection, ctrl+click included — then apply the gesture only when it is not `Select`. The drag path keeps writing pitch as it does now, and because the selection moved, the drag also decides what the keyboard edits.

A rest responds to a drag by selecting and bringing the note in at the dragged pitch; the drag's baseline is the rest's own remembered `(pitch, octave)`, never `last_pitch` (Review Focus 4, as ruled after review).

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS. The existing gesture tests that asserted click → toggle now assert click → select; switch them to ctrl+click rather than changing what they prove.

- [x] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): click selects a step, ctrl+click toggles it"
```

---

### Task 4: The keyboard moves and edits the selection

**Files:**
- Modify: `rust/crates/acid-app/src/ui/selection.rs`
- Create: `rust/crates/acid-app/src/ui/dispatch.rs` (the press loop and its ordering — moved out of `main.rs`, which is a binary and cannot be tested)
- Modify: `rust/crates/acid-app/src/main.rs` (key dispatch, pattern-change reset)
- Modify: `rust/crates/acid-app/src/ui/help.rs` (the card's gesture table — this plan missed it the first time round; the new keys and the click/ctrl+click split must be on the card)
- Test: `rust/crates/acid-app/src/ui/selection.rs`, `rust/crates/acid-app/tests/selection_live.rs`

**Interfaces:**
- Consumes: `selection::{move_selection, nudge_step}`, `PanelState::{selected, last_pitch, last_octave}`, `SharedState::{pattern, set_step}`, `acid_core::pattern::Pattern::steps` (a public array field: `shared.pattern().steps[index]`).
- Produces:
  - `selection::EditKey { NoteUp, NoteDown, SelectPrev, SelectNext, ToggleSelected }`.
  - `selection::edit_key(key: egui::Key, shift: bool) -> Option<EditKey>`.
  - `selection::Press { NotOurs, Applied, Refused }` plus `Press::consumed() -> bool` — a bool cannot express the third case (a key this module owns but refuses while the help card is up), and a refused press must be swallowed, not passed to the shortcuts.
  - `selection::dispatch(key: egui::Key, shift: bool, panel: &mut PanelState, shared: &SharedState) -> Press` — applies the key and reports what it did.
  - `dispatch::{KeyPress { key, shift }, Handled { Selection, Shortcut, Refused, Nothing }, presses(events, shift) -> Vec<KeyPress>, handle(press, panel, shared, bank) -> Handled}` — the ordering rule and the repeat filter, where they can be tested.

- [x] **Step 1: Write the failing tests**

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

- [x] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app selection`
Expected: FAIL — `cannot find function 'edit_key'` and `cannot find function 'dispatch'`.

- [x] **Step 3: Implement**

`edit_key` maps shift+arrows to the four `EditKey`s and `Key::Enter` (with or without shift) to `ToggleSelected`; everything else is `None`.

`dispatch` reads the key, then:

- selection moves: `panel.selected = move_selection(panel.selected, ±1, STEPS_PER_PATTERN)`;
- note up/down: `let step = shared.pattern().steps[panel.selected];` then `shared.set_step(panel.selected, nudge_step(step, ±1, panel.last_pitch, panel.last_octave))`;
- toggle: `apply_gesture(step, StepGesture::Toggle, panel.last_pitch, panel.last_octave)` through `shared.set_step`.

In `main.rs`, replace the hand-rolled press loop with `ui::dispatch`: `presses(&input.events, shift)` in one `ctx.input`, then `handle(press, &mut self.panel, &self.state, &mut self.bank)` for each. `handle` runs `selection::dispatch` first and only then `shortcuts::shortcut_for`, which is what keeps shift+right from also stepping the pattern (Review Focus 3); a press the step editor refuses is spent too, because the help card is modal. The pattern-change reset of `panel.selected` and the help toggle live in `handle`, since both are panel state.

- [x] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS, and `cargo clippy --workspace --all-targets -- -D warnings` clean.

- [x] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): keyboard moves the selection and transposes its note"
```

---

## Review Focus → tests

| Review Focus item | Test | Task |
|---|---|---|
| Click while the card is up must not select | `a_click_while_the_help_card_is_up_does_not_select` (`tests/selection_live.rs`: card visible, click a step, `selected` unchanged) | 3 |
| `ctrl` read from `modifiers.ctrl` | `a_plain_click_only_selects_and_ctrl_click_toggles` + the ctrl+click live test | 3 |
| Shift+arrow must not fire its unshifted action | `shift_right_moves_the_selection_and_leaves_the_pattern_alone` (the contract) and `ui::dispatch::tests::shift_right_moves_the_selection_and_never_reaches_the_pattern` (the glue, round-3 fix) | 4 |
| Dragging a rest brings the note in from the rest's own pitch | live test `dragging_a_rest_selects_it_and_brings_it_in` | 3 |
| Clamped, not wrapped | `the_selection_moves_and_clamps_at_both_ends` | 1 |
| Keyboard must not edit behind the card (round-2 fix) | `the_help_card_refuses_the_editing_keys_instead_of_passing_them_on`, and `a_press_with_the_card_away_is_ours_and_applied` for the `NotOurs` half | 4 |
| A held key must not repeat a toggle (round-3 fix) | `ui::dispatch::tests::a_held_key_repeats_only_for_the_tempo` | 4 |
| Keycap note and drag readout must not collide (round-3 fix) | `step_key::tests::a_dragged_keycap_leaves_the_pitch_to_the_readout` | 2 |

## Handoff

Read the spec before starting. Tasks 1 and 2 are independent of 3 and 4 in code, but not in behaviour: do them in
order so each task's tests can lean on the one before. `selection::dispatch` is the only place the keyboard edits
a note; if a later task needs to nudge a note from elsewhere, call it rather than opening a second path.

---

## Review Response (2026-11-07)

A reviewer subagent graded the shipped work against this plan. Its findings and the human partner's rulings:

- **Important 1 — the help card still said "click: note on or off".** Fixed in `ui/help.rs`: the card now reads
  "click: select the step" / "ctrl+click: note on or off" and names the new keys (shift+left / right, shift+up /
  down, enter). `the_help_card_names_every_gesture_and_shortcut` pins those two exact lines so it cannot rot.
  This plan never listed `help.rs` in a task's files — it is on Task 4's `Modify:` list now, and the "Review Focus → tests" table points at the tests that actually exist.
- **Important 2 — the keyboard edited behind the modal card.** Fixed: `selection::dispatch` returns the new
  `selection::Press` (`NotOurs` / `Applied` / `Refused`) and refuses its keys while `panel.help_visible`, so a
  press is swallowed rather than handed to the transport shortcuts behind the card. Pinned by
  `the_help_card_refuses_the_editing_keys_instead_of_passing_them_on`.
- **Important 3 — a drag on a rest.** Ruled: keep the existing behaviour and correct this plan (row 4 of Review
  Focus, the Task 3 test block, and the prose under Task 3 Step 3), which is what those three edits did.
- **Important 4 — toggling a note off and on changed its note.** Ruled: fix the behaviour. `edit::apply_gesture`'s
  `enable` and `selection::nudge_step` now restore (or transpose) the step's own remembered pitch and fall back to
  `last_pitch` only for a bare `Step::rest()`, which is the one value that genuinely remembers nothing. Pinned by
  `a_rested_note_comes_back_at_its_own_pitch`, `nudging_a_rested_note_moves_the_note_it_remembers`, and the pitch
  assertion in `enter_toggles_the_selected_step`.
- **Minor 8 — dead line** removed from `tests/selection_live.rs`. The remaining minors (plan checkboxes, the
  `main.rs` glue line still having no headless test, the keycap/drag-readout collision, the per-frame label
  `String`, held-Enter auto-repeat) are recorded in the ledger, not fixed.

### Review Response, round 2 (2026-11-07)

The reviewer re-ran over `351208a` (the fix commit) and confirmed all five round-1 findings resolved with no
behaviour regressions. Its remaining findings, and what was done:

- **Important 1 — the drag test could not tell the ruled rule from the rejected one.** The test dragged
  `Step::rest()` (C0) on a default panel (`last_pitch` C0), so both baselines were the same pitch and the assertion
  passed under the `last_pitch` implementation too. The test now drags a rest that remembers **G** on a panel whose
  `last_pitch` is **C**, and asserts the drag lands on **A**.
- **Minor 2 — a stale half-sentence left in `apply_gesture`'s doc comment** by the round-1 fix: removed.
- **Minor 3 — the rule's edge was understated.** A `C0` step with no flags rests to the same bits as
  `Step::rest()`, so it too comes in at `last_pitch`. Now stated in the doc comment and pinned by
  `a_c0_step_with_no_flags_rests_to_the_same_bits_as_a_bare_rest`; removing the limit needs a distinguishing bit
  in `Step`, which is its own change with a session-file migration.
- **Minor 4 — this plan still described `dispatch -> bool`** in Task 4's interfaces and Step 3: corrected to
  `-> Press` / `.consumed()`.
- **Minor 5 — the plan claimed its `help.rs` gap was closed when it was not:** `help.rs` is now on Task 4's
  `Modify:` list and the tests table names `tests/selection_live.rs` for the card-click test instead of
  `tests/dismiss_live.rs`.
- **Minor 6 — the ordering that keeps the card closable was unpinned.** Added `Space` and `Questionmark` →
  `NotOurs` (refused presses must not swallow the keys that close the card), and renamed
  `a_press_with_the_card_down_is_ours_and_applied` → `a_press_with_the_card_away_is_ours_and_applied`, which is
  what it always tested.

### Review Response, round 3 (2026-11-07)

Round 2's deferred minors, now fixed:

- **The glue line has a headless test.** The press loop moved out of `main.rs` (a binary, so untestable) into
  `ui::dispatch`: `presses(events, shift)` collects the frame's presses, `handle(press, panel, shared, bank)`
  resolves each one — step editor first, then the shortcuts — and reports `Handled`. `main.rs` is now a
  three-line call. `shift_right_moves_the_selection_and_never_reaches_the_pattern` finally tests Review Focus 3
  end to end instead of leaning on `selection::dispatch` alone.
- **Held keys no longer repeat their action.** Only the tempo nudges repeat (`repeats_allowed`), so a held
  `Enter` cannot flip a step at frame rate and a held arrow cannot race through the bank.
  `presses` reads egui's `repeat` flag, which `key_pressed` did not expose.
- **The keycap stands back while its step is dragged.** `StepView::dragging` suppresses the centre note, leaving
  the cell's live-pitch readout as the only label; pinned by `a_dragged_keycap_leaves_the_pitch_to_the_readout`.
- **The keycap label no longer allocates.** `edit::pitch_name` returns a `&'static str` from a 37-entry table
  covering `-12..=24`, checked against `from_absolute`'s spelling rule by
  `every_label_in_range_matches_the_canonical_spelling`. (The step *number* still formats per keycap per frame —
  egui takes a `ToString`, and that string is one digit.)
- **The checkboxes above are ticked**, as `AGENTS.md` asks.

### Hands-on fixes (2026-11-07)

Found by using the panel, after the reviews above. Three reported, one more in the same sweep:

- **A held modifier edited every key.** `draw_panel` asks all sixteen keys for a gesture on every frame, and
  `gesture_from_input` returned the modifier's gesture without requiring a click — so holding ctrl toggled every
  step every frame (every key flashing), shift toggled every accent, alt every slide. No click now means no
  gesture; `a_held_modifier_is_not_a_gesture` and the live
  `a_held_modifier_edits_nothing_while_the_pointer_rests_on_a_step` pin it. The live test uses **three** frames
  with the modifier down on purpose: an even number of toggles cancels out and would hide the bug.
- **A pattern lost every edit when you moved off it.** The bank holds the saved copies and `SharedState` holds the
  live pattern; switching re-published the bank's stale copy. `shortcuts::apply` now stashes the live pattern into
  its slot first (`stash_current`), the fold `persist::shared_to_saved` already did. Pinned by
  `switching_away_and_back_keeps_the_edits_you_made` and `a_pattern_you_switch_to_is_its_own_and_not_the_one_you_left`.
- **The pattern display never moved.** `panel.pattern_id` was declared, defaulted to `A1` and never written, so the
  display kept saying A1 while the arrows walked the bank. `ui::dispatch::handle` now sets it from the bank on a
  pattern change, and `main.rs` seeds it from the restored session. Pinned by
  `the_pattern_display_follows_the_bank`.
- **A held audition key re-gated the voice.** `play_computer_keys` used `key_pressed`, which counts egui's
  auto-repeats, so a held note retriggered at the repeat rate. `keys::fresh_presses` drops repeats; pinned by
  `a_held_note_key_is_one_press_not_one_per_repeat`.

Left alone deliberately: ctrl/alt+**drag** still changes pitch (a ctrl+click that slips into a drag edits the note
instead of toggling it), and the unshifted arrows and audition letters still act while the help card is up — the
first is a rounding-off decision, the second is the earlier ruling on the card's modality.

### Hands-on round 2 (2026-11-07)

From using the panel again: the slide had no visible cue and "didn't do a lot".

- **The audible half was the pattern, not the synth.** A slide glides into the next *note*: the sequencer only
  glides while the note is still sounding, so a slide before a rest closes the gate, and a slide into the same
  pitch has nowhere to travel. The factory line had its slides on steps 4 and 11, both of which lead into rests
  (this plan's sibling `plans/08-patterns-persistence.md` said so in its own table). The flags now sit on steps 6
  and 16; `every_slide_leads_into_a_sounding_step` keeps any phrase honest and `the_launcher_slide_actually_glides`
  measures the glide on the shipped pattern (near G just after the boundary, at E♭ by the end of the step).
- **The visual half was a 3 px silk strip** under the key, which read as panel shading. A slide now fills the strip
  under the key in `LED_ACCENT` and paints a dark arrow on the keycap. A slide that *cannot* glide — the next step
  is a rest — keeps a dim strip and loses the arrow, so the panel never promises a sound it will not make
  (`StepView::slide_glides`, `a_slide_key_paints_its_cue_and_dims_it_when_it_cannot_glide`).
- **CLEAR button added** to the switches row (fourth cell, after BANK/PATTERN/MODE). Emptying the pattern is the
  only action on the panel with no undo, so it takes two clicks: the first arms it and the button reads "CLEAR?",
  the second empties the pattern. An arm expires after 3 s and any edit or pattern switch disarms it
  (`ui::clear`, `tests/clear_live.rs`). Documented on the help card.
- The saved session's A1 was patched by hand (its slide sat on D♯→D♯). The patch script validated the pattern
  before writing, kept a `state.json.bak-*` beside it, and touched both copies the app keeps (`state.json`, which
  is what it loads, and `patterns/A1.json`).
