# Song saving, the songs card, and import/export — design

Date: 2026-11-08
Status: awaiting approval
Approach: **C — one song surface over the existing library, with native dialogs only for Import/Export** (chosen in
conversation over *A: library-only rework* and *B: files-first, songs are documents*).

## Intent

The song mechanic is the part of the instrument a player meets before they can play it, and it is currently the least
legible part of the app. Five specific complaints drove this:

1. **The panel forgets what you are in.** Close the card and nothing on the face names the song; only the `SAVED`/`UNSAVED`
   lamp survives.
2. **Three naming actions, two indistinguishable.** `SAVE AS` writes a copy under the typed name; `RENAME` moves the file
   the session is on. Nothing on the card says which is which, and the name field doubles as both.
3. **`NEW SONG` is destructive and unguarded** — it discards the name and loads the starter phrase with no confirmation,
   from the corner furthest away.
4. **Feedback dies with the card.** "loaded Slow Acid: rename it to keep it" is written to the card and the card closes.
5. **The library is a closed box.** Songs exist only inside `~/.local/share/takka-acid/songs/`: no sharing, no backup, no
   "open the file someone sent me".

The goal is a surface where the *state* is always visible, there is **one save**, destructive things ask twice, and a song
can leave and re-enter the app as a file — without turning the app into a file manager.

## Goals

- The song's name and saved-state are visible on the panel at all times, not only inside the card.
- One primary action on the card (`SAVE`); one advanced action (`SAVE A COPY`); no separate rename.
- Any action that discards unsaved work asks a second time.
- Import and Export are the only places a file dialog appears; everyday use never sees one.
- The song format is unchanged (`SavedState` JSON), so an exported song is the same object as a library song.

## Non-goals

- No files-first model: no "recent files" list replacing the library, no Save-As-as-the-default-save, no rename or delete
  through the OS.
- No new file format, no version bump beyond what the recents/UI needs.
- No change to patterns, patches, MIDI, or the demo list (plan 33).
- No multi-document windows, no tabs.

## The journey after this change

**On the panel, always:** the top strip gains `SONG <name>` beside the pattern readout (dim silk when untitled), so the
song is named where the pattern is named. The `SAVED`/`UNSAVED` lamp keeps its place.

**The card** (opened by `SONGS`, closed with `CLOSE` or Escape):

```
SONGS                                       NOW: Slow Acid · UNSAVED
[ name field ......................... ]    [ SAVE ]   [ SAVE A COPY ]

DEMO SONGS
  Slow Acid        Hard Techno Acid        Blues        Rock        Funk
MY SONGS
  Acid 1          Acid 2          Sunday Funk

[ NEW ]   [ IMPORT… ]   [ EXPORT… ]                                   [ CLOSE ]
```

- **Header** names the loaded song and its state, so the card's own state is legible even at a glance.
- **`SAVE`** writes the session and the library song. If the name field is empty, `SAVE` focuses it instead of failing
  silently. If the typed name differs from the loaded song, `SAVE` writes a **copy** under the typed name and says so
  ("saved as …") — it never silently moves a file, which removes `RENAME` from the vocabulary.
- **`SAVE A COPY`** is for the deliberate duplicate; it is the same action as changing the name, made explicit.
- **Demos** stay pinned above the player's songs and load as "New song", unsaved (plan 33 behaviour).
- **Rows**: a click loads; replacing a dirty session takes a second click (existing guard). Deletion keeps the two-click
  `x`/`SURE`. No rename on rows.
- **`NEW`** asks twice when the session is dirty.
- **`IMPORT…` / `EXPORT…`** open the native dialog: import copies a chosen `.json` song into the library (name from the
  file, made unique if taken) and loads it; export writes the current session to a chosen path.
- **Messages** — loaded, saved, imported, failed — move to the panel's status strip, where they survive the card closing.

**Outside the card:** `SAVE` (switches row) is the same action as the card's `SAVE`. `AUTO` keeps writing on the interval
it does now, and therefore keeps the song file current too.

## Approaches considered

| Approach | What it gives | Why not |
|---|---|---|
| **A** — library-only rework | Fixes naming, visibility, feedback; no new dependency | Songs stay trapped in the app folder; the sharing complaint (5) survives |
| **B** — songs are documents | Everything, and the familiar DAW model | Replaces list/delete/rename with OS dialogs, drops the library the demos live in, and asks the player to manage paths for everyday play |
| **C** — library + import/export (chosen) | A is fixed, and a song can leave and return as a file; the library stays the everyday surface; dialogs appear only when you deliberately reach for a file | Two models coexist (library + files), so the card must say which is which: solved by `IMPORT`/`EXPORT` being the only doors |

**Dialogs.** `rfd` (blocking `FileDialog`) with the **XDG-portal** backend on Linux so no GTK development package is
required. If the portal backend cannot build in this environment, the fallback is `rfd` with its GTK3 backend (needs
`libgtk-3-dev`) — and only if neither builds, a minimal in-app directory browser, which would be a follow-up plan rather
than a silent substitution. **This call is the builder's**, not the player's: it changes a dependency, not the journey.

## Constraints

- Dialogs block the UI thread while open. That is acceptable for a modal file dialog, and it must never run on the audio
  thread — it is opened from `main`'s frame, after the request is read from the card, never from the renderer.
- Nothing opens a dialog in a test: `persist::import_song(from, dir)` and `persist::export_song(to, state)` take paths, so
  they are testable headlessly and the dialog only supplies a path.
- The session file (`state.json`) remains the scratch session: recents, last patch, settings. A song file is a document.
- Exported files are the same `SavedState` JSON the library holds, so an export can be dropped into the library by hand.
- Existing library songs and the demos keep working untouched; `SAVE_VERSION` only moves if a saved field is added.

## The parts the player decided

- Approach **C**, with the file dialogs confined to import/export.
- The library stays the everyday surface; songs are not loose documents.

## What is left to the builder

- The card's exact geometry and copy, including whether `MY SONGS` needs a scroll region once it grows.
- How a recents-less import names a colliding file (`Acid 1 (2)` vs refuse-and-ask).
- Where the "now" line sits relative to the tooltips/help keys in the top strip.
- The `rfd` feature set and its build consequences (see Dialogs above).
- Test shape: persist-level import/export tests, card-level request tests, and a live test that a save with a changed name
  writes a copy rather than moving the file.
