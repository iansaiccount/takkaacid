# Plan 03 — the kick does not sound, and the drum face has no keyboard

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** the `808` kit's kick is heard on the speakers people actually have, and `shift+up` / `shift+down` walk the drum
face's selected voice up and down its grid, hearing each one it lands on.

**Spec:** none. The drum machine's spec
([`2026-11-18-drum-machine-design.md`](../../docs/superpowers/specs/2026-11-18-drum-machine-design.md)) covers what the kits
and the face are; a kick that is a speaker's blind spot and a keyboard gesture the face never had are defects and a missing
gesture, not design. This plan argues from measurements of the files that ship, and from the face as [`22-the-drum-machine`]
describes it.

## What is wrong today, with the evidence

### The `808` kit's kick is a 40 Hz sine with no attack

Reported as *"the sound on the kick drum on the 808 kick does not sound. Possibly the sample is missing."* The sample is not
missing: `rust/crates/acid-app/kits/808/KICK.wav` is tracked, 49 416 bytes, and decodes to **0.56 s, peak 0.89, RMS 0.143**,
and `tests/kits.rs::every_factory_voice_plays_the_file_named_after_it` proves the compiled-in voice is that file. What it is,
measured over its own samples, with the same two-pole 200 Hz high-pass the test uses
(`.superpowers/kick_measured_like_the_test.py`):

| file | peak of the low band | share of RMS above a two-pole 200 Hz |
|---|---|---|
| `808/KICK.wav` (`bd_808`, before this round) | **40 Hz** | **5.4%** |
| `808/KICK.wav` (`bd_fat`, after it) | 75 Hz | 21.0% |
| `909/KICK.wav` (`bd_haus`) | 90 Hz | 17.6% |
| `acoustic/KICK.wav` (VCSL) | 90 Hz | 23.2% |

A laptop, a monitor and a phone are all rolled off below 150–250 Hz, so a 40 Hz sine with 5% above that line is a driver
moving air it cannot move, with no click above it to announce the hit. The kit then tunes the voice **2 semitones down**
(`kits.rs`'s `FACTORY` row), which is 35 Hz. That is the report: not a missing sample, a sample outside the speaker.

### The drum face has no keyboard

`ui::dispatch` gates the step editor off for a drum device (its editor is the grid) and then falls through to
`shortcuts::shortcut_for`. `shift+up` / `shift+down` are the 303's note nudges, matched only in `selection::edit_key`, so on
a drum face they mean nothing at all: the press is a panel key (`is_panel_key` asks `edit_key`) and resolves to
`Handled::Nothing`. A player who has selected a voice with a click has no keyboard way to reach the next one, which is the
gesture the face is shaped for — sixteen rows, one selection.

## The design

- **The `808`'s kick becomes an audible one.** Change the one row in `kits/refresh.py`'s table from `bd_808` to **`bd_fat`** (a
  75 Hz analog kick, 0.232 s, 21.0% of its RMS above a two-pole 200 Hz where the `909`'s kick keeps 17.6%) and re-run the
  script, which regenerates `808/KICK.wav` **and** `CREDITS.md` from the same table, so a credit and its
  bytes cannot disagree. Nothing else in `kits.rs` changes: the kit's `-2` semitones lands the new file at ~67 Hz, which is an
  808's own pitch, and the voice's 500 ms decay is longer than the file. The other fifteen voices are untouched.
- **A kick must keep energy above the sub.** `tests/kits.rs` gains one test over all three kits: a kick that loses more than
  90% of its RMS to a two-pole 200 Hz high-pass is a kick nobody hears on a laptop. The defect measures 5.4%; the three fixed
  kicks measure 17.6%, 21.0% and 23.2%.
- **`shift+up` / `shift+down` walk the drum face's selection.** `drum_panel::voice_move(key, shift) -> Option<i8>` is the one
  mapping (-1 up the grid, +1 down it, `Drum::ALL`'s order, which is the rows' order). `dispatch::handle` applies it for a
  `Drums` device before the shortcuts: the selection moves with `selection::move_selection` over `Drum::ALL.len()` — so it
  **clamps**, it does not wrap — and each voice it lands on is **auditioned**, the same `request_audition` a click on the
  voice's label makes. A card up refuses the press (`Handled::Refused`), the rule every keyboard writer keeps; on the acid face
  the same keys still nudge the note, because the two faces are never up together.
- **The book says both.** Page 22 gains the keyboard walk under *The grid* and loses the clause that the keyboard does
  nothing on a drum machine; page 15's drum sentence and page 20's key table gain the same fact; `check.sh` pins the phrase.

**Rulings this round rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| The `808`'s kick is replaced by `bd_fat` rather than keeping the inaudible `bd_808` sine | The kit's kick is no longer literally Sonic Pi's "808 bass drum". Putting it back is one row in `refresh.py` and a re-run; the synthesized sweep is in the wishlist. |
| Audibility is pinned as "at least 10% of RMS survives a two-pole 200 Hz high-pass" | A genuinely deep kick that only a large speaker can move would fail the test. The number is the defect's own (5.4% before, 17–23% after) and is a threshold to argue with, not a law. |
| The walk auditions every voice it lands on | A player who wants silent navigation hears a hit per press, and a held key walks the kit by ear (which is the point). The other way is deleting one `request_audition` line. |
| The walk clamps rather than wraps | `shift+down` from `SHK` reaching `KICK` would be a jump across the whole grid; clamping is `move_selection`'s existing rule for the step border. |

## Tests that make it safe

- `tests/kits.rs::every_factory_kick_is_audible_above_the_sub_bass` — a two-pole 200 Hz high-pass helper in the test file, and
  every factory kit's `KICK` keeps **≥ 10%** of its RMS above it. Fails on today's `808` at 5.4%.
- `tests/drum_live.rs::shift_up_and_down_walk_the_selected_voice` — a drum panel and a `PatternBank`, presses delivered through
  `dispatch::presses` and `dispatch::handle` the way `main` delivers them: `shift+down` takes `KICK` → `SNR` and leaves
  `Some(Drum::Snare)` in the audition slot; `shift+up` takes it back; at `KICK`, `shift+up` stays `KICK`; at `SHK`,
  `shift+down` stays `SHK`; and the pattern's steps do not change, because a selection is not an edit.
- `tests/drum_live.rs::a_card_refuses_the_voice_keys` — with the help card up, `shift+down` answers `Handled::Refused`, leaves
  the selection on `KICK` and audits nothing, the `Press::Refused` rule the step editor already keeps.
- `tests/drum_live.rs::the_voice_keys_leave_the_acid_note_alone` — on an acid device, `shift+down` still nudges the selected
  step's note, so the drum mapping did not swallow the 303's.
- `drum_panel`'s own unit test — `voice_move` is `-1`/`+1`/`None` for exactly `shift+up`, `shift+down` and everything else, and
  the two directions are the grid's up and down.

## Tasks

### Task 1 — the `808`'s kick is heard

**Files:**
- Modify: `rust/crates/acid-app/kits/refresh.py` (the `KITS["808"]` kick row)
- Regenerate: `rust/crates/acid-app/kits/808/KICK.wav`, `rust/crates/acid-app/kits/CREDITS.md`
- Test: `rust/crates/acid-app/tests/kits.rs`
- Modify: `plans/future/wishlist.md` (the synthesized 808 sweep, as the option not taken)

**Interfaces:**
- Consumes: nothing.
- Produces: nothing in code; the factory `808` kit's `Drum::Kick` sample changes its bytes and its credit line.

1. Write the failing test `every_factory_kick_is_audible_above_the_sub_bass` in `tests/kits.rs`, with the helper
   `fn share_above(samples: &[f32], rate: f32, hz: f32) -> f32` (two cascaded one-pole high-passes; RMS out over RMS in).
   Run it: `cargo test -p acid-app --test kits every_factory_kick` — expected FAIL on the `808` kit.
2. Change `refresh.py`'s `808` row to `("sp", "bd_fat", "fat bass drum")`, back the ship folder up to `/tmp`, run
   `python3 rust/crates/acid-app/kits/refresh.py`, and confirm with `git status` that only `808/KICK.wav` and `CREDITS.md`
   moved (restore any other file from the backup — they are regenerated, not edited, and an unrelated byte change is churn).
3. Run the test again — expected PASS; then `cargo test --workspace`, clippy and fmt.
4. Add the wishlist line: a synthesized 808 sweep (pitch-swept sine plus click) would be the truest kick, at the cost of a
   third provenance kind in `CREDITS.md`.
5. Review, then commit.

### Task 2 — the voice keys

**Files:**
- Modify: `rust/crates/acid-app/src/ui/drum_panel.rs` (`voice_move`, and its unit test beside the others)
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (the `Drums` arm of the face match)
- Test: `rust/crates/acid-app/tests/drum_live.rs`
- Modify: `docs/guide/src/22-the-drum-machine.md`, `docs/guide/src/15-keyboard.md`, `docs/guide/src/20-reference.md`,
  `docs/guide/check.sh`, `plans/phase 4/00-phase-4.md`

**Interfaces:**
- Consumes: `Drum::ALL`, `Drum::index()`, `selection::move_selection(current: usize, delta: i8, count: usize) -> usize`,
  `SharedState::request_audition(&self, drum: Drum)`, `PanelState::a_card_is_up()`, `PanelState::showing_kind()`.
- Produces: `pub fn voice_move(key: egui::Key, shift: bool) -> Option<i8>` in `ui::drum_panel`;
  `dispatch::handle` answers `Handled::Selection` for a walk, `Handled::Refused` for a walk behind a card.

1. Write the three `drum_live.rs` tests and the `voice_move` unit test. Run them — expected FAIL (no such function, presses
   resolve to `Handled::Nothing`).
2. Implement `voice_move` and the `Drums` arm. Run the tests — expected PASS.
3. Mutation check, recorded in Progress: (a) drop the clamp, (b) drop the audition, (c) drop the card refusal, (d) swap the two
   directions — each must be caught by a named test.
4. The book: page 22's grid section and *What is not here*; page 15's drum sentence; page 20's key table; a `check.sh` pin;
   the round listed in `00-phase-4.md`. Run `./docs/guide/check.sh`.
5. Full gates, review, commit.

## Non-goals

A synthesized kick generator; the keyboard's letters playing drum voices (they stay silent on a drum face, as page 22 says);
per-voice hover help; per-voice MIDI; a save-file change (`SAVE_VERSION` stays 11); a screenshot (the face draws exactly what
it drew — the change is a sample's bytes and a key that leaves no mark); and the rack document, which is still the phase's
next round and still needs its own design.

## Progress

- [x] **Task 1 — the `808`'s kick is heard.** The failing test first: `tests/kits.rs::every_factory_kick_is_audible_above_the_sub_bass`
      on the kit as it shipped printed *"808: the kick keeps only 5.4% of its level above 200 Hz, which a small speaker will not
      sound"* (the shipped helper, at 48 kHz). Then `refresh.py`'s one row changed to `("sp", "bd_fat", "fat bass drum")` and the
      script re-run, which regenerated `808/KICK.wav` (10 241 frames, 0.232 s) and `CREDITS.md`'s row from the same table; the
      other 47 WAVs came back byte-identical, so the commit's binary change is the kick and nothing else. The test now passes,
      and the same filter measures 20.95% for the new kick against 17.62% (909) and 23.22% (acoustic). Gates at the commit:
      `cargo test --workspace` **876** green, clippy `-D warnings`, `fmt --check`. The book did not move and does not have to:
      no page names a kit's sample file, the face is unchanged, and `check.sh`'s kit pins are `KIT`, `MY KITS` and `Acoustic`.
      **Ruling:** audibility beats the sample's name — the kit's kick is a recording with an attack, not a synthesized 808
      (wishlist), because it keeps `CREDITS.md`'s two-collection claim true. If wrong, one row in `refresh.py` and a re-run.
      **The listen is the player's**: no test can hear the mix, and the new file's RMS (0.35) is level with the 909's kick
      (0.40), so it is not a hot sample — the old one was 0.14 because a sine spends its peak budget in one slow excursion.
      **Review: PASS on two Importants and five Minors, every one actioned.** (1) The evidence table mixed two filters (a
      four-pole 150 Hz column beside a two-pole 200 Hz one, an impossible pair), so it is now one filter — the test's own two-pole
      200 Hz — re-measured on the files with `.superpowers/kick_measured_like_the_test.py`, which reproduces the shipped helper.
      (2) This entry is that fix's record, with the failing output it demanded. Of the Minors: the helper now calls
      `acid_core::analysis::rms` instead of a second inline RMS; the plan's "in absolute terms" was wrong about a ratio and is
      gone; the wishlist's "no recording in the two collections" overclaimed an audition of ~14 Sonic Pi kicks, so it names
      those; the `__pycache__` left by importing `refresh.py` from a scratch script is deleted and the tree is clean. The
      reviewer's one question it could not settle — whether the 200 Hz column was 4-pole or 2-pole — is answered by the
      re-measurement: 2-pole, 5.42% before and 20.95% after, at 44.1 kHz on disk.
- [x] **Task 2 — the voice keys.** `drum_panel::voice_move(key, shift) -> Option<i8>` is the one mapping (-1 up the grid,
      +1 down it, `Drum::ALL`'s order), and `dispatch::handle`'s `DeviceKind::Drums` arm walks `panel.drum.selected` with
      `selection::move_selection` — clamped, not wrapped — and requests the audition of every voice it lands on. The acid arm is
      the call it was, now inside `DeviceKind::Acid`. Tests: `shift_up_and_down_walk_the_selected_voice` (down to `SNR` and
      heard; a held key's repeat walks on and is heard; up again; clamped at both ends; the pattern untouched; the plain arrow
      still nudging the tempo without moving a voice), `a_card_refuses_the_voice_keys`,
      `the_voice_keys_leave_the_acid_note_alone` (`E` → `D#`), and `drum_panel`'s `the_voice_keys_are_shift_and_the_two_arrows`.
      **Mutations: 4 of 4 caught, 0 skipped** (`.superpowers/mutate_voice_keys.py`) — no clamp (wraps), no audition, no card
      refusal, directions swapped — each named by the test that caught it. Gates: `cargo test --workspace` **880**, clippy, fmt,
      `./docs/guide/check.sh`. **Rulings:** the walk auditions (a click on a label is a preview, and a held key walks the kit by
      ear); it clamps, like the step border; it does not disarm a pending `CLEAR`, because it changes no pattern — and that face
      draws no `CLEAR` to arm. If the audition is wrong, one `request_audition` line; wrapping would need a different walk than
      `move_selection`. **The book moved:** page 22 gains the walk under *The grid* (heard, repeats while held, spent behind a
      card) and its *No notes* bullet no longer claims every key is dead; page 15's two drum sentences and the
      four-editing-keys paragraph say the same; page 20's key table names both faces; `check.sh` pins `walk it up and down the
      grid`; `00-phase-4.md` lists this round and plan 02, which was missing. No screenshot: the face paints nothing new.
      **Review: PASS on eight Minors, every one actioned.** The page-22 paragraph had been split from the click sentence it
      belonged to (moved below it); the held-key repeat was untested and unsaid (a `repeat: true` press now walks on, and pages
      22 and 20 say it repeats); `Handled::Refused` and `a_card_is_up` named only the step editor (both name the walk now);
      `is_panel_key` claimed the keys through the acid mapping alone, so it now asks `drum_panel::voice_move` too — redundancy by
      construction today, and the comment says which question each mapping answers; one assertion message claimed a check its
      line did not make (the selection is asserted); and this entry is the record the reviewer asked for.
      **Superseded by [`04-hit-is-the-only-sound`](04-hit-is-the-only-sound.md): the label click and the walk are silent, and `HIT`
      is the one sound.** This entry is the record of what plan 03 shipped, not of what the face does today.
