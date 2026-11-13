# Plan 28 — highlight the learnable controls

## Why

Pressing LEARN turns the panel into a question — "which one?" — and today the only answer on the face is the hint in the
top strip. The player has to know already that the knobs, VOLUME, the BPM readout, the PATTERN key and SHUFFLE can be
taught, and that the step keys, DENSITY and SCALE cannot.

## Decisions

- **The colour is cyan**, `theme::LEARN_TARGET` (#3DE8F0). Nearly complementary to the amber silkscreen, and no other
  light on the face uses it, so it reads as a *different question* rather than a brighter answer. A test pins the distance
  from `SILKSCREEN`, `LED_ON` and `LED_ACCENT` so a future palette change cannot quietly collapse the two signals.
- **The outline sits outside the control** (`learn_highlight_bounds`, the control's rect grown 2.5 px), so it never covers
  the face or the caption it is pointing at.
- **Only while choosing.** `Choosing` is the moment the question is open; `Listening` has an answer and `Idle` has no
  question, so both draw nothing.
- **Eleven controls**: the seven tone knobs, VOLUME, the BPM readout, the PATTERN key, SHUFFLE. That is exactly the set
  with a `learn_pick`/`control_for_knob` today — DENSITY and SCALE are randomiser settings, not MIDI targets, and the step
  keys are the instrument.

## Tests

`tests/learn_highlight_live.rs` counts painted shapes: egui returns every shape it drew, so a stroke in `LEARN_TARGET` is
countable from a headless frame.

| Test | What it pins |
|---|---|
| `nothing_is_highlighted_when_no_learn_is_running` | zero strokes when idle |
| `every_teachable_control_wears_the_pick_colour_while_choosing` | exactly eleven while choosing |
| `the_highlight_goes_away_once_a_control_is_picked` | zero after a pick moves the flow to `Listening` |
| `the_pick_colour_is_not_one_of_the_panels_own_lights` | contrast, and the outline is outside the control |

A click needs three frames (hover, press, release); a press and a release in one frame is not a click, which is why the
test helper drives frames rather than batches.

## Review focus

- The count is the contract: adding a learnable control means updating eleven, which fails loudly rather than silently
  leaving a control unlit.
- The highlight must not swallow input: it is a stroke on the shared painter, drawn before the pick target, so the click
  still lands.
- No new colour literals outside `theme.rs`.
