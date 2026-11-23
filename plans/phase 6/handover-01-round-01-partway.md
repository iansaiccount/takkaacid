# Handover — phase 6, round 01 partway

Paste this whole file as the first message of the next session. It is written to be self-contained: that session has no
memory of the one that wrote it.

You are picking up **takka-acid**, a TB-303-style acid bass synth and drum machine in Rust, at `/home/ian/Projects/takkaacid`,
on branch **`phase-6`** (pushed, no PR yet), based on `main` at `ce81da4` — the merge of phase 5's PR #12. Read
`/home/ian/Projects/takkaacid/AGENTS.md` first; it is the project's standing instruction and outranks anything here that
contradicts it. Then read, in this order:

1. `plans/phase 6/00-phase-6.md` — what the phase is, and the rulings table with what each costs if wrong.
2. `plans/phase 6/01-the-pattern-knows-its-bar.md` — round 01. **Its `## Progress` is the record: Tasks 1–4 are done.**
3. `plans/phase 6/02-the-three-grids.md` — round 02, untouched. Four tasks.
4. `docs/superpowers/specs/2026-12-01-pattern-grids-design.md` — the design both plans argue from.

**Do not re-do anything in round 01's `## Progress`.** Trust it and `git log` over any recollection.

## Where the branch stands

`origin/phase-6`, five commits on top of `main`:

| commit | what |
|---|---|
| `7f8b4d7` | the spec and both plans |
| `659ff85` | `pattern::Grid` — a catalogue of three bars (16 / 32 / 12), serde by caption, unknown reads as `1/16` |
| `4f7e550` | `timing` divides a bar; `Clock` holds `steps_per_bar` and rescales the step in flight via the same `rescale` as tempo and swing |
| `5fcc4aa` | `Pattern { grid, steps: [Step; PATTERN_SLOTS] }`, `len()`, `empty_in`, `from_steps`/`from_steps_in`, `with_grid`, `iter`/`iter_mut`, hand-written serde `{ grid, steps }` writing exactly `len()` steps |
| `f765485` | `sounding_step(position, reversed, steps)`; `Sequencer::set_pattern` publishes the grid and wraps `position`; `swap_now(.., steps)` with `main` passing the **playing** bar's count |

Gates at `f765485`: `cargo test --workspace` **954**, clippy `--workspace --all-targets -- -D warnings`, `cargo fmt --check`
— all green. The guide has not moved (round 01 changes no pixels) and `check.sh` is untouched.

## What is left in round 01

**Task 5 — the panel asks the pattern.** `layout::step_cells(area, count) -> Vec<Rect>` (and `layout::STEP_COUNT` disappears);
the acid row in `ui/mod.rs` draws `state.pattern.len()` keys and loses its five `i % STEPS_PER_PATTERN` uses;
`step_key::step_view(.., grid: Grid)` derives `beat: grid.is_beat(index)`; the slide look-ahead is `(i + 1) % pattern.len()`;
`ui/selection.rs`'s two `STEPS_PER_PATTERN` call sites pass the pattern's len; `drum_panel::row_cells` / `cell_rect` /
`column_rect` take the count and the drum face's beat framing becomes `grid.is_beat(step)`. Live tests:
`step_mode_live.rs`, `playhead_live.rs`, `selection_live.rs`, `drum_live.rs`. **Nothing visible should change yet** — the
only grid any face can reach is still `1/16`.

**Task 6 — the rest of the app, and the file.** `SAVE_VERSION` → **13** (rename the pin test to
`the_save_version_is_pinned_at_thirteen`); clipboard `VERSION` → **2**; `state.rs`'s `set_step` bound becomes
`index >= self.pattern().len()`; `ui/randomise.rs` rolls `0..pattern.len()` and keeps the pattern's grid; `demo.rs`'s
`line`/`bar` build `[Step; PATTERN_SLOTS]` (still `Grid::Sixteenth` until round 02); then **delete
`pattern::STEPS_PER_PATTERN`**, whose only reason to exist is the bridge note in Task 3, and every use of it. New tests are
in the plan: the persist pin, a `1/32` pattern writing thirty-two steps, an unknown caption, a short list padding and a long
one truncating, and the clipboard round trip.

Round 02 (the `GRID` key taking one `TBC` cell with the other as its well, the row at 32 keys, the drum face at 12 and 32,
`Acid Jazz`, `Fast Acid`, the book and the pictures) starts only once round 01 is green.

## Things that will cost you time if you do not know them

- **`STEPS_PER_PATTERN` is a bridge, not a constant to use.** It is `Grid::Sixteenth.steps_per_bar()` — derived, not
  hand-kept — and Task 6 deletes it. It must not gain a new use.
- **`from_steps` / `from_steps_in` take `impl AsRef<[Step]>`, not `&[Step]`.** Pass the array as it is; `clippy::needless_borrow`
  fires on a stray `&` and the gate is `-D warnings`.
- **`Pattern::len()` carries `#[expect(clippy::len_without_is_empty, reason = …)]`.** Removing it breaks the gate.
- **The invariant that makes everything else work: every slot at or above `len()` is a rest.** Six test assertions already
  moved from `steps` to `iter()`/`len()` because `all(|s| s.on)` over 32 slots is false by construction (`clear_live`,
  `perform_live`, `state_live`, `live_edit`, `bank`). Expect more of the same wherever a test says "every step".
- **A stale pi-lens cache can report argument-count errors after a file rewrite.** The compiler is ground truth; a
  `lens_diagnostics source=lsp` probe on a rewritten file may say "cannot confirm clean", which is not a finding. Re-running
  `cargo test` on that file and re-probing clears it.
- **The player's ruling on round 02: the corner step number stays at 32 keys.** It is a **manual check** on the round's
  picture, not a test, and it is recorded in `02-the-three-grids.md`'s Task 1 step 10 and Task 4 step 7. Do not "fix" it by
  hiding the number.
- **The shell blocks** heredocs into `python3`, `perl`, and any unlisted binary. Write a script to a file and run it.
  `.superpowers/` is the git-ignored scratch directory.
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect to a file and sum the `test result` lines.
- **Never `git checkout <file>` to undo a scratch edit** — it reverts to HEAD and takes uncommitted work with it.
- **Screenshots (round 02 only):** check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400` *before* launching
  (something on this machine pre-starts `Xvfb` at 1280×800), then `xdotool getwindowgeometry` must say `1960x1340`. A fresh
  `TAKKA_ACID_DATA_DIR`, the pointer parked outside the window, and a crop is a second grab, never a cut-out. Text fields
  cannot be typed into under Xvfb — seed the file with a throwaway test and relaunch, as `docs/guide/src/img/README.md`
  records.

## How to work (unchanged)

One task at a time, in plan order. For each: write the failing test, run it, implement, run it, keep the plan's `## Progress`
line current — the test that proved it and any ruling with what it costs if wrong — and commit with the gates green
(`cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, plus
`./docs/guide/check.sh` when the book moves). Watch every test fail for its own reason: twice in round 01 the code was
written before its test and deleted again so the failure was honest, and both times the ledger says so.

**The review is at the pull request, not per commit**, by a subagent that did not write the code (`AGENTS.md`). It reports;
the executor fixes what it found and records it. Open the PR only when a round is green.
