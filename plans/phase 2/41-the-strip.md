# Plan 41 — The strip: one device on the panel, the rest in a row

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This plan touches `draw_panel`'s body
> and every layout number, so one session for the whole plan. Steps use checkbox (`- [ ]`) tracking; each task ends in a green
> commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md` — the diff is reviewed against this task's **Files** and
> **Interfaces** blocks, and every finding is actioned, before the commit. No subagents elsewhere: this plan runs inline, in one
> session, by one pair of hands.

**Goal:** a strip band above the top strip, one cell per device (name, play lamp, MUTE, lit border when selected), `+` to add,
`Ctrl+1..4` to select, a click to select, and a panel that remembers each device's editor — the selected step, step mode, step
mode's placed keys, the last pitch, the randomise dials — so switching devices and back loses nothing.

**Architecture:** `ui/strip.rs` owns the strip's geometry and painting and nothing else; `main` fills `PanelState::strip` as a
mirror every frame (like `state.pattern` is a mirror today) and drains `PanelState::device_request` after the frame. The panel
keeps its twenty-odd flat fields as "the selected device's editor"; `PanelEdit` is what travels when the selection moves, and
`draw_panel` notices the move by comparing `strip.selected` against `focused`. `draw_panel`'s signature does not change, so the
twenty-four existing `tests/*_live.rs` keep compiling.

**Tech Stack:** Rust 2024, egui/eframe 0.37 (painter shapes, no glyphs), `acid_core::pattern::PatternId`.

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — read "The panel" and "Panel state" before starting.

## Global Constraints

Inherited from `plans/phase 2/00-phase-2.md`. This plan is the one that can break the panel:

- **`draw_panel(&mut PanelState, &SharedState, &mut egui::Ui)` does not change.** `shared` is the selected device's state.
- **No font glyphs.** The lamp is a filled circle, the `+` two strokes, the border a `Rect` stroke. No `▶`, no `…` unless it
  is `ui::text::fit` cutting a name.
- **`egui::Id`s are behaviour.** The strip's cells are `"device-cell-{n}"`, its mute keys `"device-mute-{n}"`, the add key
  `"device-add"`. Nothing renames one later for tidiness.
- **Every existing layout test must still pass unmodified**, or say why in the commit message — the six bands keep their pixel
  heights and only move down 30 px.
- **The panel may see a removal one frame late.** `edit_slots` is truncated to `strip.cells.len()` and `strip.selected` clamped
  at the top of `draw_panel`, so that frame cannot index out of bounds.
- A device's name is drawn through `ui::text::fit`, so a long name is truncated in the cell and never in the store.

## Review Focus

1. **A strip that indexes a device that no longer exists.** The panel is a painter and `main` owns the list; the one-frame gap
   after a removal must be safe. Task 3.
2. **Losing the step you were editing when you look at the other device.** `save_edit`/`load_edit` is the whole of "the panel
   remembers". Task 5.
3. **A `Ctrl+1` that edits a step instead.** The number keys are not step keys today, but `is_panel_key` must accept the
   combination or `presses` drops it before `dispatch` ever sees it. Task 4.
4. **A MUTE that mutes the displayed device rather than the clicked one.** The click carries the index it was drawn for, not
   the selected one. Task 6.
5. **A strip that hides itself when there is one device,** making `+` undiscoverable. The strip draws at any count. Task 2.

---

### Task 1: The panel grows a strip band

**Files:**
- Modify: `rust/crates/acid-app/src/ui/theme.rs` (`DESIGN_SIZE`, `WINDOW_SIZE`)
- Modify: `rust/crates/acid-app/src/ui/layout.rs` (`BANDS`, `PanelLayout`, `layout`)
- Test: `rust/crates/acid-app/src/ui/layout.rs` (unit tests)

**Interfaces:**
- Produces: `PanelLayout::strip: egui::Rect`, and `theme::DESIGN_SIZE = vec2(980.0, 670.0)`,
  `theme::WINDOW_SIZE = [980.0, 670.0]`. The six existing bands are unchanged in height and each moves down `30.0 / 670.0` of
  the panel: `46, 114, 342, 416, 532, 600` replace `16, 84, 312, 386, 502, 570`.

- [ ] **Step 1: Write the failing tests** in `layout.rs`'s test module:

```rust
#[test]
fn the_strip_sits_above_the_top_strip_and_nothing_else_moves_height() {
    let window = Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0));
    let l = layout(window);
    assert!(l.strip.top() < l.top.top(), "the strip is the first band");
    assert!((l.strip.height() - 30.0).abs() < 1.0, "the strip is 30 design px: {}", l.strip.height());
    // Every band keeps its pixel height: the strip cost the panel 30 px, not the face.
    for (band, height) in [
        (l.top, 56.0),
        (l.knobs.area, 216.0),
        (l.perform.area, 62.0),
        (l.steps, 104.0),
        (l.switches, 58.0),
        (l.transport, 44.0),
    ] {
        assert!((band.height() - height).abs() < 1.5, "a band changed height: {} vs {height}", band.height());
    }
    assert!(l.transport.bottom() < l.panel.bottom(), "the bottom margin survives");
}

#[test]
fn the_strip_has_one_cell_per_device_and_one_add_key() {
    let window = Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0));
    let l = layout(window);
    let cells = super::strip_cells(l.strip, 3);
    assert_eq!(cells.len(), 3);
    assert!(cells[2].right() < super::strip_add_rect(l.strip).left(), "the add key is past the last cell");
    assert!(cells.windows(2).all(|pair| pair[0].right() < pair[1].left()), "cells do not overlap");
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --lib ui::layout`
Expected: FAIL — `no field strip`.

- [ ] **Step 3: Move the numbers**

`BANDS` keeps its six entries as `(top, bottom)` in *design* pixels over 670 rather than 640. Add the strip band as its own
constant, not as a seventh `BANDS` entry: `BANDS` is the six bands `paint_panel` walks, and a seventh would change what
"every band" means to every existing test. `strip_cells(area, count)` and `strip_add_rect(area)` are pure helpers beside
`knob_cells`, sharing the same `GAP`, and `layout` does not call them — the strip's count depends on the rack, which the
layout cannot know.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib ui::layout && cargo test -p acid-app --test panel_live`
Expected: PASS. Any live test that hard-codes `560.0` for the screen still works: `layout` scales the design to whatever
window it is given.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/theme.rs rust/crates/acid-app/src/ui/layout.rs
git commit -m "feat(ui): the panel grows thirty pixels and a strip band"
```

---

### Task 2: The strip draws itself

**Files:**
- Create: `rust/crates/acid-app/src/ui/strip.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (add `pub mod strip;`)
- Modify: `rust/crates/acid-app/src/ui/panel.rs` (`paint_panel` leaves the strip band to the strip)
- Test: `rust/crates/acid-app/src/ui/strip.rs` (unit tests for geometry only — the rest is task 6's live test)

**Interfaces:**
- Consumes: `layout::{strip_cells, strip_add_rect}`, `theme`, `ui::text::fit`, `ui::tips::Control`.
- Produces:
  ```rust
  #[derive(Clone, PartialEq, Eq, Debug)]
  pub struct StripCell { pub name: String, pub running: bool, pub muted: bool }

  /// What the panel knows about the rack without owning it. `main` fills it every frame.
  #[derive(Clone, Default, Debug)]
  pub struct Strip {
      pub cells: Vec<StripCell>,
      pub selected: usize,
      /// What the strip asked for this frame; `main` drains it.
      pub request: Option<crate::device::Request>,
  }

  /// Paint the band. Returns nothing: clicks are recorded in `strip.request`, not reported.
  pub fn paint(ui: &mut egui::Ui, state: &mut PanelState, area: egui::Rect, s: f32);
  ```
  `tips::Control` gains `DeviceCell`, `DeviceMute`, `DeviceAdd` — three variants and three arms, nowhere else.

- [ ] **Step 1: Write the failing test** in `strip.rs`:

```rust
#[test]
fn a_cell_is_wide_enough_for_a_name_and_the_lamp_never_moves() {
    let area = Rect::from_min_size(Pos2::new(16.0, 8.0), vec2(948.0, 30.0));
    for count in 1..=crate::device::MAX_DEVICES {
        let cells = super::super::layout::strip_cells(area, count);
        assert_eq!(cells.len(), count);
        // The lamp and the MUTE key are at the cell's right edge, so a wide name eats the name, never the lamp.
        let lamp = lamp_rect(cells[0]);
        let mute = mute_rect(cells[0]);
        assert!(lamp.right() <= cells[0].right());
        assert!(mute.left() >= lamp.right(), "the lamp and the key do not overlap");
        assert!(
            name_rect(cells[0]).right() <= lamp.left(),
            "a name can never be drawn over the lamp"
        );
    }
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --lib ui::strip`
Expected: FAIL — the module does not exist.

- [ ] **Step 3: Write the strip**

Three pure rect helpers (`name_rect`, `lamp_rect`, `mute_rect`) so the geometry is testable without a window, then `paint`:

- the band's well (`theme::WELL`) and a bottom hairline, so the strip reads as a shelf above the panel;
- each cell: `theme::PANEL_RAISED` face, name in `SILKSCREEN` (fit to `name_rect`), a lamp filled `LED_ON` when
  `cell.running` else `LED_OFF`, a `MUTE` key using `transport::transport_button` with `lit = cell.muted`, and a
  `theme::LED_ACCENT` border when `index == strip.selected`;
- the `+` key past the last cell, drawn as two strokes in `SILKSCREEN_DIM` (greyed, i.e. `LED_OFF`, at `MAX_DEVICES`);
- `ui.interact` per cell with `Id::new(("device-cell", index))`, `Sense::click()`, so a test can find it and a click cannot be
  stolen by a neighbouring cell.

`paint_panel` keeps painting the panel face and the screws; the strip band needs the face behind it, so the call order in
`draw_panel` is `paint_panel` then `strip::paint`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib ui::strip`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/strip.rs rust/crates/acid-app/src/ui/mod.rs rust/crates/acid-app/src/ui/panel.rs
git commit -m "feat(ui): a strip of device cells above the panel"
```

---

### Task 3: The panel's mirror, and the requests going back

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs` (the `Request` enum)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState::{strip, focused, device_request}`, `draw_panel` calls the strip)
- Modify: `rust/crates/acid-app/src/main.rs` (fill the mirror, drain the request)
- Test: `rust/crates/acid-app/tests/devices_live.rs` (new file)

**Interfaces:**
- Produces:
  ```rust
  // device.rs
  #[derive(Clone, PartialEq, Eq, Debug)]
  pub enum Request {
      Select(usize),
      SetMute(usize, bool),
      Add(DeviceKind),
  }
  // ui/mod.rs
  impl PanelState {
      pub strip: strip::Strip,
      /// The device the flat editor fields currently hold. Follows `strip.selected` one frame later.
      pub focused: usize,
      pub device_request: Option<device::Request>,
  }
  ```
  `main` fills `panel.strip.cells` from `rack.devices()` and `panel.strip.selected` from `rack.selected()` every frame, and after
  the frame does `if let Some(request) = panel.device_request.take() { .. }` — a `match` in `main`, written out in full, because
  `Rack::add`'s refusals have to be answered somewhere and the panel cannot see the rack.
- [ ] **Step 1: Write the failing test** in `tests/devices_live.rs`:

```rust
//! The strip: one cell per device, a click to select, a click on MUTE for that device, and `+`.

use std::sync::Arc;

use acid_app::device::{DeviceKind, Rack};
use acid_app::persist;
use acid_app::state::SharedState;
use acid_app::ui::strip::StripCell;
use acid_app::ui::{PanelState, draw_panel};
use eframe::egui::{Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn run(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) -> FullOutput {
    let input = RawInput { screen_rect: Some(screen()), events, ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

fn panel_of(names: &[(&str, bool, bool)]) -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = names
        .iter()
        .map(|(name, running, muted)| StripCell { name: (*name).to_string(), running: *running, muted: *muted })
        .collect();
    panel
}

fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run(ctx, panel, shared, vec![button(true)]);
    run(ctx, panel, shared, vec![button(false)]);
}

#[test]
fn the_add_key_asks_for_the_only_kind_of_device_there_is() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false)]);
    panel.strip.selected = 0;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let area = acid_app::ui::layout::layout(screen()).strip;
    let add = acid_app::ui::layout::strip_add_rect(area);
    click(&ctx, &mut panel, &state, add.center());
    assert_eq!(panel.strip.request, Some(acid_app::device::Request::Add(DeviceKind::Acid)));
}

#[test]
fn clicking_the_second_cell_selects_the_second_device() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", false, false)]);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let cells = acid_app::ui::layout::strip_cells(acid_app::ui::layout::layout(screen()).strip, 2);
    click(&ctx, &mut panel, &state, cells[1].center());
    assert_eq!(panel.strip.request, Some(acid_app::device::Request::Select(1)));
}

#[test]
fn the_mute_key_carries_the_cell_it_was_drawn_for() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", true, true)]);
    panel.strip.selected = 0; // device 1 is on the panel; the click is on device 2's key
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let cells = acid_app::ui::layout::strip_cells(acid_app::ui::layout::layout(screen()).strip, 2);
    let mute = acid_app::ui::strip::mute_rect(cells[1]);
    click(&ctx, &mut panel, &state, mute.center());
    assert_eq!(
        panel.strip.request,
        Some(acid_app::device::Request::SetMute(1, false)),
        "muting a device that is already muted must ask to unmute that device"
    );
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --test devices_live`
Expected: FAIL — `no field strip`.

- [ ] **Step 3: Add the fields, the call, and the drain**

`PanelState::default` gets `strip: Strip::default()`, `focused: 0`, `device_request: None`. `draw_panel` calls
`strip::paint(ui, state, l.strip, s)` right after `panel::paint_panel`, and clamps the mirror it was handed before anything
draws it, because `main` owns the list and may have removed a device since the last frame:

```rust
state.strip.selected = state.strip.selected.min(state.strip.cells.len().saturating_sub(1));
```

In `main::logic`, fill the mirror before `ui::draw_panel` runs (which happens in `TakkaApp::ui`, so fill it in `logic`, one
frame ahead — the cell names change only when a name changes, so a frame of lag is invisible). Drain after the frame in `ui`:

```rust
if let Some(request) = self.panel.device_request.take() {
    match request {
        device::Request::Select(index) => { self.rack.select(index); }
        device::Request::SetMute(index, muted) => {
            if let Some(device) = self.rack.devices().get(index) {
                device.state.set_mute(muted);
            }
        }
        device::Request::Add(kind) => {
            match self.rack.add(kind) {
                Ok(index) => { self.rack.select(index); self.panel.dirty = true; }
                Err(device::AddError::AtCapacity) => self.notify("four devices is the limit"),
            }
        }
    }
}
```

The `match` is written out in full: `Request` is a catalogue enum and a new variant must fail to compile here.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test devices_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/device.rs rust/crates/acid-app/src/ui/mod.rs rust/crates/acid-app/src/main.rs rust/crates/acid-app/tests/devices_live.rs
git commit -m "feat(ui): the strip mirrors the rack and asks it for changes"
```

---

### Task 4: `Ctrl+1..4` selects

**Files:**
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (`KeyPress`, `presses`, `handle`, `is_panel_key`, `Handled`)
- Modify: `rust/crates/acid-app/src/main.rs` (the `presses` call site)
- Test: `rust/crates/acid-app/src/ui/dispatch.rs` (unit tests), and `rust/crates/acid-app/tests/settings_live.rs` — its
  `press` helper calls `presses`, so the signature change reaches it (the plan first missed it)

**Interfaces:**
- Produces: `KeyPress { key: Key, shift: bool, ctrl: bool }`, `presses(events: &[Event], modifiers: egui::Modifiers) ->
  Vec<KeyPress>`, `Handled::Rack`, and `dispatch::device_key(key: Key) -> Option<usize>` — `Num1..Num4` as `0..3`, nothing
  else. `presses`'s signature replaces its `shift: bool` argument, so five call sites in its own tests move from `false`/`true`
  to `Modifiers::NONE`/`Modifiers::SHIFT`.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn ctrl_and_a_digit_asks_for_a_device() {
    assert_eq!(device_key(Key::Num1), Some(0));
    assert_eq!(device_key(Key::Num4), Some(3));
    assert_eq!(device_key(Key::Num5), None, "there are four devices");
    assert_eq!(device_key(Key::ArrowUp), None);
}

#[test]
fn a_ctrl_digit_is_a_panel_key_and_a_bare_digit_is_not() {
    assert!(is_panel_key(Key::Num2, false, true));
    assert!(!is_panel_key(Key::Num2, false, false));
    assert!(!is_panel_key(Key::Num5, false, true), "four devices, four digits");
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --lib ui::dispatch`
Expected: FAIL — `cannot find function device_key`.

- [ ] **Step 3: Teach the dispatcher**

`presses` takes `Modifiers` and fills both flags. `is_panel_key(key, shift, ctrl)` returns true for a ctrl digit before it asks
`shortcut_for`. `handle` checks the ctrl digit **first**, before the step editor: a ctrl digit is not a step edit and must not be
eaten by one. It writes `panel.device_request = Some(Request::Select(index))` and returns `Handled::Rack`. Extend `Handled`'s
`match` arms everywhere they are exhaustive.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --lib ui::dispatch && cargo test -p acid-app`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/dispatch.rs rust/crates/acid-app/src/main.rs rust/crates/acid-app/tests/settings_live.rs
git commit -m "feat(ui): ctrl and a digit picks the device the panel shows"
```

---

### Task 5: The editor follows the device

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelEdit`, `save_edit`, `load_edit`, `edit_slots`, the swap in `draw_panel`)
- Test: `rust/crates/acid-app/tests/devices_live.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, PartialEq)]
  pub struct PanelEdit {
      pub pattern_id: PatternId,
      pub selected: usize,
      pub step_mode: bool,
      pub placed: Vec<egui::Key>,
      pub last_pitch: Pitch,
      pub last_octave: i8,
      pub density: f32,
      pub scale: scale::Scale,
      pub scale_knob: f32,
  }
  impl PanelState {
      pub fn save_edit(&mut self) -> PanelEdit;
      pub fn load_edit(&mut self, edit: PanelEdit);
  }
  impl PanelState { pub edit_slots: Vec<PanelEdit> }
  ```
  These nine fields are exactly the ones that are *not* re-read from `SharedState` each frame: `knobs`, `volume`, `pattern`,
  `tempo`, `playing` and `position` are mirrors and need no slot.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn looking_at_the_other_device_and_back_keeps_the_step_you_were_on() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", true, false), ("BASS", true, false)]);
    panel.selected = 4;
    panel.step_mode = true;
    panel.density = 0.75;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    // `main` applied the selection: the rack says device 2, the panel has not caught up yet.
    panel.strip.selected = 1;
    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.focused, 1);
    assert_eq!(panel.selected, 0, "a device the panel has never edited starts on step 1");
    assert!(!panel.step_mode);
    assert!((panel.density - acid_app::ui::randomise::DEFAULT_DENSITY).abs() < 1e-6);

    panel.strip.selected = 0;
    run(&ctx, &mut panel, &state, vec![]);
    assert_eq!(panel.focused, 0);
    assert_eq!(panel.selected, 4, "the step the hand was on came back");
    assert!(panel.step_mode);
    assert!((panel.density - 0.75).abs() < 1e-6);
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test devices_live`
Expected: FAIL — `no field edit_slots`, and the step does not survive.

- [ ] **Step 3: Implement the swap**

At the top of `draw_panel`, after filling the mirrors:

```rust
state.edit_slots.truncate(state.strip.cells.len().max(1));
state.strip.selected = state.strip.selected.min(state.strip.cells.len().saturating_sub(1));
if state.strip.selected != state.focused {
    let leaving = state.save_edit();
    if state.edit_slots.len() <= state.focused {
        state.edit_slots.resize(state.focused + 1, leaving.clone());
    }
    state.edit_slots[state.focused] = leaving;
    state.focused = state.strip.selected;
    let arriving = match state.edit_slots.get(state.focused) {
        Some(edit) => edit.clone(),
        None => PanelEdit::of_defaults(),
    };
    state.load_edit(arriving);
    // A learn belongs to the device it was started on: switching away cancels it rather than teaching the neighbour.
    state.learn = learn::Step::Idle;
    learn::cancel(shared);
}
```

`PanelEdit::of_defaults()` is a constructor from `PanelState::default`'s own values — do not copy them by hand, or the two
drift.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test devices_live && cargo test -p acid-app --test selection_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/mod.rs rust/crates/acid-app/tests/devices_live.rs
git commit -m "feat(ui): each device keeps its own step, step mode and randomise dials"
```

---

### Task 6: The strip says what it looks like

**Files:**
- Modify: `rust/crates/acid-app/src/ui/strip.rs` (the lamp, the border, the greyed `+`)
- Test: `rust/crates/acid-app/tests/devices_live.rs`

**Interfaces:**
- Produces: no new API. The observable is what `draw_panel` paints, which is what the test reads back.

- [ ] **Step 1: Write the failing tests**

```rust
/// Every filled rectangle and every stroked one egui drew: a cell's face, its lamp, its border.
fn painted(output: &FullOutput) -> (Vec<Rect>, Vec<Rect>) {
    use eframe::egui::Shape;
    fn walk(shape: &Shape, filled: &mut Vec<Rect>, stroked: &mut Vec<Rect>) {
        match shape {
            Shape::Rect(rect) if rect.fill != eframe::egui::Color32::TRANSPARENT => filled.push(rect.rect),
            Shape::Rect(rect) => stroked.push(rect.rect),
            Shape::Circle(circle) => filled.push(Rect::from_center_size(circle.center, vec2(circle.radius * 2.0, circle.radius * 2.0))),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, filled, stroked)),
            _ => {}
        }
    }
    let mut filled = Vec::new();
    let mut stroked = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut filled, &mut stroked);
    }
    (filled, stroked)
}

#[test]
fn the_selected_cell_wears_the_accent_and_the_others_do_not() {
    let state = SharedState::new();
    let mut panel = panel_of(&[("ACID 1", false, false), ("BASS", true, false)]);
    panel.strip.selected = 1;
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &state, vec![]);

    let cells = acid_app::ui::layout::strip_cells(acid_app::ui::layout::layout(screen()).strip, 2);
    let (_, stroked) = painted(&output);
    assert!(
        stroked.iter().any(|rect| rect.expand(-1.0).contains_rect(&cells[1])),
        "the selected cell's border is drawn"
    );
    assert!(
        !stroked.iter().any(|rect| rect.expand(-1.0).contains_rect(&cells[0])),
        "and its neighbour's is not"
    );

    // The running device's lamp is lit and the stopped one's is not: compare the two cells' brightest pixel areas.
    let (filled, _) = painted(&output);
    let lit = |cell: Rect| filled.iter().any(|rect| cell.contains_rect(rect) && rect.width() < 20.0);
    assert!(lit(cells[1]), "a running device shows its lamp");
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test devices_live`
Expected: FAIL — no accent border is drawn.

- [ ] **Step 3: Paint it**

Border: `painter.rect_stroke(cell, rounding, Stroke::new(1.5 * s, theme::LED_ACCENT), StrokeKind::Inside)` for the selected
cell only. Lamp: a filled circle of `LED_ON` when running, `LED_OFF` otherwise, radius scaled by `s`. `+` at the cap: painted in
`theme::LED_OFF` and its `ui.interact` still reports the click — the request is refused by `Rack::add` with a notice, which is
what the player needs to know.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test devices_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/strip.rs rust/crates/acid-app/tests/devices_live.rs
git commit -m "feat(ui): the strip lights the device it is showing and lamps the ones playing"
```

---

### Task 7: The whole gate, and the app by hand

**Files:**
- Test: the workspace

- [ ] **Step 1: Run the three gates**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS, every one. The two things most likely to need a fix: `tips::Control`'s exhaustive matches (three new variants)
and any `layout` test that asserted the old band tops literally.

- [ ] **Step 2: Run it and use it**

`cargo run -p acid-app --bin takka-acid`. Add a device with `+`, select with `Ctrl+2` and with a click, mute one, play both,
then switch back and forth and confirm the step you were editing is where you left it. Type a pattern on device 2 and confirm
device 1's pattern is untouched.

- [ ] **Step 3: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "chore(ui): plan 41's gate, run and used"
```

---

## Progress

- [x] **Task 1 — the strip band.** `9b60dd9`. `DESIGN_SIZE` 980x670; the six inherited bands moved down 30 px with their heights;
      `layout` gains `strip`, `strip_cells`/`strip_add_rect`, and `band_of` as the one place the inset arithmetic lives. One
      existing test changed (the aspect assertion hard-coded 980/640). Self-reviewed: geometry only.
- [x] **Task 2 — the strip draws itself.** `85ab8c1`. `ui/strip.rs` with `StripCell`/`Strip`, the three rect helpers and `paint`;
      `strip::paint` is called from `draw_panel` right after the face (a strip that is never painted is not a deliverable, so
      the call moved up from task 3). `device::Request` (`Select`/`SetMute`/`Add`) and `PanelState::{strip, device_request}`
      land here — task 2's own Interfaces block needs the request type, and task 2's `Strip.request` was a second queue the
      spec does not have (ruling below). `tips::Control` gains `DEVICE`/`MUTE`/`ADD DEVICE`, appended. `panel.rs` needed no
      change: it paints the face and the strip paints its own well on top. Self-reviewed: painting and geometry.
- [x] **Task 3 — the mirror and the requests.** `main` fills `panel.strip` at the *end* of `logic` (after every drain that could
      change the rack), and `TakkaApp::ui` drains `panel.device_request` straight after `draw_panel` — one `take`, one full
      `match`. `draw_panel` clamps `strip.selected` before anything draws. Tests: `tests/devices_live.rs` (4; the three the
      plan gives plus the clamp). Not tested here: `main`'s drain, because it lives in the binary — task 7's hand-run is its
      coverage.
- **Task 3 review** (fresh `reviewer` subagent, 7 findings: 1 Critical, 2 Important, 4 Minor/nit; all actioned). The Critical
      one was a data-loss path this commit made reachable: `save`/`session_for_disk` built the session from the *selected*
      device, and nothing until now could move the selection — so `+` then SAVE would have rewritten the open song with the
      new device's single starter pattern. The session now comes from `Rack::saved_device()`, which is the first device by
      definition (the spec's rule, and the frozen format), with `a_session_saves_the_first_device_whatever_is_selected` in
      `tests/rack.rs` pinning it; `Add` no longer marks the session dirty, because nothing on disk changed. Important two:
      `Select`/`Add` ran `pattern_swap::adopt`, which zeroes `panel.selected` at drain time — *after* the frame — so task 5's
      `save_edit` would have stored the leaving device at step 1 and its own test could not see it; that is now
      `pattern_swap::follow` (`adopt` = `follow` + `selected = 0`), which sets what a switch changes and leaves the step alone.
      Important three: a controller's held pattern swap (`pending_pattern`) belonged to the device it was asked of, so a
      switch now clears it — the same line `main` already uses for "a held swap belongs to the device that asked".
      Also actioned: the mirror moved to the end of `logic` so the list the painter drew and the list a click is answered
      against cannot differ by a removal in the same frame; a blank line. Recorded, not done: `PanelState::focused` is task
      5's field (the plan's task 3 Interfaces block lists it, but nothing in task 3 needs it); `SetMute`/`Select` index by
      position, so plan 42's remove path will want `Device::number` as the durable identity — accepted now because nothing in
      the app can remove a device yet, and recorded here so the card round does not discover it. The mirror allocates a `Vec`
      and a `String` per device per frame; accepted (UI thread, ≤ 4 devices), and noted because its neighbours in `logic` are
      deliberately allocation-free.
- [x] **Task 4 — `Ctrl+1..4`.** `KeyPress` carries `ctrl`; `presses` takes `egui::Modifiers`; `is_panel_key` answers a ctrl digit
      before it asks the shortcuts; `device_key` reads a `DEVICE_DIGITS: [Key; MAX_DEVICES]` table, so a five-device rack fails to
      compile rather than leaving `Ctrl+5` dead; `handle` answers the ctrl digit **first** and returns `Handled::Rack`. A digit
      past the last device is now a notice ("no device there") rather than a dead key: the keyboard is the only thing that can
      name an absent cell. `tests/settings_live.rs`'s helper moved to the new `presses` signature — the plan's Files block and
      `git add` line missed that file.
- **Task 4 review** (fresh `reviewer` subagent, 1 Important + 4 Minor, all actioned). The Important: the new `handle` branch and
      `presses`'s ctrl plumbing had *no* test — `device_key` and `is_panel_key` were pinned, so deleting the branch left every
      suite green. `a_ctrl_digit_asks_the_rack_to_select_that_device` now drives `presses` → `handle` and asserts `Handled::Rack`
      and `Select(2)`; mutation-checked by disabling the branch (`Nothing` vs `Rack`, then restored). Also actioned: `device_key`
      derives from `MAX_DEVICES` instead of four hand-copied arms; the refused `Select` says something; the plan's Files/`git add`
      corrected. **Ruling:** `presses` reads the *frame's* `Modifiers`, not the `Event::Key`'s own — the plan's Interfaces block
      says so and `ui::keys` documents the opposite for the audition keys. Cost if wrong: a modifier pressed or released inside
      one frame can be attributed to the wrong press, which would turn a bare digit into a device select. Rare (it needs press
      and release inside ~16 ms); the `shift` half of it predates this task.
- **Task 4 follow-up, found by the headless run (task 7 step 2, done early):** that ruling was wrong, and the run proved it.
      `xdotool` delivers ctrl and the digit inside one frame, `presses` filtered on `input.modifiers` (still without ctrl), and
      `Ctrl+1..4` did nothing — with the ctrl held from a previous frame the same keys selected the device. So `presses` now
      takes no modifiers argument at all and reads them from each `Event::Key` — one source of truth, the event's, as `ui::keys`
      already documented — and `a_ctrl_digit_asks_the_rack_to_select_that_device` builds a ctrl event rather than passing a
      frame's modifiers. The plan's Interfaces block said otherwise; the device it was protecting (a bare digit becoming a
      select) is still protected, because a bare digit's event carries no ctrl.
- [x] **Task 5 — the editor follows the device.** `PanelEdit` (nine fields) with `PanelState::{save_edit, load_edit}`, `focused` and
      `edit_slots`, and the swap at the top of `draw_panel`: truncate the slots to the strip's count, save the outgoing device,
      load the incoming one's. A device the panel has never shown gets `PanelEdit::of_defaults()` (built from
      `PanelState::default`, so the two cannot drift) **except** for `pattern_id`, which keeps what `main` has just pointed the
      display at — that is that device's bank, not a default. A switch also cancels a learn in progress, because a learn belongs
      to the device it was started on. Mutation-checked: disable the swap and `looking_at_the_other_device_and_back_keeps_the_step_you_were_on`
      fails on the step. Not covered by a test: `placed` (the step-mode keys) and `last_pitch`/`last_octave` travel in the slot
      but nothing asserts it — the swap moves the whole struct, so a missing field would be a compile error at `save_edit`, and
      the nine are named there.
- [x] **Task 6 — the strip says what it looks like.** `tests/devices_live.rs` reads the paints back: a stroked rect within a pixel
      of the selected cell and none at its neighbour, and the lamp read by **colour** — `LED_ON` for the running device, `LED_OFF`
      for the stopped one (the lamp is drawn either way; only its colour is the state). Green on arrival, because task 2 painted
      the border and the lamp; the plan expected red here and this is why. Mutation-checked: removing the `if selected` stroke
      fails the border assertion. The greyed `+` at the cap is **not** covered: at four devices the key is painted `LED_OFF`, and
      a test would need to read a stroke's colour — recorded so the next hand knows the gap.
- [ ] Task 7 — the gate, and the app by hand

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The strip is its own band, not a seventh `BANDS` entry | `paint_panel`'s "every band" loops keep their meaning; the strip draws its own well. A future band would have to choose which list it belongs to. |
| `PanelEdit` is nine fields, not the whole `PanelState` | A field added later that is *not* a mirror and *not* in `PanelEdit` silently resets on every device switch. The swap's test names them; a new one needs a line there. |
| One request queue, `PanelState::device_request`, not a second one on `Strip` | Task 2's Interfaces block gave `Strip` its own `request`; the spec and tasks 3–4 put the queue on `PanelState`. One queue is the only shape that cannot have two answers pending at once. The strip's tests assert on `panel.device_request`. |
| A learn in progress is cancelled by a device switch | Teaching a control on device 1 and then looking at device 2 costs the player a re-click on `LEARN`. The alternative teaches the wrong device. |
| `KeyPress` carries both modifiers rather than a new enum | `shift` and `ctrl` can both be true; a `Modifier` enum would have to say what shift+ctrl means before anything needs it. |
| `Ctrl+digit` is dispatched before the step editor | The number keys are not step keys today, so nothing is taken from anything. If a step shortcut ever lands on `Num1`, this order decides — and the order is written down here so it is a decision, not an accident. |
