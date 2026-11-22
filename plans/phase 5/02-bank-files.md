# Plan 02 — banks are files, and a rack has a place

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** a pattern bank is a document — saved under a name, listed, loaded into the selected device of either kind, and deleted
— and the `BANKS` card's four sections say what they mean, with the two `RACKS` keys drawn and refusing.

**Spec:** [`2026-11-22-banks-racks-and-songs-design.md`](../../docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md),
sections *The bank document*, *The face*, *5.2*. Depends on round [`01-the-whole-rack`](01-the-whole-rack.md) for
`SAVE_VERSION` 12 and the card's `kind` argument.

## What is wrong today

- A bank has no file. It lives inside `state.json` or a song, so a player cannot keep one bank and try another, and the
  clipboard's text (`clipboard.rs`) is the only way a pattern leaves the app.
- The card is captioned `BANKS` and its second list is `MY BANKS` — every row of which is a **song** file (`songs/<name>.json`).
  The panel's switch that opens it is also `BANKS`. With no bank file in the app, neither word is true.
- `ui::banks::Request` has no bank verb, and the two rack verbs the phase wants have no key to come from.
- `persist::song_name` is the one name validator and is named after one of the two documents it would have to serve.

## The design

**The document.**

```rust
// <data>/banks/<name>.json
pub struct BankDocument {
    pub version: u32,  // SAVE_VERSION — one number for every saved document
    pub kind: String,  // DeviceKind::name() of the device it was written from
    pub bank: PatternBank,
}
```

**`persist` grows seven functions**, in the same module and the same shape as the song's: `save_bank`, `load_bank`,
`list_banks`, `delete_bank`, `bank_path`, `free_bank_name`, and the shared `document_name`. No `export_bank` and no
`import_bank`: the file in `banks/` is its own export, which page 12 already says about `songs/`. `load_bank` calls
`bank.normalise()` and keeps an unknown `kind` caption as written — the file still loads and the mismatch is said once.

**The card is four sections, and one heading is renamed** so the words are true: `DEMO BANKS` (unchanged), `PATTERN BANKS`
(new: the player's bank files), `SONGS` (today's `MY BANKS`), `RACKS` (new: two keys, `SAVE RACK` / `LOAD RACK`, refusing).

| key | id | drawn | a click asks for |
|---|---|---|---|
| bank row | `bank-{index}` | like a song row | `LoadBank(name)` — armed twice while the session is dirty |
| its cross | `bank-delete-{index}` | `x`, then `SURE` | `DeleteBank(name)` |
| name field + `SAVE BANK` | `bank-save` | one field and one key under the rows | `SaveBank(typed)` |
| `SAVE RACK` | `rack-save` | like `IMPORT` (always enabled) | `SaveRack` — a notice, no file |
| `LOAD RACK` | `rack-load` | the same | `LoadRack` — a notice, no dialog |

**Loading replaces the selected device's bank**, all 64 patterns, exactly as `DEMO BANKS` already does: `guarded_load`'s
two-click arm while the session is dirty, `Device::load_bank`, `pattern_swap::adopt`, `panel.dirty = true`, and a notice naming
what was replaced. The `Banks` section's arm key is `bank:<name>` in the existing `load_armed` field, and its delete arm is
`bank:<name>` in `delete_armed`, so one click at a time is still the rule and there is no second field to keep in step.

**Where a save lands** is `save_target(panel.songs.bank_file.as_deref(), typed)`: in place when the typed name is the bank's own,
a new file otherwise, and `free_bank_name` when that new name is taken — so a bank never destroys another by being given its
name, the same rule `import_song` has. `SongState::bank_file` is cleared by `NEW`, a song load and a change of selection.

**Ruling, and what it costs if wrong:** the bank file lives in the library and its `kind` mismatch is a warning, not a refusal. If
wrong, a player who wants to hand a bank over must find the data folder, and a drum bank loaded on an acid face loads and plays
nothing — which is what the notice says out loud.

## Tests that make it safe

In `rust/crates/acid-app/tests/persist.rs` and a new `rust/crates/acid-app/tests/banks_live.rs`:

- `a_bank_document_round_trips` — save, `list_banks` sees it, `load_bank` gives all 64 patterns and the same `kind`.
- `a_document_name_has_to_be_a_name_and_not_a_path` — replaces `a_song_name_...`; `../x`, ``, 60 characters and `a/b` are all
  refused, for both documents.
- `a_free_bank_name_is_the_wanted_one_or_a_numbered_neighbour`.
- `saving_a_bank_through_the_card_writes_the_file_and_lists_the_row`.
- `saving_a_bank_twice_under_its_own_name_rewrites_one_file` — the in-place rule; no `(2)` appears.
- `loading_a_bank_replaces_all_sixty_four_patterns_and_marks_the_session_dirty`.
- `loading_a_bank_while_the_session_is_dirty_takes_two_clicks`.
- `a_drum_bank_on_an_acid_device_loads_and_says_so` — one notice, and the patterns are there.
- `deleting_a_bank_takes_two_clicks_and_removes_the_file`.
- `the_rack_keys_refuse_and_write_nothing` — `save_rack` / `load_rack` do not exist as functions; the notice names the later
  round; no dialog is opened (the test asserts on the `Request` and on `main`'s notice, not on a file).

Mutations (a `SKIP` is never a pass): save always making a new file; the arm skipped while dirty; `load_bank` not normalising;
the delete arm firing on one click; the `kind` notice dropped; a rack key that opens a dialog.

## Tasks

### Task 1 — the bank document

**Files:**
- Modify: `rust/crates/acid-app/src/persist.rs` (`BankDocument`, the seven functions, `song_name` → `document_name`)
- Modify: `rust/crates/acid-app/src/ui/banks.rs` (the one `song_name` call site)
- Test: `rust/crates/acid-app/tests/persist.rs`

**Interfaces:**
- Consumes: `SAVE_VERSION`, `PatternBank`, `write_atomic`, `free_name`.
- Produces: `pub struct BankDocument { pub version: u32, pub kind: String, pub bank: PatternBank }`;
  `pub const BANKS_DIR: &str = "banks"`; `save_bank(dir, name, doc) -> Result<()>`;
  `load_bank(dir, name) -> Result<BankDocument>`; `list_banks(dir) -> Vec<String>`; `delete_bank(dir, name) -> Result<()>`;
  `bank_path(dir, name) -> PathBuf`; `free_bank_name(dir, wanted) -> String`;
  `pub fn document_name(typed: &str) -> Option<String>` (renamed from `song_name`, every call site updated).

1. Write `a_bank_document_round_trips`, `a_free_bank_name_is_the_wanted_one_or_a_numbered_neighbour` and the renamed
   `a_document_name_has_to_be_a_name_and_not_a_path`; run — expected FAIL to compile.
2. Rename `song_name` to `document_name` at its three call sites in `persist` and one in `ui::banks`; run the suite — expected
   PASS (a pure rename, so every existing test is the proof).
3. Implement the bank functions by the shape of their song twins: `write_atomic`, `create_dir_all`, `bank.normalise()` on read,
   `free_name(wanted, |name| list_banks(dir).iter().any(|n| n == name))` for the free name.
4. Run — expected PASS.
5. Gates and commit.

### Task 2 — the card's four sections

**Files:**
- Modify: `rust/crates/acid-app/src/ui/banks.rs` (`Banks` fields, `paint`, sections, keys, `Request`)
- Modify: `rust/crates/acid-app/src/ui/mod.rs` (`SongState::bank_file`)
- Test: `rust/crates/acid-app/tests/banks_live.rs` (new)

**Interfaces:**
- Consumes: `persist::list_banks` (by way of `Banks::bank_names`, which `main` fills like `names`).
- Produces: `Banks { bank_names: Vec<String>, bank_name: String, .. }`; `SongState::bank_file: Option<String>`;
  `banks::Request::{SaveBank(String), LoadBank(String), DeleteBank(String), SaveRack, LoadRack}`; `paint` gains
  `bank_file: Option<&str>` after `session_dirty`.

1. Write the card-level tests: `the_card_lists_the_four_sections_and_asks_for_a_bank_load` (drive `draw_panel`, click a
   `PATTERN BANKS` row, assert `Request::LoadBank`), `a_dirty_session_arms_a_bank_load_first`, `the_rack_keys_ask_and_write_nothing`,
   `saving_a_bank_asks_with_the_typed_name`. Run — expected FAIL to compile.
2. Rename the `MY BANKS` heading to `SONGS`, add the `PATTERN BANKS` section with its rows, cross and field/`SAVE BANK` row, and
   the `RACKS` section's two keys. Reuse `section`, `load_row`, `guarded_load`, `card_button` and the `list_limit` rule; a
   section that does not fit is not drawn, heading included.
3. Add the five `Request` variants and the arm keys (`bank:<name>`) in the shared `load_armed`/`delete_armed` fields.
4. Run — expected PASS, and every existing `banks` card test still passing is the proof the ids did not move.
5. Gates and commit.

### Task 3 — `main` answers the card

**Files:**
- Modify: `rust/crates/acid-app/src/main.rs` (the card's request drain, `load_bank_now`, `panel.songs.bank_file` bookkeeping)
- Test: `rust/crates/acid-app/tests/banks_live.rs`

**Interfaces:**
- Consumes: Task 1's functions, Task 2's requests.
- Produces: `fn load_bank_now(&mut self, dir: &Path, name: &str)`; `fn save_bank_now(&mut self, dir: &Path, typed: &str)`.

1. Write the file-level live tests: `saving_a_bank_through_the_card_writes_the_file_and_lists_the_row`,
   `saving_a_bank_twice_under_its_own_name_rewrites_one_file`,
   `loading_a_bank_replaces_all_sixty_four_patterns_and_marks_the_session_dirty`,
   `loading_a_bank_while_the_session_is_dirty_takes_two_clicks`, `a_drum_bank_on_an_acid_device_loads_and_says_so`,
   `deleting_a_bank_takes_two_clicks_and_removes_the_file`. Run — expected FAIL.
2. Implement `save_bank_now` (`save_target` on `panel.songs.bank_file`, `free_bank_name` for a new name, then
   `persist::save_bank(&dir, &name, &doc)` with `kind` from `rack.device().kind().name()`), `load_bank_now` (kind mismatch →
   `kit_notice`-style notice, then `Device::load_bank`, `pattern_swap::adopt`, `panel.dirty = true`), and the delete arm.
3. Wire the two rack requests to a notice naming plan `02`'s later round — `"a rack file is not saved yet"` and
   `"a rack file does not load yet"`. No dialog, no path function.
4. Run — expected PASS. Gates and commit.

### Task 4 — the book, the pins and the pictures

**Files:**
- Modify: `docs/guide/src/12-songs-and-demos.md`, `20-reference.md`
- Modify: `docs/guide/check.sh`
- Modify: `docs/guide/src/img/README.md` and the `banks-card` pictures (`docs/guide/src/img/banks-card-*.png`)
- Modify: `plans/phase 5/00-phase-5.md`

1. Page 12: the three documents — a bank, a rack, a song — what each is, that `PATTERN BANKS` loads over the selected device's
   bank (and that the demos already did this), and the two `RACKS` keys with what they say today.
2. Page 20: the files table gains `banks/<name>.json` (a bank's sixty-four patterns and the kind it was written from), and the
   `DEMO BANKS` row's wording is checked against the card.
3. `check.sh`: pin `PATTERN BANKS` and `SAVE RACK`, drop any pin that names `MY BANKS`, and add a negative guard for the old
   heading so it cannot come back.
4. Retake the three `banks-card-*` pictures by the recipe in `AGENTS.md` (fresh `TAKKA_ACID_DATA_DIR`, display checked at
   2000 × 1400, window `1960x1340`, pointer parked), and update their rows in `img/README.md`.
5. `./docs/guide/check.sh` — expected ok. Gates and commit.

## Non-goals

A rack document that loads; a pattern file (`clipboard.rs` keeps that job); bank reordering, renaming or merging into a bank;
loading a bank into a device other than the selected one; the `DEMO BANKS` behaviour, which does not move; a screenshot of the
`RACKS` refusal (the notice is the status line, and the card's own picture shows the keys); every `SAVE_VERSION` refusal case.

## Review focus

- The card at the **smallest window**: four sections and a footer in 670 design pixels. `PATTERN BANKS` must either fit or not be
  drawn — a heading over nothing is the bug, and the picture is how it is seen.
- A bank named like an existing **song**: the two lists share a folder root and must not share a file. `banks/Easy.json` and
  `songs/Easy.json` are different documents with the same name.
- A bank file whose `kind` caption this build does not know: it must load, and the notice must name the caption it read.
- A bank load **while the device card is open**, or while a pattern swap is pending: `pattern_swap::adopt` must leave nothing
  pointing at the bank that was replaced.
- Deleting the bank the selected device came from: `SongState::bank_file` must be cleared, or the next `SAVE BANK` writes the
  deleted file in place.

## Progress

- [x] **Task 1 — the bank document.** `BankDocument { version, kind, bank }` with `BankDocument::of(&PatternBank, DeviceKind)`
      as the one place one is built; `BANKS_DIR`, `bank_path`, `list_banks`, `save_bank` (through `write_atomic`, establishing
      `banks/` first), `load_bank` (normalises the patterns, stamps `SAVE_VERSION`), `delete_bank` and `free_bank_name` (through
      `free_name`, so a bank cannot destroy another by being given its name). `persist::song_name` became
      `persist::document_name` — it already validated kit file names as well as songs — with every call site updated
      (`save_song`, `delete_song`, `save_target`, `free_name`, `ui::banks` twice), and the test for it renamed.
      **Tests:** `a_bank_document_round_trips`, `a_bank_without_a_usable_name_is_refused_and_a_taken_one_lands_beside_it`,
      `deleting_a_bank_removes_its_file_and_refuses_a_bad_name`, and `a_document_name_has_to_be_a_name_and_not_a_path`.
      **Mutations: 3 of 3 caught, 0 skipped** (`.superpowers/mutate_phase5_02_task1.py`) — the kind always an acid, an unusable
      name written anyway, a taken name returned as it is. The first run **skipped** one because its target line also appears in
      `saved_device_from`; the mutation was narrowed until it applied exactly once, because a `SKIP` is never a pass.
      Gates: `cargo test --workspace` **923**, clippy `-D warnings`, `cargo fmt --check`.
- [ ] **Task 2 — the card's four sections.**
- [ ] **Task 3 — `main` answers the card.**
- [ ] **Task 4 — the book, the pins and the pictures.**
