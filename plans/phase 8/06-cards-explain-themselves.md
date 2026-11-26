# Plan 06 — cards that explain themselves

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** every key on every card says what it does on hover, in the panel's existing voice — and the keys whose effect
depends on state the player cannot see say *which* document they would write, so `SAVE BANK` answers "which bank?" before it
is clicked.

**Spec:** [`2026-11-25-cards-explain-themselves-design.md`](../../docs/superpowers/specs/2026-11-25-cards-explain-themselves-design.md),
approved by the player on 2026-11-25 with the slot chosen as **phase 8, round 06**. It is the design; this plan argues from it
and where they disagree the spec wins and the plan is wrong. The round's own choices are listed under *Rulings this plan
makes*.

**Architecture:** the one tip catalogue (`ui::tips`) grows by seventeen entries and gains one formatter that puts a computed
line after the sentence; the catalogue stays a static table and `Tip` keeps its three `&'static str` fields. Each card supplies
its own computed line from the module that owns its state, and the strip's comes from `persist::save_target` — the same
function `main` uses to decide in place vs new file — so a hint cannot describe a rule the save does not follow.

**Tech Stack:** Rust, egui 0.37 / eframe 0.37. No new dependency.

**Branch:** stacked on `phase-8-round-03` while PR #16 is open, because the card this round tips is round 03's browser. Rebase
onto `main` once #16 merges, before opening this round's pull request.

## Global Constraints

- **One catalogue.** `Control` gains seventeen entries, **appended** (`ui::tips`), so no existing control's position moves and
  the two numbers the tests pin (`ALL`/`TIPS` length, the drum face's slice) move with them.
- **No `_` arm.** `Control::tip()` is one exhaustive `match`, written out; `ALL` and `TIPS` stay index-coupled.
- **No font glyphs.** Titles and sentences are ASCII, three words or more, and every title is distinct — the catalogue's own
  tests enforce all three.
- **No tip on a field, a chip, a document row or an engraving.** `NAME`, the strip's save field, the search field, the five
  chips, the list's rows and `FACTORY` / `MY KITS` stay silent. A field's caption says what it wants and a hint that follows the
  pointer into it is in the way.
- **The save file is a contract.** No `SavedState` field, no `SAVE_VERSION` bump, no change to `songs/`, `banks/` or `racks/`.
- **`Request` gains nothing and `main.rs` is not edited.** The flag reaches `banks::paint` through `ui::mod`.
- **An `egui::Id` is behaviour.** No id changes; a tooltip hangs off a `Response`, never off a recomputed rect.
- **Gates, every task:** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`,
  and `./docs/guide/check.sh` from the repo root once the book moves.

## Review Focus

Five states the spec implies and no single test names, most likely to bite first. Each gets its test in the task that owns the
code.

1. **A hint whose line says "in place" while the click writes a new file.** The line must be built from
   `persist::save_target(current, typed)` — the click's own decision — with the field's text as typed, and never from a rule the
   card re-derives.
2. **A hint that lags a keystroke.** The card builds its detail from the same `field_text` the key reads, in the same frame; a
   detail built before the `TextEdit` is painted would describe the previous frame's name.
3. **A tooltip over a key whose rect moves** — a chip click re-places the strip, the wheel re-places the rows. The hover must hang
   off the `Response` `transport::card_button` returns, never off a rect placed beside it.
4. **`TOOLTIPS OFF` must silence the cards and leave them working.** The flag is a paint parameter; nothing about a card's
   behaviour may depend on it.
5. **A card key that is dim, and its hint says it works.** `SAVE A COPY` with nothing typed is dim and does nothing, and its line
   says so; every other dim key's sentence is true of a lit one and says nothing about being dim.

## What is wrong today

- **The cards are the only silent surface.** `Control`, `hover` and `text_for` serve the knobs, the transport row, the tape deck,
  the strip, the settings switches, the drum face and the two readouts; the `BANKS` card's nine keys, the device card's nine, the
  kit browser's two and the picker's three have no tips at all. A device's `MUTE` borrows the strip's tip and is the one
  exception.
- **`SAVE BANK` names nothing.** Whether it rewrites a file or makes one is decided by `persist::save_target` from the field's
  text and the bank the device came from — two things the key does not show. The player asked this in so many words.
- **`banks::paint` is not even given the flag**, so no `BANKS` key can hover whatever else changes: its signature is
  `paint(ui, panel, scale, songs, session, kind)`.
- **The book already claims more than the panel does.** `13-settings` says turning `TOOLTIPS ON` off makes "the switches and
  keys go quiet"; on the cards there was never anything to quiet.

## The design

**The formatter**, in `ui::tips`, and nothing else there:

```rust
pub fn text_with(wants_help: bool, control: Control, detail: Option<&str>) -> Option<String>;
pub fn hover_with(response: &Response, wants_help: bool, control: Control, detail: Option<&str>);
```

`text_for` and `hover` stay, as one-line wrappers over these with `None`, so no existing surface changes and no call site
churns. The composed text is `title`, `what`, the detail when there is one, then the shortcut. `None` and `Some("")` are the
same thing: no line.

**The catalogue** grows by the seventeen entries in the spec's table, appended in that table's order (cards first, `BANKS`
before the device card before the kits before the picker, with the shared `CardClose` after the `BANKS` group). The spec's
column *what the sentence must say* is the requirement; the exact prose is the implementer's, in the catalogue's voice. Two
titles are the interesting ones: the card's song key is **`SAVE SONG`** because the panel's `SAVE` already owns `SAVE`, and
**one `CLOSE`** serves four cards.

**Where each line comes from** — one builder per card, in the module that owns the state:

| builder | module | the line |
|---|---|---|
| `save_detail(current: Option<&str>, typed: &str, copy: bool) -> String` | `ui::banks` | `Writes Acid 1 in place.` / `Makes a new file, Acid 2.` / `No name yet: the key opens the field.` / `Type a name first: this key is dim until you do.` |
| `cross_detail(kind: Kind, name: &str) -> String` | `ui::banks` | `Deletes the bank Easy.` |
| `export_detail(session: Option<&str>) -> String` | `ui::banks` | `Writes the session, Acid 1.` / `The session has no name yet.` |
| `remove_detail(name: &str) -> String` | `ui::device_card` | `Takes ACID 2 out of the rack.` |
| `picker_detail(kind: DeviceKind) -> String` | `ui::device_card` | `Adds an ACID machine.` |

**The flag** reaches the `BANKS` card as one more parameter on `banks::paint`, copied out in `ui::mod` before the card borrow
takes `&mut state.songs.card`, exactly as `bank_file` and `rack_file` are. The other three cards already hold `PanelState`.

**What the player sees**, with the pointer resting on `SAVE BANK`:

```
SAVE BANK
Writes the selected device's whole bank under the field's name; the bank's own name rewrites that file, and a blank field keeps it.
Makes a new file, Easy 2.
```

## Tests that make it safe

- **The formatter** (unit, `tips.rs`): `None` gives exactly `text_for`'s text; a detail lands on its own line after the sentence
  and before the key; `Some("")` gives no line.
- **The catalogue** (unit, `tips.rs`): the seventeen appended in the spec's order at `ALL[49..66]`; the length 66; every title
  still distinct; every sentence still ASCII and three words or more. The drum face's slice becomes `[40..49]` so it goes on
  meaning "the nine drum controls".
- **`BANKS`** (live, `banks_live.rs`): `SAVE` on a named session shows its sentence and `Writes Acid 1 in place.`; `SAVE BANK`
  with a new name shows `Makes a new file, Easy 2.`; `SAVE BANK` with a blank field and `bank_file` set shows the bank's own
  name **in place**; `SAVE A COPY` with nothing typed shows the dim line; a row's cross names the file; `EXPORT` names the
  session; `NEW`, `IMPORT`, `CLOSE` and the two remaining doors show their sentences; with `tooltips = false` nothing appears and
  `SAVE` still saves.
- **The device card** (live, `device_card_live.rs`): all six keys plus `CLOSE` answer; every port row answers with the same tip;
  `REMOVE`'s line names the device; `tooltips = false` silences them.
- **The kits and the picker** (live, `kits_live.rs` and `device_card_live.rs`): `IMPORT` and `CLOSE` answer; a kind row's line
  names the kind; `CLOSE` on the picker answers.
- **The book**: `check.sh` pins the new sentence and `gone`-guards nothing new; `guide: ok`.

## Rulings this plan makes

Each is a choice the spec left to the builder, with what it costs if wrong.

| Ruling | Cost if wrong |
|---|---|
| **`save_detail` takes `(current, typed, copy)`, not the `Door`.** The door decides the *title* and the *label*; the line is about a name and a rule, and the same for all three documents. A `Door` argument would be a parameter the function never reads. | A reader looking for the door's part in the line finds it in `save_target`, one call away. |
| **`save_detail` asks `persist::save_target`, and the copy key asks it too, overriding only the in-place case** it cannot produce. | One branch that cannot be reached by the copy key, written out rather than assumed; the alternative is a second copy of the "same name means in place" rule. |
| **The seventeen are appended in the spec's table order, as one task.** Splitting per card would re-pin the same two numbers four times, and the words are worth reading as one voice. | A reviewer reads a long table once instead of four short ones. |
| **`hover` for tests lives in `tests/common/mod.rs` as `hover(ctx, panel, shared, at) -> Vec<String>`**, five frames from `drum_live.rs`'s idiom, the caller setting `panel.tooltips` first. `drum_live.rs`'s own helper is left alone. | One more helper in the card's shared module, and two copies of a five-frame hover in the tree until another round needs it. |
| **`CHANNEL`'s tip hangs off both arrows, `PORTS`' off `ALL PORTS` and every port row, `CLOSE`'s off four cards.** One control, several responses — the question is one, and the responses are the places a pointer can land. | A tip that appears only on one of two arrows would read as an accident. |
| **The overlap check is a recorded grab, not a gate.** The last task takes the card on `Xvfb` with the pointer resting on `SAVE BANK` and on `CLOSE`, the executor reads them, and `## Progress` records what it saw. | If the hint does cover the key, the player meets it on their own screen rather than in the book — which is where they would meet it anyway. |
| **No `gone` guard for the words this round adds**, only the positive pin. | A rewrite could drop the sentence and the pin catches it; a rewrite cannot "put back" words that never existed before. |

---

## Tasks

### Task 1 — a tip that carries one computed line

**Files:** modify `rust/crates/acid-app/src/ui/tips.rs`; tests in its own `#[cfg(test)] mod tests`.

**Interfaces:** produces `text_with(wants_help: bool, control: Control, detail: Option<&str>) -> Option<String>` and
`hover_with(response: &Response, wants_help: bool, control: Control, detail: Option<&str>)`; keeps
`text_for(wants_help, control) -> Option<String>` and `hover(response, wants_help, control)` as wrappers over them with `None`.
Consumes nothing new.

- [ ] **Step 1: Write the failing tests** in `tips.rs`'s `mod tests`, by name:
      `a_detail_lands_on_its_own_line_after_the_sentence` (for `Control::Cutoff`, `text_with(true, .., Some("Writes Acid 1 in
      place."))` is the three lines `CUT OFF` / its sentence / the detail), `a_key_comes_after_the_detail` (for a control with a
      shortcut — `Control::Copy` — the text's last line is the shortcut and the detail is the line before it),
      `no_detail_is_the_text_the_panel_already_had` (`text_with(true, control, None) == text_for(true, control)` for every
      `Control::ALL`), `an_empty_detail_is_no_detail` (`Some("")` equals `None`), and `the_hint_stays_silent_when_help_is_off`
      (`text_with(false, Control::Cutoff, Some("…"))` is `None`).
- [ ] **Step 2: Run** `cargo test -p acid-app --lib tips` **and expect FAIL** (`cannot find function text_with`).
- [ ] **Step 3: Implement** `text_with` as the one composer — `title`, `what`, the detail when it is non-empty, `shortcut` — and
      `text_for` as `text_with(wants_help, control, None)`; `hover_with` attaches `on_hover_text` when `text_with` says
      something, and `hover` calls it with `None`.
- [ ] **Step 4: Run** the file, then `cargo test --workspace` — nothing else may change behaviour, because no caller passes a
      detail yet.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): a tip can carry one computed line"`.

### Task 2 — the catalogue grows by seventeen

**Files:** modify `rust/crates/acid-app/src/ui/tips.rs` (the enum, `Control::ALL`, `tip()`'s arms, `TIPS`); tests in its
`mod tests`.

**Interfaces:** produces the seventeen variants named in the spec's table, in that order, at `Control::ALL[49..66]`:
`SongSave`, `SongCopy`, `BankSave`, `RackSave`, `CardNew`, `SongImport`, `SongExport`, `CardDelete`, `CardClose`,
`DeviceRole`, `DevicePorts`, `DeviceChannel`, `DeviceColour`, `DeviceKit`, `DeviceRemove`, `KitsImport`, `PickerKind`.
`Control::ALL` is `[Control; 66]`, `TIPS` is `[Tip; 66]`. Consumes `text_with` from Task 1.

- [ ] **Step 1: Write the failing test** `the_card_keys_are_appended_in_card_order`: `Control::ALL.len() == 66`, and
      `&Control::ALL[49..]` equals the seventeen names above, in that order. Update the two numbers the existing tests pin:
      `the_drum_face_has_its_own_help_for_every_control`'s `assert_eq!(Control::ALL.len(), 49)` becomes `66`, and its
      `&Control::ALL[40..]` becomes `&Control::ALL[40..49]`.
- [ ] **Step 2: Run** `cargo test -p acid-app --lib tips` **and expect FAIL** (`left: 49, right: 66`).
- [ ] **Step 3: Implement** the seventeen variants in the spec's order, each with its `ALL` entry, its `tip()` arm and its `Tip`
      in `TIPS`, its `what` following the spec's *what the sentence must say* column. Titles: `SAVE SONG`, `SAVE A COPY`,
      `SAVE BANK`, `SAVE RACK`, `NEW SONG`, `IMPORT`, `EXPORT`, `DELETE`, `CLOSE`, `ROLE`, `PORTS`, `CHANNEL`, `COLOUR`, `KIT`,
      `REMOVE`, `IMPORT KIT`, `DEVICE KIND`. No shortcut: no card key has a key. The existing tests (distinct titles, ASCII,
      three words or more, `ALL`/`TIPS` agreement) are the gate on the words.
- [ ] **Step 4: Run** the file, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the catalogue holds every key the cards draw"`.

### Task 3 — the `BANKS` card explains itself

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs` (`paint`, the three detail builders); modify
`rust/crates/acid-app/src/ui/mod.rs` (the `banks::paint` call); modify `rust/crates/acid-app/tests/common/mod.rs` (the hover
helper); modify `rust/crates/acid-app/tests/banks_live.rs`.

**Interfaces:** produces `banks::paint(ui, panel, scale, songs, session, kind, wants_help: bool)`;
`fn save_detail(current: Option<&str>, typed: &str, copy: bool) -> String`;
`fn cross_detail(kind: Kind, name: &str) -> String`; `fn export_detail(session: Option<&str>) -> String`; and
`pub fn hover(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) -> Vec<String>` in `tests/common/mod.rs`.
Consumes `Control::{SongSave, SongCopy, BankSave, RackSave, CardNew, SongImport, SongExport, CardDelete, CardClose}`,
`tips::hover_with`, `persist::save_target`.

- [ ] **Step 1: Write the failing tests** in `banks_live.rs`, using the new `hover` helper (which runs five frames with
      `input.time` advancing, the pointer moved once, as `drum_live.rs` does):
      `the_strip_names_the_document_it_would_write` (`SAVE` on a named session shows the sentence *and* `Writes Acid 1 in
      place.`; `SAVE BANK` with `bank_name = "Easy 2"` shows `Makes a new file, Easy 2.`; `SAVE BANK` blank with `bank_file =
      Some("Easy")` shows the bank's own name in place), `the_copy_key_says_what_it_would_copy`
      (`SAVE A COPY` with nothing typed shows the dim line; with a name it shows a new file), `a_rows_cross_names_the_file`
      (the `Easy` bank's cross shows `Deletes the bank Easy.`), `the_footers_keys_explain_themselves` (`NEW`, `IMPORT`,
      `EXPORT` — naming the session — and `CLOSE` each show their sentence),
      `a_name_typed_into_the_field_is_the_name_the_hint_names` (type `easy` through the field as `the_search_narrows_the_list_
      to_the_row_that_loads` does, then hover `SAVE BANK` and read `Makes a new file, easy.` — the line must be built in the
      frame the field is painted in, not the one before), and
      `help_that_is_off_leaves_the_card_silent_but_working` (`tooltips = false`, a hover paints none of those sentences, and
      clicking `SAVE` still asks for `Save`).
- [ ] **Step 2: Run** `cargo test -p acid-app --test banks_live` **and expect FAIL** (`cannot find function hover` first, then
      a card that paints no tip text).
- [ ] **Step 3: Implement** the three builders and the hovers: the strip's two keys ask `save_detail` with the field's text as
      typed and `session.bank_file` / `session.rack_file` / `songs.open` as the current file; the cross asks `cross_detail` with
      the row's `kind` and `name`; `EXPORT` asks `export_detail` with the session's name; `NEW`, `IMPORT` and `CLOSE` take no
      detail. Add `wants_help` to `banks::paint` and copy `state.tooltips` out in `ui::mod` before the card borrow, as
      `bank_file` is.
- [ ] **Step 4: Run** the file, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the banks card explains itself, and names the document it would write"`.

### Task 4 — the device card explains itself

**Files:** modify `rust/crates/acid-app/src/ui/device_card.rs` (the card, not the picker); modify
`rust/crates/acid-app/tests/device_card_live.rs`.

**Interfaces:** consumes `Control::{DeviceRole, DevicePorts, DeviceChannel, DeviceColour, DeviceKit, DeviceRemove, CardClose}`
and `tips::hover_with`; produces `fn remove_detail(name: &str) -> String`. Consumes `common::hover` from Task 3.

- [ ] **Step 1: Write the failing tests** in `device_card_live.rs`: `every_key_on_the_device_card_explains_itself` (hover each
      of `ROLE`, `ALL PORTS`, a port row, both channel arrows, `COLOUR`, `KIT`, `MUTE`, `REMOVE`, `CLOSE` and assert each tip's
      sentence is painted), `every_port_row_answers_with_the_same_tip` (two rows, one sentence), and
      `remove_names_the_device_it_would_take_out`.
- [ ] **Step 2: Run** `cargo test -p acid-app --test device_card_live` **and expect FAIL**.
- [ ] **Step 3: Implement** `tips::hover_with` on each key's `Response`: `ROLE` (`device-card-role`), `ALL PORTS` and every port
      row (`DevicePorts`), both channel arrows (`DeviceChannel`), `COLOUR`, `KIT`, `REMOVE` with
      `remove_detail(&state.device_card.name)`, `CLOSE` (`CardClose`). Leave `MUTE`, which already borrows `DeviceMute`. Read
      `state.tooltips` into a local before any borrow of `state` the row loop needs.
- [ ] **Step 4: Run** the file, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the device card explains itself"`.

### Task 5 — the kit browser and the picker explain themselves

**Files:** modify `rust/crates/acid-app/src/ui/kits.rs`; modify `rust/crates/acid-app/src/ui/device_card.rs` (the picker);
modify `rust/crates/acid-app/tests/kits_live.rs` and `rust/crates/acid-app/tests/device_card_live.rs`.

**Interfaces:** consumes `Control::{KitsImport, CardClose, PickerKind}`, `tips::hover_with` and `common::hover` from Task 3;
produces `fn picker_detail(kind: DeviceKind) -> String`.

- [ ] **Step 1: Write the failing tests**: `the_kit_browsers_keys_explain_themselves` (`IMPORT` and `CLOSE` answer, and a kit
      row still does not) and `a_picker_row_names_the_kind_it_adds` (`ACID`'s line says `Adds an ACID machine.`, `DRUMS`'s names
      drums, and the picker's `CLOSE` answers).
- [ ] **Step 2: Run** `cargo test -p acid-app --test kits_live --test device_card_live` **and expect FAIL**.
- [ ] **Step 3: Implement** `KitsImport` and `CardClose` on the kit browser's two keys, and `PickerKind` (with
      `picker_detail(*kind)`) on each kind row plus `CardClose` on `device-picker-close`. Kit rows are names and stay silent.
- [ ] **Step 4: Run** both files, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the kit browser and the picker explain themselves"`.

### Task 6 — the book, the pin, and the eye

**Files:** modify `docs/guide/src/12-songs-and-demos.md`, `docs/guide/src/13-settings.md`,
`docs/guide/src/20-reference.md`, `docs/guide/check.sh`, `plans/phase 8/00-phase-8.md` (its State line and a `06-` entry in
the rounds list, so the phase says which plan is in flight), and this plan's `## Progress`.

- [ ] **Step 1: Page 12** gains a paragraph on the card explaining itself: every key says what it does, and the keys that write
      a document say *which* — with the `SAVE BANK` example, and the rule that a hint obeys `TOOLTIPS`. Written so the pinned
      sentence sits on one line.
- [ ] **Step 2: Page 13**'s switch sentence already claims the keys go quiet; it now names the cards, so a player knows the
      switch reaches them.
- [ ] **Step 3: Page 20**'s device-card and kit-browser entries gain "each key explains itself on hover".
- [ ] **Step 4: `check.sh`** pins one new sentence — the card explaining itself, on **one line**, because a pin is raw
      `grep -F` — and adds no `gone` guard, since the words did not exist before this round to be put back.
- [ ] **Step 5: The eye.** Take the card on `Xvfb` with the pointer **resting on `SAVE BANK`**, and again on `CLOSE`:
      `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` before the launch, `getwindowgeometry` must say
      `1960x1340`, `TAKKA_ACID_DATA_DIR` must be fresh. Read the two grabs, and record in `## Progress` whether the hint covers
      the key it describes and what was done about it. These pictures are **not** for the book — its recipe parks the pointer
      away — so `img/README.md` gains no row.
- [ ] **Step 6: `./docs/guide/check.sh` → `guide: ok`**, the three gates, then commit — `-m "docs: the cards explain themselves"`.

## Non-goals

- **A tip on a field, a chip, a document row or an engraving**, for the reasons in the spec.
- **A second tips catalogue, a per-card table, or a field on `Tip`.** The seventeen join `Control`.
- **Changing a tooltip's look, delay or placement**, or adding a switch.
- **A picture of a tooltip in the book**, and a new `img/README.md` row.
- **Keyboard shortcuts for card keys.**
- **The save file, `Request`, `main.rs`, and every `egui::Id`.**
- **Refactoring `drum_live.rs`'s hover helper** into the shared module: this round copies the idiom for the cards and leaves the
  drum face's test alone.

## Review focus for the pull request

1. **One formatter.** `text_with` composes; nothing else builds a tip's text, and no card quotes a tip's words.
2. **One answer per fact.** `save_detail` asks `persist::save_target`; no task re-derives "same name means in place".
3. **Every catalogue entry is reached.** Seventeen new controls, and a test hovers each one on a real card — the spec's
   anti-goal, "a tip nobody can reach".
4. **The pinned numbers moved with the append**, and the drum face's slice still means the nine drum controls.
5. **The words and the book agree**, and `guide: ok` with the new pin.
6. **Nothing on the cards changed but a hover**: no id, no request, no rect, no save-file byte.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if wrong.
The branch review's findings and what was done about each go here too, before the pull request.)*
