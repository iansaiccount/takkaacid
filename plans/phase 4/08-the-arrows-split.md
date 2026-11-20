# Plan 08 — the arrows split the other way: shift is the rack's, plain is the face's

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** `shift+up` / `shift+down` nudge the tempo, and plain `up` / `down` move the selection — the selected note on the 303's
step editor, the selected voice on the drum face. The arrows then split one way only: **`shift` is the rack's** (the pattern on
`shift+left` / `shift+right`, the tempo on `shift+up` / `shift+down`) and **the plain arrows are the face's** (the step or the
selection on `left` / `right`, the note or the voice on `up` / `down`).

**Spec:** none. The player's words: *"Let's swap the up/down and shift up/down features around. I want shifted to adjust the tempo,
and just the normal up/down to change the selection in the drum machine or change the note in the 303. This is much more
intuitive. Please also update the docs/tooltips and also any historical markdown files. We can pretend this was always the
case."* The last sentence is a ruling of its own and is recorded below: the guide, the older plans and the step-selection spec
are **rewritten to read as if the arrows had always split this way**, and this plan is the only place in the tree that records
the change.

## What is wrong today

- **The arrows split the wrong way round.** The frequent edit — a note up or down on the 303, a voice up or down on the drums —
  is behind `shift`, while the tempo, which is a performance nudge and not an edit, is on the bare key. Every other pair of
  keys on the face reads the other way: plain `left` / `right` move the step, `shift+left` / `shift+right` are the rack's
  pattern switch, and the arrows are the only pair that mixes the two.
- **The card and the guide have to explain it in three places.** `ui/help.rs` carries `up / down: tempo by 1 BPM` beside
  `shift+up / down: note up or down; on drums, which voice`; page 8, page 9, page 15, page 20 and page 22 each say the same
  thing again, and every one of them has to say which face first.
- **The tempo's own help says the wrong key.** `Control::TempoUp` / `TempoDown` print `up arrow: one BPM faster` and
  `Control::Tempo` prints `up / down arrow: one BPM faster or slower`, so a hover on the readout teaches the key the swap moves.

## The design

- **One line each, and only the modifiers move.** `shortcuts::shortcut_for` takes the tempo on `(ArrowUp, true)` and
  `(ArrowDown, true)`; `selection::edit_key` takes the note on `(ArrowUp, false)` and `(ArrowDown, false)`;
  `drum_panel::face_key` takes the voice the same way. Nothing else moves: `left` / `right` and `shift+left` / `shift+right`
  keep their jobs, `enter` stays the write/toggle on both faces, and the drum face's `h` and `l` / `b` / `p` are untouched.
- **No key is added or removed.** `is_panel_key` still asks the face first and then the shortcuts; on an acid face the step
  editor and the transport now want *different* arrows rather than the same ones, so a press is decided by its modifier alone.
  The step editor is still asked first, because a card is modal for it and its refusal must not fall through to a shortcut —
  which is now a live distinction: **behind a card the plain arrows are refused and `shift+up` / `shift+down` still nudge the
  tempo**, exactly as `space` still stops the transport. `Handled::Refused`'s doc and `a_card_refuses_the_voice_keys` say so.
- **The arrows repeat under either modifier.** `dispatch::repeats_allowed` is keyed on the key, not the modifier, and stays:
  a held plain arrow walks the selection and a held `shift` sweeps the tempo. Its doc, and the two tests that press it, say
  that rather than "only the tempo nudges repeat".
- **The face's words follow the keys.** The two keyboard lines in `ui/help.rs` swap their subjects; `Control::Tempo`,
  `Control::TempoUp` and `Control::TempoDown` name `shift` in their shortcut strings. No control gains or loses a tip.
- **The book is rewritten, not annotated.** Pages 8, 9, 15, 20 and 22 say the new mapping; `check.sh` keeps its pins and gains
  one for the tempo's modifier; `img/help-card.png` is retaken because the card's two lines changed, and `drum-face.png` is not,
  because nothing on the face's drawing moved.
- **The history is rewritten too.** `plans/phase 1/14-step-selection.md`, `plans/phase 1/18-step-sequencer.md`,
  `docs/superpowers/specs/2026-11-07-step-selection-design.md` and the phase-4 plans that quote the keys (00, 03, 04, 05, 07)
  read as they would have read had the arrows always split this way. Git history is not rewritten — the commits are what they
  are — and this plan is the record of the change.

## Tests that make it safe

- `drum_live::plain_up_and_down_walk_the_selected_voice`** — plain `down` walks to the next voice, plain `up` back; no
  audition; both ends clamp; a held plain `down` walks on (the repeat); `shift+up` is `Handled::Shortcut(TempoUp)` and moves no
  voice; no step changed.
- `drum_live::a_card_refuses_the_voice_keys` — the **plain** walk is refused behind the card, and `shift+up` still nudges the
  tempo, because a card is modal for the face and not for the rack.
- `drum_live::the_voice_keys_leave_the_acid_note_alone` — the pairing inverted: plain `down` on an acid face moves the note and
  no drum voice; `shift+up` on the same face is the tempo and moves neither.
- `selection_live::up_raises_the_selected_note_one_semitone` — plain `up`, one semitone; and `shift+up` is not the step
  editor's (`selection_dispatch` answers `false`), so the shortcut keeps it.
- `selection::tests::the_plain_arrows_edit_the_step_and_shift_is_the_transport` — the pure mapping, both directions: plain up/down are
  `NoteUp` / `NoteDown`, `shift` up/down and `shift` left/right are `None`.
- `shortcuts::tests::the_arrows_split_at_the_modifier` — `shift+up` / `shift+down` are `TempoUp` / `TempoDown`, the plain pair
  is `None`, `shift+left` / `shift+right` are still the pattern and the plain pair is still `None`.
- `dispatch::tests::only_the_arrows_repeat_out_of_the_frame` — a held `shift+up` comes back as a tempo nudge with its shift,
  a held plain `down` comes back as a walk with no shift, and a held `Enter` is refused.
- `keys::only_the_arrows_repeat_out_of_the_frame` — the same through `take_presses`, with `shift: true` on the press that came
  from a shifted arrow.
- `drum_panel::tests::the_face_keys_are_the_walk_the_step_and_the_write` — plain `up` / `down` are `MoveVoice`, `shift+up` /
  `shift+down` are `None` (the rack's), as `shift+left` / `shift+right` already were.
- `tips`' own tests — every tip still names a real key: the two tempo keys and the tempo readout say `shift`, and nothing else
  changes.
- `help_live::the_help_card_knows_the_drum_face` — the two swapped lines as the card paints them, plus a new needle for
  `shift+up / down: tempo by 1 BPM`.
- `ui::help::tests::the_help_card_names_every_gesture_and_shortcut` — unchanged in shape; it already pins `shift+up`.

Mutations: the tempo left on the plain arrows (the shortcut not swapped); the note left on `shift` (the editor not swapped);
the voice left on `shift` (the face not swapped); `repeats_allowed` narrowed to the tempo, so a held walk stops; the card's two
lines not swapped — each must fail a named test.

## Tasks

### Task 1 — the keys

**Files:** `rust/crates/acid-app/src/ui/shortcuts.rs`, `src/ui/selection.rs`, `src/ui/drum_panel.rs`, `src/ui/dispatch.rs`
(the docs and comments that name the mapping), `tests/selection_live.rs`, `tests/drum_live.rs`, `tests/keys.rs`,
`tests/device_card_live.rs`.

**Interfaces:** unchanged in shape — `shortcut_for(key, shift)`, `edit_key(key, shift)`, `face_key(key, shift)` keep their
signatures; what moves is which modifier each answers. No new public function, no new variant.

1. Write the failing tests: `drum_live::plain_up_and_down_walk_the_selected_voice`,
   `selection_live::up_raises_the_selected_note_one_semitone`, `shortcuts::tests::the_arrows_split_at_the_modifier`,
   `selection::tests::the_plain_arrows_edit_the_step_and_shift_is_the_transport`, and the reworked repeat and card tests. Run — expected
   FAIL.
2. Swap the three mappings and the comments that name them; update every test that presses an arrow so it presses the modifier
   the new mapping puts the job behind. Run — expected PASS.
3. Mutation check, recorded in Progress.
4. Commit.

### Task 2 — the face's words

**Files:** `rust/crates/acid-app/src/ui/tips.rs` (the three tempo tips' shortcut strings), `src/ui/help.rs` (the two keyboard
lines), `tests/help_live.rs`.

1. Write the failing needles: the card's `shift+up / down: tempo by 1 BPM` and `up / down: note up or down; on drums, which
   voice`, and a tips assertion that the tempo's three tips name `shift`. Run — expected FAIL.
2. Swap the lines and the strings; run — expected PASS. No control gains or loses a tip, and the card's line budget is unchanged
   (the same number of lines, the same length within a character or two).
3. Mutation check, recorded in Progress.
4. Commit.

### Task 3 — the book, the history and the picture

**Files:** `docs/guide/src/08-writing-steps.md`, `09-playing.md`, `15-keyboard.md`, `20-reference.md`, `22-the-drum-machine.md`,
`docs/guide/check.sh`, `docs/guide/src/img/help-card.png`, `img/README.md`, `plans/phase 4/00-phase-4.md`,
`plans/phase 4/03-the-kick-and-the-voice-keys.md`, `04-hit-is-the-only-sound.md`, `05-the-hit-key.md`,
`07-the-drum-keys-and-tooltips.md`, `plans/phase 1/14-step-selection.md`, `18-step-sequencer.md`,
`docs/superpowers/specs/2026-11-07-step-selection-design.md`.

1. Pages 8, 9, 15, 20 and 22: the note, the voice and the tempo, the modifier on each. The pinned phrases `walk it up and down
   the grid` and `strikes the selected voice` survive; add a pin for the tempo's modifier (`shift+up / down: tempo`).
2. The history: the phase-1 plans, the step-selection spec and the phase-4 plans that quote the keys are rewritten to the new
   split, with no "it used to be" and no "changed in round 08". `00-phase-4.md` lists 08 as the round in flight.
3. Retake `help-card.png` at the guide's size: `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` and the
   window must measure `1960x1340`; `F1` is sent with `xdotool key --window <id> F1`, because an unfocused window under Xvfb
   ignores a bare key. Record the session and the command in `img/README.md`, and look at the grab: the two swapped lines.
   `drum-face.png` is not retaken — nothing the face draws moved.
4. Run `./docs/guide/check.sh`; commit.

### Task 4 — the branch review

The three tasks are three commits. The review is of the **branch**, at the pull request (`AGENTS.md`): one dispatched subagent,
the three tasks' Files and Interfaces blocks and `git diff` against the base, and what it finds is fixed on the branch before the
PR opens. Nothing in this plan is pushed until that review has been acted on and the gates are green again.

## Non-goals

A settings switch for either arrangement; moving `left` / `right` or the pattern keys; a different repeat rate for a held walk;
`Ctrl`/`Alt` doing anything with the arrows; rewriting git history; retaking `drum-face.png` or any other picture; a tip or a
card line for the arrows beyond what is already there.

## Rulings, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| The tempo is on `shift+up` / `shift+down`, the note and the voice on the plain pair | It is the player's call, and it makes `shift` mean "the rack" for every arrow. If it reads wrong by hand, it is one line in each of the three mappings and the card's two lines — but every page of the book and the tips name the keys, so a second swap is a round of its own rather than a line. |
| A held arrow keeps repeating under either modifier, as it does today | Holding `up` to walk the note is convenient and cannot run away (the pitch clamps); if a held walk should not repeat, `repeats_allowed` takes the shift flag and the walk becomes one step per press. |
| Behind a card the plain arrows are refused and `shift+arrows` are not | The card is modal for the face, not for the rack, the same rule `space` and the pattern keys already keep. If a player expected the tempo frozen behind a card, the guard is one line — but the transport keys are what a card leaves working. |
| The history is rewritten to match, and this plan is the only record of the change | Someone reading the phase-1 plans or the spec cannot see that the arrows ever moved, which is what the player asked for. Git log is the record. If that is ever wanted back, the old text is in the commits. |

## Progress

- [x] **Task 1 — the keys.** Three mappings and nothing else: `shortcuts::shortcut_for` takes the tempo on `(ArrowUp, true)` /
      `(ArrowDown, true)`, `selection::edit_key` the note on the plain pair, `drum_panel::face_key` the voice on the plain pair.
      `left` / `right`, `enter`, `h` and `l` / `b` / `p` are untouched, no variant or `egui::Id` moved, and the set of keys the
      panel claims is the same before and after — only the handler behind each modifier changed, so no press became unreachable
      on either face. Behind a card the plain arrows are refused and `shift+up` / `shift+down` still nudge the tempo;
      `Handled::Refused`'s doc now says so, and the drum and device-card card tests assert it. `repeats_allowed` stays keyed on
      the key, so a held plain arrow walks the selection and a held shifted one sweeps the tempo; its doc, `main.rs`'s note and
      `dispatch::as_press`'s comment say "the arrows" rather than "the tempo nudges". Tests:
      `drum_live::plain_up_and_down_walk_the_selected_voice`, `selection_live::up_raises_the_selected_note_one_semitone` and
      `shift_up_is_the_transports_and_leaves_the_note_alone`, `selection::tests::the_plain_arrows_edit_the_step_and_shift_is_the_transport`,
      `shortcuts::tests::the_arrows_split_at_the_modifier` (all eight arrow/modifier pairs, both directions),
      `dispatch::tests::only_the_arrows_repeat_out_of_the_frame` (a held shifted `up` comes back as a tempo nudge with its shift,
      a held plain `down` as a walk without one), `keys::only_the_arrows_repeat_out_of_the_frame` (with a `shifted` event helper
      — the file's `press` sets repeat, not modifiers), and the rewritten `the_voice_keys_leave_the_acid_note_alone` (plain
      `down` moves the acid note, `shift+up` is the tempo on the same face and moves no note). **Mutations: 5 of 5 caught**
      (`.superpowers/mutate_arrows.py`): the tempo left on the plain arrows, the note left on shift, the voice left on shift, a
      held arrow no longer repeating, and the tempo refused behind a card.
      **Review: dispatched under the rule this round's last commit retires, and its findings were taken anyway.** BLOCK on two
      Importants and three Minors, all actioned: `dispatch::as_press`'s comment and `main.rs`'s both still said "only the tempo
      nudges repeat" (the second was in no task's Files list, so it would have survived the round); `key_repeat`'s doc still
      walked a `shift+down` the only caller no longer sends; three sentences of this plan named a test that does not exist, a
      `shift+down` where the test presses `shift+up`, and a `Handled::Refused` doc that said nothing of the rack (the doc gained
      the clause). **Deliberate, recorded:** the help card and the three tempo tips still teach the old arrow for exactly this
      commit — Task 2 is the next one, and no gate catches it because `ui::help`'s own test only pins that `shift+up` appears;
      and a *held plain arrow on the acid face* is covered only at the dispatch seam (the press repeats, and a repeat is spent
      the same way as a press) rather than end to end, because the note nudge clamps to `-12..=24` and a second live test would
      only re-assert the clamp.
      Gates: `cargo test --workspace` **901**, clippy `-D warnings`, `fmt --check`.
- [x] **Task 2 — the face's words.** The card's keyboard block now shows the arrows as two pairs — `shift+left / right: previous
      or next pattern`, `shift+up / down: tempo by 1 BPM`, then `left / right: move the selection, on either face` and
      `up / down: note up or down; on drums, which voice` — with the same seventeen lines, so the card's height is untouched
      (`the_card_fits_inside_the_panel_at_both_scale_extremes` and `the_painted_card_stays_inside_its_rect_at_every_scale` both
      still pass). The three tips that name the tempo's key say `shift`: `TEMPO -` `shift+down arrow: one BPM slower`, `TEMPO +`
      `shift+up arrow: one BPM faster`, and the `TEMPO` readout `shift+up / down arrow: one BPM faster or slower`. No control
      gained or lost a tip and the catalogue's order did not move. Tests: `help_live::the_help_card_knows_the_drum_face` asks
      for the two swapped lines as painted, and `tips::tests::the_tempo_tips_name_the_modifier` walks the three tempo tips for
      the word `shift`. **Mutations: 3 of 3 caught** (`.superpowers/mutate_words.py`): the tempo line back on the plain arrows,
      the note line back on `shift`, and the readout's tip teaching the plain pair. The second one **survived the first run** —
      a card with *both* arrow pairs behind `shift` satisfied every needle — so the live test now also asks that
      `shift+up / down: note up or down` is *not* painted, which is the assertion the mutation caught. No review was dispatched:
      the branch is reviewed at the pull request now.
      Gates: `cargo test --workspace` **902**, clippy `-D warnings`, `fmt --check`.
- [ ] Task 3 —


