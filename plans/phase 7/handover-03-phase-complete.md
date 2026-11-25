# Handover — phase 7 complete, phase 8 next

Written 2026-11-24, at the end of the session that built phase 7. Read this, then `plans/future/wishlist.md`, then the
plan you are picking up. `AGENTS.md` outranks everything below.

## Where the branch stands

`main` is at `e192c45` (phase 6, PR #13, merged). **`phase-7` is open as PR #14** — not merged as this was written:

https://github.com/iansaiccount/takkaacid/pull/14
It carries eleven commits on top of `main`: three planning commits, then one per task — round 01 (`ff8b4cd`, `3cc7d53`, `d8f3ad0`),
the player-reported grid bug and the file that carries held notes (`d01f8fa`, `6ebf28d`), the book (`c322027`), and the review
fixes (`6c29ac8`). Gates on its head: **985 tests**, clippy `-D warnings`, `fmt --check`, `guide: ok`.

**Phase 7 has had its branch review** (a subagent that wrote none of it) and every finding is fixed and recorded in the
plans' `## Progress` — including the P0 it caught, that quitting the app wrote over the very file the launch had refused.

## What phase 7 shipped, in one paragraph each

**Round 01 — the file refuses a newer build.** `persist::LoadError::{Newer, Other}` and one `load_document` that probes
`version` alone first (so a truncated file is `Other`, never "somebody's newer one"); `Opened { state, first_run, refused }`
from `open`; `TakkaApp.refused_version` gates the **autosave tick and `on_exit`**, and a successful `save()` clears it;
`refusal_notice(version)` says which version it found; `load_song` and `import_song` follow. `17-trouble` has the section
a player looks for, pinned in `check.sh`.

**Round 02 — a grid change keeps the notes it stops playing.** `with_grid` is `Self { grid, ..self }`: narrowing stops
playing the outer steps and keeps them. The invariant reads "not played", not "a rest". The file grows a `held` list,
written only up to the last **sounding** held step and skipped when empty (so an untouched pattern writes its version-13
bytes); `SAVE_VERSION` **14**. The notice says `32 TO 12 STEPS, 2 HELD`. `RANDOM` keeps held notes; `held_steps_beyond` is
the renamed counter.

## Things that will cost you time if you do not know them

- **`theme::GAP` is a design-space constant and does not scale with the panel**, so a 32-key row loses proportionally more
  width than the font does. That is why `step_key` now shrinks a note that would otherwise spill (phase 6's ruling, kept).
- **The save file is a contract with older builds.** A pattern with nothing held must write **no `held` key** — a test
  pins it, and `a_version_nine_file_upgrades_with_empty_drum_masks` catches the naive version (it counts masks per step).
- **`main.rs` is a binary no `tests/*_live.rs` can construct.** Anything in `TakkaApp` — the autosave tick, `on_exit`, the
  launch notice — is testable only by extracting the decision into `persist` (a two-line `fn session_writes(..) -> bool`
  would have caught the P0). Prefer the extraction; when you do not, say so in the plan's Progress.
- **A flaky test exists**: `the_audio_path_never_allocates` failed once under a full parallel `cargo test --workspace` and
  passed alone and on re-run. Re-run before believing it.
- **`cargo test --workspace | head` dies of SIGPIPE** — redirect to a file and sum `test result` lines.
- **The shell blocks heredocs into interpreters** (`python3`, `perl`) and `xargs | sh`. Write a script to a file and run it.
  `.superpowers/` is git-ignored scratch; the SDD ledger lives at `.superpowers/sdd/01-the-pattern-knows-its-bar/`.
- **Screenshots**: check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400` *before* launching (something
  pre-starts Xvfb at 1280×800), then `getwindowgeometry` must say `1960x1340`; a crop is a second grab, never a cut-out.

## What is next, and what is obvious

The wishlist was amended in this handover: *A file from a newer build is refused* is marked **done**, and *Pattern length
per device* is marked **answered by the player** — there is no length field, a "short pattern" is a bar whose later steps
hold nothing, and a narrower grid holds what it cannot play.

That leaves no ambiguity about the next round; take them in this order.

1. **Phase 8 — the rack document.** The largest remaining thing the plans name: a session that holds **every machine**, so
   a song is the whole instrument rather than one device's bank and knobs. Everything it needs is already in the file —
   `SavedState.devices` serialises each device with its own bank, knobs, routes and kit — but `open` still builds a rack
   from one device and the rest are re-created at launch. It wants a spec first (what a document *is*, what a bank is
   beside it, what `SAVE A COPY` means for a rack), then a plan in the phase-6/7 shape.
2. **Duplicate a device** (wishlist, *Panel*) — `Device::new` plus a copy of the source's bank, knobs and routes. Small,
   and worth doing only once the rack document decides what a rack *is*, because a duplicate is the first thing that
   makes two devices indistinguishable in a file.
3. **Drag to reorder devices** (wishlist, *Panel*) — rejected in phase 2 because the selection, the strip's cells and the
   input filters all key on position. Do it after the rack document lands, so a reorder is written once.
4. **The remaining seam phase 6 left**: `Sequencer::set_pattern`'s short→long wrap is asymmetric (a sixteen-step request
   released at step 15 begins a thirty-two-step bar on its second half, for good). Recorded in `00-phase-6.md` and
   PR #13's description, and **still the player's call** — it changes which step a swap into a longer bar is heard from.
5. **The ear check** on `Acid Jazz` and `Fast Acid` (`EAR_TESTING.md`) is still outstanding from phase 6.

## How to work (unchanged)

One task at a time, in plan order. For each: write the failing test, run it, implement, run it, keep the plan's
`## Progress` current — the test that proved it and any ruling with what it costs if wrong — and commit with the gates
green. The review is of the branch at the pull request, by a subagent that wrote none of it; it reports, the executor
fixes what it found and records it. Do not open a PR on the strength of the gates alone.
