# Plan 05 — `h` strikes the selected voice

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** on a drum machine, `h` strikes the voice the dials are on — the keyboard's `HIT` — so a kit can be walked with
`up` / `down` and heard with one key, without the pointer.

**Spec:** none. It is the player's answer to "is there a shortcut for the hit?", which the harness could not give: `HIT` is a
click only today. This plan adds the key; the rest of the face is plan 03's and plan 04's.

## What is wrong today

`HIT` is a button and nothing else. `ui/dispatch`'s `Drums` arm answers `up` / `down` (the walk) and then falls
through to the shortcuts, none of which is a hit; the audition keys in `ui::mod`'s `play_computer_keys` are the 303's and mean
nothing on a drum face. So plan 04's silent selection leaves a keyboard player with no way to hear the voice they selected.

**Why `h`, and why it is not a global key.** The letters are dead on a drum face and are the 303's notes on an acid one, so a
keyboard hit has to be face-aware. `h` is the mnemonic, it is not one of the transport, pattern or card keys, and a held key
must make one hit rather than a burst.

## The design

- **One mapping for the drum face's keys.** `drum_panel::FaceKey { Move(i8), Hit }` and
  `drum_panel::face_key(key, shift) -> Option<FaceKey>` replace `voice_move`: `up`/`down` are `Move`,
  `h` with no shift is `Hit`, everything else is `None`. A face's keys live in one place, so a reader asking "what does this
  face own" has one function to read.
- **The key filter asks the face.** `dispatch::is_panel_key(key, kind, shift, ctrl)`, `dispatch::presses(events, kind)` and
  `dispatch::take_presses(events, kind, typing)` take the `DeviceKind` the panel is showing, because the same key is a note on
  one face and a hit on the other. The `match kind` is written out — a third machine must decide its own keys. Ctrl-digit,
  transport, pattern and card keys stay shared; the acid arm keeps `selection::edit_key`'s step editor, the drum arm asks
  `face_key`.
- **A hit is not an edit, so a card does not stop it.** The walk refuses behind a card (it moves a selection nobody can see);
  `h` still sounds, exactly as the 303's letter keys audition through a card today. That is the rule the two faces now share.
- **`Handled::Hit`** says what happened: the press was the panel's, a voice was struck, and nothing changed. `main` discards
  the answer, so this is for the tests and the reader.
- **One hit per press.** `repeats_allowed` already lets only the arrows repeat, so a held `h` is one hit; nothing here changes
  that.

The book says it: page 22's grid section gains "`h` strikes the selected voice, the same as `HIT`" beside the walk; page 15's
two drum sentences gain the exception; page 20's keyboard table says what `h` does on each face; `check.sh` pins the phrase.

**Rulings, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| The key is `h`, not `enter` | `enter` keeps one meaning (the 303's step toggle) and is free to become something else on the drum face later. If the player prefers `enter`, it is one line in `face_key` and its test. |
| A hit sounds behind a card | The 303's letters already do, so this is the existing rule rather than a new one; a player who wants the face fully silent behind a card loses a key, not a feature. |
| `h` is silent on the acid face | It is one of the 303's notes, and `take_presses` must leave it there — that is the test that makes the face-aware filter worth having. |

## Tests that make it safe

- `tests/drum_live.rs::h_strikes_the_selected_voice` — select `SNR` with the walk (which is silent), then `h`:
  `take_audition()` is `Some(Drum::Snare)` and the answer is `Handled::Hit`; with the help card up it still strikes; and on an
  acid panel `h` auditions nothing and answers `Handled::Nothing`.
- `tests/keys.rs::the_hit_key_is_taken_on_a_drum_face_and_left_for_the_note_on_an_acid_one` — the same event through
  `dispatch::take_presses`: on a drum device it is taken out of the frame, on an acid device it is left in it for
  `play_computer_keys` to play as a note.
- `drum_panel`'s unit test, replacing the `voice_move` one — `face_key` maps exactly `up` → `Move(-1)`,
  `down` → `Move(1)`, `h` → `Hit`, and everything else (including `shift+h`) → `None`.
- `dispatch`'s unit test — `is_panel_key(Key::H, …)` is true on `Drums` and false on `Acid`.

Mutations: `h` mapped on the acid face too (the note is stolen), `h` returning `Move`, and the hit not auditioning — each must
fail a named test.

## Tasks

### Task 1 — the hit key

**Files:**
- Modify: `rust/crates/acid-app/src/ui/drum_panel.rs` (`FaceKey`, `face_key`, replacing `voice_move`)
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (the face-aware filter, the `Drums` arm, `Handled::Hit`)
- Modify: `rust/crates/acid-app/src/main.rs` (pass the face to `take_presses`)
- Test: `rust/crates/acid-app/tests/drum_live.rs`, `rust/crates/acid-app/tests/keys.rs`
- Modify: `docs/guide/src/22-the-drum-machine.md`, `15-keyboard.md`, `20-reference.md`, `docs/guide/check.sh`,
  `plans/phase 4/00-phase-4.md`

**Interfaces:**
- Consumes: `PanelState::showing_kind()`, `PanelState::a_card_is_up()`, `PanelState::drum.selected`,
  `SharedState::request_audition(&self, drum: Drum)`, `DeviceKind`.
- Produces: `pub enum FaceKey { Move(i8), Hit }` and `pub fn face_key(key: egui::Key, shift: bool) -> Option<FaceKey>` in
  `ui::drum_panel`; `Handled::Hit`; `is_panel_key(key, kind, shift, ctrl)`, `presses(events, kind)`,
  `take_presses(events, kind, typing)`.

1. Write the failing tests; run them — expected FAIL (`face_key` does not exist; `h` answers `Handled::Nothing`).
2. Implement the mapping, the filter's `kind`, the `Drums` arm's `Hit` and `main`'s argument. Run the tests — expected PASS.
3. Mutation check, recorded in Progress.
4. The book, `check.sh`'s pin, and the phase index. Run `./docs/guide/check.sh`.
5. Full gates, review, commit.

## Non-goals

Any other letter striking a voice (a full set of keyboard pads is the per-voice MIDI/pads round); `h` on the acid face; a key
for `HIT` on the 303 (it has no HIT); making the hit repeat while held; a settings switch; a save-file change; a screenshot
(the face paints nothing new — the key leaves no mark).

## Progress

- [x] **Task 1 — the hit key.** `drum_panel::FaceKey { Move(i8), Hit }` and `face_key` replaced `voice_move` as the face's one
      key mapping: `up`/`down` are `Move`, `h` with no shift is `Hit`, `shift+h` is nothing. `dispatch`'s
      `is_panel_key`, `presses` and `take_presses` now take the `DeviceKind` the panel is showing — a `match kind` written out,
      so a third machine must choose — because the same `h` is one of the 303's notes; `main` passes `showing_kind()`. The
      `Drums` arm answers `Hit` with `shared.request_audition(panel.drum.selected)` and `Handled::Hit`, and a card does not stop
      it: an audition is not an edit, and the 303's letters already play through a card. One hit per press — `repeats_allowed`
      still lets only the arrows repeat. Tests: `h_strikes_the_selected_voice` (select `SNR`, `h` strikes it and
      answers `Handled::Hit`; still strikes behind a card; on an acid panel `h` answers `Handled::Nothing` and strikes nothing),
      `the_hit_key_is_taken_on_a_drum_face_and_left_for_the_note_on_an_acid_one` (the frame keeps `h` for the 303),
      `the_face_keys_are_the_walk_and_the_hit` and `the_face_decides_whether_a_letter_is_the_panels` (the mappings themselves).
      **Mutations: 3 of 3 caught, 0 skipped** (`.superpowers/mutate_hit_key.py`) — the acid face taking `h` too, `h` walking
      instead of striking, and a hit that sounds nothing — each named by the test that caught it. The filter change touched the
      test helpers that build presses (`drum_live`, `settings_live`, `keys`), which now pass the panel's own kind. Gates:
      `cargo test --workspace` **883**, clippy `-D warnings`, `fmt --check`, `./docs/guide/check.sh` ok.
      **The book moved:** page 22 gains `h` beside the walk and its *No notes* bullet is rewritten — no letter plays a note, and
      `h` strikes a voice; page 15's two drum sentences gain the exception; page 20's keyboard table gains an `h` row; `check.sh`
      pins `strikes the selected voice`; `00-phase-4.md` lists the round. No screenshot: the face paints nothing new.
      **Rulings:** the key is `h`, not `enter` (which keeps the 303's step toggle); a hit sounds behind a card, the rule the
      303's letters already keep; and `h` stays a note on the acid face. If the player wants `enter` instead it is one line in
      `face_key` and its test; if the card should silence the hit, one `a_card_is_up()` line and a test.
      **A second consequence, recorded because nothing else would say it:** the filter now asks each face for its keys, so
      `Enter` and unshifted `Left`/`Right` — which the old filter claimed for the 303's step editor on *every* face — are no
      longer taken from the frame on a drum face. That is right (a drum face has no step editor, and the panel's rule is that a
      key it does not own is the widgets'), and `tests/keys.rs` now pins it: with `DeviceKind::Drums`, `Enter` and `ArrowLeft`
      stay in the buffer. `main`'s one new line — passing `panel.showing_kind()` — is the line that makes the key work in the
      app, and `main` is a binary no test can call: it is read, not tested, which is the same gap every `main` decision has.
      **Review: BLOCK on two Importants and three Minors, every one actioned.** The Importants were both records rather than
      behaviour: this entry now names the `Enter`/`Left`/`Right` ownership move and its new test, and it names `main`'s line as
      read-not-tested so "3 of 3" is not read as covering it. Of the Minors: the new test is renamed `h_strikes_the_selected_voice`
      (`cargo test` could not tell it from the existing `HIT`-button test `the_hit_key_auditions_the_selected_voice`); page 22's
      `h` paragraph now says a card does not stop it; and the in-app help card (`ui/help.rs`) still lists only the 303's keys —
      the drum face's walk and hit are a face change (line count, `help_content_height`, and the pinned `help-card.png`), so it
      stays deferred and is written into the wishlist rather than smuggled into this commit.
