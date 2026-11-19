# Handover — phase 4, task 8 (for a fresh session)

> **Superseded — kept for the record, not for instructions.**
>
> Picked up and carried through in the commit after it: task 8 is done, its Progress entry is in
> `plans/phase 4/01-the-drum-machine.md`, and the live handover is `plans/phase 4/handover-task-9.md`.

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

**Where the branch stands.** `b998751` is **task 7** (`feat(core): the drum machine — sixteen voices on the 303's clock`) and
this handover's own commit is the one after it. `b998751` was reviewed **six times**; every round's findings are in the plan's
Progress and every one is fixed in that commit. Before it: `c7f8040` task 7's handover, `d4ba97f` task 6's follow-up, `044d5e0`
task 6, `cc6cd2e` task 6's handover, `e1b0640` task 5's handover, `00af7c1` task 5, `718b8d2` task 4, `0520760` task 3,
`b74179d` task 2, `0f7890d` task 1.

Gates green at `b998751`: `cargo test --workspace` (**848**), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` from the repo root; the mutation run is **42 of 42 caught, 2 recorded as
equivalent** (`.superpowers/mutate_task7.py`). The branch is **local only** — nothing is pushed. Tasks 1–7 are done; tasks 8–10
are not started. `SAVE_VERSION` is **11** (`SavedState::kit`); the rack round takes **12**.

## Your task: task 8 of the plan — the renderer holds a machine

From the plan, verbatim:

> ### Task 8 — the renderer holds a machine
>
> **Files:** modify `rust/crates/acid-app/src/audio/renderer.rs` (`Machine`), `audio/mixer.rs` if the slot type changes,
> `state.rs` if the kit/mask/audition need wiring, `main.rs` (the kit reload on a rate change, the audition key's request);
> tests in `tests/mixer.rs`, `tests/no_alloc.rs`.
>
> **Interfaces (consumes Task 7; produces):** `AudioRenderer` holds `Machine::{Acid(Engine), Drums(Drums)}` and keeps its public
> surface (`render_mono`, `bar_line`, `start_later`, `seek`, `stop`, `pause`, `resume`, `has_pending_start`).
> - [ ] Tests: `a_drum_device_sounds_through_the_mixer` (a bar of the stub kit is audible and its bar line matches the 303's in the
>       same rack); `no_alloc` still passes with a drum slot rendering; `all_sixteen_voices_struck_at_once_do_not_exceed_the_ceiling`
>       (Review Focus 5's second half: the mixer's limiter holds the sum); `every_voice_muted_is_silence` (through the mixer, with the
>       steps still on); `a_kit_swapped_while_a_voice_sounds_keeps_the_renderer_legal` (the `Arc` outlives the block, no panic, no
>       allocation).
> - [ ] Run; expect failures. Implement, including `apply_drums` and the audition drain. Run; PASS. Commit:
>       `feat(audio): a device's machine is a machine, not always the 303`.

**The one question the plan does not answer, and you must rule on first.** `AudioRenderer` learns nothing but a
`SharedState` — `mixer::reconcile` builds it as `AudioRenderer::new(rate, 1, Arc::clone(state), max_block)` from
`BusState.slots`, which is `Vec<Arc<SharedState>>` and carries **no `DeviceKind`**. So a renderer cannot yet tell a drum device
from a bass. Two ways, both small:

- put the kind on `SharedState` (a field set where the device is made; not saved, so no `SAVE_VERSION` move), or
- change `BusState.slots` to carry `(Arc<SharedState>, DeviceKind)` beside `routes`, which is paired by index with `slots` in
  `device::publish` and read by the MIDI routers — the one rule there is that `routes[i]` belongs to `slots[i]`.

Record the ruling in the Progress with its cost. The larger, later option (a device owning a `Machine` trait object) is the
wishlist's entry, not this round's.

## What tasks 1–7 left you

**Task 1** (`acid-core::drums`): `Drum` — sixteen variants in grid order with `ALL: [Drum; 16]`, `index()`, `label()`
(`KICK SNR RIM CLAP CHH OHH LOT MID HTOM CRS RDE CWB LCO HCO CLV SHK`), `choke_group()` (1 for the hat pair, 0 otherwise) and
**`bit() -> u16`** (task 7 added it: `1u16 << index()`, the one mask-bit spelling; `pattern::drum_bit` was deleted onto it) —
and `FilterKind` (`Low | Band | High`, `ALL`, `index()`, `label()` = `LP | BP | HP`). Every `match` is written out.

**Task 2** (`acid-core::pattern`): `Step` carries `hits: u16` and `accents: u16` after `velocity`, with `hit(voice, accented)`
as the one writer of both masks *and* `on`, `clear_hit(voice)`, `is_hit(voice)`, `is_accented(voice)`. `SAVE_VERSION` 11.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice { sample:
Arc<[f32]>, tune, attack_ms, decay_ms, sustain, release_ms, filter, cutoff, resonance, mute }`, `KitVoice::default()`,
`default_kit()`, `DEFAULT_KIT_NAME = "Init"`, `Kit::mute_mask()`/`set_mute_mask()`. On `SharedState`: `kit: ArcSwap<Kit>`,
`drum_mute: AtomicU32`, `audition: AtomicI32`, with `kit() -> Arc<Kit>`, `set_kit(Arc<Kit>)` (**the edit path**),
`adopt_kit(Kit)` (**the load path** — seeds `drum_mute` from the kit, which `set_kit` deliberately does not),
`request_audition(Drum)` and `take_audition() -> Option<Drum>` (the `-1` sentinel lives there). **Task 8 must consume
`adopt_kit`/`request_audition`/`take_audition`, not spell them again.**

**Task 4** (`acid-app`): `DeviceKind::Drums` (`name()` = `DRUMS`, `has_kit()`), `Device::on`, `StripCell.kind`,
`PanelState::showing_kind()`, `PanelState::drum: DrumView`, `ui/drum_panel.rs` (the face), `ui::kits::edit` as the one seam a
kit edit goes through. The drum face's `HIT` key already calls `request_audition`; the panel needs nothing from you.

**Task 5** (`acid-app::wav`): `decode(bytes, target_rate) -> Result<Arc<[f32]>, WavError>` and `encode(samples, rate) -> Vec<u8>`.

**Task 6** (`acid-app::persist`, `kits`, `ui::kits`): a kit is a folder, `<data>/kits/<name>/kit.json` + one `<VOICE>.wav` per
sampled voice, with `save_kit`, `load_kit` (**partial**: a broken voice costs one voice and arrives in `problems`), `import_kit`,
`kit_names`, `free_kit_name`, `kit_name`, `free_name`; `kits::factory_kits(rate)`, `factory_names()`, `is_factory()`,
`is_reserved()`, `default_name()` = `"808"`. `main` decodes once the device reports `AudioStatus::Running { sample_rate }`,
loads through `adopt_kit`, and writes an edited kit's folder on save.

**Task 7** (`acid-core::sequencer`, `voice`, `filter`, `drums`) — the machine itself:

```rust
// acid_core::sequencer
pub trait Performer: Sized {
    fn step(&mut self, step: &Step);
    fn release(&mut self);
    fn render(&mut self, out: &mut [f32]);
    fn arm(&mut self);
}
impl Performer for Voice;            // `prev_slide` lives here now; `Voice` has NO inherent `render`
impl Sequencer {
    pub fn render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32]);
    // ...and everything it had: new, set_pattern, pattern, set_bpm, set_swing, start, start_at, start_later,
    // bar_line, stop, settle_direction, is_running, set_reverse, reverse_is_pending, is_reverse, resume,
    // position, clock
}
// acid_core::drums
pub struct Drums;                    // a `Sequencer` and a private `DrumMachine` (kit, sixteen voice states, mask, transpose,
                                     // audition); `DrumMachine` — not `Drums` — is the `Performer`
impl Drums {
    pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self;   // seeds the mute mask from the kit (the load path)
    pub fn set_kit(&mut self, kit: Arc<Kit>);              // the edit path: no mask reseed, voices keep their state
    pub fn set_mute_mask(&mut self, mask: u16);
    pub fn audition(&mut self, drum: Drum);                // one slot, served at the next render
    pub fn render(&mut self, out: &mut [f32]);
    pub fn set_pattern(&mut self, pattern: Arc<Pattern>);
    pub fn set_bpm(&mut self, bpm: f32);
    pub fn set_swing(&mut self, swing: f32);
    pub fn position(&self) -> usize;
    pub fn bar_line(&self) -> Option<usize>;
    pub fn start(&mut self);  pub fn start_at(&mut self, step: usize);  pub fn start_later(&mut self, step: usize, silence: usize);
    pub fn stop(&mut self);   pub fn pause(&mut self);  pub fn resume(&mut self);
    pub fn set_reverse(&mut self, on: bool);  pub fn set_transpose(&mut self, semitones: i32);
}
// acid_core::filter
pub struct MultimodeFilter;   // new, set_kind, set_cutoff_hz, cutoff_hz, set_resonance, resonance, reset, process
```
`Drums::stop` and `pause` call a private `hush()` (clears a pending audition, releases every voice) because an audition sounds
with no transport under it, and `Sequencer::stop` closes only a gate it opened. `Envelope`'s three time setters are now
**idempotent** (no-op when the value has not moved) — that matters if your renderer re-states anything per block.

**The app side as it is now** (`audio/renderer.rs`): `AudioRenderer { engine: Engine, transport, reversed, transpose, state,
mono, pattern, channels, max_block, midi_gate_on, pending_start }`; `apply(&Parameters, &mut Engine)` is a free function;
`settle_transport` holds the stop-vs-pause knowledge; `render_mono` reads `state.snapshot()`, then `apply`, then
`settle_transport`, then reverse/transpose, then `pending_start`, then the MIDI gate, then the pattern (with `Arc::ptr_eq`), then
renders in `max_block` chunks and stores `state.position`. `mixer::reconcile` keeps one `Slot { renderer, … }` per published
device, slot 0 is the bar-line reference and later slots may be `waiting`.

## Traps this task owns

1. **One machine per device, chosen by the device's kind** — `Machine::{Acid(Engine), Drums(Drums)}` inside `AudioRenderer`,
   with its public surface unchanged so the mixer needs no rewiring. See the question above: the kind has to arrive first.
2. **`apply` must split.** A drum device wants `tempo_bpm`, `swing`, `transport`, `reversed` and `transpose`; the seven 303
   knobs (`waveform`, `tuning`, `cutoff`, `resonance`, `env_mod`, `decay`, `accent`) and `volume` are the bass's, and on a drum
   device the face keeps its values **in the kit** (task 4's ruling), not in the 303's atomics. Pushing them nowhere is right;
   pushing them into a `Drums` would be a second source of truth. `volume` (the master) is the mixer's or the Engine's — check
   where it lands today before moving it.
3. **The kit and the mask are read once per block**: `state.kit()` compared with `Arc::ptr_eq` against the last one handed to
   `drums.set_kit`, and `state.drum_mute.load(Relaxed) as u16` passed to `set_mute_mask`. Never per sample, never a guard held
   across the block. A kit swapped mid-block stays alive because `Drums` holds the `Arc`.
4. **The audition is drained, not polled**: `state.take_audition()` answers once per call, so call it once per block and hand
   the `Drum` to `drums.audition`. `HIT` must sound with the transport stopped and `STOP` must cut it (it does: `Drums::hush`).
5. **`state.position` must be the drum machine's step on a drum device** — the panel's chase reads it — and `bar_line()` must be
   the machine's, or a device joining a playing rack waits on nothing.
6. **`pending_start` is the paused/playing dance**: a joining device on a *paused* rack takes `start_at`, on a *playing* one
   `start_later(step, silence)` at the bar line, and it must not run while the rack says paused. Copy the existing shape
   (`engine.start_later` then `pause`) rather than inventing one.
7. **Nothing allocates or locks on the audio path** (`tests/no_alloc.rs`). One `Arc` read, one `u32` load, one `Option<Drum>`.
8. **The ceiling is the mixer's.** Sixteen voices struck at once sum to about sixteen; `DrumMachine` clamps each voice to ±1 and
   does not limit the sum, because the spec puts the ceiling in the mixer's limiter. `all_sixteen_voices_struck_at_once_do_not_exceed_the_ceiling`
   is the test that says so.
9. **A kit swapped while a voice sounds keeps the renderer legal**: `Arc<Kit>` held by `Drums`, voices keep their positions and
   filter states (task 7's trap 14), no panic, no allocation.
10. **The rate is a parameter everywhere.** `main` decodes kits at the running rate and reloads on a rate change — and writes a
    pending kit **at the old rate first** (`write_kit()` before `kit_rate = Some(rate)`): `save_kit` stamps the rate it is given
    into every WAV, so adopting first would label old samples with the new rate for ever. Do not decode anything in the renderer.
11. **No new save-file field.** A drum device's kit name is already in `SavedState::kit`; the rack round takes `SAVE_VERSION`
    12 and this task should take none.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` when the book moved. Task 8 probably does not move the book: the drum face
  has no page yet (task 10 writes page 22) and what a player hears on a drum device changes from the 303's line to the drum
  machine — which is the one thing page 22 will describe. If you decide the book must move, say so in the Progress and say why;
  if not, say that too.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Save the diff (a new file needs
  `git add -N` before `git diff` sees it) under `.superpowers/review/`, dispatch the `reviewer` agent with this task's **Files**
  and **Interfaces** blocks — not the whole plan — and act on what it finds before committing. Task 7 took **six** rounds and
  every one after the first still found something real; a review of a stale diff is a review that did not happen, so regenerate
  it after you fix anything.
- **Prove each test bites.** Mutate, watch the named test fail, revert, write down what you mutated.
  `.superpowers/mutate_task7.py` is the current pattern and the best one to copy: it uses the full test path for a lib test
  (`filter::tests::name`), the **bare** name for an integration test (`tests/drums.rs`'s root is the file, so `drums::name`
  matches nothing and "passes" with zero run), refuses a filter that matches nothing, supports a mutation of several edits (a
  list of `old` texts and a list of `new` ones), and marks an `"equivalent"` mutation — one that *cannot* be caught because the
  expression it rewrites is equivalent — so the run fails if such a mutation is ever caught. An **equivalent expression is not a
  mutation**; record it rather than counting it.
- **No `_` arm on a catalogue enum** (`Drum`, `FilterKind`, `DeviceKind`), **no hand-kept second list** (derive from
  `Drum::ALL`, `FilterKind::ALL`, `DeviceKind::ALL`), **no font glyphs**, and **nothing on the audio path allocates**.
- **The plan's Progress is the record.** One entry per task: what shipped, the mutations, the review's findings and what was
  done, every ruling with what it costs if wrong, and — when the change is invisible to the guide — why.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write the script with the editor and
  run `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside it,
  git-ignored, and is the right place for scratch scripts, diffs and commit messages (`git commit -F .superpowers/commitmsg.txt`).
  A shell *command* may still write to `/tmp` (`cargo test > /tmp/t.log`).
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
- **pi-lens reports type errors as you edit** but sometimes reports a *stale* snapshot (it will insist a type does not exist
  after you have written it); `cargo check` and `cargo test` are the authority.
- **`tests/no_alloc.rs` is the audio-path gate**: if you add a `Vec`, a `format!`, a lock or a per-block `Arc::clone` inside
  `render`, it fails there.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; `cargo run -p acid-app --bin takka-acid --midi-map` prints
  the note map without opening a device or a window. `cargo run -p acid-app --bin takka-acid` is the app.
- **The screenshots recipe in AGENTS.md is load-bearing**: check `xdotool getdisplaygeometry` says 2000×1400 before trusting a
  grab, because something on this machine pre-starts Xvfb at 1280×800 and a bigger request fails silently.

## The end of this session

When task 8 is committed: add its `## Progress` entry (what shipped, the mutations, the review's findings, every ruling with its
cost), **rewrite this file as `plans/phase 4/handover-task-9.md`** for the next task — task 9 is *factory banks for the demo
variations*: `factory::drum_banks() -> Vec<(&'static str, PatternBank)>` with the acid factories' five names, eight patterns
each, the drum device's `DEMO BANKS` in `ui/banks.rs`, and tests in `tests/banks_live.rs` — and prepend the one-line
`> **Superseded — kept for the record, not for instructions.**` note to this file, as task 7's handover did to task 6's.

Carry these into that handover: **`adopt_kit` vs `set_kit` are the load and edit paths and the renderer uses the first only for
a kit it was handed whole**; **`request_audition`/`take_audition` are drained once per block**; **`main`'s `kit_rate`/`write_kit()`
order is load-bearing** (the write happens at the old rate, before the new one is adopted); **`SAVE_VERSION` is 11 and the rack
round takes 12**; and **the guide is task 10's, except anything task 8 makes a player hear differently** — say in the Progress
which it was and why. Do not push; the player decides when the branch goes up.
