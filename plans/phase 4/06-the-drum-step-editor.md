# Plan 06 — the drum face's step editor

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`). This plan is one of two written together; plan 07 is the other.

**Goal:** the drum face edits a *step*, like the 303's: `left` / `right` move a visible selection across the sixteen steps,
`enter` writes the selected voice into it, `shift+enter` is the accent gesture, and the selection does not move on a write
because a step holds sixteen voices.

**Spec:** none. It is the player's request in their words: *"We absolutely do need to be able to step edit the drum face too.
The shift and left/right should select the step and we should be able to 'hit' a selected voice to add it to the step with the
same accent keys (similar to the 303) … The only difference with this step editor is that it won't automatically select the
next step when a hit is selected as it can have multiple voices per step."* Where the words left a choice, the **Rulings**
table below records the call.

## What is wrong today

- **There is no step selection on the drum face.** `PanelState::selected` is the step the 303 edits; it is real, clamped and
  per device (it travels in `PanelEdit`, so each device remembers its own), and the drum face never reads it and never draws
  it. The grid's columns are steps with no mark of which one a key would write.
- **The grid's only writes are the pointer's.** `drum_panel::draw_grid`'s cell click is the whole editor: a plain click
  toggles the voice's hit on that cell, a `SHIFT`-click is the accent gesture. A keyboard player can select a voice (plan 03)
  and hear it (plan 05), but cannot write it into a step at all.
- **The two editors disagree where they should not.** On the 303, plain `left`/`right` move the selection and `enter` toggles
  the step; on a drum face plain `left`/`right` fall through to nothing (they are the tempo's only on `up`/`down`) and
  `enter` is not even a panel key — plan 05 deliberately left it to the widgets, because a drum face then had no step editor.

## The design

- **One step, shared by both faces.** The drum face reads and writes `PanelState::selected` — the 303's own field, already
  per device through `PanelEdit`, so switching devices restores each one's step. The *voice* selection stays panel-wide
  (`PanelState::drum`), which is plan 01's ruling and is not changed here.
- **The selection is a lit column.** `drum_panel::column_rect(l, step)` is the union of the sixteen cell rects for that step
  (from the first row's band to the last's), derived from the layout so it is the same at every scale. After the cells are
  painted, the face strokes it in `p.led_accent`, `1.5 * s` wide — the pointer's selection on the row stays as it is, so the
  lit plate at the left and the lit column cross at the voice and step a key would write.
- **`left` / `right` move it, clamped.** `FaceKey::MoveStep(i8)`, applied with `selection::move_selection` over
  `STEPS_PER_PATTERN` — the 303's own clamp. `shift+left` / `shift+right` stay the pattern switch: that is the 303's split,
  and it is the only keyboard way to change pattern on either face.
- **`enter` writes, silently.** `FaceKey::Write { accented: bool }` applies the grid's own click gesture to (selected voice,
  selected step) through one pure function, `drum_panel::toggle_hit(step, drum, accent) -> Step`, lifted out of the click
  handler so the pointer and the keyboard cannot answer differently:
  - plain: an empty cell gets the hit, a struck one loses it — `enter` toggles, exactly as it does on a 303 step;
  - `shift`: an empty cell gets an *accented* hit, a plain strike becomes accented, an accented one is taken off — the three
    cases the `SHIFT`-click already has.
  A write is an edit, so `drum_panel::strike_step(state, shared, drum, accent)` does what the click does today (`set_step`,
  mirror `panel.pattern`, mark the session dirty) **and** disarms a pending `CLEAR` — the rule every pattern edit keeps.
- **Writing is silent, and `h` is the ear.** Plan 05's `h` still strikes the voice *without* writing it. The player chose the
  silent write, so this is the one deliberate difference between the keys of the two editors: `enter` writes, `h` sounds,
  and the 303 has no second key because a note is heard by playing the letter it came from.
- **No auto-advance.** The selection stays where it is after a write; `STEP` stays the 303's (page 22 says it lights and
  writes nothing on a drum face, which this round keeps true).
- **`enter` returns to the panel on a drum face.** Plan 05 recorded that `enter` and unshifted `left`/`right` became the
  widgets' there; `tests/keys.rs` pins that, and this round reverses it for those three keys, with the test updated in the
  same commit and the reason written down.

## Tests that make it safe

In `rust/crates/acid-app/tests/drum_live.rs`, through `draw_panel` for the pointer and `dispatch::presses` /
`dispatch::handle` for the keys, the way `main` delivers them:

- `the_arrows_move_the_selected_step_on_a_drum_face` — `right` takes 1 → 2, `left` back, clamped at 0 and at 15; the voice
  selection does not move; the pattern does not change; plain `up`/`down` are still the tempo.
- `enter_writes_the_selected_voice_into_the_selected_step` — select `SNR`, step 2, `enter`: step 2 strikes `SNR` unaccented,
  the audition slot is empty (the write is silent), step 1 is untouched and the session is dirty; a second `enter` clears it.
- `shift_enter_is_the_accent_gesture` — empty → accented; struck plain → accented; struck accented → cleared.
- `the_keyboard_and_the_pointer_write_the_same_step` — the same action by click and by key leaves equal `Step`s, because both
  go through `strike_step`.
- `a_card_refuses_the_write_and_still_allows_the_hit` — help card up: `enter` is `Handled::Refused` and the pattern is
  unchanged, while `h` still sounds the selected voice.
- `the_selected_step_is_painted` — pick a step that is not a multiple of four, so the beat frame is not the rect under test;
  `tests/drum_live.rs`'s existing `strokes()` walker grows from `Vec<Rect>` to `Vec<(Rect, egui::Stroke)>` so it can assert
  the stroke is `p.led_accent` and the rect is `column_rect(l, selected)`.
- `a_write_disarms_a_pending_clear` — a panel whose `clear_armed_at` is set (the field directly: the drum face draws no
  `CLEAR`, but an arm made on the 303 survives a device switch, which is exactly why the disarm matters) loses it on `enter`,
  and keeps it on `h`, because an audition is not an edit.
- In `tests/keys.rs`, `the_hit_key_is_taken_on_a_drum_face_and_left_for_the_note_on_an_acid_one`: `Enter` and `ArrowLeft` are
  taken on **both** faces for different jobs — the drum write on drums, the step editor's `Selection` on acid — so the test
  says that rather than "left on an acid face".
- `tests/drum_live.rs::a_drum_device_ignores_the_keyboard_and_the_note_queue` now expects `Handled::Selection` from `Enter`
  where it expected `Handled::Nothing`; the note-queue and gate assertions stay, because a drum face still writes no 303 note.
- `drum_panel::tests::the_face_keys_are_the_walk_and_the_hit` is rewritten for the new arms: `Enter` is `Write`, plain
  `left` / `right` are `MoveStep`, and `shift+left` / `shift+right` are still `None`, because they are the pattern shortcut.
- `drum_panel`'s unit tests walk `toggle_hit`'s three cases.

Mutations: no clamp on the step walk; `enter` sounding an audition; `shift+enter` writing an unaccented hit; the write not
disarming `CLEAR`; the column not painted — each must fail a named test.

## Tasks

### Task 1 — the column is drawn

**Files:** `rust/crates/acid-app/src/ui/drum_panel.rs` (`column_rect`, the stroke in `draw_grid`), `tests/drum_live.rs`.

**Interfaces:** produces `pub fn column_rect(l: &layout::PanelLayout, step: usize) -> Rect`.

1. Write `the_selected_step_is_painted` and run it — expected FAIL (no stroke).
2. Draw the column after the cells; run the test — expected PASS; the face still paints from the palette.
3. Review, commit.

### Task 2 — the keys write

**Files:** `rust/crates/acid-app/src/ui/drum_panel.rs` (`toggle_hit`, `strike_step`, `FaceKey::{MoveVoice, MoveStep, Write}`),
`rust/crates/acid-app/src/ui/dispatch.rs` (the `Drums` arm), `src/ui/mod.rs` if a doc names the keys, `tests/drum_live.rs`,
`tests/keys.rs`.

**Interfaces:** consumes `selection::move_selection`, `SharedState::set_step`, `PanelState::{selected, pattern, dirty,
a_card_is_up}`, `super::clear::disarm`; produces `pub fn toggle_hit(step: Step, drum: Drum, accent: bool) -> Step`,
`pub fn strike_step(state: &mut PanelState, shared: &SharedState, drum: Drum, accent: bool)`.

1. Write the six live tests and the unit tests; the click's handler now calls `strike_step` so there is one writer. Run —
   expected FAIL.
2. Implement the variants, the pure function, the writer and the arm; `enter` / `left` / `right` become panel keys on a drum
   face. Run — expected PASS; update `tests/keys.rs`'s ownership test and the two `drum_live` tests the new arms change
   (`a_drum_device_ignores_the_keyboard_and_the_note_queue` and the `drum_panel` unit test named above).
3. Mutation check, recorded in Progress.
4. Review, commit.

### Task 3 — the book and the picture

**Files:** `docs/guide/src/22-the-drum-machine.md`, `15-keyboard.md`, `20-reference.md`, `docs/guide/check.sh`,
`docs/guide/src/img/drum-face.png` + `img/README.md`, `plans/phase 4/00-phase-4.md`.

1. Page 22: the lit column, `left`/`right`, `enter` and `shift+enter`, the silent write, no auto-advance, and `h` beside it;
   its *No notes* bullet ("`shift` and the arrows move between its voices, not notes") is rewritten to say what the arrows now
   move and what `enter` writes. Page 15's editing-keys paragraph and its two drum sentences; page 20's table rows
   (`left`/`right` and `enter`; `shift+enter`).
2. `check.sh` pins a phrase for the write. Retake `drum-face.png` at the guide's size with a fresh data directory: first
   `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` (a small display silently wins otherwise), then the
   window must measure `1960x1340`; park the pointer, record the command in `img/README.md`, and look at the grab — the column
   is the thing it must show.
3. List the round in `00-phase-4.md`; run `./docs/guide/check.sh`.
4. Review, commit.

## Non-goals

The playhead on the drum face (it has no running-step mark today; this round adds the selection, not the clock); a drum
"record" from a pad; `backspace`, `STEP` and the rest's meaning on drums; MIDI notes writing drum steps; the voice selection
becoming per device; a step's length or a per-step edit beyond the hit and its accent.

## Rulings, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| Step selection is plain `left` / `right`, not `shift+left/right` | The 303's split is kept and the pattern switch survives on the drum face. If the player meant the shift pair, it is one line in `face_key` and its test — and the pattern key needs a new home on that face. |
| The write is silent | Chosen by the player. `h` is the ear. If it should sound as it writes, one `request_audition` line in `strike_step`, and it becomes a difference from the 303 instead of a match. |
| `shift+enter` is the grid's accent gesture, not the 303's shift+`enter` (also a plain toggle there) | The gesture the player already knows on that face wins over the 303's key for key. The alternative makes `shift+enter` a duplicate of `enter` and leaves accenting to the pointer. |
| A write does not move the selection | The whole point of the round: a step holds sixteen voices, so a hand writes several before moving on. |

## Progress

- [x] **Task 1 — the column is drawn.** `drum_panel::column_rect(l, step)` is the step's sixteen cells as one rect (x from the
      first row's cell for that step, y from the first row's band to the last's, clamped), and `draw_grid` strokes it after the
      cells in `p.led_accent` with a `1.5 * s` **gutter in `p.panel` just outside it**, because an accented hit is filled with
      the same accent and the column's edge would otherwise vanish wherever it crossed one. Tests:
      `drum_live::the_selected_step_is_painted` (step 6, not a multiple of four so no beat frame is near; painted in the black
      scheme **and** in `ChoirPastel`, so a literal colour could not pass — the black scheme's `led_accent` and `lit_ink` are the
      same value) and `drum_panel::tests::the_column_is_one_step_wide_and_sixteen_rows_tall` (the rect against the cells and the
      bands, plus the clamp — the live test compares against `column_rect`, so without this a rect that was one cell tall would
      still pass). Gates: `cargo test --workspace` **885**, clippy `-D warnings`, `fmt --check`. The book is deferred to Task 3:
      page 22 describes no step selection today, so it is knowingly one round behind for these two commits, and `check.sh` is
      run with Task 3.
      **Review: PASS on one Important and three Minors, every one actioned.** The Important was the accent-on-accent collision
      (fixed with the panel gutter above, and it matters precisely because Task 2 writes accents into the selected step); the
      Minors were the geometry not pinned (the unit test above), the palette test painting one scheme (the second-scheme half
      above), a stale doc line above `strokes_with_ink` (deleted), and a doubled clamp (`draw_grid` now passes `state.selected`
      and `column_rect` clamps). The reviewer's note that `column_rect` builds a row's 16 rects for one cell was left as it is:
      it is one small allocation a frame on the UI thread, not on the audio path, and `row_band`/`cell_rect` already own that
      geometry.
- [x] **Task 2 — the keys write.** `FaceKey` is now `MoveVoice`, `MoveStep`, `Write { accented }` and `Hit`; `face_key` maps plain
      `left`/`right` to the step, `shift`+`up`/`down` to the voice, `enter` to the write (`shift` = accent) and `h` to the hit.
      `drum_panel::toggle_hit(step, drum, accent)` is the grid's whole editor as one pure function, lifted out of the click, and
      `strike_step(state, shared, step, drum, accent)` is the writer both hands use — **it takes the step**, which the plan's
      Interfaces did not: the click writes the cell under the pointer, not the selected step, and one writer with a step
      parameter is what keeps the two from disagreeing. `dispatch`'s `Drums` arm clamps the step with `move_selection` (so the
      arrows stop at step 1 and 16), refuses both the walk and the write behind a card, writes through `strike_step`, and disarms
      a pending `CLEAR`; `h` still sounds behind a card. The cell click is now four lines calling `strike_step`. Tests:
      `the_arrows_move_the_selected_step_on_a_drum_face` (both ends clamped, no voice moved, no edit, `up` still the tempo),
      `enter_writes_the_selected_voice_into_the_selected_step` (the hit lands on the selected step, unaccented, silent, dirty, and
      a second `enter` takes it off), `shift_enter_is_the_accent_gesture` (all three of the click's cases),
      `the_keyboard_and_the_pointer_write_the_same_step` (the click and `shift+enter` leave equal `Step`s),
      `a_card_refuses_the_write_and_still_allows_the_hit`, `a_write_disarms_a_pending_clear`, and
      `drum_panel::tests::the_hit_gesture_adds_toggles_and_accents`. `tests/keys.rs` now says `enter` and `left` are taken on
      **both** faces (for different jobs), and `a_drum_device_ignores_the_keyboard_and_the_note_queue` expects
      `Handled::Selection` from `enter` and asserts the write left no 303 note (pitch, octave, velocity, accent, slide — the
      `on` bit follows the drum mask by `Step::hit`'s own rule, before this round). **Mutations: 4 of 4 caught, 0 skipped**
      (`.superpowers/mutate_step_editor.py`) — the step arrows wrapping, a write that sounds, a write that leaves `CLEAR`
      armed, and `shift+enter` writing a plain hit — each named by the test that caught it. Gates: `cargo test --workspace`
      **892**, clippy `-D warnings`, `fmt --check`.
      **The pointer's write disarms too:** the disarm moved out of `dispatch` and into `strike_step`, so the click and the key
      keep the `CLEAR` rule together; `a_write_disarms_a_pending_clear` pins the key half and the click's own half is the same
      call now, which is what the parity test protects.
      **The book moved in this commit, one step ahead of Task 3**, because these keys turned two sentences into falsehoods
      rather than omissions: page 15's four-keys paragraph, page 22's *No notes* bullet and page 20's `left`/`right` and
      `enter` rows now say what the drum face does. Task 3 still owns page 22's grid prose, the pinned phrase and the picture.
      **The fifth mutation, run when the reviewer asked for it:** removing the column's accent stroke fails
      `the_selected_step_is_painted` (`.superpowers/mutate_column.py`), so Task 1's "the column not painted" is a caught
      mutation and not a claim.
      **Review: BLOCK on two Importants, five P2s, all actioned.** (1) The pointer's write did not disarm `CLEAR` while the
      key's did — the click now goes through `strike_step`, which disarms, and the doc comments that said "masks only" and
      "the mask and `on`" were corrected. (2) The book contradicted the panel at two sentences and Task 2's entry said nothing
      about it — both sentences and page 20's two rows are fixed here, and the paragraph above records the rest as Task 3's. Of
      the P2s: `PanelState::selected`'s doc, `a_card_is_up`'s list and `Handled::Refused`'s doc now name the drum face's step
      move and write; the unit test is renamed for the step and the write; and the refused-arrow assertion also pins that the
      selection did not move.
- [x] **Task 3 — the book and the picture.** Page 22's grid section now says the lit column, `left`/`right`, `enter` and
      `shift+enter`, the silent write, the 303's walk-on that this face deliberately does not do, and `HIT`/`h` beside it — on
      top of Task 2's three sentence fixes (page 15's four-keys paragraph, page 22's *No notes* bullet, page 20's `left`/`right`
      and `enter` rows). `check.sh` pins `move the selected step` and `writes the selected voice into it` beside the two drum
      pins from the last round. **`drum-face.png` retaken** on `Xvfb :87 -screen 0 2200x1500x24` (geometry checked first,
      `2000 1400` or better, then the window at `1960x1340`) with `TAKKA_ACID_DATA_DIR=/tmp/takka-guide-06`, empty: `+` →
      `DRUMS`, the notice waited out, the pointer parked outside the window. The picture shows the round's change — the first
      step's column lit — and its `img/README.md` row records the new state and command, including why the grid is empty (a
      first run's drum machine has no bank; the earlier picture's demo bank is no longer part of it). Looked at before keeping:
      the column is visible, no tooltip and no notice are in the frame. Gates: `cargo test --workspace` **892**, clippy
      `-D warnings`, `fmt --check`, `./docs/guide/check.sh` ok.
      **Review: BLOCK on one Important and two P2s, all actioned.** The Important was the session recipe I had reordered in
      `img/README.md`: opening the kit browser with `F3` puts its shield over the panel, so a later `BANKS` click would close the
      browser instead of opening the bank browser — the steps are back in their performable order, the "both pictures" sentence
      is scoped to `kit-browser.png`, and the retake carries its date. Of the P2s: page 22 said "the `HIT` key is the one to
      reach for when the bar is stopped", which implies a difference between `HIT` and `h` that the code does not have (they are
      one audition path), so it now says either of them plays the voice with the bar stopped; and the reviewer's note that the
      column sits on step 1, which is also a beat-framed column, is recorded in the picture's row rather than retaken — the
      full-height stroke and the per-cell frame read apart.
