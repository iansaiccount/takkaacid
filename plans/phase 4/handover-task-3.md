> **Superseded — kept for the record, not for instructions.** Read `handover-task-4.md` for where the branch stands; the plan Progress section records what shipped.

# Handover — phase 4, task 3 (for a fresh session)

Paste this whole file as the first message. It is written to be self-contained: the session it is for has no memory of the one
that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth in Rust, at `/home/ian/Projects/takkaacid`, on branch
**`phase-4`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the project's standing instruction and outranks
anything here that contradicts it.

**The round.** Phase 4 is the **drum machine**: sixteen sample voices against the 303's sixteen steps, sharing the bar, banks,
transport, transpose, reverse and themes. The approved design is
`docs/superpowers/specs/2026-11-18-drum-machine-design.md` (the spec — it wins any argument with the plan). The plan, ten
tasks, is `plans/phase 4/01-the-drum-machine.md`; its `## Progress` is the record and its Global Constraints and Review Focus
are binding. The phase overview is `plans/phase 4/00-phase-4.md`.

**Where the branch stands.** `b74179d` is task 2, reviewed and committed; `0f7890d` is task 1. Gates green at `b74179d`:
`cargo test --workspace` (**717**), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` from the repo root. The branch is **local only** — nothing is pushed. Tasks 1 and 2 are done; tasks
3–10 are not started.

## Your task: task 3 of the plan — the kit, in memory

From the plan, verbatim:

> ### Task 3 — the kit, in memory
>
> **Files:** create `rust/crates/acid-core/src/drums.rs` additions (`Kit`, `KitVoice`, `default_kit`); modify
> `rust/crates/acid-app/src/state.rs`.
>
> **Interfaces (produces):** `Kit { pub name: String, pub voices: [KitVoice; 16] }`, `KitVoice` as the spec defines it, and on
> `SharedState`: `pub kit: arc_swap::ArcSwap<Kit>`, `pub drum_mute: AtomicU32`, `pub audition: AtomicI32`. `fn kit(&self) ->
> arc_swap::Guard<Arc<Kit>>` mirrors `pattern()`.
>
> - [ ] Test: `a_default_kit_has_sixteen_voices_and_no_samples` (empty `Arc<[f32]>`, default envelope/filter values, every voice
>       unmuted), and `the_drum_mute_mask_starts_from_the_kit` (the mask is the kit's `mute` flags packed in catalogue order).
> - [ ] Implement; run; PASS. Commit: `feat(drums): a kit lives on the device's shared state`.

`KitVoice`, from the spec, in this field order:

```rust
pub struct KitVoice {
    pub sample: Arc<[f32]>,                 // mono, at the engine's rate, already resampled
    pub tune: f32,                          // semitones, ±12
    pub attack_ms: f32, pub decay_ms: f32, pub sustain: f32, pub release_ms: f32,
    pub filter: FilterKind,                 // Low | Band | High
    pub cutoff: f32, pub resonance: f32,    // knob positions, mapped like the 303's
    pub mute: bool,                         // the kit's starting mask
}
```

Pick the default numbers from what the 303 already ships (`voice::DEFAULT_*_KNOB` are knob positions, so a sensible kit
default is the 303's own launch patch where a value maps, and a short percussive envelope where it does not). Task 6 will set
these per kit from listening work; task 3 only has to be *sound and harmless*, and the test pins "every voice unmuted, no
sample, defaults in range".

## What tasks 1 and 2 left you

**Task 1** (`acid-core::drums`): `Drum`, sixteen variants in grid order (Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom,
MidTom, HighTom, Crash, Ride, Cowbell, LowConga, HighConga, Clave, Shaker) with `ALL`, `index()` (bit and grid row),
`label()` (`KICK` … `SHK`), `choke_group()` (1 for the two hats, 0 otherwise); and `FilterKind { Low, Band, High }` with
`ALL`/`index`/`label` (`LP`/`BP`/`HP`). Every `match` is written out. In-file tests pin the labels in grid order.

**Task 2** (`acid-core::pattern`): `Step` gained `pub hits: u16, pub accents: u16`, appended after `velocity` with
`#[serde(default)]`, and four methods: `hit(drum, accented)`, `clear_hit(drum)`, `is_hit(drum)`, `is_accented(drum)`. The bit
is one private `drum_bit(drum)` in `pattern.rs`; there is deliberately **no** `Drum::bit()` — do not add a second spelling.
`SAVE_VERSION` is **10**; `upgraded` needed no rewrite because the serde default zeroes the masks. Two rulings you inherit and
must not undo: `hit(voice, false)` *clears* the accent bit (so masks cannot disagree), and `clear_hit` rests the step only when
it actually removed a hit (so a stray clear cannot empty a 303 note).

## Traps this task owns

- **`fn kit()` — the plan and the code disagree, decide once and ledger it.** The plan says the signature is
  `arc_swap::Guard<Arc<Kit>>` and "mirrors `pattern()`", but `SharedState::pattern()` (`state.rs:315`) returns
  `Arc<Pattern>` via `load_full()`, not a Guard. Both are allocation-free; a Guard is cheaper to read and holds the load for
  the read's life, an Arc bump is what the rest of the file already does. Whichever you pick, `kit()` and `pattern()` should
  read alike, `tests/no_alloc.rs` is the gate, and task 8 will call this on the audio path — record the choice in the
  plan's `## Progress` as a Ruling with what it costs if wrong.
- **`AtomicU32` for a sixteen-bit idea.** `drum_mute` is `u32`; pack `voices[i].mute` into bit `i` **in catalogue order**
  (`Drum::ALL` / `Drum::index`), never a second positional list. The pack and the unpack belong in one place in
  `acid-core::drums` (e.g. `Kit::mute_mask() -> u16`), so task 7's `set_mute_mask` and task 4's grid cannot disagree.
- **`audition: AtomicI32` is a request slot, not a live voice.** The existing sentinel in this codebase is `-1`
  (`pattern_request`, `pattern_selected`, `learn_cc`); use it, and spell a `Drum` by `Drum::index()` as `i32` — one mapping,
  not a new `as i32` in the handler.
- **`ArcSwap<Kit>` has no `Default`.** `ArcSwap::from_pointee(default_kit())`, as `pattern: ArcSwap::from_pointee(Pattern::empty())`
  does. `Kit` needs no `Clone`, but it does need `Send + Sync` for the swap — plain data plus `Arc<[f32]>` gives you that, so
  don't add interior mutability "for later".
- **`Arc<[f32]>: Default` is the empty slice**, so `KitVoice::default()` is a plain derive-shaped impl with no allocation and
  no `unwrap`. Verify rather than assume if you change the shape (it compiles today).
- **One constructor.** `SharedState::with_tempo` is the only place a device's state is built (`new()` calls it); the starting
  mute mask must be packed there from the same default kit that goes into the swap, or a test and the app can disagree about
  which kit a device starts with.
- **Field order is the kit document's contract.** Task 6 writes `kit.json` from these structs. Spell `Kit` as
  `{ name, voices }` and `KitVoice` in the spec's order above, and keep provenance out of `Kit` (it goes in the JSON as
  `file`/`source`/`credit`, per the spec's Persistence section) — a rename or a reorder here costs a migration in task 6.
- **Tests go in-file** (`#[cfg(test)] mod tests`) in `drums.rs`, as task 1 did, and in `state.rs`'s existing `mod tests` at
  the bottom of the file, as `SharedState`'s other tests do. This task has no panel behaviour, so no `*_live.rs` test is due.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` when the book moved. This task is invisible to the book (no face, no
  player-reachable behaviour) — say so in the plan's `## Progress`, as task 2 did, rather than editing a guide page.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Dispatch `reviewer` with the task's
  **Files** and **Interfaces** blocks and the diff (`git diff > /tmp/x.diff`, and `git add -N` for new files so they appear),
  asking it to judge the diff against what the task claimed. Fix what it finds, record it in the plan's `## Progress`, then
  commit. A review whose findings land in a later commit gated nothing.
- **Prove each test bites.** Mutate the implementation, watch the test fail, revert, and write what you mutated in `## Progress`.
  Task 2's review found a test that passed for the wrong reason this way (a refused file falls back to defaults — the test never
  read the file it thought it did); assume your tests can pass for the wrong reason until a mutation says otherwise.
- **One catalogue per kind of control; no `_` arm on a catalogue enum; no hand-kept second list.** This repo has paid for that
  bug twice.
- **No font glyphs** on the face except `…` via `ui::text::fit`/`settings::shorten`.
- **The save file is a contract**: field names and order are what older builds read.
- **Nothing on the audio path allocates or locks**; `tests/no_alloc.rs` is the gate.
- **The plan's Progress section is the record.** One entry per task: what shipped, the mutation or test that proves it, what the
  review found and what you did about it, and any ruling with what it costs if wrong.

## Environment facts that cost the last session time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary** (`rustc -o /tmp/probe` is blocked;
  `rustc probe.rs` is not). Write the script to `/tmp/x.py` and run `python3 /tmp/x.py`; write a file with the editor instead
  of a heredoc. `cat > /tmp/commitmsg.txt <<'EOF'` **did** work for a commit message.
- **`cargo test --workspace | head` kills the run with SIGPIPE** and you will read a partial count. Always
  `cargo test --workspace > /tmp/t.log 2>&1` then sum:
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest**, and a `cd` inside the chain leaves later commands in the
  wrong directory. Run gates separately or join with `;`.
- **Editing anchors are fragile around doc comments.** Several edits in task 2 produced a duplicated `}`, a lost `let`, and a
  doubled blank line; after any edit that inserts a block, build immediately (`cargo build -p acid-core`) rather than after
  several. `cargo fmt` cleans up blank lines but fixes no brace.
- **Screenshots** (task 10, not yours): a headless display that reports `2000 1400` — something pre-starts `Xvfb` at 1280×800 on
  display numbers, and asking a taken display for a bigger screen fails quietly. Check `DISPLAY=:N xdotool getdisplaygeometry`
  before launching; the window must measure 1960×1340. The full recipe is in `AGENTS.md`.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; a first run there is the factory session.

## The end of this session

When task 3 is committed: add its `## Progress` entry (with the review's findings and the mutations you ran), **rewrite this
file as `handover-phase-4-task-4.md`** for the next task — task 4 is *the face*: `DeviceKind::Drums`, `ui/drum_panel.rs`, the
sixteen-row grid, the relabelled knobs, and live tests in `tests/drum_live.rs` — and tell the player. Do not push; the
player decides when the branch goes up.
