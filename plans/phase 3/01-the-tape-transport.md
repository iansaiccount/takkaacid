# Plan 01 — the tape transport, and the row it frees

> **For agentic workers:** `superpowers:executing-plans`. Each task ends green — the workspace gates and
> `docs/guide/check.sh` — and reviewed by a dispatched subagent before its commit, per `AGENTS.md`. Task 7 is the documents,
> and it is not optional: no push leaves the book describing the old transport.
>
> **Review:** a fresh subagent per commit, never the executor. The screenshots in task 7 are the part a compiler cannot check,
> so they are read back one by one before the round closes.

**Goal:** the transport stops being a row of the synth's face and becomes a conventional tape deck in the top band — play,
pause, stop, back, forward, record — with the tempo's `−`/`+` beside it and the song's load and save icons named but not wired.
The row it leaves in the bottom band carries `REVERSE`, `TBC`, `TRANSPOSE −`, `TRANSPOSE +` and `VOLUME`.

## What is wrong today

- The bottom row is `RUN STOP TEMPO − TEMPO + VOLUME` (`transport::TRANSPORT_CELLS` = 5, `ui/row_transport.rs`), so four cells
  of a single-face instrument are spent on controls that belong to the rack. The face needs them for the device it is showing.
- There is no pause. `Rack::playing` is a `bool`, and `space` toggles it — so "stop" and "not playing" are the same state and
  the position is thrown away by both.
- There is no seek, no reverse, and no transpose from the panel. The only transpose is the performance row's octave `−`/`+`,
  which transposes by **rewriting the stored pattern** and marks the song dirty (`tests/perform_live.rs`).
- The panel's bank browser is called `SONGS`, and the word is about to be needed for the document that holds every machine.

## The design

### The top band

The top band is band 0 (`layout::BANDS`), 46..102 on a 670-tall panel — 56 logical pixels. It holds the wordmark, the saved
word, the gear, `?` and the session name, and it has two readout wells on the right. The tape goes in the middle-right, in the
space the session name does not use and the tempo well currently occupies:

```
 ◉ takka-acid   UNSAVED ⚙ ?   SONG New song   [BACK][PLAY][PAUSE][STOP][FWD][REC]   − [ 120.0 ] +   [LOAD][SAVE]   [PATTERN A1]
                                               └────────── the tape ──────────┘   └─ tempo ─┘   └─ document ─┘  └ kept ┘
```

The drawn keys are **shapes, not characters** — a triangle pair for back and forward, a single triangle for play, two bars for
pause, a square for stop, a filled circle for record — because the face may not use a font glyph (`AGENTS.md`). The panel above
is a sketch for this document only.

Geometry, in the same `layout` terms as every other band: the tape block's six keys are square-ish and abut like a deck's, the
tempo stepper is `−`, a small well carrying the number, and `+`, and the document's `LOAD`/`SAVE` icons are two small keys to
their right. The pattern readout keeps the band's right edge. `PanelLayout` gains `tape: Row`, `tempo_stepper: Row` and
`document: Row` — the three new rows, derived in `layout::layout` like every other row, so no test has to guess a coordinate.

### Three states, not two

`Rack::transport: Transport` replaces `Rack::playing: bool`:

- **Stopped** — silent, position at the first step of the bar.
- **Playing** — the clock runs.
- **Paused** — silent, position kept.

`is_playing()` is true only for `Playing`, so `pattern_swap` and the strip's caret treat a pause as "not playing" — which is
what a paused deck is. `SharedState::playing` keeps its meaning for the audio thread ("the clock is running") and is written by
the fan-out, as today.

The keys, and what each does from each state:

| key | from Stopped | from Playing | from Paused |
|---|---|---|---|
| PLAY | start at the bar's first step | nothing (a deck that is playing does not restart) | resume from the kept step |
| PAUSE | nothing | pause, keeping the step | nothing |
| STOP | nothing | stop, back to the first step | back to the first step |

### The keys that are drawn but not wired

`BACK`, `FWD`, `REC`, `LOAD`, `SAVE` have no feature behind them in this round. They are drawn in their **unlit** state — the
same visual language as a greyed card row, which stays clickable and lets the model refuse — and every way of reaching them
(click, or its shortcut) sets a notice on the status line: `BACK IS NOT WIRED YET`, `RECORD IS NOT WIRED YET`,
`LOADING A SONG IS NOT WIRED YET`, `SAVING A SONG IS NOT WIRED YET`. A key that does nothing reads as broken; a notice is the
promise being honest, and `PanelState::notice` is the channel for exactly this.

### The bottom row

`REVERSE  TBC  TRANSPOSE −  TRANSPOSE +  VOLUME` — five cells, same geometry, same `switch_key` idiom.

- **REVERSE** toggles the selected device's direction. A flag in memory (`SharedState::reversed: AtomicBool`), lit while on,
  never saved. The engine reads the pattern **mirrored**: the position still walks forward, and the step whose note sounds at
  position *i* is `STEPS - 1 - i`. That makes the audible line run backwards while every clock in the app keeps agreeing on the
  position, which is what `last_position` and the pattern swap depend on. A slide flag slides into the next step *in playback
  order*, which is the mirrored neighbour — the design decision this shape makes for us, and the one to check by ear.
- **TRANSPOSE −/+** move the whole rack one semitone, clamped to ±12, and are a **performance offset**: `Rack::transpose:
  AtomicI32`, fanned out to each device's engine as an extra offset added to the tuning value the TUNING knob already sets.
  The knob is the knob and stays saved; the offset is not saved, so a rehearsal does not write itself into the song. The
  performance row's octave keys keep their job — editing the pattern — and the guide now says which is which.
- **TBC** is a key with nothing decided behind it: it says `TBC` and sets `TBC IS NOT DECIDED YET`. It is one cell kept for the
  player rather than a feature.
- **VOLUME** does not move.

### `SONGS` becomes `BANKS`

The switch's caption, the card's title and its two section headings (`DEMO BANKS`, `MY BANKS`), the module (`ui/songs.rs` →
`ui/banks.rs`, `songs::Songs` → `banks::Banks`), the `egui::Id`s (`song-*` → `bank-*`), and the guide's page 12. What is saved
does **not** change: `persist::save_song`, `import_song`, `export_song` and `SavedState` keep their words, because they name the
session document — and today that document is one device's bank, which is exactly why the panel's list is not called songs any
more. Renaming the save file's words would be a `SAVE_VERSION` bump spent on a word.

### Shortcuts

| key | does |
|---|---|
| `space` | play / pause (today: run / stop — this changes a documented shortcut) |
| `,` | back (notice) |
| `.` | forward (notice) |
| `R` | record (notice) |

`Escape`, the arrows, `Ctrl+1..4`, `F1..F3` and `Ctrl+C`/`Ctrl+V` are untouched. Every new control joins `tips::Control`, so a
tooltip is not something an implementer can forget: the catalogue is exhaustive and `the_table_matches_the_panel_order…` fails
until it is complete.

## The tests that make it safe

Through `draw_panel`, with real `RawInput` frames, in `rust/crates/acid-app/tests/`:

1. `the_tape_is_drawn_where_the_layout_says` — each of the six tape rects, the two tempo keys and the two document keys is
   clicked at the rect `layout` hands out, and each answers.
2. `play_starts_the_rack_and_pause_keeps_the_bar` — play from stopped starts at step 0; pause keeps the position; play resumes
   from it; stop returns to 0. Asserted on `SharedState` and the rack's transport state.
3. `space_plays_and_pauses_rather_than_stopping` — the key press toggles Playing/Paused and never reaches Stopped.
4. `a_key_that_is_not_wired_says_so_and_changes_nothing` — `,` `.` `R` and the two document keys set the right notice, and the
   transport state is unchanged after each.
5. `transpose_moves_the_rack_a_semitone_at_a_time_and_stops_at_an_octave` — asserted through the engine's tuning, on the
   shared state the audio thread reads.
6. `reverse_plays_the_line_backwards_without_moving_the_clock` — a sequencer/engine test: the same position yields the mirrored
   note, and `last_position` still advances.
7. `the_bank_browser_no_longer_says_songs` — the caption and card read `BANKS`, `DEMO BANKS`, `MY BANKS`, and `SONGS` is absent
   from the painted strings.
8. `every_new_control_has_a_tooltip` — the `tips::Control` catalogue test, extended by the new variants.
9. `the_tape_keys_draw_shapes_and_no_text` — the painted strings contain no label inside the tape rects, which is the no-glyph
   rule made checkable.

Mutation checks before the round closes, one per rule: the tape keys' hit rects moved off their paint, pause made to discard the
position, a not-wired key made silent, transpose riding the saved knob instead of an offset, and the mirrored read replaced by a
stored reversed pattern.

## Tasks

1. **Layout.** Band 0 gains `tape_cells` and `tempo_stepper_cells` (in `panel.rs`, beside the band's other geometry), the
   pattern readout keeps the wall, the tempo well becomes the stepper's middle cell, and the session name becomes the corridor
   before the deck. The bottom row's cell identities are task 5's: a caption with nothing behind it — not even a notice — is the
   "key that does nothing reads as broken" this plan forbids. Tests: geometry only, over three window sizes.
2. **The transport state.** `Transport::{Stopped, Playing, Paused}` on the rack, `Rack::playing` removed, `is_playing()`,
   the fan-out, and the three states' semantics for `pattern_swap` and the caret. Tests: unit + the state machine part of 2.
3. **The tape, drawn.** The six keys as shapes, the tempo stepper and its well, lit/unlit states, tooltips. This is also where
   `TAPE_COUNT` stops being a hand-kept six: a `TapeKey` catalogue (`Back, Play, Pause, Stop, Forward, Record`) with `ALL`, and
   `TAPE_COUNT` derived from it the way `KNOB_COUNT` derives from `Knob::ALL` — indexing the rects by position against the tips
   catalogue is the hand-kept second list this repo has paid for twice. Tests: 1, 8, 9.
4. **The tape, wired.** Play/pause/stop requests, tempo `−`/`+`, notices for the not-wired keys, `space`/`,`/`.`/`R`. Tests: 2,
   3, 4.
5. **The bottom row.** `REVERSE` (flag, mirrored read), `TRANSPOSE −/+` (rack offset, ±12), `TBC` (notice), `VOLUME`
   unchanged. Tests: 5, 6.
6. **`SONGS` → `BANKS`.** Module, type, ids, caption, card title, sections, notices. Test 7; the guide's page 12 rename lands
   with task 7.
7. **Documents.** Guide pages 04 (the face), 09 (playing), 12 (banks), 15 (keyboard), 20 (reference); `check.sh` facts; the
   screenshots that contain the transport band — `transport.png`, `top-strip.png`, `switches.png`, `first-sound.png`,
   `running.png`, the card shots that include the bottom band, and the `SONGS` card shot renamed to `banks-card.png`. Each row
   in `src/img/README.md` updated with its state and command. Pages 04 and 20 still say the session's name is "in the middle"
   of the strip; it is not any more, and those sentences are this task's to fix.
8. **The gate.** Three workspace commands, `check.sh`, the mutation checks above, and the round's whole-branch read.

## Non-goals

- **Recording.** `REC` sets a notice. What record would record (automation? a performance? audio?) is phase 4 and later.
- **Back and forward actually moving the playhead.** The rack-level seek is designed in the wishlist and built in the round
  that needs it; this round draws the keys and says they are not wired.
- **Load and save actually reading or writing a song.** The global song document is not this phase; the icons are its place.
- **Storing reverse or transpose.** Both are performance state, in memory, forgotten on restart.
- **A per-device reverse indication in the strip.** The key's own lit state says it for the selected device; a mark per cell is
  a later refinement.
- **MIDI clock, MMC or tap tempo.**
- **Reworking the pattern readout** beyond what the tape needs for room.

## Progress

- [x] **Task 1 — layout**, `panel.rs` + `notice_live.rs`. Band 0's chain is now name → deck → tempo stepper → pattern readout
      at the wall, right-anchored in `panel.rs` beside the band's other geometry. Geometry only; no behaviour.
      Proven by `the_tape_and_its_neighbours_share_the_top_strip_in_order` (three window sizes: 500×340, design, 1600×1040;
      pressability and a readable name asserted at the reference size only) and
      `the_song_label_fills_the_corridor_before_the_document_icons` (the corridor's endpoints and ten neighbours it must not
      cover).
      **Ruling (player, this round):** the song's `LOAD`/`SAVE` icons are not drawn and their space is not reserved. Measuring
      the first build showed the icons cost the session-name corridor — which is also the status line — about forty pixels, and
      at 980×560 that left a notice ~57 px, twelve characters, where `a_long_notice_keeps_its_beginning` needs ~73 px to keep
      its beginning. Cost if wrong: the icons move again when the song document lands — the round that must reconcile them with
      the existing `SAVE` switch anyway.
      **Ruling (this round):** the session name is no longer centred. The deck and stepper spend the room the symmetry used, and
      a centred label ending at the deck would be a few pixels wide. The two centring tests were replaced by the corridor test,
      not deleted; the guide's "in the middle" sentences are task 7's.
      **Review found, and what was done:** the first build hung the band the wrong way round — documents before the deck, tempo
      on the wall — against the sketch the player approved (P1; fixed by re-anchoring); a doc comment still promised the
      centring (fixed); one window size tested, with a 12 px key floor that only holds above ~583×399 (fixed: three sizes,
      floors at the reference size); this section was empty (fixed); the plan blamed a module cycle for putting the geometry in
      `panel.rs`, which is not how Rust works (fixed: the real reason is that band 0's right-anchored pieces have no band-wide
      `area` for a `layout::Row`); `transport.rs`'s comment named the fifth transport cell `VOLUME` when it is `LEARN` (fixed,
      and the reason this plan's row description was wrong too); two new functions took a `_scale` they ignored while their
      sibling scaled its gaps (fixed by dropping it, matching `layout::knob_cells`).
- [ ] **Task 2 — the transport state** (started). The **engine half** has landed: `Sequencer::pause`/`resume` and
      `Engine::pause`/`resume`, with `tests/pause.rs` fixing the three semantics — a pause keeps the step and a resume carries
      on from it; a stop returns to the top and a pause does not; a resume restarts the clock so the first step after it is a
      whole step. Still to come in this task: `Rack::transport: Stopped | Playing | Paused`, the fan-out and the `Parameters`
      seam so the renderer can tell a stop from a pause, `main`'s request handling, and `transport::Request::Pause`.
- [ ] Task 3 — the tape, drawn
- [ ] Task 4 — the tape, wired
- [ ] Task 5 — the bottom row
- [ ] Task 6 — `SONGS` → `BANKS`
- [ ] Task 7 — documents and screenshots
- [ ] Task 8 — the gate and the mutation checks

Awaiting the player's approval of this design before task 1.
