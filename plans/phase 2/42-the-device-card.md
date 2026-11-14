# Plan 42 — The device card: names, routes, and the picker

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This plan is the last piece of UI in the
> phase and depends on plan 41's request queue and caret; do not start it before 41's gate is green. Steps use checkbox
> (`- [ ]`) tracking; each task ends in a green commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md` — the diff is reviewed against this task's **Files** and
> **Interfaces** blocks, and every finding is actioned, before the commit. No subagents elsewhere: this plan runs inline, in one
> session, by one pair of hands.

**Goal:** `F3` or the caret opens a card for the selected device — its name, its role, the ports it hears, the channel it
listens on, its MUTE, and a removal that asks twice — and `+` opens a picker that offers every `DeviceKind` there is, with one
honest footnote about what is not saved yet.

**Architecture:** one new card module in the shape of `ui/settings.rs` and `ui/songs.rs`: `card_rect`/`rows` are pure geometry a
test can click, `paint` draws, and every answer goes out on `PanelState::device_request` for `main` to apply. The card holds no
device state of its own except what is being typed: the device's name, its role and its route are read from the rack by `main`
into the card's mirror on open, and written back as requests.

**Tech Stack:** Rust 2024, egui/eframe 0.37 (`TextEdit` for the name, the same widget the songs card uses), no new dependency.

**Spec:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — read "Routing" (the device card paragraph) and "What is
deliberately not saved" before starting.

## Global Constraints

Inherited from `plans/phase 2/00-phase-2.md`. This plan is the one that can break the card conventions:

- **One card at a time.** Escape closes the device card before anything else; opening the device card closes help, songs and
  settings; opening the picker closes the device card. The rule already lives in `ui::dispatch::escape` for the other two —
  extend it there, not in a second place.
- **A message is a notice, not a dialog.** The footnote is a line on the card, and a refusal ("four devices is the limit")
  is `PanelState::notify`.
- **No font glyphs.** The caret is two strokes; the channel stepper's arrows are triangles drawn as three points, or the words
  `-` and `+` in the bundled font — choose the words, they are ASCII and unambiguous.
- **Names are the store's job.** `Rack::rename` trims, falls back and de-duplicates (plan 38, task 4); the card passes what was
  typed and displays what came back.
- **A request is not panel state.** Opening a card asks `main` for the device's current name, role and route, because the panel
  does not own the rack.

## Review Focus

1. **A rename that appears to do nothing** because the card shows the typed string and not what `Rack::rename` returned.
   `BASS` typed twice must show `BASS (2)` after the click. Task 3.
2. **A port list that loses ticks when the card is reopened.** The ticks come from the device's `Ports::Named`, not from the
   widget's own memory. Task 4.
3. **A removal that takes the wrong device** because the card's `open: Option<usize>` is stale after an earlier removal in the
   same frame. Task 5.
4. **A channel that reads one too low.** The store is `0..=15`; the card shows `1..=16`. Task 4.
5. **A footnote nobody reads,** so a player loses a name on restart without ever being told. Task 6.

---

### Task 1: The request queue grows

**Files:**
- Modify: `rust/crates/acid-app/src/device.rs` (`Request`, and `Rack::rename`'s callers)
- Modify: `rust/crates/acid-app/src/main.rs` (the drain `match`)
- Test: `rust/crates/acid-app/tests/rack.rs`

**Interfaces:**
- Produces:
  ```rust
  pub enum Request {
      Select(usize),
      SetMute(usize, bool),
      Open(usize),            // open the device card on this device
      Close,                  // close whichever card is open
      Add(DeviceKind),
      OpenPicker,
      Rename(usize, String),
      SetRole(usize, crate::midi::route::Role),
      SetPorts(usize, crate::midi::route::Ports),
      SetChannel(usize, crate::midi::route::Channel),
      AskRemove(usize),
      ConfirmRemove(usize),
  }
  ```

- [ ] **Step 1: Write the failing test** in `tests/rack.rs`:

```rust
#[test]
fn a_rename_request_is_the_racks_rename() {
    // The card types a name and sends it; the rack decides what the name becomes.
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.rename(0, "BASS");
    assert_eq!(rack.rename(1, "BASS"), "BASS (2)");
    assert_eq!(rack.devices()[1].name, "BASS (2)");
}
```

- [ ] **Step 2: Run it**

Run: `cd rust && cargo test -p acid-app --test rack`
Expected: PASS if plan 38 task 4 is done — this is a check, not a red test. Then add the enum variants and let the compiler
point at `main`'s drain `match`, which is written out in full: one arm per variant. Three kinds of arm exist in this task:

- **answered by the rack now**: `Select`, `SetMute`, `Add`;
- **routing edits**: `SetRole`, `SetPorts`, `SetChannel` — read the device's `inputs`, replace one field, `set_inputs`;
- **waiting for the card**: `Open`, `Close`, `OpenPicker`, `AskRemove`, `ConfirmRemove`, and `Rename`'s write-back — these get
  an arm that does nothing with the comment `// plan 42 task N fills this`. Not `todo!()`: a panic is waiting for a click, and
  this commit ships.

- [ ] **Step 3: Add the arms that have a rack answer**

`Rename` → `let landed = self.rack.rename(index, &typed);` and, if the card is open on that device, set the card's name field to
`landed` so the panel shows the store's answer (Review Focus 1). `SetRole`/`SetPorts`/`SetChannel` → read the device's current
`inputs`, replace the one field, `rack.set_inputs(index, updated)`. `ConfirmRemove` → `rack.remove(index)`, with
`RemoveError::LastDevice` becoming a notice and `selected` following in the panel mirror next frame.

- [ ] **Step 4: Run the tests and the gate**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "feat(device): the request queue names every answer a card can give"
```

---

### Task 2: The card's shell

**Files:**
- Create: `rust/crates/acid-app/src/ui/device_card.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod device_card;`, `PanelState::device_card`)
- Modify: `rust/crates/acid-app/src/ui/strip.rs` (a caret on the selected cell)
- Modify: `rust/crates/acid-app/src/ui/shortcuts.rs` (`Shortcut::ToggleDeviceCard` on `F3`)
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (toggle it, and the one-card rule)
- Test: `rust/crates/acid-app/tests/device_card_live.rs` (new file)

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, Default)]
  pub struct Card {
      pub open: Option<usize>,     // the device this card is about
      pub name: String,            // what is in the text field
      pub role: midi::route::Role,
      pub ports: Vec<(String, bool)>,   // every port the machine reports, and whether this device hears it
      pub channel: midi::route::Channel,
      pub armed_remove: bool,
  }
  impl Card {
      /// Fill the card from a device: its name, role and route. `main` calls this when it opens the card.
      pub fn offer(&mut self, name: &str, inputs: &midi::route::InputFilter, ports: Vec<String>);
  }
  pub fn card_rect(panel: egui::Rect, s: f32) -> egui::Rect;
  pub struct Rows { pub name: egui::Rect, pub role: egui::Rect, pub ports: Vec<egui::Rect>, pub channel_down: egui::Rect, pub channel_up: egui::Rect, pub mute: egui::Rect, pub remove: egui::Rect, pub close: egui::Rect }
  pub fn rows(card_area: egui::Rect, s: f32, card: &Card) -> Rows;
  pub fn paint(ui: &mut egui::Ui, state: &mut PanelState, panel: egui::Rect, s: f32);
  ```
  `ui/strip.rs` also produces `caret_rect(cell: egui::Rect) -> egui::Rect` for the selected cell, drawn as two strokes.

- [ ] **Step 1: Write the failing test** in `tests/device_card_live.rs`:

```rust
//! The device card: F3 opens it, Escape closes it, and every row is a request.

use acid_app::state::SharedState;
use acid_app::ui::{PanelState, dispatch, draw_panel};
use eframe::egui::{Context, Event, Key, Pos2, RawInput, Rect, vec2};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
}

fn run(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput { screen_rect: Some(screen()), events, ..Default::default() };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

fn two_device_panel() -> PanelState {
    let mut panel = PanelState::default();
    panel.strip.cells = vec![
        acid_app::ui::strip::StripCell { name: "ACID 1".into(), running: false, muted: false },
        acid_app::ui::strip::StripCell { name: "BASS".into(), running: false, muted: false },
    ];
    panel
}

#[test]
fn f3_opens_the_selected_devices_card_and_escape_closes_it() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let mut bank = acid_core::bank::PatternBank::new_empty();
    dispatch::handle(
        dispatch::KeyPress { key: Key::F3, shift: false, ctrl: false },
        &mut panel,
        &state,
        &mut bank,
    );
    assert_eq!(panel.device_card.open, Some(1), "F3 opens the card of the device on the panel");

    dispatch::escape(&mut panel, &state);
    assert_eq!(panel.device_card.open, None);
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: FAIL — `no module named device_card`.

- [ ] **Step 3: Write the shell**

`card_rect` is centred over the panel, wide enough for a name and a port list, and as tall as the room the rows are laid out in
— the face `paint` draws is as tall as the rows actually reach, so a card with two ports is shorter than the rect. `rows`
returns one rect per row; the ports list has one rect per port and is allowed to grow, because there are never more than a
handful. `paint` draws the shade (`theme::SHADE`) over the panel, the card face, the title `DEVICE <name>`, the rows, and
`CLOSE`. It writes `Request::Close` and nothing else: CLOSE and a click outside the card are its own lifetime, and the four
rows that change a device wait for tasks 3-5. Leaving `Close` unsent would make it a variant nothing sends and a
permanently dead arm in `main`'s drain — as it is, `main` answers it by dropping the mirror, so the card cannot show a name
that belongs to another device.

`F3` goes through `dispatch::handle` the way `F2` does: `shortcuts::apply` does not touch the panel, so the toggle lives in
`dispatch`, next to the settings toggle, and honours the one-card rule. The caret on the selected cell writes
`Request::Open(selected)` — a one-line addition in `strip::paint`.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test device_card_live && cargo test -p acid-app --test devices_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "feat(ui): F3 opens a card for the device on the panel"
```

Step 2 also lands three files the block above does not name, and the reason is in the Progress line: `main.rs` (the `Open`
answer and the rising-edge fill, without which the caret is a dead key), `ui/tips.rs` (`Control::DeviceCaret`, because every
hoverable control in this repo has a tip), and `ui/help.rs` (one line, so the help card still names every key).

---

### Task 3: The name field

**Files:**
- Modify: `rust/crates/acid-app/src/ui/device_card.rs`
- Modify: `rust/crates/acid-app/src/main.rs` (`device::Request::Rename` arm: write the answer back into the card)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` and `rust/crates/acid-app/src/ui/selection.rs` (a focused text field owns the
  keyboard: the audition keys and the step editor must not read a letter typed into the name as a note)
- Test: `rust/crates/acid-app/tests/device_card_live.rs`

**Interfaces:**
- Produces: the `name` row draws an `egui::TextEdit` bound to `card.name`, and **Enter** — and only Enter — writes
  `Request::Rename(open, card.name.clone())`. Not focus loss: `device_request` is one slot, so a click that dismisses the card
  cannot carry both a Rename and a Close, and a dismissal that silently renames cannot be taken back. (The ruling below said
  "or focus loss" on the premise that the songs card behaves that way; it does not — it reads its field when SAVE is clicked.
  The field keeps the typed text while the card stands, and the caption says `NAME (ENTER)`.) This task's test adds
  `use acid_app::device::Request;` and `use acid_app::ui::device_card;` to the test file's imports, and a `click` helper copied
  from `tests/devices_live.rs` — an integration test may not import another test file's helpers, and a shared
  `tests/common/mod.rs` is a bigger change than the duplication.
`draw_panel` calls `device_card::paint(ui, state, l.panel, s)` at the very end, after every row and every other card: a device
card is the last thing painted, so nothing can draw over it.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn typing_a_name_and_confirming_asks_the_rack_to_rename_the_device() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    panel.device_card.name = "BASS".into();
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let rows = device_card::rows(
        device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, acid_app::ui::layout::scale(screen())),
        acid_app::ui::layout::scale(screen()),
        &panel.device_card,
    );
    // Clicking the field, typing and pressing Enter: the card asks for the name it now holds.
    click(&ctx, &mut panel, &state, rows.name.center());
    run(&ctx, &mut panel, &state, vec![Event::Text(" 2".into())]);
    run(&ctx, &mut panel, &state, vec![Event::Key {
        key: Key::Enter, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::default(),
    }]);
    assert_eq!(panel.device_request, Some(Request::Rename(1, "BASS 2".into())));
}
```

Reuse `devices_live.rs`'s `click` helper by copying it — an integration test may not import another test file's helpers, and a
shared `tests/common/mod.rs` is a bigger change than the duplication.

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: FAIL — nothing is requested.

- [ ] **Step 3: Draw the field and ask**

`egui::TextEdit::singleline(&mut card.name)` in `rows.name`, with `ui.scope_builder` clipped to the card. Confirm on
`response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))` — the idiom the songs card does *not* use, so it is this card's own. In `main`'s
`Rename` arm, write the landed name back:

```rust
device::Request::Rename(index, typed) => {
    let landed = self.rack.rename(index, &typed);
    if self.panel.device_card.open == Some(index) {
        self.panel.device_card.name = landed;
}
```

Two corrections to the sketch above, both settled in task 1 and recorded in the ruling table: there is **no**
`self.panel.dirty = true` (names are not in the save format, so a rename changes no byte on disk), and `self.notify` is
`self.notice`. The songs card does **not** confirm its name on Enter — it reads the field when SAVE is clicked — so the
`lost_focus()` idiom is this card's own and must not be justified by citing that one.

The panel's strip mirror picks the new name up on the next frame, because `main` fills it from the rack.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "feat(ui): a device can be renamed from its card"
```

---

### Task 4: The routing rows

**Files:**
- Modify: `rust/crates/acid-app/src/ui/device_card.rs`
- Modify: `rust/crates/acid-app/src/main.rs` (open the card with the machine's port list)
- Test: `rust/crates/acid-app/tests/device_card_live.rs`

**Interfaces:**
- Produces: three request-shaped rows. `role` toggles between `PlayAndControl` and `ControlOnly` and asks
  `Request::SetRole(open, next)`; each port row toggles a name in or out of `Ports::Named` and asks
  `Request::SetPorts(open, next)`; `channel_down`/`channel_up` step through `Channel::All` then `One(0..=15)` and ask
  `Request::SetChannel(open, next)`.
  `Ports::All` is offered as the first row, labelled `ALL PORTS`, so a player can get back to the factory behaviour.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_port_row_ticks_and_unticks_and_asks_for_the_new_set() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    panel.device_card.ports = vec![("Launchkey MIDI".into(), true), ("nanoKONTROL".into(), false)];
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, s), s, &panel.device_card);
    click(&ctx, &mut panel, &state, rows.ports[1].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(1, acid_app::midi::route::Ports::Named(vec![
            "Launchkey MIDI".into(),
            "nanoKONTROL".into(),
        ]))),
        "ticking a port adds it to the set the device already hears"
    );
}

#[test]
fn the_channel_stepper_walks_from_any_channel_into_one_two_three() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.channel = acid_app::midi::route::Channel::All;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, s), s, &panel.device_card);
    click(&ctx, &mut panel, &state, rows.channel_up.center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetChannel(0, acid_app::midi::route::Channel::One(0))),
        "past ANY comes channel 1, which the store calls 0"
    );
}

#[test]
fn the_role_row_switches_between_playing_and_controlling() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_card.open = Some(0);
    panel.device_card.role = acid_app::midi::route::Role::PlayAndControl;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, s), s, &panel.device_card);
    click(&ctx, &mut panel, &state, rows.role.center());
    assert_eq!(panel.device_request, Some(Request::SetRole(0, acid_app::midi::route::Role::ControlOnly)));
}
#[test]
fn a_device_that_hears_nothing_is_given_one_port_at_a_time() {
    // Review Focus 1 of plan 40, at the level a player meets it: a device added by `+` arrives silent, so its card opens
    // with every port unticked, and the first port it is given is exactly that one — never "all of them".
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    panel.device_card.ports = vec![
        ("Launchkey MIDI".into(), false),
        ("nanoKONTROL".into(), false),
    ];
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(
        device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, s),
        s,
        &panel.device_card,
    );
    click(&ctx, &mut panel, &state, rows.ports[0].center());
    assert_eq!(
        panel.device_request,
        Some(Request::SetPorts(
            1,
            acid_app::midi::route::Ports::Named(vec!["Launchkey MIDI".into()])
        )),
        "a new device hears one port because it was given one, not because it hears everything"
    );
}
```

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: FAIL — nothing is requested.

- [ ] **Step 3: Draw the rows**

Two private helpers carry the arithmetic — `next_channel(current: Channel) -> Channel` (`All → One(0)`, `One(n) → One((n + 1) %
16)`) and its *down* partner — and the three tests above are what prove them, because the row's click is the only thing that
matters. The store is `0..=15`; the label is `format!("CH {}", n + 1)` or `CH ANY`. Ports: `Ports::All` means every port is
ticked, and a port row clicked from `All` produces `Named(everything_except_that_one)` — the honest reading of "I do not want
this one".

When `main` opens a card (`Request::Open`), it calls `Card::offer(name, &device.inputs, midi::ports::list_ports().unwrap_or_default())`
so the list is the machine's, read on the click rather than per frame. The device's own filter is what decides which rows are
ticked, so a device added by `+` opens with every row unticked: that is Review Focus 1 of plan 40, and this card is where a
player can finally see it.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/device_card.rs rust/crates/acid-app/src/main.rs rust/crates/acid-app/tests/device_card_live.rs
git commit -m "feat(ui): a device's ports, channel and role live on its card"
```

---

### Task 5: Removing a device, twice

**Files:**
- Modify: `rust/crates/acid-app/src/ui/device_card.rs`
- Modify: `rust/crates/acid-app/src/main.rs` (the two remove arms)
- Test: `rust/crates/acid-app/tests/device_card_live.rs`

**Interfaces:**
- Produces: `remove` asks `Request::AskRemove(open)` on the first click and `Request::ConfirmRemove(open)` on the second, with
  `card.armed_remove` as the switch — the same shape as `PanelState::clear_armed_at`, and disarmed by anything else the card
  does. `Rack::remove`'s `LastDevice` refusal becomes a notice.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_remove_key_asks_twice_before_it_asks_the_rack() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.strip.selected = 1;
    panel.device_card.open = Some(1);
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let rows = device_card::rows(device_card::card_rect(acid_app::ui::layout::layout(screen()).panel, s), s, &panel.device_card);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(panel.device_request, Some(Request::AskRemove(1)));
    assert!(panel.device_card.armed_remove, "and the card says so");

    panel.device_request = None;
    run(&ctx, &mut panel, &state, vec![]);
    click(&ctx, &mut panel, &state, rows.remove.center());
    assert_eq!(panel.device_request, Some(Request::ConfirmRemove(1)));
}
```

- [ ] **Step 2: Run to watch it fail**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: FAIL — `no field armed_remove`, or nothing is requested.

- [ ] **Step 3: Arm, confirm, and refuse the last one**

The armed key reads `REMOVE` then `CONFIRM?` in `theme::LED_ACCENT` while armed, goes unarmed after `PanelState::NOTICE_FOR`,
and any other click on the card disarms it. `main`'s `ConfirmRemove` arm:

```rust
device::Request::ConfirmRemove(index) => {
    match self.rack.remove(index) {
        Ok(()) => {
            self.panel.device_card.open = None;
            self.panel.dirty = true;
        }
        Err(device::RemoveError::LastDevice) => self.notice("the last device cannot go".to_string()),
        Err(device::RemoveError::OutOfRange) => {}
    }
}
```

A removal does not stop the app, does not touch the file, and does not renumber anything — the survivors keep their names
(plan 38, task 4).

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test device_card_live && cargo test -p acid-app --test devices_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add rust/crates/acid-app/src/ui/device_card.rs rust/crates/acid-app/src/main.rs rust/crates/acid-app/tests/device_card_live.rs
git commit -m "feat(ui): removing a device asks twice, and the last one stays"
```

---

### Task 6: The picker, and the footnote

**Files:**
- Modify: `rust/crates/acid-app/src/ui/device_card.rs` (`Picker` and its rows)
- Modify: `rust/crates/acid-app/src/ui/strip.rs` (`+` asks `Request::OpenPicker` instead of `Request::Add`)
- Modify: `rust/crates/acid-app/src/main.rs` (the picker arms, and the fold of `Add` into it)
- Test: `rust/crates/acid-app/tests/device_card_live.rs`, and one changed expectation in `tests/devices_live.rs`

**Interfaces:**
- Produces:
  ```rust
  #[derive(Clone, Debug, Default)]
  pub struct Picker { pub open: bool }
  pub fn picker_rect(panel: egui::Rect, s: f32) -> egui::Rect;
  pub struct PickerRows { pub kinds: Vec<egui::Rect>, pub close: egui::Rect }
  pub fn picker_rows(area: egui::Rect, s: f32) -> PickerRows;   // one row per DeviceKind::ALL
  pub fn paint_picker(ui: &mut egui::Ui, state: &mut PanelState, panel: egui::Rect, s: f32);
  ```
  `PanelState::device_picker: Picker` — not `picker`, which is already the MIDI/output choice queue. `DeviceKind` gains
  `enabled(self, count: usize) -> bool` — false at `MAX_DEVICES` — so the picker greys its rows for the same reason the strip
  greys `+`, from one rule rather than two.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn the_picker_offers_every_kind_and_says_what_is_not_saved() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_picker.open = true;
    let ctx = Context::default();
    let output = run_capture(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(acid_app::ui::layout::layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    assert_eq!(rows.kinds.len(), acid_app::device::DeviceKind::ALL.len());

    let said = strings(&output).join(" ");
    assert!(said.contains("ACID"), "every kind is named: {said}");
    assert!(
        said.contains("not saved"),
        "the card must say that extra devices and names do not survive a restart: {said}"
    );
}

#[test]
fn a_picker_row_asks_for_that_kind() {
    let state = SharedState::new();
    let mut panel = two_device_panel();
    panel.device_picker.open = true;
    let ctx = Context::default();
    run(&ctx, &mut panel, &state, vec![]);

    let s = acid_app::ui::layout::scale(screen());
    let area = device_card::picker_rect(acid_app::ui::layout::layout(screen()).panel, s);
    let rows = device_card::picker_rows(area, s);
    click(&ctx, &mut panel, &state, rows.kinds[0].center());
    assert_eq!(panel.device_request, Some(Request::Add(acid_app::device::DeviceKind::Acid)));
}
```

`strings` and `run_capture` are the `settings_live.rs` helpers under another name: a `FullOutput` walker collecting every
`Shape::Text`, and a `run` that returns the output. Copy them.

- [ ] **Step 2: Run to watch them fail**

Run: `cd rust && cargo test -p acid-app --test device_card_live`
Expected: FAIL — `no field picker`.

- [ ] **Step 3: Draw the picker, and move `+` onto it**

`picker_rows` derives its count from `DeviceKind::ALL`, so a second kind is a variant and a row. The footnote is one line in
`theme::SILKSCREEN_DIM`: `NAMES AND EXTRA DEVICES ARE NOT SAVED YET`. That sentence is the promise the plan keeps — it is
short, it is true, and it disappears in the round that adds `SavedState::devices`.

In `strip::paint`, `+` writes `Request::OpenPicker`; `main`'s `OpenPicker` arm sets `panel.picker.open = true` and closes the
device card. `tests/devices_live.rs`'s `the_add_key_asks_for_the_only_kind_of_device_there_is` changes its expectation to
`Request::OpenPicker` — that is the one test this task rewrites, and it is a behaviour change, not a rename.

- [ ] **Step 4: Run the tests**

Run: `cd rust && cargo test -p acid-app --test device_card_live && cargo test -p acid-app --test devices_live`
Expected: PASS.

- [ ] **Step 5: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "feat(ui): the add key opens a picker, and the picker says what is not saved"
```

---

### Task 7: The gate, and the journey by hand

**Files:**
- Test: the workspace

- [ ] **Step 1: Run the three gates**

```bash
cd rust && cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Expected: PASS, all three.

- [ ] **Step 2: Walk the journey the spec claims**

`cargo run -p acid-app --bin takka-acid`, then:

1. `+` → the picker → the `ACID` row. A second cell appears, selected, and hears nothing.
2. `F3` on it: name it `BASS` — then again `BASS`, which must land as `BASS (2)`, because Review Focus 1 is a store answer and
   nothing else proves it — switch it to `CTRL` (it stops playing notes), tick the controller's port only.
3. Play a key: only `ACID 1` sounds. Turn the controller's cutoff knob: `BASS`'s cutoff moves, `ACID 1`'s does not.
4. `Ctrl+1`, `Ctrl+2` to move between them; edit a step on each; confirm they are separate patterns.
5. Mute `BASS`, unmute it mid-bar, and confirm it is in time rather than restarting.
6. `F3` → `REMOVE` → `CONFIRM?`. The cell goes; `ACID 1` keeps its number and its name.
7. Restart. One device, called `ACID 1` or whatever device 1 was called: names and extra devices are gone, exactly as the
   footnote said.

Record anything that surprised you in `## Progress`.

- [ ] **Step 3: Review the diff, action it, then commit** (see the phase overview review gate)

```bash
git add -A
git commit -m "chore(ui): plan 42's gate, and the journey walked"
```

---

## Progress

- [x] Task 1 — the request queue grows. `9f4193c`: `device::Request` carries all twelve answers a card can give
      (`Open`, `Close`, `OpenPicker`, `Rename`, `SetRole`, `SetPorts`, `SetChannel`, `AskRemove`, `ConfirmRemove` added), and
      `main`'s drain answers the ones that have a rack answer: `Rename` → `Rack::rename`, the three routing edits each
      replacing one field of the device's `InputFilter` and publishing it, `ConfirmRemove` → `Rack::remove` with
      `LastDevice` as a notice. `Open`/`Close`/`OpenPicker`/`AskRemove` and Rename's write-back are marked stubs, not
      `todo!()`. **Review: fresh `reviewer` subagent.** Four findings, all actioned: (1) P1 — `ConfirmRemove` marked the
      session dirty for *every* removal, so removing a non-first device flipped the SAVE lamp and triggered an autosave
      write for a file that would not change; now `if index == 0`, because a save writes the first device and only losing
      that one changes what the file would hold. (2) P1 — the commit's only new behaviour is `main`'s drain, which a binary
      cannot be tested from, so this entry is the record AGENTS.md asks for (see "Deliberately untested" below); the
      reviewer also confirmed the dropped Step 1 test was a strict subset of `tests/rack.rs`'s
      `renaming_lands_on_one_device_and_keeps_names_distinct`. (3) `apply_device_request`'s doc comment still said "select a
      device, mute one, or add one". (4) the `Rename` arm's deferred write-back was unmarked. Task 1 adds **no test**:
      `tests/rack.rs` already pins the rename chain, and the new arms are reachable only from the binary.
- [x] Task 2 — the card's shell. `ui/device_card.rs` (Card/`offer`/`card_rect`/`rows`/`paint`), `PanelState::device_card`
      with `open_device_card`/`close_device_card` as the only way in and out, `F3` → `Shortcut::ToggleDeviceCard` in
      `dispatch`, the caret on the strip's selected cell → `Request::Open`, and the card painted last in `draw_panel`.
      **Review: fresh `reviewer` subagent.** Nine findings (3 should-fix, 6 nits), all actioned:
      (1) **the one-card rule was one-way** — F3 closed settings/help/songs but F2/F1 left the device card standing, which is
      two shades over one face and a live card hidden under another two keystrokes away; both arms now call
      `close_device_card`, and `opening_another_card_puts_the_device_card_away` pins it.
      (2) **a click on the card's own rows dismissed it** — only CLOSE was registered after the shield, so a player aiming at
      MUTE (dead until task 4) lost the card; the face is now its own click target, pinned by
      `a_click_on_one_of_the_cards_own_rows_leaves_it_standing`.
      (3) the Progress and step-5 `git add` list (this entry).
      Nits: `caret_rect` wore the name's old doc line; `escape`'s new doc claimed "the paint order backwards" while help and
      songs were already inverted (the doc now says so rather than pretending); the help card did not name `F3`; two doc
      claims in the new file were not what the code does and one assertion pinned nothing (the test now bounds the key row by
      its neighbours); and `Default` on `Channel`/`Role` was reverted — `Card`'s default is written out, so `midi::route`
      gained no "the permissive one" default for a UI mirror.
      **Deviations from the task's Files block, all recorded above:** `main.rs` (`Open` → `open_device_card`, `Close` →
      drop the mirror, and the rising-edge fill of the card's mirror in `card_filled: Option<usize>`, the shape
      `settings_was_open` already uses, so F3 and the caret are one path and no blank card is ever painted), `midi/route.rs`
      (`Ports::holds`, so a tick and the router's filter cannot disagree — `hears` now delegates to it), `ui/tips.rs`
      (`Control::DeviceCaret`, appended at index 29 so no existing position moves), `ui/help.rs` (one line). `Rows` gained
      `all_ports`, which the plan's Interfaces block omits and task 4's `ALL PORTS` row needs. The port list passed to
      `offer` is `Vec::new()`: the machine's ports arrive with the rows that show them, which is task 4's `main.rs` line.
      **Deliberately untested:** `main`'s `Open`/`Close` arms and the fill (`card_filled`, the `armed_remove` reset) — the
      binary again, and task 7's journey is what walks them; the caret's *strokes* (its hit area is tested, so deleting the
      two `line_segment` calls would leave every test green — reading that back would mean teaching `devices_live.rs`'s
      `painted()` to collect `Shape::LineSegment`); and `paint`'s stale-index close, which `devices_live.rs` covers one
      layer up (`a_selection_past_the_end_is_clamped_rather_than_panicking`).
- [x] Task 3 — the name field. The `name` row is now a `TextEdit` in a display well (`theme::WELL` + silkscreen stroke, the
      idiom `ui::display` uses for the readouts), Enter asks `Request::Rename`, and `main` writes `Rack::rename`'s answer back
      into the field so a collision reads `BASS (2)` on the card (Review Focus 1). **Review: fresh `reviewer` subagent.**
      Eleven findings, all actioned — one of them a **BLOCK** the commit would not have shipped without:
      (1) **BLOCK — the field was not modal for the panel's global key readers.** Typing `BASS` with shift held wrote four
      notes into the selected step and dirtied the session, Backspace in STEP mode emptied a step, and Space stopped the
      transport, because `play_computer_keys` and `dispatch::presses` read raw input events regardless of focus. Both now
      ask egui's `text_edit_focused()` (not `wants_keyboard_input`, which is also true for a focused button), pinned by
      `typing_into_the_name_field_writes_no_step_and_stops_no_transport` — which fails when the guard is disabled, checked by
      mutation. This also fixes the songs card's field, which had the same hole.
      (2) **Enter only, not focus loss** — the plan's Interfaces line and its ruling row both said "loses focus or presses
      Enter"; both are rewritten above. `device_request` is one slot, so a click that dismisses the card cannot carry a
      Rename and a Close at once, and a dismissal that silently renames cannot be taken back. The plan's premise ("the songs
      card behaves the same way") is false, and that is now written into the plan instead of being inherited.
      (3) the caption reads `NAME (ENTER)`: a field has no tooltip, and the card is not dismissed by clicking away from the
      field, so nothing else tells a player what applies the name.
      Nits: a stale "plan 42 task 3" comment in `main`; the card title is now fitted with `ui::text::fit` (a name is never
      truncated in the store, so a long one spilled past the face); task 7's journey now renames a device `BASS` twice, which
      is the only thing that exercises the dedupe.
      **Files outside the plan's block:** `ui/mod.rs` and `ui/selection.rs` (the focus gate and the step editor's refusal),
      both added to the block above. **Deliberately untested:** `main`'s `Rename` write-back (`main.rs`, binary), named here as
      the plan's "Deliberately untested" paragraph asks — task 7's journey is what walks it; and `presses`' focus gate, which
      lives only in `main::logic` (the `draw_panel` half is pinned by the typing test).
- [ ] Task 4 — the routing rows
- [ ] Task 5 — removing a device, twice
- [ ] Task 6 — the picker and the footnote
- [ ] Task 7 — the gate and the journey

### Deliberately untested

`main`'s `apply_device_request` arms for `Rename`, `SetRole`, `SetPorts`, `SetChannel` and `ConfirmRemove` (task 1) cannot be
tested from this repo: `main` is a binary and cannot be called, which is true of every arm that was already there and is why
`ui::dispatch` and the rack live in the library. Three things stand in for a test, and each is named here rather than trusted:
every one of those arms is a two-line fold over a `Rack` method that `tests/rack.rs` and `midi/route.rs`'s own tests already
pin; tasks 2-6 assert the *requests* those arms receive through `draw_panel` in `tests/device_card_live.rs`; and task 7's
by-hand journey is the only thing that walks `main`'s answers — the rename landing as `BASS (2)`, the removal asking twice,
the last-device refusal. A reviewer of task 2-6 should say which arm looks unexercised rather than assume the journey covers
it.

Rulings made while writing this plan, and their cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| The card holds a *mirror* and `main` fills it on open | A device renamed from somewhere else while the card is open shows a stale name until it is reopened. Nothing else can rename it today. |
| `DeviceKind::enabled(count)` decides greying for both the strip's `+` and the picker's rows | Two rules for one question would be the bug AGENTS.md warns about; the cost is that `DeviceKind` now knows about a rack size. |
| `Ports::All` is a ticked row rather than a separate mode | A player who unticks every port gets `Named(vec![])`, which is "silent" — the same state a new device starts in, so it is reachable and explainable. |
| The footnote names both losses in one line | One more line on a small card. The alternative is a player losing a name with no warning, which the design calls an anti-goal. |
| The name field confirms on **Enter only** | Amended in task 3: the original row said "or focus loss" on the premise that the songs card behaves that way, and it does not — it reads its field when SAVE is clicked. A click that dismisses the card cannot also carry the rename in `device_request`, which is one slot. The cost if wrong: a player types a name and clicks the card's own face, and the field keeps the text but the rack keeps the old name until Enter — the caption says `NAME (ENTER)` for that reason. |
| A rename does not mark the session dirty | Names are not in the save format (`SavedState` has no device name), so `dirty` would light the SAVE lamp and trigger an autosave write for a file whose bytes do not change — the same reason `Add` does not dirty the session. The cost if wrong: a player renames a device, sees the lamp stay dark, and restarts expecting the name to have been kept. The picker's footnote is what tells them it was not. |
| A removal marks the session dirty only when it takes the **first** device | A save writes the first device, so removing any other changes nothing about what the file would hold. The cost if wrong: a player removes the third device, the lamp stays dark, and a restart brings the device list back — which is what the footnote already promises. |
