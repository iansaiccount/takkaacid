# Plan 23 — teach the pattern selector and the BPM

The learn flow can teach eight knobs. Two more controls on the top strip deserve it: the pattern the transport is on,
and the tempo beside it. Both are steppers rather than dials, so both need the same thing the waveform needed — a
controller value snapped onto a fixed set of positions — plus a way for the MIDI thread to ask `main` for something only
`main` can do (moving the bank).

## What MIDI learn means for each

| Control | Controller travel means | Position count |
|---|---|---|
| PATTERN | the eight patterns of the **current bank**, left to right | 8, so 16 CC steps each |
| BPM | the whole tempo range, `MIN_BPM..=MAX_BPM` (already how CC 72 worked) | continuous |

Tempo was the one control with a *fixed* controller (72) that the flow refused to teach. It becomes learnable like the
rest; its factory controller stays 72, so an existing rig keeps working. Swing keeps its fixed 73: a performance that
lands on the wrong shuffle can be heard instantly and corrected by hand, while a performance that lands on the wrong
*tempo* cannot be judged at all — and the point of learning is that a fader the player knows can own it.

## Task 1 — the catalogue

- `CcTarget::Pattern`; `Tempo` moves from `FIXED_CC` into `LEARNABLE`, which becomes ten entries:
  `[Waveform, Tuning, Cutoff, Resonance, EnvMod, Decay, Accent, Volume, Tempo, Pattern]`. The first eight keep their
  slots, so no existing map moves.
- `DEFAULT_CC` grows to `[20, 3, 74, 71, 76, 75, 5, 7, 72, 21]`: tempo keeps 72, the pattern selector takes **CC 21**
  (undefined in the spec, next door to the waveform's 20).
- `FIXED_CC` keeps only swing (73).

## Task 2 — the pattern request

The bank lives in `main`; the MIDI thread may only write atomics. So `SharedState` gains a pair:

- `pattern_index: AtomicI32` — which of the 64 patterns is playing, kept in step by `main` wherever it already sets
  `panel.pattern_id`.
- `pattern_request: AtomicI32` — `-1` when there is nothing to do, else the absolute pattern to select.

`apply_cc(CcTarget::Pattern)` snaps the controller value onto the eight numbers of the **current bank** and stores the
request; `main` drains it once per frame, calls `bank.select`, and publishes the pattern exactly as the shift+arrow
shortcut does (reuse `shortcuts::stash_current` + `select` + `set_pattern` so a controller cannot leave the bank and the
engine disagreeing).

## Task 3 — teaching them

The learn flow picks targets by clicking a knob (`learn::control_for_knob`). Two more targets, each a click while
`Choosing`:

- the **tempo display** in the top strip → `CcTarget::Tempo`;
- the **PATTERN key** in the switches row (engraving today) → `CcTarget::Pattern`.

Both are guarded on `state.learn == Step::Choosing`: outside a learn a click on either does nothing, which is checked,
because a dark control that still reports clicks is the bug this codebase keeps meeting.

## Task 4 — saved files

`SAVE_VERSION` 6. `upgraded()` learns two shapes: a seven-entry map (before the waveform) still gains the switch's
controller at the front, and any map shorter than ten gains tempo and pattern at the **end**, in that order. Nothing
that already had a slot may move.

## Tests (failing first)

| Test | What it pins |
|---|---|
| `cc::the_pattern_controller_snaps_onto_the_eight_of_the_bank` | CC 0/15 → 1, 16/31 → 2, 127 → 8, always inside the current bank |
| `cc::a_learned_pattern_controller_leaves_a_request_and_a_fixed_one_does_not` | `apply_cc` writes `pattern_request`; learning the slot elsewhere frees CC 21 |
| `cc::the_tempo_can_be_taught_and_still_spans_the_range` | `set_cc(Tempo, 22)`, CC 22 at 0/127 → `MIN_BPM`/`MAX_BPM`, CC 72 now unmapped |
| `cc::swing_keeps_its_fixed_controller` | 73 still answers, and is not learnable |
| `persist::a_map_from_before_the_pattern_selector_keeps_every_fader_it_had` | 8 entries → 10, tempo 72 and pattern 21 appended |
| `persist::the_learned_midi_mapping_round_trips` | ten entries |
| `learn_live::clicking_the_tempo_display_while_choosing_asks_for_a_tempo_fader` | the pick, and its guard outside a learn |
| `learn_live::clicking_the_pattern_key_while_choosing_asks_for_a_pattern_fader` | same for the pattern |
| `restore` / `state_live` | `pattern_index` and `pattern_request` start at 0 and -1 |

## Global constraints

- The MIDI thread writes atomics only; `tests/no_alloc` stays green.
- ASCII legends only; no control face may be a font glyph.
- `main` stays thin: the snapping lives in `cc::`, the apply in one small drain.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Review focus

- A learned pattern controller must not desynchronise the bank and the engine: one drain, one code path, shared with
  the keyboard shortcut.
- Two controls on the same display line (tempo and pattern) must not both take the click; the PATTERN key and the tempo
  display are separate rects, and the tests click each.

## Result

Landed as planned, with three things the build settled:

- **Pattern zones are equal, not detent-rounded.** The waveform snaps with `round(knob * (n-1))`, which spreads four
  faces unevenly; for eight patterns that would make the first zone nine steps wide and put a fader on a hair-trigger
  boundary. The pattern controller uses `value / 128 * 8`, so each pattern owns sixteen controller steps. A test walks
  the boundaries (0, 15, 31, 127).
- **The request is absolute and the bank is read, not guessed.** `apply_cc` reads `SharedState::pattern_index` (kept in
  step by `main` and by `apply_saved`) to know which bank to point into, and stores an absolute pattern number. `main`
  drains it through `shortcuts::select_pattern` — the same path as shift+arrow — so the bank, the panel and the engine
  cannot drift apart.
- **`CC_MAP` became `cc::mapped()`.** The static list of nine controllers was a second copy of the factory pairing and
  would have gone stale the moment tempo and pattern joined the learnable map. It is now derived from `LEARNABLE` +
  `DEFAULT_CC` + `FIXED_CC`, and the help-card test checks the card documents every one of them — which is how the
  missing `72 tempo` line got caught.

One regression found and fixed on the way: the waveform face adoption from plan 22 marked the session dirty, so merely
opening the panel turned `SAVED` into `UNSAVED` (`step_mode_live::backspace_alone_is_not_ours` caught it). Only the hand
moving a knob marks the session unsaved now; the face following a learned controller is the panel catching up.

Mutation checks: removing the `Choosing` guard on the new pick targets fails
`the_tempo_readout_and_the_pattern_key_can_be_taught`; removing the v6 append fails
`a_map_from_before_the_pattern_selector_keeps_every_learned_fader`.
