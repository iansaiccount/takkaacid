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

## Status — design approved, implementation not started

- Design: `docs/superpowers/specs/2026-11-08-song-save-card-design.md` (approved by the player: approach C, import/export
  as the only dialogs).
- **Build feasibility confirmed:** `rfd` 0.18 with `default-features = false, features = ["xdg-portal"]` builds cleanly on
  this machine — no GTK development package needed. The dependency was added to verify that and then removed again, since
  no code uses it yet.
- **No product code landed.** The card rework (`SAVE`/`SAVE A COPY`, the `NOW:` header, `IMPORT`/`EXPORT`, `NEW` guard) and
  the `main` wiring were started and then reverted rather than left half-wired: the tree is green at `b658d19`. The work
  stopped at the session's context budget, not at a design problem.
- **What is left** is exactly the four tasks above, in order. Task 1 (`persist`: `export_song`, `import_song`,
  `save_target` + tests) is the smallest and unblocks the rest; Task 3 (the `SONG <name>` label and the status-line
  notice) is the part that fixes the two visibility complaints, and Task 4 wires the dialogs.
