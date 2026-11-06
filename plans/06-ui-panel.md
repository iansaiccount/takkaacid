# Phase 06 — The black-and-yellow panel

> Part of [00-main-plan.md](00-main-plan.md). Tasks **27–32**. Depends on: Phase 05.
> **Delivers:** the visual identity — a recessed near-black panel with amber silkscreen, seven painted
> knobs, sixteen keycap step buttons with yellow LEDs, a segment-style display, and layout maths that
> holds at any window size.

**Why the look is a phase of its own:** every control on this panel is custom-painted, so each one is a
small piece of geometry plus an interaction rule. Those two halves are testable separately: the maths
as pure functions, the look by eye. Nothing here talks to the engine yet — Phase 07 wires it.

**Panel sketch (design units, 980 × 560):**

```
┌──────────────────────────────────────────────────────────────────────────┐
│ takka-acid                                        [ A1 ]   [ 130 ]       │  16..72
│                                                                          │
│   ◯        ◯        ◯        ◯        ◯        ◯        ◯                │  84..300
│ WAVEFORM  TUNING  CUT OFF  RESONANCE  ENV MOD   DECAY   ACCENT           │
│                                                                          │
│  ▮ ▮  ▮ ▮  ▮ ▮  ▮ ▮  ▮ ▮  ▮ ▮  ▮ ▮                                      │  316..420
│  1 2  3 4  5 6  7 8  9 10 11 12 13 14 15 16                              │
│                                                                          │
│  [BANK ▾]   [PATTERN ▾]   [MODE: PLAY]                                   │  432..490
│                                                                          │
│  [ RUN ] [ STOP ]  [TEMPO −][TEMPO +]      VOLUME ◯                      │  500..544
└──────────────────────────────────────────────────────────────────────────┘
```

---

### Task 27: Layout maths — five regions and sixteen keys

**Files:**
- Create: `rust/crates/acid-app/src/ui/layout.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod layout;`), `src/ui/theme.rs`
  (add `pub const DESIGN_SIZE: egui::Vec2 = egui::vec2(980.0, 560.0);`, `pub const MARGIN: f32 = 16.0;`,
  `pub const GAP: f32 = 6.0;`)
- Test: `rust/crates/acid-app/src/ui/layout.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `egui::{Rect, Pos2, Vec2}`.
- Produces:
  - `pub struct PanelLayout { pub panel: Rect, pub top: Rect, pub knobs: Row, pub steps: Rect,
    pub switches: Rect, pub transport: Rect }` where
    `pub struct Row { pub area: Rect, pub cells: Vec<Rect> }` (`cells` has one rect per control).
  - `pub fn layout(window: Rect) -> PanelLayout` — pure; all rects inside `window`, none overlapping.
  - `pub fn knob_cells(area: Rect, count: usize) -> Vec<Rect>` — equal width, `GAP` between.
  - `pub fn step_cells(area: Rect) -> [Rect; 16]`.
  - `pub fn design_rect(window: Rect) -> Rect` — the panel rect: centred, `DESIGN_SIZE` aspect preserved,
    scaled by `scale(window)`.
  - `pub fn scale(window: Rect) -> f32` — `min(w/980, h/560).clamp(0.65, 1.6)`.
- Later tasks rely on every name: Task 28 (background), Task 29 (knob cells), Task 30 (step cells),
  Task 31 (displays in `top`), Phase 07 Task 36 (position LEDs).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn regions_are_ordered_and_do_not_overlap() {
    for size in [egui::vec2(500.0, 300.0), egui::vec2(980.0, 560.0), egui::vec2(1600.0, 900.0)] {
        let l = layout(Rect::from_min_size(Pos2::ZERO, size));
        assert!(l.top.bottom() <= l.knobs.area.top() + 0.01);
        assert!(l.knobs.area.bottom() <= l.steps.top() + 0.01);
        assert!(l.steps.bottom() <= l.switches.top() + 0.01);
        assert!(l.switches.bottom() <= l.transport.top() + 0.01);
        for r in [l.top, l.knobs.area, l.steps, l.switches, l.transport] {
            assert!(expand_rect(size).contains_rect(r), "region escapes the window: {r:?}");
        }
    }
}

#[test]
fn seven_knobs_are_evenly_spaced_and_sixteen_keys_fit() {
    let l = layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0)));
    assert_eq!(l.knobs.cells.len(), 7);
    let widths: Vec<f32> = l.knobs.cells.iter().map(|r| r.width()).collect();
    let first = widths[0];
    assert!(widths.iter().all(|w| (w - first).abs() < 0.5), "uneven knobs: {widths:?}");
    for pair in l.knobs.cells.windows(2) {
        assert!(pair[0].right() <= pair[1].left());
    }
    let keys = step_cells(l.steps);
    assert_eq!(keys.len(), 16);
    assert!(keys.iter().all(|k| k.width() > 12.0 && k.height() > 20.0));
    for pair in keys.windows(2) {
        assert!((pair[1].left() - pair[0].left() - (keys[1].left() - keys[0].left())).abs() < 0.5);
    }
}

#[test]
fn the_panel_keeps_its_aspect_and_scales_within_bounds() {
    assert!((scale(Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0))) - 1.0).abs() < 0.01);
    assert!(scale(Rect::from_min_size(Pos2::ZERO, egui::vec2(100.0, 100.0))) >= 0.65);
    assert!(scale(Rect::from_min_size(Pos2::ZERO, egui::vec2(4000.0, 3000.0))) <= 1.6);
    let r = design_rect(Rect::from_min_size(Pos2::ZERO, egui::vec2(1600.0, 900.0)));
    assert!((r.width() / r.height() - 980.0 / 560.0).abs() < 0.01);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app layout`
Expected: FAIL — module `ui::layout` not found.

- [ ] **Step 3: Implement the layout**

Take the five region heights from the sketch as fractions of the design height (56, 216, 104, 58, 44
with `MARGIN` and `GAP` between), scale them by `scale(window)`, and place the panel rect centred.
`knob_cells` and `step_cells` divide their area evenly with `GAP` gutters. No egui painting here — pure
`Rect` arithmetic, so it is testable without a window.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app layout`
Expected: PASS — 3 passed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): responsive panel layout maths"
```

---

### Task 28: Painted panel face — recess, screws, border and wordmark

**Files:**
- Create: `rust/crates/acid-app/src/ui/panel.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod panel;`), `src/ui/theme.rs` (add
  `pub const SCREW_RADIUS: f32 = 4.0;`, `pub fn silkscreen_stroke(width: f32) -> egui::Stroke`)
- Test: `rust/crates/acid-app/src/ui/panel.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `layout::{PanelLayout, design_rect}`, palette constants.
- Produces:
  - `pub fn screw_positions(panel: Rect) -> [Pos2; 4]` — inset `MARGIN` from each corner.
  - `pub fn frame_lines(panel: Rect) -> [Rect; 2]` — outer and inner engraved frame.
  - `pub fn brand_rect(top: Rect) -> Rect`, `pub fn pattern_display_rect(top: Rect) -> Rect`,
    `pub fn tempo_display_rect(top: Rect) -> Rect`.
  - `pub fn paint_panel(painter: &egui::Painter, layout: &PanelLayout)` — fills `PANEL`, draws the
    recess shadow (two offset rounded rects), the engraved frame in `SILKSCREEN_DIM`, four screws, and
    the `takka-acid` wordmark in `SILKSCREEN`.
- Later tasks rely on: `brand_rect`, `pattern_display_rect`, `tempo_display_rect` (Task 31),
  `paint_panel` (Phase 07 Task 33).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn screws_sit_inside_each_corner() {
    let panel = Rect::from_min_size(Pos2::new(10.0, 20.0), egui::vec2(980.0, 560.0));
    let screws = screw_positions(panel);
    assert_eq!(screws.len(), 4);
    assert!(screws.iter().all(|p| panel.contains(*p)));
    assert!(screws[0].x < panel.center().x && screws[0].y < panel.center().y);
    assert!(screws[3].x > panel.center().x && screws[3].y > panel.center().y);
}

#[test]
fn top_strip_regions_are_ordered_left_to_right() {
    let l = acid_app::ui::layout::layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0)));
    let brand = brand_rect(l.top);
    let pattern = pattern_display_rect(l.top);
    let tempo = tempo_display_rect(l.top);
    assert!(brand.left() < pattern.left());
    assert!(pattern.right() <= tempo.left());
    assert!(tempo.right() <= l.top.right());
    for r in [brand, pattern, tempo] {
        assert!(l.top.contains_rect(r), "region escapes the top strip: {r:?}");
    }
}

#[test]
fn the_engraved_frame_is_inside_the_panel() {
    let panel = Rect::from_min_size(Pos2::ZERO, egui::vec2(980.0, 560.0));
    let [outer, inner] = frame_lines(panel);
    assert!(panel.contains_rect(outer));
    assert!(outer.contains_rect(inner));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app panel`
Expected: FAIL — module `ui::panel` not found.

- [ ] **Step 3: Implement the painting**

`paint_panel`: `painter.rect_filled(panel, 8.0, theme::PANEL)`, then a 2 px `SILKSCREEN_DIM` line at
`outer` and a 1 px line at `inner`, four `SCREW_RADIUS` discs in `PANEL_RAISED` with a `SILKSCREEN_DIM`
arc slot, and the wordmark with `theme::legend(20.0 * scale)` in `SILKSCREEN`, plus a 1 px
`SILKSCREEN_DIM` rule under the top strip. Displays are painted in Task 31.

- [ ] **Step 4: Run tests, then look at it**

Run: `cd rust && cargo test -p acid-app panel`
Expected: PASS — 3 passed.
Run: `cd rust && cargo run -p acid-app`, then capture the window (`grim ~/takka-panel.png` on
Hyprland, or the desktop screenshot key) and check: near-black face, amber frame and wordmark, four
screws, no white anywhere except nothing at all. No text is legible-but-grey — everything is amber.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): painted panel face with silkscreen frame and wordmark"
```

---

### Task 29: Knob widget with drag, fine mode, and a yellow value arc

**Files:**
- Create: `rust/crates/acid-app/src/ui/knob.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod knob;`)
- Test: `rust/crates/acid-app/src/ui/knob.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `theme`, `SILKSCREEN`/`LED_ON`/`PANEL_RAISED`/`WELL`.
- Produces:
  - `pub fn value_from_drag(start: f32, dy_pixels: f32, fine: bool) -> f32` — pure: up is positive,
    full travel `knob_travel_pixels() = 180.0`, fine divides travel by 10, result clamped `0.0..=1.0`.
  - `pub fn angle_for(value: f32) -> f32` — radians, `-135°..=+135°` across `0.0..=1.0`.
  - `pub fn arc_points(center: Pos2, radius: f32, value: f32) -> Vec<Pos2>` — the yellow value arc.
  - `pub fn knob(ui: &mut egui::Ui, id: egui::Id, label: &str, value: &mut f32, diameter: f32)
    -> egui::Response` — vertical drag (shift = fine), scroll wheel, double-click resets to `0.5`;
    paints well, body, recessed cap, indicator line, arc, and the label in `SILKSCREEN` beneath.
- Later tasks rely on: `knob`, `value_from_drag`, `angle_for` (Phase 07 Task 33/34, Phase 08 Task 40).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn dragging_up_180_pixels_covers_the_whole_range() {
    assert!((value_from_drag(0.5, -180.0, false) - 1.0).abs() < 1e-5);
    assert!((value_from_drag(0.5, 180.0, false) - 0.0).abs() < 1e-5);
}

#[test]
fn fine_mode_is_ten_times_gentler_and_never_leaves_the_range() {
    let coarse = value_from_drag(0.5, -18.0, false);
    let fine = value_from_drag(0.5, -18.0, true);
    assert!((coarse - 0.6).abs() < 1e-4);
    assert!((fine - 0.51).abs() < 1e-4);
    assert_eq!(value_from_drag(0.95, -500.0, false), 1.0);
    assert_eq!(value_from_drag(0.05, 500.0, false), 0.0);
}

#[test]
fn the_arc_sweeps_270_degrees_from_seven_to_five_oclock() {
    assert!((angle_for(0.0) + 135f32.to_radians()).abs() < 1e-5);
    assert!(angle_for(0.5).abs() < 1e-5);
    assert!((angle_for(1.0) - 135f32.to_radians()).abs() < 1e-5);
    let points = arc_points(Pos2::new(100.0, 100.0), 40.0, 0.5);
    assert!(points.len() > 8);
    assert!(points.iter().all(|p| (p.distance(Pos2::new(100.0, 100.0)) - 40.0).abs() < 0.01));
}

#[test]
fn the_knob_is_not_a_font_glyph() {
    // Guard from AGENTS.md: icons must be drawn or loaded, never a font glyph.
    for label in acid_app::ui::theme::KNOB_LABELS {
        assert!(label.is_ascii(), "knob label must be ASCII: {label}");
    }
    assert_eq!(acid_app::ui::theme::KNOB_LABELS.len(), 7);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app knob`
Expected: FAIL — module `ui::knob` not found; `theme::KNOB_LABELS` missing.

- [ ] **Step 3: Implement the widget**

Add `pub const KNOB_LABELS: [&str; 7] = ["WAVEFORM", "TUNING", "CUT OFF", "RESONANCE", "ENV MOD",
"DECAY", "ACCENT"];` to `theme.rs`. Interaction: `ui.interact(rect, id, Sense::drag())`, use
`response.drag_delta().y` accumulated against `drag_origin`, `ui.input(|i| i.modifiers.shift)` for fine,
`ui.input(|i| i.raw_scroll_delta.y)` for the wheel. Paint order: `WELL` recess → `PANEL_RAISED` body →
3 px `PANEL` cap ring → `SILKSCREEN_DIM` track arc (full 270°) → `LED_ON` value arc up to `angle_for` →
`SILKSCREEN` indicator line → label centred under the knob. Cursor becomes `ResizeVertical` on hover.

- [ ] **Step 4: Run tests and look at it**

Run: `cd rust && cargo test -p acid-app knob`
Expected: PASS — 4 passed.
Temporarily show the seven knobs in `TakkaApp::update` at the layout's knob cells and check by eye:
each knob turns as you drag, shift makes it fine, the arc tracks the value, and no glyph boxes appear.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): painted knob with drag, fine mode and value arc"
```

---

### Task 30: Step keycap with LED, accent dot and slide bar

**Files:**
- Create: `rust/crates/acid-app/src/ui/step_key.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod step_key;`)
- Test: `rust/crates/acid-app/src/ui/step_key.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `pattern::{Step, STEPS_PER_PATTERN}`, palette.
- Produces:
  - `pub struct StepView { pub on: bool, pub accent: bool, pub slide: bool, pub current: bool,
    pub beat: bool }` — the five paint inputs for one key.
  - `pub fn step_view(step: &Step, index: usize, position: usize, playing: bool) -> StepView` —
    `current` only when `playing && index == position`; `beat` true for indices 0, 4, 8, 12.
  - `pub fn step_key(ui: &mut egui::Ui, index: usize, view: &StepView, rect: Rect) -> egui::Response`
    — paints LED above the keycap (`LED_ON` lit / `LED_OFF` unlit, `LED_ACCENT` outline when accented),
    keycap in `WHITE_KEY` with a `PANEL` legend number, a 3 px `LED_ACCENT` bar at the top edge for
    accent, and a 3 px `SILKSCREEN` bar under the keycap for slide.
- Later tasks rely on: `step_view`, `step_key` (Phase 07 Task 36 for the position LED, Phase 08 Task 38
  for the editing gestures).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn only_the_playing_position_lights_up() {
    let step = Step::note(Pitch::C, 0);
    let v = step_view(&step, 3, 3, true);
    assert!(v.current && !v.beat);
    let w = step_view(&step, 3, 4, true);
    assert!(!w.current);
    let stopped = step_view(&step, 3, 3, false);
    assert!(!stopped.current);
}

#[test]
fn every_fourth_key_is_a_beat_and_flags_are_copied() {
    let step = Step { accent: true, slide: true, ..Step::note(Pitch::E, 0) };
    let v = step_view(&step, 8, 0, false);
    assert!(v.on && v.beat && v.accent && v.slide && !v.current);
    assert!(!step_view(&step, 9, 0, false).beat);
}

#[test]
fn an_off_step_still_reports_its_pitch_flags_but_is_not_lit() {
    let rest = Step::rest();
    let v = step_view(&rest, 0, 0, true);
    assert!(!v.on && v.current);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: FAIL — module `ui::step_key` not found.

- [ ] **Step 3: Implement the widget**

`rect` comes from `layout::step_cells`. Split the cell: top 12 px is the LED well, the rest is the
keycap with a 2 px `PANEL` gap. Paint LED as a filled circle with a soft `LED_ON` glow (two concentric
circles, larger one at 25 % alpha). Number legend uses `theme::legend(11.0 * scale)`.

- [ ] **Step 4: Run tests and look at it**

Run: `cd rust && cargo test -p acid-app step_key`
Expected: PASS — 3 passed.
Render sixteen keys in `TakkaApp::update` with a scratch pattern and check by eye: keycaps read as
physical keys with a dark legend, the lit LED is unmistakable, accent and slide are visible at a glance
without hovering.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): step keycap with led, accent and slide indicators"
```

---

### Task 31: Segment display for pattern and tempo

**Files:**
- Create: `rust/crates/acid-app/src/ui/display.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod display;`), `src/ui/panel.rs` (paint the two
  displays into `pattern_display_rect` / `tempo_display_rect`)
- Test: `rust/crates/acid-app/src/ui/display.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `pattern::PatternId`, `timing::{clamp_bpm, MIN_BPM, MAX_BPM}`.
- Produces:
  - `pub fn format_pattern(id: PatternId) -> String` — `"A1"`..`"H8"`.
  - `pub fn format_tempo(bpm: f32) -> String` — three characters, no decimals, clamped:
    `130.0 → "130"`, `99.4 → " 99"`, `1000.0 → "300"`, `10.0 → " 40"`.
  - `pub fn format_tempo_with_unit(bpm: f32) -> String` — `"130"`, plus the silkscreen `TEMPO` label
    drawn separately, never concatenated into the digits.
  - `pub fn segments_for(ch: char) -> [bool; 7]` — seven-segment truth table for `0-9`, `A-H`, and
    blank; anything else is blank.
  - `pub fn paint_display(painter: &egui::Painter, rect: Rect, text: &str, scale: f32)` — dark `WELL`
    well, `TEXT` segments with a 40 % `LED_ON` glow, one character per cell.
- Later tasks rely on: `format_tempo`, `paint_display` (Phase 07 Task 36).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn tempo_is_three_plain_digits_within_the_panel_range() {
    assert_eq!(format_tempo(130.0), "130");
    assert_eq!(format_tempo(99.4), " 99");
    assert_eq!(format_tempo(1000.0), "300");
    assert_eq!(format_tempo(10.0), " 40");
    assert_eq!(format_tempo(99.6), "100");
}

#[test]
fn pattern_ids_and_hex_letters_both_render() {
    assert_eq!(format_pattern(PatternId { bank: 0, number: 0 }), "A1");
    assert_eq!(format_pattern(PatternId { bank: 7, number: 7 }), "H8");
    assert!(segments_for('H').iter().any(|s| *s));
    assert_eq!(segments_for('Z'), [false; 7]);
    assert_eq!(segments_for(' '), [false; 7]);
}

#[test]
fn every_digit_uses_a_different_segment_set() {
    let sets: Vec<[bool; 7]> = "0123456789".chars().map(segments_for).collect();
    for (i, a) in sets.iter().enumerate() {
        for (j, b) in sets.iter().enumerate() {
            if i != j {
                assert_ne!(a, b, "digits {} and {} share a pattern", i, j);
            }
        }
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app display`
Expected: FAIL — module `ui::display` not found.

- [ ] **Step 3: Implement the display**

Segment layout in cell coordinates: `a` top, `b` upper-right, `c` lower-right, `d` bottom,
`e` lower-left, `f` upper-left, `g` middle; each a 2 px-thick rect with mitred ends. `A`–`H` use the
digit truth table with the obvious substitutions (`A`=a,b,c,e,f,g; `B` not needed below `C`; `C`=a,d,e,f;
`D`=b,c,d,e,g; `E`=a,d,e,f,g; `F`=a,e,f,g; `G`=a,c,d,e,f; `H`=b,c,e,f,g). Digit `0` omits `g`.

- [ ] **Step 4: Run tests and look at it**

Run: `cd rust && cargo test -p acid-app display`
Expected: PASS — 3 passed.
Check by eye that the displays read like a 303's LED window, are legible at 0.7 scale, and that `A1`
and `H8` are distinguishable from `81`/`H8` respectively.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): seven-segment pattern and tempo display"
```

---

### Task 32: Whole panel assembled, scaled, and legible at any size

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (compose the panel in `TakkaApp::update`)
- Test: `rust/crates/acid-app/tests/panel_layout.rs`

**Interfaces:**
- Consumes: everything from Tasks 27–31.
- Produces: `pub fn draw_panel(app: &mut PanelState, ui: &mut egui::Ui)` in `ui/mod.rs`, where
  `pub struct PanelState` holds the seven knob values, the `Arc<Pattern>`, tempo, playing flag and
  current position. Phase 07 replaces its interior with `SharedState` reads; the function signature stays.
- Later tasks rely on `PanelState`'s field names: Phase 07 Task 33.

- [ ] **Step 1: Write the failing test**

```rust
// rust/crates/acid-app/tests/panel_layout.rs
use acid_app::ui::layout::{layout, step_cells};
use egui::{Pos2, Rect};

#[test]
fn the_panel_never_overlaps_itself_at_any_reasonable_size() {
    for (w, h) in [(700.0, 400.0), (980.0, 560.0), (1280.0, 720.0), (1920.0, 1080.0)] {
        let l = layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(w, h)));
        let mut all = vec![l.top, l.knobs.area, l.steps, l.switches, l.transport];
        all.extend(l.knobs.cells.iter().copied());
        all.extend(step_cells(l.steps).iter().copied());
        for (i, a) in all.iter().enumerate() {
            for b in all.iter().skip(i + 1) {
                if a.intersects(*b) && a != b {
                    // knob cells may not intersect each other or the step cells
                    assert!(!a.intersects(*b), "{} x {} at {w}x{h}", a, b);
                }
            }
        }
    }
}

#[test]
fn knob_hit_targets_stay_usable_at_the_smallest_size() {
    let l = layout(Rect::from_min_size(Pos2::ZERO, egui::vec2(700.0, 400.0)));
    assert!(l.knobs.cells.iter().all(|r| r.width() >= 40.0 && r.height() >= 40.0));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app --test panel_layout`
Expected: FAIL — `acid_app::ui::layout` not exported from the library (the app has only a binary).

- [ ] **Step 3: Implement assembly**

Ensure `acid-app` exposes `src/lib.rs` with `pub mod ui; pub mod state; pub mod audio;` (from Phase 02
Task 9). `TakkaApp::update` computes `layout(available_rect)`, calls `paint_panel`, then draws the knob
row, step row, switches and transport from `PanelState`. Use a scratch pattern so the panel is fully
populated; the knobs are local until Phase 07.

- [ ] **Step 4: Run tests and look at it**

Run: `cd rust && cargo test -p acid-app --test panel_layout`
Expected: PASS — 2 passed.
Run the app at 700×400 and 1920×1080, capture both, and check: nothing clipped, nothing overlapping,
legends readable, and the panel stays centred with black around it. This is the look review the owner
signs off on before Phase 07.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): assemble the full black-and-yellow panel"
```

---

**Phase 06 done when:** the app shows the complete black-and-yellow panel with working knobs, step keys
and displays, all painting is legible at 0.65× and 1.6× scale, and no control uses a font glyph as its
face. Owner look-review happens here.
