# Handover — phase 4, task 7 (for a fresh session)

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

**Where the branch stands.** `044d5e0` is task 6, reviewed (three times — the third review's findings are in the Progress
entry and every one is fixed) and committed; `e1b0640` is the committed task 6 handover plus task 5's marked superseded;
`00af7c1` task 5, `718b8d2` task 4, `0520760` task 3, `b74179d` task 2, `0f7890d` task 1. Gates green at `044d5e0`:
`cargo test --workspace` (**810**), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` from the repo root. The branch is **local only** — nothing is pushed. Tasks 1–6 are done; tasks 7–10
are not started. `SAVE_VERSION` is now **11** (`SavedState::kit`); the rack round takes **12**.

## Your task: task 7 of the plan — the machine

From the plan, verbatim:

> ### Task 7 — the machine
>
> **Files:** modify `rust/crates/acid-core/src/sequencer.rs` (the `Performer` trait), `voice.rs` (impl), `filter.rs` (a multimode
> filter); create `rust/crates/acid-core/src/drums.rs` additions (`Drums`, the voice state); tests in
> `rust/crates/acid-core/tests/drums.rs` (new).
>
> **Interfaces (consumes Tasks 1–3; produces):**
> ```rust
> pub trait Performer: Sized { fn step(&mut self, step: &Step); fn release(&mut self);
>                             fn render(&mut self, out: &mut [f32]); fn arm(&mut self); }
> impl Performer for Voice;                       // `prev_slide` moves here from `Sequencer`
> impl Performer for Drums;
> pub struct Drums;                               // owns a Sequencer, the current Kit, sixteen voice states
> impl Drums {
>     pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self;
>     pub fn set_kit(&mut self, kit: Arc<Kit>);
>     pub fn set_mute_mask(&mut self, mask: u16);
>     pub fn audition(&mut self, drum: Drum);      // a hit with no step, served at the next render
>     pub fn render(&mut self, out: &mut [f32]);
>     pub fn bar_line(&self) -> Option<usize>;  pub fn start(&mut self);  pub fn start_at(&mut self, step: usize);
>     pub fn start_later(&mut self, step: usize, silence: usize);  pub fn stop(&mut self);  pub fn pause(&mut self);
>     pub fn resume(&mut self);  pub fn set_reverse(&mut self, on: bool);  pub fn set_transpose(&mut self, semitones: i32);
> }
> pub struct MultimodeFilter;  // one state-variable filter, `FilterKind`, cutoff/resonance in knob positions
> ```
> - [ ] `Sequencer` keeps everything it has; `render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32])`.
> - [ ] Tests in `tests/drums.rs`: `a_hit_plays_its_sample_once_and_stops` (a 1 kHz buffer renders and then falls silent);
>       `the_envelope_shapes_the_tail` (a long release passes the ADSR's release point);
>       `a_stop_cuts_a_long_sample_where_the_stop_says` (a two-second sample stops inside the release, not at the buffer's end —
>       Review Focus 3); `a_retrigger_restarts_the_sample` (the second hit's first sample is the buffer's first, not the old tail);
>       `accent_is_louder_and_brighter` (peak and spectral centroid both rise); `the_filter_has_three_faces` (the same noise reads
>       brighter in High than Low, quieter in Band); `tuning_moves_the_pitch` (+12 doubles the zero-crossing rate);
>       `an_open_hat_is_choked_by_a_closed_one` (after the closed hit, the open voice's output falls below a threshold within 10 ms);
>       `audition_sounds_with_the_transport_stopped`; `a_muted_voice_is_silent_and_keeps_its_step`;
>       `every_voice_muted_is_silence_with_the_steps_still_on` (Review Focus 5's first half);
>       `the_bar_is_the_303s` (the drum `Sequencer` reports the same position and bar line as a 303 engine started on the same frame).
> - [ ] Run; expect failures. Implement. Run `cargo test -p acid-core`; PASS. Commit: `feat(core): the drum machine — sixteen
>       voices on the 303's clock`.

## What tasks 1–6 left you

**Task 1** (`acid-core::drums`): `Drum` — sixteen variants in grid order with `ALL: [Drum; 16]`, `index()`, `label()`
(`KICK SNR RIM CLAP CHH OHH LOT MID HTOM CRS RDE CWB LCO HCO CLV SHK`) and `choke_group()` (`ClosedHat`/`OpenHat` share 1,
everything else 0) — and `FilterKind` (`Low | Band | High`, `ALL`, `index()`, `label()` = `LP | BP | HP`). Every `match` is
written out; a seventeenth voice fails to compile in four places.

**Task 2** (`acid-core::pattern` + `persist`): `Step` carries `hits: u16` and `accents: u16` after `velocity`, with
`hit(voice, accented)` as the one writer of both masks *and* `on`, `clear_hit(voice)`, `is_hit(voice)`, `is_accented(voice)`.
A drum step is "on" because a voice is. `SAVE_VERSION` 10 then, 11 now.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice { sample:
Arc<[f32]>, tune, attack_ms, decay_ms, sustain, release_ms, filter, cutoff, resonance, mute }`, `KitVoice::default()` (attack
2 ms, decay 300 ms, sustain 0.0, release 20 ms, cutoff 1.0, resonance 0.0, tune 0.0, LP, unmuted), `default_kit()` (sixteen of
those, named `Init`, **no samples**), `DEFAULT_KIT_NAME = "Init"`, `Kit::mute_mask()`/`set_mute_mask()` (the one pack and unpack,
bit = `Drum::index()`). `SharedState` has `kit: ArcSwap<Kit>`, `drum_mute: AtomicU32`, `audition: AtomicI32`, with `kit()`,
`set_kit(Arc<Kit>)` (**the edit path**), `adopt_kit(Kit)` (**the load path** — seeds `drum_mute` from the kit, which `set_kit`
deliberately does not), `request_audition(Drum)` and `take_audition() -> Option<Drum>` (the `-1` sentinel lives there).
**Task 8 must consume `adopt_kit`/`request_audition`/`take_audition`, not spell them again.**

**Task 4** (`acid-app`): `DeviceKind::Drums` (`name()` = `DRUMS`, `has_kit()` = the written-out match a third kind must
answer), `Device::on`, `StripCell.kind`, `PanelState::showing_kind()`, `PanelState::drum: DrumView { selected: Drum }`,
`ui/drum_panel.rs` (the face: seven relabelled dials, `LP | BP | HP`, `HIT`, `SHUFFLE`, the sixteen-row grid), `ui/dispatch.rs`
gating the keyboard for a drum device, and `ui::kits::edit` as the one seam a kit edit goes through.

**Task 5** (`acid-app::wav`): `decode(bytes, target_rate) -> Result<Arc<[f32]>, WavError>` (hand-rolled, `std` only, PCM
8/16/24/32 and float, mono or stereo averaged, linearly resampled, `MAX_SAMPLES = 1 << 20`) and `encode(samples, rate) ->
Vec<u8>` (16-bit mono, clamped — the other half, for a kit saved back to disk). `WavError` has five variants with `Display`
sentences written for the status line.

**Task 6** (`acid-app::persist`, `acid-app::kits`, `acid-app::ui::kits`): a kit is a folder —
`<data>/kits/<name>/kit.json` + one `<VOICE>.wav` per sampled voice — with `save_kit(kit, dir, rate)`,
`load_kit(dir, name, rate) -> Result<LoadedKit, KitError>` (**partial**: a broken voice costs one voice and arrives in
`problems`), `import_kit`, `kit_names`, `free_kit_name`, `kit_name`, `free_name`. `acid_app::kits` has the three that ship
(`factory_kits(rate) -> Vec<Kit>`, cached per rate), `factory_names()`, `is_factory()`, `is_reserved()` (factory names **and**
`Init`), `default_name()` = `"808"` and `factory_sample_bytes()`. Each compiled-in voice carries only `tune`, `decay_ms` and
`cutoff`; the rest is `KitVoice::default()`. `main` decodes once the device reports `AudioStatus::Running { sample_rate }`,
loads into the selected device's `SharedState` through `adopt_kit`, and writes an edited kit's folder on save.
**`SAVE_VERSION` is 11** with `SavedState::kit: Option<String>` appended last.

**What is still missing, and is your task**: there is no `Performer`, no `Drums`, no multimode filter, and
`acid-app::audio::renderer::AudioRenderer` still owns an `acid_core::Engine` and nothing else — so a drum device's face draws
and edits a kit but **sounds the 303 line under the grid**. Task 8 is what gives the renderer a `Machine`; task 7 is the
machine itself.

## Traps this task owns

1. **The clock is not copied.** The bar, `REVERSE`, swing, pause, the bar line and the sample-accurate start stay in
   `Sequencer`; only "make this step sound" moves. `Sequencer::render<P: Performer>(&mut self, performer: &mut P, out:
   &mut [f32])` and **every** render path goes through it — the delay in `wait_out_start`, the tail after a stop, the block
   loop. `the_bar_is_the_303s` is the test that proves it is one clock and not two.
2. **`prev_slide` moves from `Sequencer` to `Voice`.** `arm()` is what clears per-run state, and `Sequencer` must keep its
   public surface (`start`, `start_at`, `start_later`, `stop`, `pause`, `resume`, `set_reverse`, `set_transpose`,
   `bar_line`, `position`, `is_playing`) so tasks 8 and the rack keep compiling.
3. **Nothing allocates or locks on the audio path** (`tests/no_alloc.rs`). A voice renders into a slice; `out` is the mixer's
   block. `Drums` holds the `Arc<Kit>` a block needs; a kit swapped mid-block must keep its samples alive because the `Arc` is
   held (task 8 tests that).
4. **A long sample is cut where the stop says, and a retrigger restarts it** (Review Focus 3). Two places: the release after a
   stop, and the click that retriggers. The old tail must not bleed into the next bar.
5. **A muted voice keeps its step.** The mask silences output, not the sequencer: `every_voice_muted_is_silence_with_the_steps_still_on`
   is the half of Review Focus 5 this task owns; the mixer's ceiling is the other half (task 8).
6. **Accent is louder *and* brighter** — the 303's accent gain, and the filter opening further the way `ENV MOD` does. Both at
   once; a test that reads only the peak passes an accent that is only a volume.
7. **The choke group is real.** A hit on `ClosedHat` releases the voice in its group (`OpenHat`). Fold through the envelope's
   release rather than cutting the sample to zero, or it clicks — and the test measures within 10 ms, so a "release" that takes
   200 ms fails it.
8. **One filter, three faces.** A state-variable filter with `LP`/`BP`/`HP`, cutoff and resonance mapped like the 303's
   (`filter::param::cutoff_hz`, `resonance` — copy that feel, do not invent one). `the_filter_has_three_faces` compares the
   same noise through all three.
9. **Tuning is a fractional sample position**, `2^(tune/12) × rate_ratio`; `+12` doubles the zero-crossing rate and the kit's
   samples are already at the engine's rate, so the ratio is 1 except where `tune` moves it.
10. **A kit with no samples is silence, not a panic.** `default_kit()` is sixteen empty `Arc<[f32]>`s, so a voice whose
    `sample` is empty must render silence and the hit must be harmless. A new drum device is exactly this.
11. **The audition is drained, not polled.** `SharedState::take_audition()` answers once; `Drums::audition(drum)` is a hit with
    no step, served at the next render, so `HIT` sounds with the transport stopped — and `STOP` must still cut it.
12. **A `u16` mask is what the engine reads** (`SharedState::drum_mute` is a `u32` holding it), packed and unpacked only by
    `Kit::mute_mask`/`set_mute_mask`. Task 3 noted a debt: `1u16 << Drum::index()` now appears in three places; if a fourth is
    needed here, `Drum::bit()` earns its place and `pattern::drum_bit` must move onto it **in the same commit**.
13. **The `Arc<Kit>` may be swapped between blocks.** Load it once at the top of `render`; do not read the `ArcSwap` per sample
    and do not hold a guard across the block.
14. **A voice's position and filter state survive a kit swap** — only the samples change. Renumbering or resetting state on
    `set_kit` would make a dial turn on the panel retrigger every sounding voice.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` when the book moved. Task 7 probably does not move the book: the drum face
  has no page yet (task 10 writes page 22), and nothing a player can reach changes until task 8 wires the machine in. Say so in
  the Progress if that is the call.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Save the diff (a new file needs
  `git add -N` before `git diff` sees it) under `.superpowers/review/`, dispatch the `reviewer` agent with this task's **Files**
  and **Interfaces** blocks — not the whole plan — and act on what it finds before committing. Task 6 took three reviews and
  every round found something real; a review of a stale diff is a review that did not happen, so regenerate it after you fix
  anything.
- **Prove each test bites.** Mutate, watch the named test fail, revert, write down what you mutated.
  `.superpowers/mutate_task6.py` is the pattern: full test path for a lib test (`sequencer::tests::name`), the **bare** name for
  an integration test (`tests/drums.rs`'s root is the file, so `drums::name` matches nothing and "passes" with zero run), and a
  `SKIP` for a mutation whose text no longer matches — never a silent pass. An **equivalent expression is not a mutation**.
- **No `_` arm on a catalogue enum** (`Drum`, `FilterKind`), **no hand-kept second list** (derive from `Drum::ALL`,
  `FilterKind::ALL`), **no font glyphs**, and **nothing on the audio path allocates**.
- **The plan's Progress is the record.** One entry per task: what shipped, the mutations, the review's findings and what was
  done, every ruling with what it costs if wrong, and — when the change is invisible to the guide — why.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write the script with the editor and
  run `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside it,
  git-ignored, and is the right place for scratch scripts, diffs and commit messages (`git commit -F .superpowers/commitmsg.txt`).
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect and sum:
  `cargo test --workspace > /tmp/t.log 2>&1`, then
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest, and a `cd` inside the chain leaves later commands in the
  wrong directory** — run gates as separate commands.
- **`cargo fmt` collapses blank lines but never adds one**, and a long line it reflows can break a mutation script's `old`
  string. Read the diff after formatting, and re-run the mutation script after it.
- **Editing anchors are fragile around doc comments, module boundaries and script blocks.** An `insert` after the last line of
  a `mod tests` puts the new test *outside* the module; a `replace` whose range ends on a closing brace can eat the next
  item's brace. Check `cargo test -- --list` (or the script's own `SKIP` lines) rather than trusting the diff.
- **pi-lens reports type errors as you edit** but flags a file it saw change outside an edit and withholds fix prompts for it;
  the diagnostics are still worth querying after a batch of edits (`lens_diagnostics source=lsp scope=paths`).
- **`tests/no_alloc.rs` is the audio-path gate**: if you add a `Vec`, a `format!`, a lock or an `Arc::clone` inside
  `render`, it fails there.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; `cargo run -p acid-app --bin takka-acid --midi-map` prints
  the note map without opening a device or a window. `cargo run -p acid-app --bin takka-acid` is the app.

## The end of this session

When task 8 is committed: add its `## Progress` entry (what shipped, the mutations, the review's findings, every ruling with
its cost), **rewrite this file as `plans/phase 4/handover-task-9.md`** for the next task — task 9 is *factory banks for the demo
machine*: `Machine::{Acid, Drums}` in `audio/renderer.rs`, `audio/mixer.rs` if the slot type changes, the kit reload on a rate
change in `main`, the audition drain, and `tests/mixer.rs` / `tests/no_alloc.rs` — and prepend the one-line
`> **Superseded — kept for the record, not for instructions.**` note to this file, as task 5's handover did to task 4's.
Carry these into that handover: **`adopt_kit` vs `set_kit` are the load and edit paths and task 8 must use the first**;
**`request_audition`/`take_audition` are already there**; **the rate is a parameter everywhere and `main`'s
`kit_rate`/`write_kit()` order is load-bearing** (the write must happen at the old rate, before the new one is adopted); and
**`SAVE_VERSION` is a queue** — 11 is taken, the rack round is 12, and task 8 should need no field. Do not push; the player
decides when the branch goes up.
