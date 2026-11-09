# Settings Card and a Quieter Top Strip — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`. One green commit per task, on
> `Deepseek-Version`.

**Goal:** move tooltips, autosave, MIDI in and sound out into one settings card opened by a gear and `F2`, remove both
device status lines and the `AUTO`/`TIPS` keys, centre the song's name in the top strip, and put the help `?` back.

**Architecture:** a fourth card over the panel (`ui/settings.rs`), the same shape as help and songs: pure painter, request
enum, dismissal by CLOSE / Escape / click outside. Device lists are inline; a click writes the same
`picker.input_choice` / `picker.output_choice` the status lines wrote before, so `main` keeps acting on them. The gear is
egui shapes in `ui/panel.rs`, next to a `?` that loses its accidental clip rect.

**Tech Stack:** Rust, egui (`eframe`), `cpal` (audio devices), `midir` (MIDI ports). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-11-09-settings-card-and-top-strip-design.md`

## Global Constraints

- **No font glyphs** (AGENTS.md): the gear is a ring, eight teeth and a hub, all `egui` shapes.
- The panel paints; `main` acts. No device or stream call inside `ui/`.
- `PickerState`'s choice fields keep their names and types: `input_choice: Option<midi::ports::PortChoice>`,
  `output_choice: Option<String>`.
- `SavedState` is untouched: no `SAVE_VERSION` bump, no new field.
- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
- One card at a time: opening settings closes help and songs; opening either closes settings.
- The top band is 56 design px tall (16..72 of 640); the song label may not overlap the wordmark, the saves lamp, the
  pattern display or the tempo display.

## Review Focus

- **A machine with no MIDI ports and no audio outputs:** the card must still open, say so plainly, and not panic.
- **A device list longer than the card:** rows must stay inside the card (scroll or truncate), not paint off its edge.
- **A device unplugged while the settings card is open:** the row that was lit is gone; clicking another must still work,
  and the stale current name must not panic anything.
- **Escape with settings open and a learn in progress:** settings closes first, the learn keeps going.
- **`F2` while the help card or songs card is open:** settings replaces it, in one keypress, and Escape then closes settings.

---

### Task 1: The top strip — gear, a `?` that reads, and the name in the centre

**Files:**
- Modify: `rust/crates/acid-app/src/ui/panel.rs` (`help_rect`, `gear_rect`, `song_label_rect`)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (draw the gear, paint `?` unclipped, centre the label)
- Test: `rust/crates/acid-app/tests/notice_live.rs`

**Interfaces:**
- Produces: `panel::help_rect(top: Rect, scale: f32) -> Rect`, `panel::gear_rect(top: Rect, scale: f32) -> Rect`,
  `panel::song_label_rect(top: Rect, scale: f32) -> Rect` (now centred). `gear_rect` sits 24 design px left of
  `help_rect`, same size and centre line — left of `?` so it never collides with the saves lamp.

- [ ] **Step 1: Write the failing test** in `tests/notice_live.rs`: `the_gear_sits_beside_the_help_dot_and_clear_of_the_lamp`
  asserting `gear_rect` does not intersect `help_rect`, `save_indicator_rect`, `brand_rect`'s text area, or
  `pattern_display_rect`; and `the_song_label_is_centred_in_the_strip` asserting the label's centre is the strip's centre
  within 1 px and it does not intersect the wordmark, the lamp, the pattern display or the tempo display.
- [ ] **Step 2: Run it** — `cargo test -p acid-app --test notice_live` — expect FAIL (`gear_rect` missing; old label
  assertion).
- [ ] **Step 3: Implement** the three rects in `panel.rs`; in `mod.rs` draw the gear (`circle_stroke` r=8, eight teeth as
  1.5 px radial strokes from r=8 to r=11 at 45° steps, hub `circle_filled` r=2), replace `help_dot` with
  `panel::help_rect(l.top, s)` and paint `?` with a plain `ui.painter()`, and draw the label with `Align2::CENTER_CENTER`
  on the new rect.
- [ ] **Step 4: Run** `cargo test -p acid-app --test notice_live` — expect PASS; then `cargo test --workspace`.
- [ ] **Step 5: Commit** — `feat(panel): a gear beside the help dot, and the song name in the centre`.

### Task 2: The settings card, and `F2`

**Files:**
- Create: `rust/crates/acid-app/src/ui/settings.rs`
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`pub mod settings;`, `PanelState::settings_open`, `settings: settings::Settings`, draw the card, gear click opens it)
- Modify: `rust/crates/acid-app/src/ui/shortcuts.rs` (`Shortcut::ToggleSettings`, `Key::F2`)
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (toggle `settings_open`, close help and songs when it opens)
- Modify: `rust/crates/acid-app/src/main.rs` (Escape order: settings, help, songs, learn)
- Test: `rust/crates/acid-app/tests/settings_live.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Settings {
      pub midi_ports: Vec<String>,      // every port midir can see, as names
      pub outputs: Vec<String>,         // every output cpal can see
      pub midi_current: Option<String>, // the port in use, for the lit row
      pub output_current: Option<String>,
      pub midi_given_up: bool,          // the supervisor has stopped retrying
  }
  pub enum Request { Tooltips, Autosave, MidiIn(String), SoundOut(String), Retry, Close }
  pub fn paint(ui: &mut Ui, panel: Rect, scale: f32, settings: &Settings, tooltips: bool, autosave: bool) -> Option<Request>
  ```
  `PanelState { pub settings_open: bool, pub settings: settings::Settings }`; `Shortcut::ToggleSettings` on `Key::F2`.
- Consumes: `transport::transport_button` for the toggle and device rows.

- [ ] **Step 1: Write the failing test** `tests/settings_live.rs`:
  `f2_opens_and_closes_the_settings_card`, `the_gear_opens_settings`,
  `a_toggle_row_asks_for_its_switch` (TOOLTIPS row → `Request::Tooltips`, AUTOSAVE → `Request::Autosave`),
  `a_midi_row_asks_for_that_port`, `a_sound_row_asks_for_that_output`,
  `retry_appears_only_while_the_supervisor_has_given_up`, and
  `opening_settings_closes_help_and_the_songs_card`.
- [ ] **Step 2: Run** — `cargo test -p acid-app --test settings_live` — expect FAIL (module missing).
- [ ] **Step 3: Implement** `ui/settings.rs` (card rect `panel.shrink(theme::MARGIN * scale)`, padding 12, row height 22,
  gap 2; heading rows 18; TOOLTIPS / AUTOSAVE on the first row pair, `MIDI IN` section, `RETRY` row while given up, `SOUND OUT`
  section, CLOSE bottom-right; truncate a device name with the existing `song_label`-style rule at 34 chars). Wire the
  module, `PanelState` fields, `F2`, dispatch and the Escape order.
- [ ] **Step 4: Run** `cargo test -p acid-app --test settings_live`, then `cargo test --workspace`.
- [ ] **Step 5: Commit** — `feat(settings): one card for tooltips, autosave and the two devices`.

### Task 3: `main` fills the card, and the two old pickers go

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (refresh lists on open; apply every `settings::Request`; delete the top MIDI picker and the bottom audio line)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (delete the MIDI status paint, the picker handle, `tips_rect`, `autosave_rect`, the `AUTO` and `TIPS` keys)
- Modify: `rust/crates/acid-app/src/ui/tips.rs` (drop `Control::Autosave`, `Control::Tooltips` and their `TIPS` rows)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PickerState` keeps only the two choice fields)
- Tests: `tests/tips_live.rs`, `tests/save_live.rs`, `tests/midi_picker_live.rs` — delete what the code removed, keep the rest

**Interfaces:**
- Consumes: `settings::Request` from Task 2; `midi::ports::list_ports() -> Result<Vec<String>>`, `audio::list_output_devices() -> Vec<String>`.
- Produces: `PanelState.settings` filled on the frame the card opens (rising edge of `settings_open`), a `main::settings_open`
  helper that fills and shows the card, and `Request::MidiIn` → `picker.input_choice = Some(PortChoice::Named(name))`,
  `Request::SoundOut` → `picker.output_choice = Some(name)`, `Request::Retry` → `reprobe_requested = true`.

- [ ] **Step 1: Write the failing test** in `tests/settings_live.rs`: `a_midi_row_writes_the_same_choice_the_status_line_used_to`
  (click a row on a panel whose `settings.midi_ports` has one port; assert `picker.input_choice == PortChoice::Named(...)`),
  matching test for `output_choice`, and `f2_with_help_open_replaces_help`.
- [ ] **Step 2: Run** — expect FAIL.
- [ ] **Step 3: Implement** the `main` wiring and delete the old entry points; trim `PickerState`.
- [ ] **Step 4: Run** `cargo test --workspace`, fix the three test files the removals touch.
- [ ] **Step 5: Commit** — `refactor(ui): the settings card owns both device pickers`.

### Task 4: The bottom line goes, and the documents follow

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (`fn ui`: the audio status text, the output list and its click handling go; a
  failed stream still reports through the status the panel shows)
- Modify: `rust/crates/acid-app/src/ui/help.rs` (key list gains `F2`; SCADA line for the settings card)
- Modify: `README.md` (key table gains `F2`; the settings sentence)
- Tests: `tests/notice_live.rs` (unaffected), plus any test that clicked the audio status line

- [ ] **Step 1: Write the failing test** in `tests/settings_live.rs`: `an_audio_failure_is_visible_in_the_settings_card`
  — set a failed `midi_status`/audio status, open the card, assert the status string appears in the card's SOUND OUT line
  through the same pure helper the card paints.
- [ ] **Step 2: Run** — expect FAIL.
- [ ] **Step 3: Implement** the removal and the copy.
- [ ] **Step 4: Run** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
- [ ] **Step 5: Commit** — `refactor(ui): no device text on the face, and the keys documented`.

## Non-goals

- No change to the SONGS card, the save model, the song format, or the transport.
- No new device backends; no per-device memory beyond what `SavedState` already carries.

## Progress

- [x] **Task 1 — the top strip.** `panel::help_rect` (the `?` dot, moved out of `mod.rs`), `panel::gear_rect` (its twin,
      one step left), `song_label_rect` centred between the wordmark and the pattern display, and `save_indicator_rect`
      shifted left so the cluster reads l-to-r: **lamp · gear · ?**. The gear is drawn by `paint_gear` — ring, eight
      teeth, hub — because the face may not use font glyphs. The `?` is painted with a plain painter again, which is the
      regression fix: my earlier clip-to-the-status-band edit had clipped the help dot out of existence.
      Tests: `notice_live` 6 pass (`the_gear_sits_beside_the_help_dot_and_clear_of_the_lamp`,
      `the_song_label_is_centred_in_the_strip`, and the label-clearance test rewritten for the new position).
- [ ] **Task 2 — the settings card and `F2`.**
- [ ] **Task 3 — `main` fills the card; the two old pickers go.**
- [ ] **Task 4 — the bottom line goes, and the documents follow.**
- [x] **Task 2 — the settings card and `F2` (code landing; tests still to write).** `ui/settings.rs`: `Settings` (port
      lists, current devices, supervisor words), `Request::{Tooltips, Autosave, MidiIn(Option<String>), SoundOut, Retry,
      Close}`, `card_rect`/`rows` as the single layout a test can click, and `paint`. Wired: `PanelState::{settings_open,
      settings, settings_request}`, `Shortcut::ToggleSettings` on **F2** (dispatch toggles it and clears help and songs),
      the gear opens it, `draw_settings` paints it over the panel with its own shield, Escape closes it before anything
      else, and `main` fills the lists on the rising edge (never per frame) and applies every request through the same
      `picker.input_choice` / `output_choice` the status lines used.
      **Not yet written:** `tests/settings_live.rs` (F2, gear, each row's request, retry visibility, an empty machine).
- [ ] **Task 3 — `main` fills the card; the two old pickers go.** Remaining: the top MIDI status text and its handle, the
      `AUTO`/`TIPS` keys, `ui::tips_rect`/`autosave_rect`, the `Control::{Autosave, Tooltips}` tips rows, and the bottom
      audio line with its output list. `tips_live`, `save_live` and `midi_picker_live` each lose the tests that clicked
      the removals.
- [ ] **Task 4 — the documents follow.**
- [x] **Task 2b — the device lists fold and scroll.** `Settings::{midi_open, sound_open}` (both start collapsed),
      `Request::{ToggleMidi, ToggleSound}`, `rows()` now returns the fold headers and the two panel rects, and an open panel
      is at most `VISIBLE_ROWS` (4) rows tall: whatever does not fit scrolls in an `egui::ScrollArea` clipped to the panel.
      Each section names the device in use above it — `IN USE: <name>` — so the answer is readable while the list is shut,
      and the fold mark is ASCII (`[+]` / `[-]`) because the face may not use font glyphs. This is the fix for a machine
      whose SOUND OUT list was taller than the card.
- [x] **The fold click closed the card (bug, fixed).** The dismissal shield was registered *after* the card's controls, and a
      later widget wins a hit-test tie — so every click inside the card was read as a click outside it. The shield is now
      registered first, and the card's own rows win. The two folds are also applied by the card itself rather than sent to
      `main` and back, because the folded/shut state is panel state and a round trip only made the click look like nothing
      happened.
      Tests: `tests/settings_live.rs` (6) — folding a panel open and shut keeps the card standing, a click outside still
      dismisses, the two switches ask for their flips, the sections fold independently, and an open panel never grows past
      four rows however long the list is.
- [x] **Task 3 — the old furniture is gone.** Deleted: the `AUTO` and `TIPS` keys (and `ui::tips_rect`,
      `ui::autosave_rect`), the top MIDI status readout with its click-to-pick list and REPROBE dot, the bottom audio line
      with its output list, and `PickerState::{inputs, outputs}` — what is left of it is the two choices `main` consumes.
      The middle of the strip now says one thing at a time: the song's name, or a notice for its four seconds. A failed
      audio stream or a MIDI supervisor that has given up reports in the settings card, where both devices live.
      Tests: `tests/midi_picker_live.rs` deleted with its subject (its coverage moved to `settings_live`), `tests/tips_live.rs`
      keeps the help-on-by-default and the readouts-answer-the-pointer tests, and `songs_live` now asserts on the picker's
      choice fields rather than its lists. 59 suites green.
- [ ] **Task 4 — the documents follow.** `help.rs` and `README.md` still need `F2` and the settings card in the key lists,
      and `settings_live` should gain the two device-row tests (a click writes `picker.input_choice` / `output_choice`),
      which `midi_picker_live` used to prove against the status line.
