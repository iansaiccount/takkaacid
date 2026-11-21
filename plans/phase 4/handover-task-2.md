> **Superseded — kept for the record, not for instructions.** Read `handover-task-4.md` for where the branch stands; the plan Progress section records what shipped.

# Handover — phase 4, task 2 (for a fresh session)

Paste this whole file as the first message. It is written to be self-contained: the session it is for has no memory of the one
that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth in Rust, at `/home/ian/Projects/takkaacid`, on branch
**`phase-4`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the project's standing instruction and outranks
anything here that contradicts it.

**The round.** Phase 4 is the **drum machine**: sixteen sample voices against the 303's sixteen steps, sharing the bar, banks,
transport, transpose, reverse and themes. The approved design is
`docs/superpowers/specs/2026-11-18-drum-machine-design.md` (the spec — it wins any argument with the plan). The plan, ten tasks,
is `plans/phase 4/01-the-drum-machine.md`; its `## Progress` is the record and its Global Constraints and Review Focus are
binding. The phase overview is `plans/phase 4/00-phase-4.md`.

**Where the branch stands.** `0f7890d` is task 1, reviewed and committed. Gates green at that commit:
`cargo test --workspace` (709), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` from the repo root. The branch is **local only** — nothing is pushed.

## Your task: task 2 of the plan — the pattern's masks

From the plan, verbatim:

> **Files:** modify `rust/crates/acid-core/src/pattern.rs`, `rust/crates/acid-app/src/persist.rs`,
> `rust/crates/acid-app/tests/persist.rs`.
> **Interfaces (consumes Task 1; produces):**
> ```rust
> pub struct Step { /* existing fields */ pub hits: u16, pub accents: u16 }
> impl Step {
>     pub fn hit(&mut self, drum: Drum, accented: bool);  // sets the hit bit, the accent bit, and `on`
>     pub fn clear_hit(&mut self, drum: Drum);            // clears both bits, and `on` when no hit remains
>     pub fn is_hit(&self, drum: Drum) -> bool;
>     pub fn is_accented(&self, drum: Drum) -> bool;
> }
> ```
> - [ ] Tests in `pattern.rs`: `a_hit_sets_the_mask_the_accent_and_on`, `clearing_the_last_hit_rests_the_step`,
>       `a_drum_edit_leaves_the_303_fields_alone`, `a_pattern_without_masks_is_a_303_pattern` (all masks zero, `on` unchanged).
> - [ ] Test in `tests/persist.rs`: `a_version_nine_file_upgrades_with_empty_drum_masks` — a pinned version-9 document loads, every
>       `hits`/`accents` is zero, and the 303's steps are byte-identical after a round trip; plus the byte pin for `SAVE_VERSION` 10.
> - [ ] Run them; expect failures. Implement the fields with `#[serde(default)]`, the four methods, and `SAVE_VERSION = 10` with an
>       upgrade that leaves the masks zero.
> - [ ] `cargo test --workspace`; expect PASS. Commit: `feat(pattern): a step carries the drum masks, and the save file moves to 10`.

**What task 1 left you** (`0f7890d`, `rust/crates/acid-core/src/drums.rs`): `Drum` with `ALL: [Drum; 16]`, `index()` (bit and grid
row), `label()`, `choke_group()`, and `FilterKind { Low, Band, High }` with `ALL`/`index`/`label` (`LP`/`BP`/`HP`). The bit for a
voice is `1u16 << drum.index() as u16` — there is deliberately **no** `Drum::bit()` helper; derive it at the one place that writes
the mask, or add the method *and* delete the second spelling. The catalogue's tests pin the labels in grid order, so renaming a
caption fails there.

**Traps this task owns.**
- `Step::accent` already exists as the 303's own `bool`; the mask fields must be `hits` and `accents` (plural), and the 303's
  fields must be untouched by a drum edit.
- `on` is written by `Step::hit` and `clear_hit` as well as the masks, so `on` cannot drift from the mask. A drum step is "on"
  because a voice hits it.
- `SAVE_VERSION` is 9 today. It goes to 10. `persist::upgraded` **upgrades** rather than refusing old files — a version-9 file must
  load with zeroed masks, and `tests/persist.rs` pins the bytes of a written document. Check what that test compares before you
  change the version, or you will spend an hour reading a byte diff.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` when the book moved.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Dispatch `reviewer` with the task's
  **Files** and **Interfaces** blocks and the diff (`git diff > /tmp/x.diff`, and `git add -N` for new files so they appear),
  asking it to judge the diff against what the task claimed. Fix what it finds, record it in the plan's `## Progress`, then commit.
- **One catalogue per kind of control; no `_` arm on a catalogue enum; no hand-kept second list.** This repo has paid for that bug
  twice.
- **No font glyphs** on the face except `…` via `ui::text::fit`/`settings::shorten`.
- **The save file is a contract**: field names and order are what older builds read.
- **Nothing on the audio path allocates or locks**; `tests/no_alloc.rs` is the gate.
- **The plan's Progress section is the record.** One entry per task: what shipped, the mutation or test that proves it, what the
  review found and what you did about it, and any ruling with what it costs if wrong.

## Environment facts that cost the last session time

- **The shell blocks heredocs and `python3 -c`.** Write the script to `/tmp/x.py` and run `python3 /tmp/x.py`. `bash` itself is
  blocked by the allowlist, so a shell *script* from a skill (`start-server.sh`) cannot be run; write files instead.
- **`cargo test --workspace | head` kills the run with SIGPIPE** and you will read a partial count (it reports ~half the tests).
  Always `cargo test --workspace > /tmp/t.log 2>&1` and then sum: `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;")
  p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **In a `cmd1 && cmd2` chain, a failed `cmd1` silently skips the rest**, and a `cd` inside the chain then leaves later commands
  in the wrong directory — that produced a misleading "not a git repository". Run gates separately, or join with `;`.
- **Screenshots** (task 10, not yours): a headless display that reports `2000 1400` — something pre-starts `Xvfb` at 1280×800 on
  display numbers, and asking a taken display for a bigger screen fails quietly. Check `DISPLAY=:N xdotool getdisplaygeometry`
  before launching; the window must measure 1960×1340. The full recipe is in `AGENTS.md`.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; a first run there is the factory session.

## The end of this session

When task 2 is committed: add its `## Progress` entry (with the review's findings and the mutations you ran), **rewrite this
file as `handover-phase-4-task-3.md`** for the next task — task 3 is *the kit, in memory* (`Kit`, `KitVoice`, `default_kit` on
`SharedState`) — and tell the player. Do not push; the player decides when the branch goes up.
