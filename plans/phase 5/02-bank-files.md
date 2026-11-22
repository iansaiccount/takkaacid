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
- [x] **The branch review of rounds `01` + `02/task 1`** (a subagent that wrote neither, against `1ad1918..fe4dba3`, as the
      player asked before task 2). Seven findings, all acted on:
      1. **Important — page 12 still taught the one-device format** (`A song is the first device … the picker says so`), and the
         `check.sh` guard for that sentence was defeated by a line wrap: the phrase was printed across two lines, so a
         line-based `grep` never saw it. Page 12's `One song, one rack` is now `One song, the whole rack` and says every machine
         is in the file; **every negative guard in `check.sh` now reads the book with its line breaks flattened**, through one
         `gone`/`gone_re` pair, so a wrapped lie cannot come back green.
      2. **Important — the launch seeded the session's file from `saved.song`, which is a display name.** A demo or a `NEW` song
         has a name with nothing on disk behind it, so the first `SAVE` after a restart would have written `songs/<that name>.json`
         over whatever the player had there. `persist::session_song_file(dir, song)` is now the one answer: the name, when a file
         of that name is in the library.
      3. **Important — `Rack::rename` kept its own copy of the `(N)` suffix rule** while `persist::distinct_name`'s doc and plan
         01's Progress both claimed one. `rename` calls `distinct_name` now.
      4. **Important — `persist::wanted_kits` asked "does this kind have a kit" by comparing to `Drums`**, the second answer
         `DeviceKind::has_kit` exists to prevent. It asks `has_kit` now.
      5. **Minor, fixed — a song saved under a new name recorded the old one**, because `write_song` built the document before
         setting the name. `save_song` stamps `song` from the name it wrote to, so no caller can get it wrong.
      6. **Minor, fixed — page 22 called `808` "the first kit that ships"**, the table-order reason the 808 constant removed.
      7. **Minor, fixed — page 19's removal sentence contradicted itself** ("nothing on disk changes … is a change to the file").
      Two more the fix uncovered, both the same class as finding 1: `main`'s removal arm still set `dirty` **only for device 1**
      (true when a save held one machine, false since round 01) and its add arm still said a device is "not in the save format
      yet"; `state.rs`'s `kind` doc still said a session saves its first device and loads it as a 303. All three are rewritten.
      **Also recorded:** plan 01 promised `a_restart_then_save_keeps_the_same_file` and it does not exist. `main` has no harness,
      so the decision moved into `persist` instead, and is tested there (`a_session_claims_a_song_file_only_when_the_file_is_there`,
      `a_saved_song_names_the_file_it_lands_in`); what stays untested is `main`'s one line that calls it, the same cost round 01
      already took for `wanted_kits` and `unopened_ports`.
      **Mutations: 4 of 4 caught, 0 skipped** (`.superpowers/mutate_phase5_review.py`) — `has_kit` answering false for drums, a
      name with no file claiming one, a song keeping its old name, the suffix limit widened so `rename`'s own rule would return.
      Gates: `cargo test --workspace` **926**, clippy `-D warnings`, `cargo fmt --check`, `./docs/guide/check.sh` ok.
- [ ] **Task 2 — the card's four sections.**
- [x] **Task 2 — the card's four sections.** `Banks` gained `bank_names` and `bank_name`; `SongState` gained `bank_file`; `Request`
      gained `SaveBank`, `LoadBank`, `DeleteBank`, `SaveRack` and `LoadRack`. The card is now `DEMO BANKS`, `PATTERN BANKS` (a row
      per bank file with its `x`/`SURE` cross in the shared `delete_armed` under `bank:<name>`, then a name field and `SAVE BANK`
      under the rows), `SONGS` (the old `MY BANKS`), and `RACKS` with `SAVE RACK` / `LOAD RACK`. `MY BANKS` is gone. Every new key
      has its stable id — `bank-{index}`, `bank-delete-{index}`, `bank-save`, `rack-save`, `rack-load` — and the bank load shares
      `load_armed` under the same `bank:<name>` key, so one click at a time is still the rule. A section that does not fit is not
      drawn, heading included; the bank's save row is drawn whether or not a bank file exists, because it is how the selected
      device's bank gets its first file.
      **Tests (new `tests/banks_live.rs`):** `the_card_lists_the_four_sections_and_asks_for_a_bank_load` (clicks a row of all
      three lists — each click lands only if the sections above it were drawn), `a_dirty_session_arms_a_bank_load_first`,
      `the_rack_keys_ask_and_write_nothing`, `saving_a_bank_asks_with_the_typed_name`,
      `a_blank_bank_field_saves_the_bank_the_device_came_from`. Every existing `songs_live` test passes untouched in behaviour —
      which is the proof the ids did not move — with its hand-computed geometry moved: the helpers now measure from `songs_top()`
      past three headings and the bank's save row, as the handover said they would have to.
      **Rulings, and what each costs if wrong:**
      1. `paint` takes one `banks::Session { name, dirty, bank_file }` rather than an eighth argument. Clippy's
         `too_many_arguments` is a real signal here, and the three are three readings of one thing. Cost: a caller that wants one
         field writes three lines instead of one.
      2. The five new `Request` variants get their arms in `main` **in this task**, because the drain matches the enum in full and
         nothing compiles without them. The bank arms say `a bank is not saved yet` and its two siblings until task 3 replaces
         them with the real work; the two rack arms already say what they will always say.
      3. **A test click needs a hover frame before it presses.** egui only reports a click on a widget it saw hovered, so a click
         sent as press-then-release alone is silently lost whenever the row was not under the pointer in the previous frame. That
         is what made this task's first run flaky — a row hit at one x and missed 20 px along — and `click()` in
         `banks_live.rs` now moves the pointer for a frame first. `songs_live`'s helper still works because its clicks happen to
         follow one another over the same rows; it is left alone rather than churned in a card task.
      4. **The book is task 4's, as planned.** Page 12 still says `MY BANKS` where the card now says `SONGS`, and says nothing
         about `PATTERN BANKS` or `RACKS`; it is rewritten in task 4 with the three documents, and this branch is one push, so no
         push carries the contradiction. Cost if the round were pushed between these two commits: a page naming a section the
         card no longer prints.
      Gates: `cargo test --workspace` **931**, clippy `-D warnings`, `cargo fmt --check`, `./docs/guide/check.sh` ok.
- [ ] **Task 3 — `main` answers the card.**
- [x] **Task 3 — `main` answers the card.** `save_bank_now` asks `save_target` where the file lands (in place on the bank's own
      name, `free_bank_name` otherwise) and writes `BankDocument::of(rack.bank(), rack.device().kind())`; `load_bank_now` writes
      the session first, gives the selected device the whole bank (`Device::load_bank`, then `pattern_swap::adopt` against the new
      bank, `pending_pattern` dropped, `dirty` set) and says one line naming the file — or the mismatch. The delete arm removes the
      file, refreshes the card's list, and **clears `SongState::bank_file` when it was that bank**, so the next `SAVE BANK` cannot
      write the deleted file back in place (plan 02's review focus). `bank_file` is cleared wherever the selected device's bank
      stops being that file: `NEW`, a demo (either face), a song load, and a change of selection. The two rack keys keep their
      notices — they are stubs this phase.
      **The one new decision, and where it lives:** the mismatch notice is `persist::bank_kind_notice(written, playing)`, tested
      there because `main` has no harness. It returns `None` when the captions agree, names both machines when they do not, and
      names the caption **as read** when this build does not know it.
      **Tests:** `a_bank_from_another_kind_is_a_notice_naming_it` and `saving_a_bank_twice_under_its_own_name_rewrites_one_file`
      (`tests/persist.rs`); `deleting_a_bank_takes_two_clicks`, `a_saved_bank_is_a_row_the_card_lists_and_loads` and
      `loading_a_bank_gives_the_device_all_sixty_four_patterns` (`tests/banks_live.rs`). The last three went green on their first
      run, because task 1 built the file and task 2 the card — they are integration coverage of what already existed, not new
      behaviour, and the only thing that failed first was the notice above. Recorded rather than dressed up as RED.
      **Mutations: 5 of 5 caught, 0 skipped** (`.superpowers/mutate_phase5_02_task3.py`) — a save that always makes a new file,
      `load_bank` not normalising, the mismatch never said, an unknown caption dropped instead of named, the delete cross firing
      on one click. **The first run missed one and it was the script's fault:** the "caption dropped" mutation still built the
      same sentence, so nothing could fail; rewritten to actually drop the caption, it bit. The `normalise` mutation also had
      nothing to bite, because no test had ever loaded a hand-edited bank — `a_bank_file_with_fewer_patterns_opens_as_sixty_four`
      was added for it, and is flagged in its own doc comment as written after the gap was found.
      Gates: `cargo test --workspace` **937**, clippy `-D warnings`, `cargo fmt --check`, `./docs/guide/check.sh` ok.
- [ ] **Task 4 — the book, the pins and the pictures.**
