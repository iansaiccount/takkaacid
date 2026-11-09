# Plan 34 — the song card you can read, and import/export

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`. One green commit per task, on
> `Deepseek-Version`. Design: `docs/superpowers/specs/2026-11-08-song-save-card-design.md` (approved).

**Goal:** make the song mechanic legible — the panel always names the song, one `SAVE` (a changed name means a copy, never a
move), `NEW` asks twice, messages survive the card, and `IMPORT…`/`EXPORT…` are the only file dialogs in the app.

**Architecture:** `rfd` 0.18 with the **XDG-portal** backend (builds here, no GTK). The dialogs live only in `main`; the
file work is in `persist` as plain path functions, so it is testable headlessly. The card keeps its two lists and gains a
header, two save buttons and two file buttons. Notices move from the card to `PanelState`, drawn on the status line.

**Constraints:** no format change (`SavedState` JSON); no `SAVE_VERSION` bump; everything else unchanged.

## Task list

1. **`persist` learns files, and where a save goes.** `export_song(to, &SavedState)`, `import_song(from, dir) -> Result<String>`
   (sanitises, uniquifies `X (2)`), and the pure `save_target(current: Option<&str>, typed: &str) -> SaveTarget`
   (`InPlace(name)` | `NewFile(name)`). Tests: export→import round trip; a colliding import lands as `Acid 1 (2)`; a file
   that is not a song errors without writing; `save_target` returns in-place only for an identical name. **Green commit.**
2. **The card.** Header `NOW: <name> · SAVED/UNSAVED`; `SAVE` and `SAVE A COPY` replace `SAVE AS`/`RENAME`; `IMPORT…` and
   `EXPORT…` join `NEW`/`CLOSE`; `Request::{Save, SaveCopy, Import, Export}` (Rename removed); `NEW` uses the same
   two-click guard as load. Tests in `songs_live`: the header shows the name and state; SAVE/SaveCopy ask for their
   requests; NEW on a dirty session arms then fires; import/export ask for their requests. **Green commit.**
3. **The panel names the song, and keeps the message.** `panel::song_label_rect` + `SONG <name>` in the top strip (dim
   silk when untitled); `PanelState::notice: Option<(String, Instant)>` replaces the card's message, drawn on the status
   line for a few seconds and clearing itself. Tests: the label rect clears the displays, the lamp and the keys; a notice
   replaces the MIDI status and expires. **Green commit.**
4. **`main` wires the four actions.** `Save` → `save_target`; `SaveCopy` → always a new file; `Import`/`Export` open the
   dialog, call the persist function, refresh the card list and set the notice; every message that used to go to the card
   now goes to the notice. `README.md` gains the save model; `AGENTS.md` gains the "dialogs only in main, paths in persist"
   rule. **Green commit.**

## Non-goals

- No files-first model, no recents list, no rename in the app.
- No change to demos, patterns, MIDI or autosave behaviour.


## Progress

- [x] **Task 1 — `persist` learns files, and where a save goes.** `SaveTarget`, `save_target`, `export_song`,
      `import_song` (+ private `unique_name`) in `crates/acid-app/src/persist.rs`; four tests in `tests/persist.rs`
      (round trip, collision lands as `Acid 1 (2)`, junk file writes nothing, in-place only for an unchanged name).
      Gates green: 34 persist tests, workspace clippy clean.
- [ ] **Task 2 — the card.**
- [ ] **Task 3 — the panel names the song, and keeps the message.**
- [ ] **Task 4 — `main` wires the four actions.**
- **Build feasibility checked:** `rfd` 0.18 with `default-features = false, features = ["xdg-portal"]` compiles on this
  machine with no GTK development package. The dependency is added in Task 4, where it is first used.
- [x] **Tasks 2 and 4 — the card, and `main` wiring them.** Header `NOW: <name> · SAVED/UNSAVED`; one `SAVE` and one
      `SAVE A COPY` in place of `SAVE AS`/`RENAME`; `IMPORT`/`EXPORT` in the footer beside a `NEW` that now asks twice while
      the session is unsaved. `Request::{Save, SaveCopy, Import, Export}`; `Rename` is gone. `main` saves through
      `persist::save_target` (unchanged name writes in place, a new name is a new file), copies through `free_copy_name`
      (never over its source), imports by copying in and then loading, exports through the dialog. The `Load` arm became
      `load_song_now`, and the session-on-disk snapshot became `session_for_disk`, so a load and an import land the same
      way. `rfd` 0.18 (XDG-portal) added. Tests: `songs_live` 16 pass, including SAVE on a typed name, SAVE on a blank
      field keeping the open song, SAVE A COPY, IMPORT/EXPORT, and NEW arming then firing.
- [x] **Task 3 — the panel names the song, and keeps the message.** `panel::song_label_rect` takes the left half of the
      status band; `SONG <name>` is drawn there, dimmer when the session is untitled and brighter once the song is a file.
      `PanelState::notice: Option<(String, Instant)>` with `notify`/`notice_now`/`NOTICE_FOR` (4 s) replaces the status
      line while it lasts, clears itself, and survives the card closing; `main`'s `notice` and every message site (save,
      save a copy, load, delete, import, export) now go through it. The card keeps its own "click again" hint, which belongs
      to the card and nowhere else. Tests: `tests/notice_live.rs` (4) — the label clears the displays and the wordmark, a
      notice shows and expires, survives a frame with no card, and drawing one changes nothing.
- [x] **Task 4, the words.** `README.md` gains the save model (one SAVE, in place vs new file, SAVE A COPY, IMPORT/EXPORT,
      NEW asking twice, notices that fade) and `AGENTS.md` gains the two rules the code now depends on: dialogs only in
      `main` with the work in `persist`, and a message is a self-clearing notice rather than a dialog.

## Done

All four tasks landed on `Deepseek-Version` in four commits (`7cd1ed4`, `24a35bb`, `93df5dd`, plus the docs commit).
Gates are green: `cargo test --workspace` (59 suites), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`. `SavedState` is unchanged, so no `SAVE_VERSION` bump.

One deviation from the plan, recorded on purpose: tasks 2 and 4 landed together, because the `Request` enum and its handler
in `main` are one change and splitting them would have left a commit that does not compile.
