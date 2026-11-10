# Plan 33 — demo songs, and a save model you can trust

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`. Each task ends in a green commit on
> `Deepseek-Version`. The design was agreed in conversation (see "Rulings").

**Goal:** ship five genre demos (slow acid, hard techno acid, blues, rock, funk) as read-only songs built into the binary;
give the songs card a `DEMO SONGS` section and a way to adopt one as a new song called "New song"; make saving manual by
default with the existing autosave kept as a persisted `AUTO` toggle in the top strip; and open a first run on the slow
acid demo.

**Architecture:** demos are Rust builders in `acid-app/src/demo.rs`, returning a `persist::SavedState` each (bank + patch),
so nothing is read from disk and nothing can be overwritten. The card lists them above the player's own songs and reports
a new `Request::LoadDemo(usize)`; `main` adopts one exactly the way it adopts a loaded song, except the name becomes "New
song" and no file is attached. Autosave becomes a field in `SavedState`/`PanelState` and a key beside TIPS.

**Tech stack:** Rust 2024, egui 0.37, serde. Crate `rust/crates/acid-app`.

**Spec:** `plans/32-learn-polish-and-pattern-controller.md` for the panel's conventions; `AGENTS.md` for the gates.

## Global constraints

- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` green at every
  commit.
- Demo songs are never written, never listed by `list_songs`, and never deletable from the card.
- An old `state.json` (version 8 and older) must still load: every new field is `#[serde(default)]`, and `SAVE_VERSION`
  becomes 9 with a note in `upgraded()`.
- No behaviour change to playing, MIDI or the pattern model.

## The demos

| Demo | BPM | Wave | What the line is |
|---|---|---|---|
| Slow Acid | 118 | saw | The classic: mid cutoff, high resonance, long-ish decay, slides on the off-beats, four accents a bar |
| Hard Techno Acid | 142 | saw | Short decay, envelope mod up, 16th chug with octave jumps and a slide into every 8th |
| Blues | 92 | square | Shuffle feel from swing, a root–sixth–flat-seven walk in A with a fifth on the turn |
| Rock | 126 | square | Straight eighths, root–fifth–octave, accent on the beat, one slide into the downbeat |
| Funk | 104 | saw | Syncopated 16ths with ghost notes (low velocity), an octave pop, and a rest on the "and" of one |

Each demo fills **A1–A8 of one bank** (main line, a fill, a register change, a sparser and a busier variation), leaving the
other 56 patterns empty. Two acid lines are transcribed from the published one-bar TB-303 references at
`acidvoice.com/midi_pattern_tb_303_x0xb0x.htm` (pattern 07 and 19), translated to this pitch model; the other three are
authored here. The transcription is by ear against the screenshots, so each demo's comment says what it is rather than
claiming to be that record.

## Task list

1. **`demo.rs`** — `pub struct Demo { name, state }`, `pub fn songs() -> Vec<Demo>`, `pub fn seed() -> SavedState` (the slow
   acid demo with `song: Some("New song")`), the eight variation builders and the five patch settings. Tests: all five
   names distinct; every pattern in every demo has a note, is inside the pitch range, and the five demos are not the same
   phrase; a demo never carries a `song` field except the seed. **Green commit.**
2. **Autosave becomes a setting** — `SavedState::autosave` (`#[serde(default)]`, false), `PanelState::autosave`,
   `SAVE_VERSION` 9, `main` carries it both ways and only runs the autosave loop when the toggle is on. Tests: a file from
   this build round-trips the flag; a file from before it loads with autosave off; the loop does not write when off.
   **Green commit.**
3. **The AUTO key** — `autosave_rect(window)` left of TIPS, lit when on, `tips::Control::Autosave` with its own text, and
   toggling marks the session dirty (it is saved in `state.json`). Tests: the key does not overlap TIPS or the SAVED lamp;
   clicking it flips the flag and the session goes unsaved; the tip exists. **Green commit.**
4. **The card** — `Songs::demos`, `Songs::load_armed`, a `DEMO SONGS` heading above `MY SONGS`, demo rows with no delete
   cross, `Request::LoadDemo(usize)`, and a two-click guard on any row that would replace a **dirty** session (the first
   click arms and says so; anything else disarms). Tests: demo rows ask for their index; a clean session loads on one
   click; a dirty one needs two; switching lists disarms. **Green commit.**
5. **`main` adopts a demo, and SAVE keeps the song** — handle `LoadDemo` (apply the demo's state, name "New song", no open
   file, dirty), seed the first run from `demo::seed()` when there is no `state.json`, and make `save()` write the named
   song file as well as the session. Tests: a fresh data dir starts on the slow acid demo named "New song"; SAVE with an
   open song rewrites that file; a session saved before this round still loads. **Green commit.**
6. **`README.md`** gains the demo list and the save model; plan progress updated. **Green commit.**

## Rulings (agreed in conversation)

- Demos live in Rust, not in JSON files: type-checked, testable, and non-writable by construction.
- One bank of eight variations per demo, so the pattern knob has somewhere to go.
- Loading a demo (or a saved song) replaces a **dirty** session only on a second click.
- Saving is manual by default; the autosave toggle is in the top strip and defaults off, including for existing sessions.
- The demos are named after their genre; the user renames on save, so no demo name is reserved, but `list_songs` never
  returns a demo and the card always shows the two sections apart.

## Non-goals

- No song arrangement, chaining or multi-bar playback: a demo is a bank and a patch, as every other song is.
- No import of `.mid` files at runtime.
- No new MIDI behaviour.

## Progress

- [x] **1. `demo.rs`** — `Demo { name, state }`, `songs()` (five), `seed()` (slow acid named "New song"), `NEW_SONG`, a
      `line(&[(step, pitch, octave, flags)])` table DSL (flags `a` accent, `s` slide, `g` ghost) and `variations(main,
      fill)` (each line as written, ±1 octave, and a `sparser` reading) filling A1–A8 of one bank. Six tests: five distinct
      names; every pattern non-empty with every note inside `NOTE_LOW..=NOTE_HIGH` and at least one accent per demo; the
      eight variations pairwise distinct; the five A1s distinct; every patch inside the engine and no two demos on the same
      patch; the seed is the first demo under "New song". Ruling: `busier()` was written and then deleted — it only adds
      ghosts on rests, so a dense techno line came out identical to itself (the distinctness test caught it). Cost if
      wrong: none; hand-authored ghosts in the funk line do that job.
- [x] **2. Autosave becomes a setting** — `SavedState::autosave` (`#[serde(default)]`, false), `PanelState::autosave`,
      `SAVE_VERSION` 9, `main` carries it both ways (start-up, save, save-song, load-song) and the autosave loop only runs
      when the toggle is on. Test `persist::the_autosave_switch_round_trips_and_an_old_file_gets_it_off`.
- [x] **3. The AUTO key** — `ui::autosave_rect(window)` sits immediately left of TIPS (its right edge a gap away, tops
      aligned), lit while on, with `tips::Control::Autosave` in the table. Toggling marks the session dirty, because the
      switch is part of what `state.json` keeps. Test `tips_live::the_auto_key_saves_the_session_only_when_it_is_on`.
- [x] **4. The card** — `Songs::demos` and `Songs::load_armed`, a `DEMO SONGS` heading above `MY SONGS`, demo rows with no
      delete cross, `Request::LoadDemo(usize)`, and `guarded_load`: a clean session loads on one click, a dirty one arms
      and needs a second, with a line of feedback. Tests in `songs_live`: `a_demo_row_asks_to_load_that_demo`,
      `replacing_an_unsaved_session_takes_two_clicks`; the existing row tests moved down by the two headings.
- [x] **5. `main` adopts a demo, and SAVE keeps the song** — `persist::open(dir) -> (SavedState, first_run)` seeds a first
      run from `demo::seed()` (Slow Acid, named "New song"); `main` marks that session unsaved, applies `LoadDemo` as a new
      unnamed song (no file, dirty, the demo's own patch switches), and `save()` now rewrites the song file the session came
      from as well as the session. `SongState::file` is that file, so a demo has none and a loaded song has one. Tests:
      `persist::a_first_run_opens_on_a_demo_under_a_new_name`, plus the existing round-trip and save tests.
- [x] **7. The saved-row guard is tested too** — `songs_live::replacing_an_unsaved_session_with_a_saved_song_takes_two_clicks`:
      a dirty session loading a *saved* row arms on the first click and loads on the second, which is the same
      `guarded_load` path the demo rows use.
- [x] **8. The demos have been rendered, and the levels fixed** — new example `acid-app/examples/earcheck-demos.rs` renders
      one take per demo (eight bars, one per pattern) to `rust/earcheck/demos/`, printing peak, RMS and brightness per demo
      and per pattern. It found a **~11 dB level spread**: the two square-wave demos measured RMS 0.20–0.23 against the
      acid demos' 0.05–0.06, which is the ladder's known passband loss at high resonance. Each demo's master volume now
      trims its patch (0.87 / 0.95 / 0.47 / 0.44 / 0.88), bringing all five inside 0.2 dB of each other; peaks still differ
      as the accents dictate. The numbers and what to listen for are in `plans/notes/33-demos-listening.md`; the verdict
      line there is for a human ear — nothing in this round claims the demos *sound* right, only that they render, are
      audible, and sit at a sane level relative to each other.
      Ruling: the master volume is part of a demo's patch, so a demo loads at the level it was mixed at. Cost if wrong: a
      demo opens quieter than a factory session and the player turns the knob up.
