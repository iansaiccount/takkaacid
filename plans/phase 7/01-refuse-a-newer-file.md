# Plan 01 — refuse a file this build is older than

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** a save file, a song or an imported song whose `version` is **greater** than `SAVE_VERSION` is refused with the
version and the file named, instead of being relabelled and rewritten by the next save. The app still always opens: the
refused session is a demo as a first run, says what happened on the status line, and **does not autosave over the file it
refused**.

**Spec / wishlist:** `plans/future/wishlist.md`, *A file from a newer build is refused, not relabelled*; the risk note in
`docs/superpowers/specs/2026-12-01-pattern-grids-design.md` (*Risks*) that names this as the next round's.

## What is wrong today

- `persist::load` parses a `SavedState` and returns it. `upgraded` then stamps `SAVE_VERSION` over `state.version`
  (`persist.rs:280-300`), so a version-14 document is a version-13 one from that moment, with whatever fields this build
  does not know already dropped by serde.
- `open` treats every failure the same: `eprintln!` and `demo::seed()` with `first_run = true` (`persist.rs:555-568`). A
  refused file is therefore indistinguishable from a first run, and a first run is a session with nothing on disk — which
  is exactly the state autosave is allowed to write from.
- `load_song` (`:766`) and `import_song` (`:825`) both `upgraded()` whatever they parse, so the same loss reaches the two
  paths a player uses to bring a file in.
- Nothing compares the number with `SAVE_VERSION` before overwriting it. Phase 6's version **13** and the grid field are
  the concrete case: a version-14 file with a fourth grid reads as `1/16` and loses its steps.

## The design

```rust
// acid-app/src/persist.rs

/// Why a document could not be read.
#[derive(Debug)]
pub enum LoadError {
    /// Written by a newer build: its `version` is greater than this one's. The file is untouched.
    Newer { version: u32, path: PathBuf },
    /// Anything else: a missing permission, a truncated file, JSON this build cannot parse.
    Other(anyhow::Error),
}
impl From<anyhow::Error> for LoadError { /* Other */ }
impl std::fmt::Display for LoadError { /* names the version and the path, or the inner error */ }

/// Read the save file. `Ok(None)` is nothing saved yet; `Err(Newer { .. })` is a document this build must not touch.
pub fn load(dir: &Path) -> Result<Option<SavedState>, LoadError>;
pub fn load_document(bytes: &[u8], path: &Path) -> Result<SavedState, LoadError>; // the one place the version is checked

/// What a launch opened.
pub struct Opened {
    pub state: SavedState,
    /// Nothing was on disk: the session is a demo, and it is unsaved.
    pub first_run: bool,
    /// A newer document was refused, with the version it named. The file is still there and must not be overwritten.
    pub refused: Option<u32>,
}
pub fn open(dir: &Path) -> Opened;
```

**The rule, in one place.** `load_document` parses `{version}` first (a `#[derive(Deserialize)] struct Version { #[serde(default)] version: u32 }`
pass, so a document that will not parse is `Other`, not `Newer`), and returns `Newer { version, path }` when
`version > SAVE_VERSION`. `load`, `load_song` and `import_song` all go through it, so the comparison exists once.

**`open`'s second face.** `Err(Newer { version, .. })` answers `Opened { state: demo::seed(), first_run: true, refused: Some(version) }`.
`Err(Other)` keeps today's behaviour exactly.

**`main`'s three duties** (`main.rs:25`): re-read the state as `Opened`; put the refusal on the status line as a notice
("the save file is version 14, newer than this build - opened a demo instead, and it has not been saved over your file");
and keep the session **out of the autosave path** while `refused.is_some()`, because one autosave tick would destroy the
document the round exists to protect. Saving by hand still works and overwrites deliberately (the player has been told);
`SAVE` clears `refused`.

`load_song` / `import_song` answer the same `LoadError`, so the two dialogs' existing error path prints "version 14 is
newer than this build". `import_song` keeps its rule that it never lands on a taken name.

**No `SAVE_VERSION` bump.** The format is unchanged; this is a reader's rule. A test pins 13 is still 13.

## Tests that make it safe

- **`persist`**: a version-`SAVE_VERSION + 1` document is `Err(Newer { version, path })`; a version-13 document still
  loads; a document with no `version` field loads as 0 (as today); a truncated file is `Other`, not `Newer` (a broken
  file must not be reported as somebody's newer one); a hand-edited `999` is refused naming 999.
- **`open`**: on a newer file it answers the seed with `first_run` and `refused == Some(14)`, and **the bytes on disk are
  unchanged afterwards**; on a version-13 file it loads it as today.
- **the live panel** (`tests/newer_file_live.rs`): launching with a newer file puts the version on the status line; a
  dirty session and an autosave tick still leave the file byte-identical; pressing `SAVE` writes it (and the notice is
  gone the next frame because a notice clears itself).
- **`load_song` / `import_song`**: both refuse a newer song naming its version; `import_song` still lands beside a taken
  name.

## Tasks

### Task 1 — the comparison, in one place

**Files:** modify `rust/crates/acid-app/src/persist.rs`; test `rust/crates/acid-app/tests/persist.rs`.

**Interfaces:** produces `LoadError`, `load`'s new return type, `load_document`.

- [ ] **Step 1** Write the failing tests: a version-14 document is `Err(Newer { version: 14, .. })`; a version-13 one
      loads; a document with no `version` loads as version 0; a truncated document is `Other`.
- [ ] **Step 2** Run them and watch them fail (the return type has no `Newer` arm yet).
- [ ] **Step 3** Implement `LoadError` and `load_document`, and route `load` through it.
- [ ] **Step 4** Run them, expect PASS. Fix the `Display`/`From` impls until clippy is quiet.
- [ ] **Step 5** Gates and commit — `-m "feat(save): a file from a newer build is refused, not relabelled"`.

### Task 2 — `open`'s second face

**Files:** modify `persist.rs`; test `tests/persist.rs`.

**Interfaces:** produces `Opened { state, first_run, refused }`; consumes `LoadError`.

- [ ] **Step 1** Write the failing test: on a newer file `open` answers `refused == Some(14)`, `first_run`, and the
      demo's state; `std::fs::read` of the file is equal before and after.
- [ ] **Step 2** Run it, expect FAIL.
- [ ] **Step 3** Implement `Opened`; keep `Err(Other)`'s behaviour byte-for-byte.
- [ ] **Step 4** Fix the call sites (`main.rs:25`, `load_or_default`), run the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(save): a launch says which version it refused"`.

### Task 3 — the notice, and no autosave over it

**Files:** modify `rust/crates/acid-app/src/main.rs` (and any `first_run` reader); test
`rust/crates/acid-app/tests/newer_file_live.rs` (new).

**Interfaces:** consumes `Opened::refused`.

- [ ] **Step 1** Write the failing live test: a newer file on disk, a launched panel, the notice naming its version, a
      dirty session and a save tick leaving the file byte-identical, and `SAVE` writing it.
- [ ] **Step 2** Run it, expect FAIL.
- [ ] **Step 3** Implement: the notice, and the refused session held out of the autosave path until `SAVE`.
- [ ] **Step 4** Run it, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(panel): the status line says what it refused to open"`.

### Task 4 — the songs follow

**Files:** modify `persist.rs` (`load_song`, `import_song`); test `tests/persist.rs`.

- [ ] **Step 1** Write the failing tests: both refuse a newer song naming its version, and `import_song` still lands
      beside a taken name.
- [ ] **Step 2** Run them, expect FAIL.
- [ ] **Step 3** Route both through `load_document`.
- [ ] **Step 4** Run them, then the workspace.
- [ ] **Step 5** Gates and commit — `-m "feat(save): a song and an import refuse a newer version too"`.

### Task 5 — the book, the pin

**Files:** the page that describes saving (find it by grepping the guide for `state.json`/`SAVE`), `docs/guide/check.sh`.

- [ ] **Step 1** Add the sentence: a file from a newer build is refused, the app opens a demo, says so on the status
      line, and does not save over the file.
- [ ] **Step 2** Pin it in `check.sh`'s fact list (the words the page must keep saying), and pin the old behaviour gone
      if the book said it.
- [ ] **Step 3** `./docs/guide/check.sh` → `guide: ok`, gates, commit — `-m "docs: a file this build is too old for"`.

## Non-goals

- Migrating a newer document — impossible without its fields.
- `deny_unknown_fields` on `SavedState` — it would refuse every later addition.
- Bumping `SAVE_VERSION`; the format does not change.
- A dialog: the refusal is a notice, and a message in this app is never something that needs dismissing.

## Review focus

1. **The refusal never writes.** The one autosave tick that would destroy the file is the whole round; Task 3's live test
   is the guard, and it must read the bytes, not a flag.
2. **`open` still always opens.** A broken file keeps today's answer; only *newer* gets the second face.
3. **`import_song` never overwrites**, including on the refusal path.
4. **The comparison exists once**, in `load_document` — three readers, not three checks.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if
wrong. The branch review's findings and what was done about each go here too, before the pull request.)*
- **Task 1 — the comparison, in one place.** `persist::LoadError { Newer { version, path }, Other(anyhow) }` with
  `Display` naming the version and the file; `load_document(bytes, path)` probes `version` on its own first, so a file
  too broken to parse is `Other` and never reported as somebody's newer one, then compares once against `SAVE_VERSION`;
  `load` returns `Result<Option<SavedState>, LoadError>` and is one line over it. Proved by
  `a_newer_document_is_refused_with_its_version`, `a_current_and_an_unversioned_document_still_load` and
  `a_truncated_document_is_a_broken_file_not_a_newer_one`, watched fail first as `no LoadError in persist`.
  Workspace suite **979/979**, clippy `-D warnings`, `fmt --check`.
  **Ruling: the test asserts `SAVE_VERSION + 1`, not a literal 14.** Round 02 bumps the number to 14, and a test
  spelling the next version would have to be edited by a round it is not about. Cost if wrong: none — the refusal is
  relative to whatever this build writes, which is the rule.
- **Task 2 — `open`'s second face.** `persist::Opened { state, first_run, refused }`; a newer document answers the demo
  seed with `first_run` and `refused: Some(version)`, and every other error keeps today's answer byte for byte. `main`
  destructures it (the refusal is bound for Task 3). Proved by `a_launch_refuses_a_newer_file_and_does_not_touch_it`
  (the bytes on disk are compared before and after) and `a_launch_on_a_current_file_refuses_nothing`, watched fail first
  as `no field refused on type (SavedState, bool)`. Suite **981/981**, clippy `-D warnings`, `fmt --check`.
- **Task 3 — the notice, and no autosave over it.** `TakkaApp` holds `refused_version: Option<u32>` and the autosave tick
  is gated on `refused_version.is_none()`, so a refused session never writes itself; the panel shows
  `persist::refusal_notice(version)` on the status line at launch ("session file is version 14; this build reads 13 -
  opened a demo, file untouched"). Proved by `the_refusal_notice_names_the_version_and_the_promise` (the words, ASCII, in
  one place) and by Task 2's byte comparison. Suite **982/982**, clippy `-D warnings`, `fmt --check`.
  **Ruling: the plan's live test is not written, and this is why.** `TakkaApp` — and so the autosave tick and the
  launch-time notice — lives in `main.rs`, a **binary** no `tests/*_live.rs` can construct; the plan's Task 3 assumed a
  panel the way the other live tests have one. The two halves are covered where they can be: the refusal's *decision*
  by `Opens`' tests (the file is byte-identical after `open`), the refusal's *words* by the notice test, and the gate
  itself is one visible condition in the tick. Cost if wrong: the wiring between them (that `main` passes the field and
  keeps the gate) is read by the branch review rather than proved by a test — so it is named here, and the review focus
  keeps it first.
