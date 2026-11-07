# WAVEFORM selector — design

**Status:** proposed, awaiting approval. **Date:** 2026-11-06. **Size:** small change.

## Intent and why

The WAVEFORM control today is a continuous knob that happens to flip at the midpoint: you
cannot see which wave is selected while turning it, and it is possible to leave the knob sitting
somewhere between the two states, where its position means nothing. A second problem is latent —
`Waveform` is two hard-coded variants whose index is converted by hand in six places, so a third
wave type would mean editing all of them and hoping none was missed.

The goal is a control that can only ever be in one of its states, says which state that is, and
absorbs a new wave type with one enum variant plus its oscillator arm.

## Goals

- The control moves only between discrete states; it can never rest between them.
- The selected wave is named on the panel, legibly, beside the control.
- Adding a wave type later touches the enum, its name, and the oscillator — not the panel, the
  save file, or the shared state.

## Non-goals

- No new wave types now (spec TBC).
- No change to how any wave sounds.
- No change to the other six knobs, the layout, or the panel's geometry.

## Constraints

- The seven-knob row keeps its geometry (Rulings 91/94: no layout redesign).
- Existing save files must keep loading: `knobs.waveform` is a `u8` index today (`0` saw, `1`
  square), and that meaning is preserved.
- The audio thread reads an atomic and never allocates.
- Legend text stays ASCII (AGENTS.md: no font glyphs as control faces).

## Approaches considered

1. **Stepped knob with detents — chosen.** The same knob body, with the value snapped to one of
   N evenly spaced positions and a tick drawn at each. Keeps the seven-knob row symmetrical, makes
   the states visible, and at N waves the positions and the caption follow for free.
2. **Cycle button** (`[ SAW ▸ ]`). Cheapest and unmistakably discrete, but it breaks the row's
   look and "click to advance" is less discoverable where people expect to turn.
3. **Knob with engraved notches and a name window.** The most like hardware at N>3, and the most
   painting code to write today for a two-state reality.

## Design

**One source of truth, in `acid-core::oscillator`.**

- `Waveform::ALL: &'static [Waveform]` — the panel's order.
- `Waveform::index(self) -> usize`, `Waveform::from_index(usize) -> Waveform` — the mapping.
  `from_index` **clamps** rather than panics, so a save file written by a version with more waves
  loads here at the nearest wave instead of failing.
- `Waveform::name(self) -> &'static str` — `"SAW"`, `"SQR"`, ASCII, distinct.
- Adding a wave = new variant + `ALL` entry + `name` arm + its arm in the oscillator's sample
  match. Nothing else changes.

**`SharedState`** keeps `waveform: AtomicUsize`, but stops hand-decoding: `set_waveform` stores
`Waveform::index`, `waveform()` decodes with `from_index`.

**`persist`** keeps `Knobs.waveform` as a `u8` index — existing saves keep their meaning — but
writes and reads it through `index`/`from_index`.

**The widget** lives in `ui/knob.rs` beside the continuous one:

```rust
pub fn stepped_knob(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,          // "WAVEFORM"
    index: &mut usize,    // always a valid position
    count: usize,         // Waveform::ALL.len()
    diameter: f32,
) -> Response
```

- Paints the same body as `knob`, a detent tick per position, and the indicator at
  `position_angle(*index, count)` — `count == 1` sits at 12 o'clock, otherwise the positions are
  spread evenly over the existing 270° sweep.
- Two captions: `label` on the legend line, and `Waveform::name` of the current index just above
  it, so the panel reads `SAW` / `WAVEFORM` at a glance.
- Input, all of which end on a valid index: **click** advances (wrapping), **drag** steps one per
  30 px from the press origin, **scroll** steps. Nothing can leave the control between states.

**`draw_panel`** keeps a `usize` for the waveform in `PanelState`, read from `SharedState` every
frame like the other knobs (index↔index round-trips exactly), and draws cell 0 with
`stepped_knob`. This retires `waveform_from_knob`, and with it the lossy read-back that made the
old control unable to cross its midpoint.

## Tests

- `acid-core`: every variant round-trips through `index`/`from_index`; out-of-range clamps to the
  last; names are distinct, non-empty and ASCII; `ALL` is ordered and complete.
- `acid-app` (headless egui frames): a click advances and wraps; a multi-frame drag steps; a
  scroll steps; the value is never outside `0..count`; the panel's caption shows the current name;
  a save written with index 1 still loads as square.

## Left to the builder

Tick geometry, caption sizes, and whether the name sits on the legend line or a line above it;
the drag step (30 px per state proposed). Sound and layout are untouched either way.
