# Plan 25 — a performance row under the knobs

Three controls that play the pattern rather than tune the voice: an octave transpose down/up, a shuffle knob, and a
randomiser for the notes.

## Decisions (agreed in conversation)

- **Transpose is part of the pattern.** The notes move, up-then-down is a true round trip, and it saves with the pattern
  because patterns already save. Notes pushed past the ends of the range are **silent**, not clamped: leaving them out of
  range is what makes the round trip exact.
- **The shuffle knob is the `swing` value that already exists.** The clock has rescaled odd steps from `swing` all
  along, and CC 73 moves it; the knob is the face it never had.
- **Randomise keeps the rhythm.** Steps that are on stay on, their accent, slide and velocity stay, and only the note
  changes — the pattern keeps its shape and gets new pitches. New notes land inside the playable range, so a randomise
  never quietly silences a step.

## The layout

The panel is five bands cut as fractions of one design height, scaled as one piece. A sixth band that squeezed the step
keys would cost the instrument more than the row is worth, so the design **grows from 980×560 to 980×620** and every
existing band keeps its pixel height:

| Band | Before (of 560) | After (of 620) | Height |
|---|---|---|---|
| top strip | 16..72 | 16..72 | 56 |
| knobs | 84..300 | 84..300 | 216 |
| **performance** | — | **312..374** | 62 |
| steps | 316..420 | 386..490 | 104 |
| switches | 432..490 | 502..560 | 58 |
| transport | 500..544 | 570..614 | 44 |

The window opens taller; nothing shrinks. `WINDOW_SIZE` follows `DESIGN_SIZE`.

## The row

Four cells, same `knob_cells` arithmetic as every other row:

- **OCT -**, **OCT +** — `Pattern::transposed(-12 / +12)`, published to the engine and marked unsaved.
- **SHUFFLE** — a `knob::knob` on `shared.swing` (0..1), the same atomic CC 73 writes.
- **RANDOM** — a seeded xorshift on the current pattern's notes.

## Silence outside the range

- `acid_core::pitch` owns the playable range (`NOTE_LOW = -12`, `NOTE_HIGH = 71`), with `Step::midi()` and
  `Step::audible()`. The panel's `PANEL_LOW`/`PANEL_HIGH` become aliases so there is one pair of numbers.
- `sequencer::trigger` treats an inaudible step as a rest: note off, and no glide — the same rule a rest already
  follows, for the same reason.
- The step captions show `--` for a note outside the range instead of clamping to the nearest name, which would be a lie.

## Tests (failing first)

| Test | What it pins |
|---|---|
| `range::transposing_up_twice_and_back_is_exact` | `+12`, `+12`, `-24` returns the original steps, including ones now out of range |
| `range::a_step_outside_the_range_is_silent_and_neighbours_still_sound` | a transposed-up pattern plays 3 notes of 4: the one past B5 is a rest |
| `range::a_silent_step_does_not_glide` | a slide into an inaudible step retunes nothing |
| `layout::the_performance_row_is_between_the_knobs_and_the_steps` | inside the panel, no overlap, four cells |
| `randomise::the_rhythm_survives_and_the_notes_change` | on/off, accent, slide, velocity unchanged; at least one pitch differs |
| `randomise::every_random_note_is_inside_the_range` | no silent steps come out of a randomise |
| `randomise::the_same_seed_gives_the_same_pattern` | reproducible, so a bug report can be replayed |
| `perform_live` (new) | OCT+ moves the displayed notes up an octave and marks unsaved; SHUFFLE writes `swing`; RANDOM changes pitches but not which steps are on |

## Global constraints

- The audio callback allocates nothing; `tests/no_alloc.rs` stays green.
- ASCII legends only; no control face may be a font glyph.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Review focus

- Existing layout tests pin the band fractions; they must be updated rather than loosened, or the row can drift.
- `pitch_name` clamps today; the `--` case must not index outside its table.
- Transpose publishes through the same path as a step edit (`set_step`), so the bank, the engine and the display agree.

## Result

Landed as designed: the panel is 980×620, the performance row sits between the knobs and the step keys, and the step keys
kept their full height because the panel grew rather than stretched.

What the build settled beyond the plan:

- **The playable range is MIDI 0..=83** (C-1..B5), not the panel's internal `-12..71`. The panel counts semitones from the
  lattice root, so the same notes are spelled twelve lower there; putting the range in `acid_core` in MIDI terms keeps the
  sequencer's check honest and leaves the panel's own arithmetic alone. `Step::midi()`/`audible()` are the only readers.
- **The shuffle knob needed a square, not a column.** A row cell is ~230 px wide and 62 tall; `knob::knob` allocates from
  the child UI's top-left, so the knob sat at the left edge of a wide cell and the cell's centre never touched it — the
  live test caught that (the knob never moved). It now gets a centred square.
- **Randomise is seeded xorshift, and the seed is the clock.** It keeps each step's gate, accent, slide and velocity and
  picks a random pitch class, with the octave pulled to wherever that pitch fits — otherwise a pattern at the top of the
  range would produce drawn-but-silent notes and look broken. `pitch_name` returns `--` for anything outside the range
  rather than clamping to a nearby name, which would have been a lie.

Two test suites needed real updates rather than loosening: the layout tests now walk six regions (and the ones hard-coding
980×560 use `theme::DESIGN_SIZE`), and `tips_live` scales the SAVED lamp with `layout::scale` so it measures the panel it
actually draws.

Mutation checks: dropping `!step.audible()` from the sequencer fails the silence test; dropping the octave clamp in
`randomise` fails the range test.
