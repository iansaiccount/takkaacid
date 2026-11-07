# MIDI Input Selector Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let the player see which MIDI input ports exist, choose one (or "any"), and have that choice survive a restart.

**Architecture:** The choice becomes a value — `midi::ports::PortChoice { Any, Named(String) }` — resolved by one pure function, so "an explicit port never silently becomes a different port" is a property of `ports` rather than of the UI. The panel carries the port list and a pending choice exactly as it already carries the audio output list; `main.rs` consumes the choice and reopens the connection; `persist` stores the name.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `midir` 0.11, `serde`; tests run through `cargo test`, the picker through headless egui frames.

**Spec:** this plan — the scope decisions below are the spec, since no separate design doc exists.

## Scope decisions

- **`Any` is the default and keeps today's behaviour**: the first port the system reports, retried until it appears.
- **An explicit port is strict**: exact name, then a case-insensitive substring, and if neither matches it fails with a message. It never falls back to a different port.
- The picker reuses `ui::picker::should_close` and the audio picker's interaction rules: click the status line to open, click a name to choose, click elsewhere to dismiss.
- No new crates and no file dialog: the list comes from `midi::ports::list_ports`.

## Global Constraints

- Nothing may block the UI or audio thread: opening a port stays on the spawned probe thread.
- The computer keyboard keeps playing whatever the MIDI choice is; MIDI shares the one voice (`AGENTS.md`, `plans/10-midi.md`).
- Panel legends are ASCII only; the status line keeps using `midi::supervisor::status_line`.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` stay clean; `tests/no_alloc.rs` stays green.
- The save file gains one optional field with a serde default, so files written before this change still load.

## Review Focus

1. **The chosen port vanishes while playing.** The connection drops, the status line says so, the supervisor keeps retrying — and the app never connects a *different* port on its own.
2. **Two interfaces attached, "any" chosen.** The first port is used, exactly as before this change.
3. **A session saved with a port that is not plugged in.** The app still opens and plays from the computer keyboard; the status line names the missing port; no panic and no retry storm.
4. **Opening the picker with no MIDI ports at all.** The list holds only the "any" row, and the REPROBE dot stays reachable.
5. **Choosing a port while the transport runs.** Audio keeps running untouched; notes from the new port arrive after the reopen, notes from the old one stop.

---

### Task 1: `PortChoice` and strict resolution

**Files:**
- Modify: `rust/crates/acid-app/src/midi/ports.rs`
- Test: `rust/crates/acid-app/src/midi/ports.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub enum PortChoice { Any, Named(String) }` with `#[derive(Clone, Debug, PartialEq, Eq, Default)]` and `Any` as the default.
  - `impl PortChoice`: `pub fn name(&self) -> Option<&str>`, `pub fn resolve(&self, names: &[String]) -> Option<usize>`, `pub fn label(&self, names: &[String]) -> String` (the status wording: the port's own name for `Any`, the requested name otherwise).
  - `MidiIn::open(choice: &PortChoice, handler: impl FnMut(u64, &[u8]) + Send + 'static) -> Result<Self>` — replaces the `Option<&str>` parameter.
  - `pick_port(names, wanted: Option<&str>)` stays, implemented through `PortChoice`, so the two cannot drift.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn any_takes_the_first_port_and_a_name_takes_its_own() {
    let names = vec!["Arturia KeyStep".to_string(), "Launchkey MIDI".to_string()];
    assert_eq!(PortChoice::Any.resolve(&names), Some(0));
    assert_eq!(PortChoice::Named("Launchkey MIDI".into()).resolve(&names), Some(1));
    assert_eq!(PortChoice::Named("keystep".into()).resolve(&names), Some(0));
}

#[test]
fn a_named_port_that_is_missing_resolves_to_nothing() {
    // The rule that makes an explicit choice safe: never substitute a different interface.
    let names = vec!["Port A".to_string()];
    assert_eq!(PortChoice::Named("Port B".into()).resolve(&names), None);
    assert_eq!(PortChoice::Named("Port B".into()).resolve(&[]), None);
    assert_eq!(PortChoice::Any.resolve(&[]), None);
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app midi::ports`
Expected: FAIL — `cannot find type PortChoice`.

- [ ] **Step 3: Implement**

`PortChoice::resolve` keeps today's rule exactly: `Any` → `0` when there are ports, `Named` → exact match, then case-insensitive substring, else `None`. `pick_port(names, wanted)` becomes `PortChoice::from(wanted).resolve(names)`; add `impl From<Option<&str>> for PortChoice`.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test -p acid-app midi::ports`
Expected: PASS — the three existing tests plus the two new ones.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app/src/midi/ports.rs
git commit -m "feat(midi): a port choice that never substitutes another interface"
```

---

### Task 2: The panel's MIDI picker

**Files:**
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`PanelState` fields, the top strip, the picker)
- Modify: `rust/crates/acid-app/src/ui/panel.rs` (`pub fn midi_status_rect`, so the picker's rect is testable)
- Modify: `rust/crates/acid-app/src/ui/help.rs` (one line)
- Modify: `rust/crates/acid-app/src/main.rs` (consume the choice)
- Test: `rust/crates/acid-app/tests/midi_picker_live.rs` (create)

**Interfaces:**
- Consumes: `midi::ports::{PortChoice, list_ports}`, `ui::picker::should_close`, `panel::midi_status_rect(top: Rect, scale: f32) -> Rect`.
- Produces:
  - `PanelState::midi_ports: Option<Vec<String>>` — `Some` while the picker is open.
  - `PanelState::midi_port_choice: Option<PortChoice>` — set by the panel, taken by `main`.
  - `ui::panel::midi_status_rect(l.top, s)` — the same rect the status text is painted in, so a test can click it.
  - `impl TakkaApp`: `Midi::choose(&mut self, choice: PortChoice)` — drop the connection, store the preference, probe now.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn clicking_the_midi_line_lists_the_ports_and_a_row_chooses_one() {
    // Two ports reported; the status line opens the list, the second row picks "Launchkey MIDI".
    // ... harness from tests/clear_live.rs (run_on/click), then:
    assert_eq!(panel.midi_ports, Some(vec!["KeyStep".into(), "Launchkey MIDI".into()]));
    assert_eq!(panel.midi_port_choice, Some(PortChoice::Named("Launchkey MIDI".into())));
}
```

Seed the module's port source through a `PanelState` field is not possible, so the test drives the picker with `panel.midi_ports` pre-filled: opening sets it from `list_ports()` (which the test cannot rely on), so the test asserts the row click only, and the open-click is covered by a second test that asserts `midi_ports.is_some()` after clicking the line.

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app --test midi_picker_live`
Expected: FAIL — `no field midi_ports`.

- [ ] **Step 3: Implement**

In `draw_panel`, replace the ad-hoc `status` rect with `panel::midi_status_rect(l.top, s)`; make it an interactive `Sense::click()` target that toggles `state.midi_ports` (filling it from `midi::ports::list_ports().unwrap_or_default()`). Draw one row per port below the status line, each `s * 18.0` tall, with an "any" row first; a clicked row sets `state.midi_port_choice = Some(...)` and closes the list; `picker::should_close(click, &rows, status)` dismisses it. Keep the existing REPROBE dot working: rows and dot must not overlap.

In `main.rs`, after the device choice handling: `if let Some(choice) = self.panel.midi_port_choice.take() { self.midi.choose(choice); }`, and `probe_if_due` opens with `&self.preference`.

Add to the help card's MIDI section: `"click the MIDI line: choose an input port"`.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS, and the help card still fits at 0.65x.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(ui): choose the MIDI input port from the panel"
```

---

### Task 3: The choice survives a restart

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`SavedState::midi_input`)
- Modify: `rust/crates/acid-app/src/main.rs` (save and load it)
- Test: `rust/crates/acid-app/src/persist.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `PortChoice`.
- Produces: `SavedState::midi_input: Option<String>` with `#[serde(default)]`; `Midi::preference(&self) -> PortChoice`; `Midi::new(preference: PortChoice)`.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn the_chosen_midi_port_round_trips_and_an_old_file_still_loads() {
    let mut saved = SavedState::default();
    saved.midi_input = Some("Launchkey MIDI".into());
    let json = serde_json::to_string(&saved).unwrap();
    assert_eq!(serde_json::from_str::<SavedState>(&json).unwrap().midi_input.as_deref(), Some("Launchkey MIDI"));

    // A file written before this field existed must still load, with "any".
    let legacy = r#"{"bank":{"patterns":[],"current":0},"knobs":{"waveform":0,"tuning":0.0,"cutoff":0.0,"resonance":0.0,"env_mod":0.0,"decay":0.0,"accent":0.0,"volume":0.0},"bpm":130.0,"swing":0.5}"#;
    assert_eq!(serde_json::from_str::<SavedState>(legacy).unwrap().midi_input, None);
}
```

(If the `bank` shape above does not deserialize, copy a real file's JSON from `SavedState::default()` and remove only the new field — the point is the missing-field case, not the hand-written shape.)

- [ ] **Step 2: Run to verify it fails**

Run: `cd rust && cargo test -p acid-app persist`
Expected: FAIL — `no field midi_input`.

- [ ] **Step 3: Implement**

Add the field with `#[serde(default)]`; `main.rs` writes `saved.midi_input = self.midi.preference_name()` inside `save()` and builds `Midi::new(...)` from `saved.midi_input` at startup. `Main`'s probe closes when the choice changes; nothing else changes.

- [ ] **Step 4: Run to verify it passes**

Run: `cd rust && cargo test --workspace`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): remember the chosen input port"
```
