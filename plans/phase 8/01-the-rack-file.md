# Plan 01 — the rack is a file

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** a rack can be saved as its own document, listed on the `BANKS` card, loaded into any session and deleted — the four
verbs a bank already has. A recall **replaces** the machines, keeps the song and the tempo, and forgets what a restart
forgets. A rack file from a newer build is refused with its version named. Nothing in the session, song or bank formats
moves: no `SAVE_VERSION` bump.

**Spec:** [`2026-11-25-rack-documents-design.md`](../../docs/superpowers/specs/2026-11-25-rack-documents-design.md), sections
*The document*, *The rack's two halves*, *Where a recall lives*, *The card*, *What a recall does to a machine*, *The book*.
The spec's *What the builder check changed* names the seven findings that shaped it.

## What is wrong today

- **`RACKS` is two keys that refuse.** `ui::banks::Request::{SaveRack, LoadRack}` are answered by `main` with a notice
  naming "the rack round" (`main.rs:333`). There is no `save_rack`, no `load_rack`, no `racks/`, and no row list, so the
  card's third section is the one place on the face that does nothing.
- **A rack cannot be written outside a song.** `persist::rack_to_saved` builds a whole `SavedState` — panel state, tempo,
  song name and all — because that is what a session is. A preset that carried a tempo would retune whatever song it was
  recalled into.
- **`Rack::replace_from_saved` writes the document's tempo** (`device.rs:476`). That is right for a song and wrong for a
  document that owns no clock, and there is no second seam to use instead.
- **A recall's decisions would live in `main`.** `main::load_song_now` is the model, and it is a binary no `tests/*_live.rs`
  can construct — the shape of the P0 phase 7's review found (an untestable quit path that wrote over a refused file).
- **The version rule is one reader short.** `load_document` parses a `SavedState`; a rack is a different shape, so
  `load_rack` would otherwise carry the fourth copy of `version > SAVE_VERSION`.
- **Two listings are one function written twice.** `list_songs` and `list_banks` are the same read-dir, strip `.json`, sort.
  A rack list would be the third.
- **Four sentences still say a rack is not saved**, after phase 5 saved it: `ui/help.rs:69`, `ui/tips.rs:400`, page 05's
  picker caption and the prose under it, and `ui/device_card.rs`'s module doc. Page 12's `RACKS` paragraph says both keys do
  nothing yet, which stops being true the moment this round lands.

## The design

**The document**, in `persist`, beside the bank's:

```rust
pub const RACKS_DIR: &str = "racks";

pub struct RackDocument {
    #[serde(default)] pub version: u32,     // SAVE_VERSION: a third kind of document, not a third version
    pub devices: Vec<SavedDevice>,
}
impl RackDocument {
    pub fn of(rack: &Rack) -> Self;         // rack_to_saved(rack).devices: the live pattern merged, sanitize already run
    pub fn into_saved(self) -> SavedState;  // devices + the flat mirror sanitize rebuilds; bpm is nobody's
}

pub fn rack_path(dir: &Path, name: &str) -> PathBuf;
pub fn list_racks(dir: &Path) -> Vec<String>;
pub fn save_rack(dir: &Path, name: &str, document: &RackDocument) -> Result<()>;
pub fn load_rack(dir: &Path, name: &str) -> Result<RackDocument>;
pub fn delete_rack(dir: &Path, name: &str) -> Result<()>;
pub fn free_rack_name(dir: &Path, wanted: &str) -> String;

/// The version rule alone — the probe, then the comparison — for every document kind that has a `version`.
pub fn checked_version(bytes: &[u8], path: &Path) -> Result<u32, LoadError>;
```

`load_document` becomes `checked_version` plus a parse, so the session, a song, an import and a rack all compare in one
place. `list_documents(dir, subdir)` is the private lift of the two identical bodies; `list_songs`, `list_banks` and
`list_racks` are its three callers.

**The rack's two halves**, in `device`:

```rust
/// Replace every machine, keeping this rack's tempo cell, transport and bus and resetting the selection to 0.
/// A document's `bpm` is **not** read.
pub fn replace_devices_from(&mut self, saved: &SavedState);

pub fn replace_from_saved(&mut self, saved: &SavedState) {
    self.replace_devices_from(saved);
    self.set_tempo_bpm(saved.bpm);
}
```

**The recall**, in a new `src/rack_recall.rs`:

```rust
pub struct Recalled {
    pub kits: Vec<(usize, String)>,      // the machines to decode, at the running device's rate
    pub unopened_ports: Vec<String>,     // what the new rack names and this machine has not got
}

/// Replace the session's rack with a document's and put the panel back on slot 0. Every decision here is a test's to make;
/// decoding a kit and speaking to MIDI are `main`'s and stay there.
pub fn recall(rack: &mut Rack, panel: &mut PanelState, document: &RackDocument) -> Recalled;
```

`recall` does, in order: `rack.replace_devices_from(&document.into_saved())`; `panel.close_device_card()`;
`pattern_swap::adopt`; `pending_pattern = None`; `songs.bank_file = None`; `dirty = true`; then answers the two lists from
`persist::wanted_kits` and `persist::unopened_ports`.

**The card.** `RACKS` becomes a section shaped like `PATTERN BANKS`: the rows of `racks/*.json` (a click loads, the cross
deletes twice), then a name field and `SAVE RACK`. `ui::SongState` gains `rack_file`, because it must outlive the card;
`banks::Banks` gains `rack_names` and `rack_name`, and `banks::Session` carries `rack_file` the way it carries `bank_file`.
`Request::{SaveRack, LoadRack}` take the name; `DeleteRack(String)` joins them. A rack save is `save_target`'s question, as a
bank save is: in place when the field names the rack the session came from, `free_rack_name` otherwise.

**`main`'s three arms** are sequencing of functions the tasks above already test: `save_rack_now` (`write_kit`, the target,
the write, the card's refresh, a notice), `delete_rack_now` (the file, the list, `rack_file` cleared if it was that file), and
`load_rack_now` (read and refuse with a notice, `save()`, `recall(..)`, `load_wanted_kits`, the ports notice, `rack_file` and the card's rows and name field,
"loaded rack {name}").

**What a recall does to a machine.** Everything the file holds comes back as written. Everything a restart forgets, a recall
forgets: mute off, `REVERSE` forwards, transpose in tune. The open MIDI ports are the session's and do not change.

## Tests that make it safe

- **`persist`**: a rack document round-trips every device field; its bytes hold `version` and `devices` and **no** `bpm`,
  `song`, `midi_input`, `density` or `scale`; a newer rack file is `Err(Newer { version, .. })`; `into_saved`'s `sanitize`
  truncates a five-machine rack to four and repairs duplicate numbers, a blank name and an unknown `kind`; `save_rack`
  refuses a blank and a path-shaped name; `list_racks` matches what `save_rack` wrote.
- **`rack`**: `replace_devices_from` leaves the tempo cell's value alone while `replace_from_saved` applies the document's
  (the existing test, unchanged); a rack through `RackDocument::of` → `into_saved` → `replace_devices_from` comes back device
  for device.
- **`rack_recall`**: on a one-machine session with a two-machine document, the document's two replace it with their names,
  colours, patches and banks; selection 0; `dirty`; `bank_file` cleared; the kits and ports answer lists are the new rack's.
- **`racks_live`** (the card, through `draw_panel`): `SAVE RACK` asks `SaveRack` with the typed name; the row list is the
  files on disk; a row asks `LoadRack(name)` once, and twice while the session is unsaved; a cross asks `DeleteRack` on the
  second click; the section is skipped when the card has no room.
- **the book**: `check.sh` pins what the pages now say and `gone` guards the two claims that stop being true.

## Tasks

### Task 1 — the document, its file, and the version rule

**Files:** modify `rust/crates/acid-app/src/persist.rs`; test `rust/crates/acid-app/tests/persist.rs`.

**Interfaces:** produces `RACKS_DIR`, `RackDocument { version, devices }` with `of` / `into_saved`, `rack_path`,
`list_racks`, `save_rack`, `load_rack`, `delete_rack`, `free_rack_name`, `checked_version`; consumes `rack_to_saved`,
`sanitize`, `document_name`, `free_name`, `write_atomic`, `LoadError`.

- [ ] **Step 1** Write the failing tests: a two-device `RackDocument` round-trips every field of both machines; the written
      bytes contain `version` and `devices` and none of `bpm`, `song`, `midi_input`, `tooltips`, `density`, `scale`; a
      document whose `version` is `SAVE_VERSION + 1` is `Err(Newer { .. })` naming it (relative, not a literal — phase 7's
      ruling); a five-device document's `into_saved()` has four, with their numbers made distinct and their names distinct;
      `save_rack` refuses `""` and `"../x"`; `load_rack` on a truncated file is an error and not `Newer`.
- [ ] **Step 2** Run them, expect FAIL (`no RackDocument in persist`).
- [ ] **Step 3** Implement: split `load_document` into `checked_version` + parse so its behaviour is byte-for-byte what it
      was; lift `list_documents`; build `RackDocument` on `rack_to_saved(rack).devices`; `into_saved` as
      `SavedState { devices, ..SavedState::default() }` through `sanitize`; the five file functions on the bank's pattern.
- [ ] **Step 4** Run the tests, then the workspace — `load_document`'s callers (session, song, import) must not change
      behaviour.
- [ ] **Step 5** Gates and commit — `-m "feat(save): a rack is a document of its own"`.

### Task 2 — the rack's two halves

**Files:** modify `rust/crates/acid-app/src/device.rs`; test `rust/crates/acid-app/tests/rack.rs`.

**Interfaces:** produces `Rack::replace_devices_from`; `replace_from_saved` keeps its signature and its behaviour.

- [ ] **Step 1** Write the failing tests: a rack at 150 BPM replaced from a document whose `bpm` is 90 is still at 150
      (`replace_devices_from`), and the same call through `replace_from_saved` is at 90; a rack of three devices through
      `RackDocument::of` → `into_saved` → `replace_devices_from` has the same numbers, names, kinds, schemes, banks, knobs
      and kits.
- [ ] **Step 2** Run them, expect FAIL (`no method replace_devices_from`).
- [ ] **Step 3** Implement by moving `replace_from_saved`'s body into `replace_devices_from` without the `set_tempo_bpm`
      line, and calling it from `replace_from_saved`.
- [ ] **Step 4** Run `tests/rack.rs` — `replacing_the_rack_publishes_in_order_selects_zero_and_keeps_the_tempo_cell` must
      still pass unchanged — then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(rack): the machines come from a document, the clock does not"`.

### Task 3 — the recall's decisions, in the library

**Files:** create `rust/crates/acid-app/src/rack_recall.rs`, modify `rust/crates/acid-app/src/lib.rs`; test
`rust/crates/acid-app/tests/rack_recall.rs` (new).

**Interfaces:** produces `rack_recall::Recalled { kits, unopened_ports }` and `rack_recall::recall(&mut Rack, &mut
PanelState, &RackDocument) -> Recalled`; consumes `Rack::replace_devices_from`, `close_device_card`, `pattern_swap::adopt`,
`persist::{wanted_kits, unopened_ports}`.

- [ ] **Step 1** Write the failing test: a rack of one ACID device at 140 BPM with a bank file and an unsaved panel, a
      document of a DRUMS machine on `909` and an ACID machine on a port this machine has not got; after `recall` the rack
      has those two in that order with their names, kinds, schemes, banks and kits, selection 0, `panel.dirty`, `songs.bank_file`
      `None`, a closed device card, and `Recalled.kits` names both machines' kits while `Recalled.unopened_ports` names the
      missing port once, and every recalled machine unmuted, forwards and in tune.
- [ ] **Step 2** Run it, expect FAIL (`no rack_recall`).
- [ ] **Step 3** Implement `recall` in the spec's order; keep `main`-shaped work (decoding, MIDI) out of it.
- [ ] **Step 4** Run it, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(rack): a recall is a decision a test can make"`.

### Task 4 — the card, the requests, and `main`'s three arms

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs`, `rust/crates/acid-app/src/ui/mod.rs`,
`rust/crates/acid-app/src/main.rs`; test `rust/crates/acid-app/tests/racks_live.rs` (new).

**Interfaces:** produces `SongState::rack_file`, `Banks::{rack_names, rack_name}`, `banks::Session::rack_file`,
`Request::{SaveRack(String), LoadRack(String), DeleteRack(String)}`, ids `rack-save` / `rack-name` / `rack-load-{index}` /
`rack-delete-{index}`; consumes Task 1's file functions and Task 3's `recall`.

- [ ] **Step 1** Write the failing live tests: a card with two racks on disk draws two rows and `SAVE RACK` asks
      `SaveRack("Live")` for that name; a row click asks `LoadRack` once, and on a dirty session arms with
      `<name> - click again to replace` first; the cross asks `DeleteRack` on the second click; `rack_name` is prefilled from
      `SongState::rack_file`.
- [ ] **Step 2** Run them, expect FAIL (the section draws two keys and no rows).
- [ ] **Step 3** Paint the section, add the fields and the variants, and answer the three arms in `main` — the compiler
      forces the last, because `main`'s drain matches `Request` in full.
- [ ] **Step 4** Run the live tests, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(panel): a rack has a place on the card, and it works"`.

### Task 5 — the book, the stale claims, the pins and the picture

**Files:** modify `docs/guide/src/12-songs-and-demos.md`, `18-a-rack-of-devices.md`, `20-reference.md`,
`05-the-three-cards.md`, `docs/guide/check.sh`, `docs/guide/src/img/README.md`, `rust/crates/acid-app/src/ui/help.rs`,
`rust/crates/acid-app/src/ui/tips.rs`, `rust/crates/acid-app/src/ui/device_card.rs`; retake `docs/guide/src/img/banks-card.png`.

- [ ] **Step 1** Find every claim the round makes stale: `grep -n "RACK" docs/guide/src/*.md`, and `grep -rn "not saved"`,
      and read each hit rather than rewriting it blind.
- [ ] **Step 2** Rewrite page 12's `RACKS` paragraph for a section that works: what a rack file holds, that a load replaces
      the machines and keeps the song and the tempo, that mute and `REVERSE` come back reset, and that the file's name is
      remembered for the session rather than across a restart. Page 18 gains the rack file beside the session file; page 20
      names the rows and the field.
- [ ] **Step 3** Delete the four stale claims — the picker caption and its prose on page 05, the help card's line, the `ADD
      DEVICE` tip, `device_card.rs`'s doc. The help card's line is a face's words: keep the line's shape, change its claim.
- [ ] **Step 4** Pin the new facts in `check.sh` (`SAVE RACK` and `LOAD RACK` are already there; add what page 12 must keep
      saying about a rack file and a recall) and `gone`-guard the two claims that stopped being true ("does nothing yet" /
      "a later round").
- [ ] **Step 5** Take `banks-card.png` again: check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400`
      **before** launching, `getwindowgeometry` says `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, the pointer parked away
      from the panel, and the `RACKS` section showing its field and a row. Record the row in `img/README.md`.
- [ ] **Step 6** `./docs/guide/check.sh` → `guide: ok`; gates; commit — `-m "docs: a rack is a file you can carry"`.

## Non-goals

- A merge, a tempo in the file, a `SAVE A COPY` key on `RACKS`, `IMPORT`/`EXPORT` for racks, a `SAVE_VERSION` bump, a
  remembered `rack_file` across a restart. Each is the spec's own Non-goals.
- Duplicating a device and dragging to reorder it: rounds 02 and 03.
- Carrying mute, `REVERSE` or transpose across a recall.
- A rack that carries the app's MIDI port choice, `tooltips`, `autosave`, `density` or `scale`: those are the session's.

## Review focus

1. **The rack file has no tempo in it, and a recall does not touch the clock.** The two halves of Task 2 are what make this
   true; the mutation to try is `set_tempo_bpm(saved.bpm)` inside `replace_devices_from`, which Task 2's first test must
   catch.
2. **A fourth reader of the version rule, not a fourth copy of it.** `load_document`'s behaviour must be byte-for-byte what
   it was after the split, and a newer rack file must be refused, not parsed.
3. **`main`'s arms are sequencing only.** Everything decidable is `persist`'s or `rack_recall`'s; what is left in `main` is
   named in the Progress, since a binary no test constructs is where phase 7's P0 lived.
4. **A rack save is `save_target`'s question**, as a bank save is. A second in-place rule is the bug this project has paid
   for twice.
5. **No `SAVE_VERSION` bump, and no field added to `SavedState`.** If a test needs one, the design is wrong.
6. **The book's words and the face's line agree.** The help card and the tip say what the panel now does; a guide that
   contradicts the panel is a bug (`AGENTS.md`).

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if
wrong. The branch review's findings and what was done about each go here too, before the pull request.)*
- **Task 1 — the document, its file, and the version rule.** `RackDocument { version, devices }` with `of` and `into_saved`,
  `RACKS_DIR`, `rack_path`, `list_racks`, `save_rack`, `load_rack`, `delete_rack`, `free_rack_name`; `load_document` split
  into **`checked_version`** plus a parse, so the session, a song, an import and a rack compare in one place; and
  `list_songs` / `list_banks` / `list_racks` are now one `list_documents`. Proved by
  `a_rack_document_round_trips_lists_and_deletes`, `a_rack_file_holds_the_machines_and_nothing_else` (the file's top level
  is `devices` and `version` and nothing else), `a_newer_rack_file_is_refused_and_a_broken_one_is_not_newer`,
  `a_rack_file_is_repaired_by_the_same_sanitize_as_a_song` and `a_rack_name_must_be_a_name_and_not_a_path` — each watched
  fail first, as unresolved imports in `tests/persist.rs`. Suite **990/990**, clippy `-D warnings`, `fmt --check`.
  **Ruling: `into_saved` takes `&self`.** The spec wrote a consuming conversion, and Task 3's `recall(&RackDocument)` cannot
  move out of a reference. Cost if wrong: one four-device `Vec` clone per recall.
  **Ruling: `load_rack` answers `Result<RackDocument, LoadError>`**, where the spec's shorthand `Result` was `anyhow`'s. The
  refusal is this document's own answer and a test can match `Newer`; `save_rack` stays `anyhow::Result`, like `save_bank`,
  because nothing needs to tell its failures apart. Cost if wrong: one error type the caller converts, where a song's path
  prints an `anyhow` chain.
- **Task 2 — the rack's two halves.** `Rack::replace_devices_from` replaces every machine, resets the selection, numbers
  above the file's highest and republishes the bus — and **does not read the document's `bpm`**; `replace_from_saved` is now
  the tempo write plus that call, so a song's behaviour is unchanged and a rack document's tempo cannot leak in. Proved by
  `a_documents_machines_do_not_move_the_clock` (150 kept through `replace_devices_from`, 90 applied through
  `replace_from_saved`) and `a_rack_through_a_document_is_the_rack_it_was` (rack → `RackDocument::of` → `into_saved` →
  `replace_devices_from` is device for device), each watched fail first as `no method replace_devices_from`; the existing
  `replacing_the_rack_publishes_in_order_selects_zero_and_keeps_the_tempo_cell` passes unchanged. Suite **992/992**, clippy
  `-D warnings`, `fmt --check`.
  **Ruling: `replace_from_saved` sets the tempo before it builds the machines**, where the old body set it after. The bus is
  then published with the clock the file asked for rather than the one the session had. Cost if wrong: a tempo change lands
  one statement earlier inside one synchronous call — nothing reads it in between.
- **Task 3 — the recall's decisions, in the library.** `rack_recall::recall(&mut Rack, &mut PanelState, &RackDocument,
  &[String]) -> Recalled` replaces the machines, closes the device card, adopts the new pattern at step 0, clears the bank
  file and marks the session dirty, and answers the kits to decode and the ports the new rack names and this machine has not
  got. Proved by `a_recall_replaces_the_machines_and_answers_what_is_left_to_do` (a DRUMS machine carrying `909` and an ACID
  machine on an unopened port replace a one-machine session; selection 0, tempo kept, every machine unmuted and in tune,
  `bank_file` gone, the card closed, `(0, "909")` and `["Port B"]` answered), watched fail first as
  `unresolved import acid_app::rack_recall` — after two mistakes of the test's own were fixed first (`Role::Notes` is
  `Role::PlayAndControl`, and a `Device`'s bank is a field). Suite **993/993**, clippy `-D warnings`, `fmt --check`.
  **Ruling: `recall` takes `open_ports: &[String]`.** The spec's signature left it out, but `persist::unopened_ports` needs
  the list and only `main`'s MIDI system knows it; without the argument the recall would have to guess or `main` would have
  to re-derive the document. Cost if wrong: one more argument at the one call site.
  **Ruling: `recall` does not clear `pending_pattern`**, though the plan's step list said it does. That field is
  `TakkaApp`'s, not `PanelState`'s — a held controller request, which `main` clears on a song load too — so the recall cannot
  reach it and `load_rack_now` clears it beside the call. Cost if wrong: a held pattern request could land on the recalled
  rack; the wiring is Task 4's and is named in the review focus.
- **Task 4 — the card, the requests, and `main`'s three arms.** `RACKS` is the bank's shape now: a row per `racks/*.json`
  that loads (twice while the session is unsaved), a cross that deletes, and a name field with `SAVE RACK` under the rows.
  `Banks` gains `rack_names` and `rack_name`, `SongState` gains `rack_file` (beside `file` and `bank_file`, because it must
  outlive the card), `banks::Session` carries it in, and `Request::{SaveRack, LoadRack}` take a name with `DeleteRack` beside
  them; ids are `rack-save` (kept), `rack-name`, `rack-load-{index}`, `rack-delete-{index}`. `bank_save_name` became
  **`document_save_name`**, one function for both doors. `main` gains `save_rack_now` (target → `free_rack_name` → `write_kit`
  → `RackDocument::of` → `save_rack`, card refreshed), `delete_rack_now`'s arm (file, list, `rack_file` cleared when it was
  that file) and `load_rack_now` (read, `save()`, `rack_recall::recall`, kits, ports, `rack_file`, notice); `rack_file` is
  cleared by `NEW`, a song load, a demo and a delete of that file. Proved by `tests/banks_live.rs`:
  `the_rack_section_lists_the_files_and_asks_for_a_load`, `a_dirty_session_arms_a_rack_load_first`,
  `saving_a_rack_asks_with_the_typed_name_or_the_file_it_came_from`, `a_rack_rows_cross_asks_to_delete_twice`, all watched
  fail first as `no field rack_names` / `expected function, found Request`. Suite **996/996**, clippy `-D warnings`,
  `fmt --check`.
  **Ruling: the plan's `tests/racks_live.rs` is a section of `tests/banks_live.rs`, not a file.** The card's geometry — where
  a section's rows are — is measured by helpers that live in `banks_live.rs` and mirror `banks::paint`'s arithmetic; a second
  copy for racks would be the hand-kept list this project has paid for twice. Cost if wrong: one file carries the card's
  tests.
  **Ruling: `main`'s three arms are sequencing, and are not tested.** Every decision in them is a `persist` function or
  `rack_recall::recall`, both tested; what is left is the write, the kit decode and the notice, in a binary no
  `tests/*_live.rs` can construct. The live tests stop at the request, as they do for a bank. Cost if wrong: the wiring
  between the request and the call is read by the branch review rather than proved — the same gap phase 7's review found a
  P0 in, which is why the decision was moved out and only the side effects remain.
  **Ruling: the ports line is said after "loaded rack {name}"**, so it is the sentence that survives on the status line. A
  song's load says its ports line before its own and loses it; that pre-existing order is left as it is, because changing
  what a song load says is not this round.
  **Not re-pinned: "the section is skipped when the card has no room."** That is the card's one `list_limit` rule, already
  asserted for `PATTERN BANKS`; racks obey the same `if`, and a second arrangement of the same test would pin the painter
  twice.
- **Task 5 — the book, the stale claims, the pins and the picture.** `12-songs-and-demos`'s `RACKS` paragraph is rewritten
  (a rack is a file of the whole instrument; a row replaces every machine; the song's name and the tempo stay; the armed row's
  words; `SAVE RACK`'s field rule; what a recall forgets), its intro stops calling a rack "a document this build only has
  keys for", and `18-a-rack-of-devices` gains *A rack of your own*; `20-reference`'s browser line loses the `LOAD RACK` key
  and gains a row per rack file; `05-the-three-cards`'s picker caption and prose lose "what is not saved". The same claim is
  gone from the code: the help card's line reads "four devices; every one of them is saved with the song", the `ADD DEVICE`
  tip says so too, and `device_card.rs`'s picker doc no longer promises a footnote. `check.sh` pins `racks/<name>.json`,
  `click again to replace the rack` and `unmuted, running forwards and in tune`, moves `LOAD RACK` from the must-say list to a
  `gone` guard, and guards `does nothing yet`. `banks-card.png` is retaken on a fresh `Xvfb :87` (`getdisplaygeometry` →
  `2000 1400`, window `1960x1340`, a fresh `TAKKA_ACID_DATA_DIR`, `BANKS` clicked at `762 561` logical, pointer parked) and
  its `img/README.md` row records the state and the command. `./docs/guide/check.sh` → `guide: ok`. Suite **996/996**, clippy
  `-D warnings`, `fmt --check`.
  **The suite caught a test that pinned the stale claim as behaviour.** `help_live`'s
  `the_help_card_names_the_devices_and_what_is_not_saved` asserted the card says "not saved", so this round's own words broke
  it — the test was pinning a lie the panel has contradicted since phase 5. It is now
  `the_help_card_names_the_devices_and_that_they_are_saved`: the same list with "saved with the song", plus one assertion that
  "not saved" cannot come back. (The first cut of that edit left a duplicate `#[test]` attribute on the function, which the
  harness ran twice — 997 instead of 996 — caught by counting the suite, and fixed.)
  **Ruling: the picture shows the rack's field and no row.** A row would be a rack file the photographer made, and a guide
  picture is taken on a fresh `TAKKA_ACID_DATA_DIR` so it shows a first run; the row's paint is pinned by
  `the_rack_section_lists_the_files_and_asks_for_a_load`. Cost if wrong: the book has no picture of a rack row until one is
  taken.
  **Ruling: `LOAD RACK` moved from `check.sh`'s must-say list to a `gone` guard.** The key is gone from the face — a row
  loads a rack now — so the pin had to move from "the book says it" to "the book does not".
