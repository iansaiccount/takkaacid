> **Superseded — kept for the record, not for instructions.** Read `handover-task-7.md` for where the branch stands; the plan Progress section records what shipped.

# Handover — phase 4, task 6 (for a fresh session)

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

**Where the branch stands.** `00af7c1` is task 5, reviewed and committed; `957d90b` is the committed task 5 handover plus task
4's marked superseded; `718b8d2` task 4, `0520760` task 3, `b74179d` task 2, `0f7890d` task 1. Gates green at `00af7c1`:
`cargo test --workspace` (**771**), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` from the repo root. The branch is **local only** — nothing is pushed. Tasks 1–5 are done; tasks 6–10
are not started. `SAVE_VERSION` is still **10**, and **task 6 is the bump** — see trap 2.

## Your task: task 6 of the plan — kit documents and the factory kits

From the plan, verbatim:

> ### Task 6 — kit documents and the factory kits
>
> **Files:** modify `rust/crates/acid-app/src/persist.rs` (kit document, save/load/import, name validation), create
> `rust/crates/acid-app/kits/{acoustic,808,909}/` (the WAVs) and `kits/CREDITS.md`, `rust/crates/acid-app/src/kits.rs` (the
> compiled-in factory kits); tests in `tests/kits.rs` (new).
>
> **Interfaces (consumes Task 5; produces):**
> ```rust
> // in acid-app/src/persist.rs
> pub struct KitDocument { pub name: String, pub voices: [KitVoiceDoc; 16] }
> pub fn kits_dir() -> PathBuf;                       // <data>/kits
> pub fn save_kit(kit: &Kit, dir: &Path) -> io::Result<PathBuf>;
> pub fn load_kit(dir: &Path, name: &str, rate: f32) -> Result<Kit, KitError>;   // a missing WAV is a `KitError::Missing{voice,file}`
> pub fn import_kit(from: &Path, name: &str, dir: &Path, rate: f32) -> io::Result<PathBuf>;  // lands beside a taken name
> pub fn kit_names(dir: &Path) -> Vec<String>;        // factory names then the player's, sorted
> // in acid-app/src/kits.rs, beside the factory banks' code
> pub fn factory_kits(rate: f32) -> Vec<Kit>;         // compiled in, read-only
> ```
> - [ ] Tests: round trip through `save_kit`/`load_kit` with a one-voice sine written by hand; `import_kit` never overwrites
>       (second import of the same name lands as `name (2)`); a kit with a missing WAV loads every other voice and reports the
>       missing one by voice and file (Review Focus 2); a name that is a path (`../x`, `/abs`, `a/b`) is refused; `factory_kits`
>       has three kits, sixteen voices each, every voice audible, and names that are reserved (a player kit may not take `808`).
> - [ ] **The browser and the card's row.** `ui/kits.rs` draws the list as the banks card's sibling — `FACTORY` then `MY KITS`,
>       the factory three marked read-only, `IMPORT` beside them — and `ui/device_card.rs` gains the kit's name and a `KIT` key
>       that opens it. Live tests: `the_kit_browser_lists_the_factory_kits_first`;
>       `a_factory_kit_is_marked_read_only_and_selecting_it_loads_it`;
>       `editing_a_factory_voice_copies_the_kit_under_a_free_name` (no write to the factory folder, and the session names the copy).
> - [ ] Source the samples (CC0 only) into `kits/`, record each file's URL and licence in `kits/CREDITS.md`, and set the per-voice
>       defaults so each kit sounds like itself (listening work; `EAR_TESTING.md`).
> - [ ] Run; PASS. Commit: `feat(kits): a kit is a folder, and three of them ship`.

**The spec's decisions this task turns into code** (`docs/superpowers/specs/2026-11-18-drum-machine-design.md`, Persistence):
a kit is `<data>/kits/<name>/kit.json` plus one WAV per voice (`KICK.wav`); the JSON holds the voice table and, per entry,
`file`, `source` and `credit` for provenance — so `KitVoiceDoc` is the voice's playable fields **plus those three**; the three
factory kits are compiled in (`include_bytes!`, ~2 MB), **read-only**, and editing one makes a copy under a free name by
`free_song_name`'s rule; the session gains `kit: Option<String>` (`#[serde(default)]`, absent means the default); kit names are
validated like song names, so a name can never become a path; the card gains the kit's name and a `KIT` key; the device card
gains the kit row, `ui/device_card.rs`'s rows are otherwise kind-agnostic.

## What tasks 1–5 left you

**Task 1** (`acid-core::drums`): `Drum` (sixteen variants, `ALL`, `index()`, `label()`, `choke_group()`) and `FilterKind`
(`LP | BP | HP`). Every `match` is written out.

**Task 2** (`acid-core::pattern` + `persist`): `Step` carries `hits: u16` and `accents: u16`; `hit/clear_hit/is_hit/is_accented`;
`SAVE_VERSION` is 10 and `tests/persist.rs` pins the bytes.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice`
(`sample: Arc<[f32]>`, `tune`, `attack_ms`, `decay_ms`, `sustain`, `release_ms`, `filter`, `cutoff`, `resonance`, `mute`),
`KitVoice::default()`, `default_kit()`, `DEFAULT_KIT_NAME = "Init"`, `Kit::mute_mask`/`set_mute_mask`; `SharedState` has
`kit: ArcSwap<Kit>`, `drum_mute`, `audition`, `kit()`, `set_kit(Arc<Kit>)` (clone-on-write), `request_audition`,
`take_audition`.

**Task 4** (`acid-app`): `DeviceKind::Drums` (`DRUMS`, same rack cap), `Device::on`, `StripCell.kind`,
`PanelState::showing_kind()`, `PanelState::drum: DrumView { selected: Drum }`, `ui/drum_panel.rs` (the face: seven relabelled
dials, `LP | BP | HP`, `HIT`, `SHUFFLE`, the sixteen-row grid), and `ui/dispatch.rs` gating the keyboard for a drum device. The
guide's picker pages (`05`, `18`, `20`), `img/picker.png` and `check.sh` name `DRUMS`; **the drum face has no page yet — task 10
writes page 22.**

**Task 5** (`acid-app::wav`, new module, registered in `lib.rs`): `pub fn decode(bytes: &[u8], target_rate: f32) ->
Result<Arc<[f32]>, WavError>` — hand-rolled, `std` only, PCM 8/16/24/32-bit and 32-bit float, mono or stereo averaged,
linearly resampled, `LIST`/`INFO` skipped, every slice through one checked helper. `WavError` has **five** variants:
`NotRiff`, `NotWave`, `Unsupported { what: &'static str }`, `Truncated`, `TooLong`, with `Debug`, `PartialEq`, `Copy`, `Display`
and `std::error::Error`. 26 unit tests in the module. `MAX_SAMPLES` is 1 << 20 (21.8 s at 48 kHz), the cap is checked against
the output length before either `Vec` exists, an empty `data` chunk is silence while an absent one is refused, and
`WAVE_FORMAT_EXTENSIBLE` is refused **by name**. A float WAV's samples are passed through as recorded (no clamp, no NaN check) —
task 7's limiter is where the spec puts the ceiling.

## Traps this task owns

1. **`decode` is the only way a WAV becomes samples, and the rate comes from the running audio device.** `AudioStatus::Running
   { sample_rate, .. }` is what `main` has (see `main.rs`'s status block); pass that rate into `load_kit`/`import_kit` and into
   `factory_kits`. Do not default it to 44 100 or 48 000 "for now": a kit decoded at the wrong rate plays every voice at the
   wrong pitch, and nothing in the face would say why. A rate change reloads the kit — task 8 owns that reload; task 6 must leave
   the rate a parameter rather than a constant so task 8 can.
2. **`SAVE_VERSION` goes to 11 in this task**, with `SavedState` gaining `#[serde(default)] pub kit: Option<String>` — appended
   after the existing fields, as task 2 appended the masks, so a v10 file's prefix still reads. `tests/persist.rs` gets the new
   byte pin and a `a_version_ten_file_upgrades_with_no_kit` test in the same commit. **The queue moves:** the rack round takes
   **12** now, so the plan's non-goal line ("that phase takes `SAVE_VERSION` 11") and the wishlist's `### Save the whole rack`
   entry ("`SAVE_VERSION` goes to 11") must both be corrected in this commit. Task 2 and task 3 each recorded this same kind of
   queue move; a stale number here is the wishlist lying to the next round.
3. **Name validation and "lands beside a taken name" already exist — reuse them.** `persist::song_name(typed) ->
   Option<String>` is the path-proofing rule and `free_song_name(dir, wanted)` is the `name (2)` rule. A second implementation
   for kits is exactly the hand-kept second list `AGENTS.md` has paid for twice: `../x`, `/abs` and `a/b` must be refused by the
   *same* rule a song name is, and the test needs all three. If a kit name needs a different rule (say, no spaces), that is a
   change to the shared function with both callers' tests moving, not a fork.
4. **`import_kit` never overwrites** and `save_kit` decides in one place whether it writes in place or makes a new directory —
   the rule `save_target` owns for songs. A player importing `808` must not be able to overwrite the factory kit of that name,
   and `kit_names` must therefore be able to say which names are taken: the factory three are **reserved**.
5. **The factory kits are read-only, and the copy-under-a-free-name must happen once, not once per dial tick.** A dial drag
   publishes a new `Kit` every frame (task 4's ruling: the value lives in the kit, clone-on-write). If the "editing a factory
   kit makes a copy" rule sits in the publish path naively, one drag produces `Acoustic (2) (3) (4)`… and dozens of directories
   if it also writes to disk. Put the rule where the edit enters (one seam, before the first write), remember the copy's name,
   and make `editing_a_factory_voice_copies_the_kit_under_a_free_name` assert **exactly one** copy after several edits. Nothing
   is written into the factory folder, ever, and the session names the copy (`SavedState.kit`).
6. **A missing, renamed or unreadable WAV must not stop the kit loading** (Review Focus 2): every other voice loads, and the
   panel says which voice's file failed in a **notice** (`PanelState::notice`), naming the voice and the file — never a dialog,
   and never a panic or a silently-reused previous sample. `WavError`'s `Display` is written for exactly that sentence; task 6
   must not invent a second error type for the same bytes. `KitError::Missing { voice, file }` is the plan's shape for the
   missing-file case and needs its own `Display`/notice wording.
7. **The factory kits load at startup, not per frame.** `factory_kits(rate)` decodes ~48 WAVs (~2 MB) the first time it is
   called; cache the result (`OnceLock`) keyed on the rate, or `main` decodes once and hands the `Arc<Kit>`s to the picker and
   the browser. A per-frame decode is a 2 MB allocation per frame, and `tests/no_alloc.rs` will not catch it because it is not on
   the audio path.
8. **A first run has no `kits/` directory.** `kit_names` and the browser must treat a missing directory as "the factory three
   only", not as an error, because `TAKKA_ACID_DATA_DIR` on a fresh path is how the guide's screenshots and the demo session are
   taken. `songs/` has the same shape — copy how `list_songs` behaves when the directory is absent.
9. **`ui/device_card.rs` geometry is tested and the kit row is a new row.** `device_card::rows()` builds `Rows` from the card's
   rect and every key's hit target comes from it; a new row changes that list, so `tests/device_card_live.rs` is in this task's
   Files (the plan's line names only `ui/device_card.rs`) and the row must stay inside the card at **0.65× and 1.6×** (Review
   Focus 4). The kit row draws for `DeviceKind::Drums` only, and a drum device's card must not grow a 303 row.
10. **The browser is the banks card's sibling, not a new pattern.** `ui/banks.rs` already draws `DEMO BANKS` and `MY BANKS`
    sections with one `section()` helper, a `load_row()` and a `guarded_load()`. `FACTORY` / `MY KITS` should be the same shape;
    a second list-drawing idiom is a second place to fix the next layout bug.
11. **The samples are CC0 or public domain, each with its URL and licence in `kits/CREDITS.md`**, in the same commit. A wrong
    CC0 claim is a legal problem, not a bug (the spec's Risks). Keep every voice ≤ 0.6 s and 16-bit mono where the material
    allows: the spec's size estimate is ~2 MB for three kits, and `include_bytes!` puts that in the binary.
12. **The guide moves with the card row.** A `KIT` key and a kit name on the card are player-visible, so `19-a-devices-card.md`
    says what the row does and `docs/guide/check.sh` pins the words worth pinning (`MY KITS` is the browser's, `KIT` is the key)
    — in this commit, not task 10's. **Page 22 is still task 10's**: do not write the drum face's page here. A screenshot is
    task 10's too unless the card's picture becomes wrong, in which case retake it by the `AGENTS.md` recipe and update
    `img/README.md`'s row.
13. **Nothing on the audio path and no lock across it.** Kit loading is UI-thread I/O; the renderer keeps reading
    `ArcSwap<Kit>` through `SharedState::kit()`. `tests/no_alloc.rs` stays green.
14. **`KitVoiceDoc`'s field order is a document contract**, like `SavedState`'s: the playable fields in `KitVoice`'s order, then
    `file`, `source`, `credit`. `KitVoice`'s own doc comment already says where provenance sits and why it is not in the voice
    that plays it.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` from the repo root. Task 6 moves the book, so the fourth gate is not optional.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Save the diff (a new file needs
  `git add -N` before `git diff` sees it) under `.superpowers/review/`, dispatch the `reviewer` agent with this task's **Files**
  and **Interfaces** blocks — not the whole plan — tell it the traps above, and act on what it finds before committing. The
  task's `## Progress` line records what it found and what was done, including a finding you disagree with and why it stood.
- **Prove each test bites.** Mutate, watch the named test fail, revert, write down what you mutated.
  `.superpowers/mutate_task5.py` is the pattern: full test path, and it refuses a filter that matches nothing. Two lessons from
  task 5's run: an **equivalent expression is not a mutation** (swapping `WAVE` for `RIFF` changed nothing, because `AVI `
  differs from both, so the test "passed" the mutation), and a mutation whose `old` string no longer matches must be reported as
  `SKIP`, never silently counted as caught.
- **No `_` arm on a catalogue enum; no hand-kept second list; no font glyphs except `…` via `ui::text::fit`; a message is a
  notice, not a dialog.** Task 6 adds a browser, a card row and a kit name that the card prints — all four rules land here.
- **The plan's Progress is the record.** One entry per task: what shipped, the mutations, the review's findings and what was
  done, every ruling with what it costs if wrong, and — when the change is invisible to the guide — why.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write the script with the editor and
  run `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside it,
  git-ignored, and is the right place for scratch scripts, diffs and commit messages (`git commit -F .superpowers/commitmsg.txt`).
  `identify` is not allowlisted (use `file` for a PNG's dimensions); `Xvfb`, `xdotool` and `import` are.
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect and sum:
  `cargo test --workspace > /tmp/t.log 2>&1`, then
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **`cargo test -- --exact` needs the full test path** (`kits::tests::name`, `persist::tests::name`); a filter that matches
  nothing exits **0** with "0 passed", which reads exactly like a pass. Assert a test ran.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest, and a `cd` inside the chain leaves later commands in the
  wrong directory** — `cargo test` from the repo root fails with "could not find `Cargo.toml`" and the chain still prints the
  next command's success. Run gates as separate commands.
- **`cargo fmt` collapses blank lines but never adds one**, and a long line it reflows can break a mutation script's `old`
  string. Read the diff after formatting, and re-run the mutation script after it.
- **Editing anchors are fragile around doc comments, module boundaries and script blocks.** An `insert` after the last line of
  a `mod tests` puts the new test *outside* the module; a patch whose replacement ends with `]` can swallow the next
  `def` line in a Python script. Check `cargo test -- --list` (or the script's own `SKIP` lines) rather than trusting the diff.
- **pi-lens reports type errors as you edit** (it caught an `f32` compared with `0` in task 5) but flags a file it saw change
  outside an edit, and withholds fix prompts for it — the diagnostics are still worth querying after a batch of edits.
- **Live tests** (`tests/*_live.rs`) drive the real panel through egui `RawInput` frames and assert on `SharedState` or
  `PanelState`. The kit browser and the card row are panel behaviour, so their tests go there; `tests/kits.rs` is for the plain
  path functions (`save_kit`, `load_kit`, `import_kit`, `kit_names`, `factory_kits`), which need no display.
- **`tests/persist.rs` uses `tempfile`** for the directory-shaped tests; the same is available for `tests/kits.rs`.
- **Screenshots** (task 10, not yours unless the card's picture goes wrong): something pre-starts `Xvfb` at 1280×800 on display
  numbers and a request for a bigger screen on a taken display fails quietly. `DISPLAY=:N xdotool getdisplaygeometry` must say
  `2000 1400`; the window must measure 1960×1340. The full recipe is in `AGENTS.md`.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; a first run there is the factory session, and it has no
  `kits/` directory (trap 8).

## The end of this session

When task 6 is committed: add its `## Progress` entry (what shipped, the mutations, the review's findings, every ruling with its
cost), **rewrite this file as `plans/phase 4/handover-task-7.md`** for the next task — task 7 is *the machine*: the `Performer`
trait in `acid-core::sequencer`, `impl Performer for Voice` (taking `prev_slide` off `Sequencer`), the new multimode
state-variable filter, `Drums` with sixteen voice states, and `acid-core/tests/drums.rs` — and prepend the one-line
`> **Superseded — kept for the record, not for instructions.**` note to this file, as task 5's handover did to task 4's.
Carry these into that handover: **the `Init` word is what the card prints** (settled in task 3; the player may overrule it for
one constant, one pinned assertion and one guide sentence); **`SAVE_VERSION` is a queue** — task 6 takes 11 and the rack round
takes 12, and if task 7 needs no field the number stays at 11; and **`decode`'s rate parameter is task 8's reload seam**, not a
constant. Do not push; the player decides when the branch goes up.
