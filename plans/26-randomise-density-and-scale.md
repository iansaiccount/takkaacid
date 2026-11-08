# Plan 26 — randomise density and scale

RANDOM rerolls the whole pattern now, but it is always half-full and always chromatic. Two knobs make it a tool rather
than a coin: **DENSITY** (how many steps land on) and **SCALE** (which notes the roll may use). Both are settings of the
randomiser, not of the pattern, so they persist with the session and never touch what is already written.

## Decisions

- **The row grows to six cells**: OCT −, OCT +, SHUFFLE, RANDOM, DENSITY, SCALE. 62 px tall, ~155 wide each; the knobs
  follow the waveform knob's pattern — snap on release, a tick per detent, and a caption naming the value.
- **DENSITY is a probability**, 0..1, shown as a percentage over the knob: 0 → only the guaranteed step sounds, 1 → every
  step. The default stays 0.5, which is the "one in two" the button already had.
- **SCALE is a catalogue rooted on C** — the lattice's own root — with six members: CHROM, MAJOR, MINOR, DORIAN,
  MIN PEN, MAJ PEN. A root selector would be a second control and a second saved field; the 303 sits in one key, and a
  pattern that wants another key can be transposed by the octave keys' neighbour later. This is my call, stated so it can
  be overruled.
- **Both settings live in the panel and save with the session**, exactly like the TIPS switch: `PanelState` fields,
  `SavedState` mirrors, `main` copies them across. Nothing reaches the audio thread — the engine plays patterns, not
  dice.

## The scale catalogue

`ui::scale::Scale` with `ALL`, `index`, `from_index` (clamps), `name` (ASCII, ≤ 7 chars for the caption), and
`members() -> &'static [u8]` giving semitone offsets from C:

| Scale | Offsets |
|---|---|
| CHROM | 0 1 2 3 4 5 6 7 8 9 10 11 |
| MAJOR | 0 2 4 5 7 9 11 |
| MINOR | 0 2 3 5 7 8 10 |
| DORIAN | 0 2 3 5 7 9 10 |
| MIN PEN | 0 3 5 7 10 |
| MAJ PEN | 0 2 4 7 9 |

A scale is the only place a pitch may come from in `randomise`; the octave is still pulled to wherever the pitch fits, so
the range guarantee is unchanged.

## Tests (failing first)

| Test | What it pins |
|---|---|
| `scale::every_scale_starts_on_its_root_and_stays_in_the_octave` | first member 0, all members `< 12`, ascending |
| `scale::the_knob_snaps_to_one_scale_per_detent` | position ↔ scale round trip, ties round up, ends clamp |
| `randomise::a_scale_limits_the_notes_it_can_write` | for every scale and 32 seeds, each note's pitch class is a member |
| `randomise::density_sets_how_many_steps_land_on` | 0.0 → the one guaranteed step; 1.0 → all sixteen; 0.25 is sparse |
| `randomise::the_same_seed_and_settings_give_the_same_pattern` | density and scale join the seed in deciding the result |
| `persist::the_randomise_settings_round_trip` | density and scale survive a save, and a file without them gets the defaults |
| `perform_live` | the DENSITY knob writes a probability; the SCALE knob snaps to a catalogue entry and the caption names it; RANDOM obeys both |

## Global constraints

- The audio callback allocates nothing; `tests/no_alloc.rs` stays green.
- ASCII legends only; no control face may be a font glyph. Knob captions and detent ticks are drawn, not glyphed.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Review focus

- Snapping the SCALE knob on release, not per frame: the waveform knob taught that lesson (a per-frame snap resets the
  accumulated drag and a slow drag never crosses a midpoint).
- Density 0 must still leave the guaranteed step, or the button looks broken at one end of its travel.
- `SAVE_VERSION` 7 with serde defaults rather than a migration: both fields are new, so an older file simply gets the
  factory values.

## Result

Landed as planned: the row is six cells, the two dials are panel-side settings carried by `main` exactly like the TIPS
switch, and `SAVE_VERSION` is 7.

What the build added beyond the plan:

- **`Scale::pitches()`** returns the scale in the panel's own spelling, so the randomiser never maps semitones to `Pitch`
  itself — the catalogue stays the only decoder, the rule the wave catalogue already follows.
- **Density is counted in thousandths.** `below(1000) < density * 1000` means 25% is really 25%; `below(2)` on a 0..1
  float would have made the knob four stops long.
- **The DENSITY caption is a percentage over the knob**, and the SCALE caption is the scale's own name — both drawn as
  text, no glyph faces, and the scale caption comes from the catalogue so a new entry names itself.
- **One knob helper for the whole row** (`perform_knob`): the square-in-the-middle fix from the shuffle knob is now the
  rule for every knob in a wide cell, not a special case that a future cell could forget.

Two test lessons: a snapping knob only lands on its detent on the frame *after* the hand comes off, so the live drag
helper now sends a release; and a test that leaves the pointer down makes the next `click` a no-op, which is how the
density test silently measured the wrong pattern for a round.

Mutation checks: replacing the scale lookup with a chromatic pick fails the scale test and the settings test; replacing
the density compare with a coin flip fails the density test and the settings test.

## Revision — the frame, and two groups

The player found the transport row bleeding outside the silkscreen frame, and asked for the row to read as two sections.

- **The panel is 640 tall, not 620.** The frame is drawn *inside* the panel edge, so the last band has to leave room for
  it: at 620 the transport band ended at 614 while the inner rule sat at 609. Growing the panel to 640 keeps every band's
  design pixels exactly as they were (transport still 570..614) and puts it 15 px inside the rule.
- **`frame_lines` is scaled with the panel.** It used the raw `MARGIN`/`GAP`, so on a window below design size the frame
  stayed pixels thick while the face shrank, and the top band could reach past it — a latent bug the new guard found. The
  guard is `layout::every_band_sits_inside_the_frame_lines`, which walks all six bands against the inner rule at three
  window sizes.
- **The row is two groups.** `PERFORM_LEFT` (octave down, octave up, SHUFFLE) and `PERFORM_RIGHT` (RANDOM, DENSITY,
  SCALE), cut by `perform_cells` with a 24 px gap: a play control and a dice dial are different jobs, and the gap says so.
- **`PanelState::set_scale` / `set_density`.** The SCALE knob's position and the scale itself are one setting in two
  fields — the panel snaps the scale from the knob whenever the hand is off it — so a bare assignment to `scale` was
  undone on the next frame. The setters keep them in step, and the song-load path uses them.
