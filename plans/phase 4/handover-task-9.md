# Handover — phase 4, task 9 (for a fresh session)

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

**Where the branch stands.** `HEAD` is **task 8** (`feat(audio): a device's machine is a machine, not always the 303`) and this
handover's own commit is the one after it. Task 8 was reviewed **five times**; every round's findings are in the plan's
Progress and every one is fixed in that commit. Before it: task 7 (`b998751`, six review rounds), its handover, and tasks 1–6
in order. Gates green at task 8: `cargo test --workspace` (**860**), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` from the repo root; the mutation run is **22 of 22 caught**
(`.superpowers/mutate_task8.py`). The branch is **local only** — nothing is pushed. Tasks 1–8 are done; tasks 9 and 10 are not
started. `SAVE_VERSION` is **11** (`SavedState::kit`); the rack round takes **12**.

## Your task: task 9 of the plan — factory banks for the demo variations

From the plan, verbatim:

> ### Task 9 — factory banks for the demo variations
>
> **Files:** modify `rust/crates/acid-core/src/factory.rs` (five drum banks), `rust/crates/acid-app/src/ui/banks.rs` (the drum
> device's `DEMO BANKS`), `rust/crates/acid-app/src/audio/renderer.rs` if a bank needs its kit; tests in `tests/banks_live.rs`.
>
> **Interfaces (produces):** `factory::drum_banks() -> Vec<(&'static str, PatternBank)>` with the five names the acid factories
> use (`Slow Acid`, `Hard Techno Acid`, `Blues`, `Rock`, `Funk`), eight patterns each, every pattern sounding something.
> - [ ] Tests: `every_demo_variation_has_a_drum_bank_of_eight_patterns`; `the_demo_names_are_the_acid_names` (the two lists
>       agree, so the later whole-rack song can pair them); each bank's patterns have at least one hit per bar.
> - [ ] Write the patterns (listening work: a four-on-the-floor for the acid demos, a backbeat for Rock, a shuffle for Funk).
> - [ ] Run; PASS. Commit: `feat(factory): a drum bank for every demo variation`.

**Correct the plan's Files line where the work says so**, as every task so far has (see each Progress entry's "Corrected after
the fact"). Expect `ui/banks.rs` to need the device's kind — `PanelState::showing_kind()` already tells the face which machine it
is drawing (`ui/mod.rs`), and `DeviceKind` is the catalogue; do not add a second list of "kinds with demo banks".

## What tasks 1–8 left you

**Task 1** (`acid-core::drums`): `Drum` — sixteen variants in grid order with `ALL: [Drum; 16]`, `index()`, `label()`
(`KICK SNR RIM CLAP CHH OHH LOT MID HTOM CRS RDE CWB LCO HCO CLV SHK`), `choke_group()` (1 for the hat pair, 0 otherwise),
**`bit() -> u16`** (the one mask-bit spelling) — and `FilterKind` (`Low | Band | High`, `ALL`, `index()`, `label()` = `LP | BP | HP`).
Every `match` is written out.

**Task 2** (`acid-core::pattern`): `Step` carries `hits: u16` and `accents: u16` after `velocity`, with `hit(voice, accented)`
as the one writer of both masks *and* `on`, `clear_hit(voice)`, `is_hit(voice)`, `is_accented(voice)`. `SAVE_VERSION` 11.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice { sample:
Arc<[f32]>, tune, attack_ms, decay_ms, sustain, release_ms, filter, cutoff, resonance, mute }`, `KitVoice::default()`,
`default_kit()`, `DEFAULT_KIT_NAME = "Init"`, `Kit::mute_mask()`/`set_mute_mask()`. On `SharedState`: `kit: ArcSwap<Kit>`,
`drum_mute: AtomicU32`, `audition: AtomicI32`, `kit() -> Arc<Kit>`, `set_kit(Arc<Kit>)` (**the edit path**), `adopt_kit(Kit)`
(**the load path** — seeds `drum_mute` from the kit, which `set_kit` deliberately does not), `request_audition(Drum)` and
`take_audition() -> Option<Drum>`. **The renderer drains `take_audition` once per block**; nothing else may.

**Task 4** (`acid-app`): `DeviceKind::Drums` (`name()` = `DRUMS`, `has_kit()`), `StripCell.kind`, `PanelState::showing_kind()`,
`PanelState::drum: DrumView`, `ui/drum_panel.rs` (the face: seven repurposed dials, `LP | BP | HP`, `SHUFFLE`, `HIT`, the grid),
`ui::kits::edit` as the one seam a kit edit goes through.

**Task 5** (`acid-app::wav`): `decode(bytes, target_rate) -> Result<Arc<[f32]>, WavError>`, `encode(samples, rate) -> Vec<u8>`.

**Task 6** (`acid-app::persist`, `kits`, `ui::kits`): a kit is a folder, `<data>/kits/<name>/kit.json` + one `<VOICE>.wav` per
sampled voice — `save_kit`, `load_kit` (partial: a broken voice costs one voice and arrives in `problems`), `import_kit`,
`kit_names`, `free_kit_name`, `kit_name`, `free_name`; `kits::factory_kits(rate)`, `factory_names()`, `is_factory()`,
`is_reserved()`, `default_name()` = `"808"`. `main` decodes at `AudioStatus::Running { sample_rate }`, loads through
`adopt_kit`, and writes an edited kit's folder on save.

**Task 7** (`acid-core::sequencer`, `voice`, `filter`, `drums`) — the machine itself:

```rust
// acid_core::sequencer
pub trait Performer: Sized {
    fn step(&mut self, step: &Step);
    fn release(&mut self);
    fn render(&mut self, out: &mut [f32]);
    fn arm(&mut self);
}
impl Performer for Voice;                        // the 303's performer; `Voice` has NO inherent `render`
impl Sequencer {
    pub fn render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32]);
    // ...and the clock it always had: new, set_pattern, pattern, set_bpm, set_swing, start, start_at, start_later,
    // bar_line, stop, settle_direction, is_running, set_reverse, reverse_is_pending, is_reverse, resume, position, clock
}
// acid_core::drums
impl Performer for DrumMachine;                  // private; NOT `Drums` (the performer must be a field of the machine)
pub struct Drums;                                // a `Sequencer` and the sixteen voices
impl Drums {
    pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self;   // seeds the mute mask from the kit (the load path)
    pub fn set_kit(&mut self, kit: Arc<Kit>);              // the edit path: no mask reseed, voices keep their state
    pub fn set_mute_mask(&mut self, mask: u16);
    pub fn audition(&mut self, drum: Drum);
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
// acid_core::param
pub const VOLUME_MS: f32;     // the one volume-ramp time constant, used by `Engine` and by the renderer's drum gain
```

**Task 8** (`acid-app::audio::renderer` + `state.rs` + `device.rs`) — the renderer holds a machine:

```rust
// acid_app::state
pub struct SharedState {
    pub kind: DeviceKind,    // what this device is; set at construction, never changed, **not saved**
    // ...every field it had
}
impl SharedState {
    pub fn new() -> Arc<Self>;                                   // the 303's
    pub fn for_kind(kind: DeviceKind) -> Arc<Self>;
    pub fn with_tempo(tempo: Arc<AtomicF32>) -> Arc<Self>;       // the 303's, on the rack's tempo cell
    pub fn with_tempo_for(kind: DeviceKind, tempo: Arc<AtomicF32>) -> Arc<Self>;
}
// acid_app::device — `Device` has NO `kind` field any more
impl Device {
    pub fn kind(&self) -> DeviceKind;                            // reads `self.state.kind`
    pub fn new(kind: DeviceKind, number: usize) -> Self;
    pub fn on(number: usize, state: Arc<SharedState>) -> Self;   // the state already says which machine it is
    pub fn adopted(number: usize, state: Arc<SharedState>, bank: PatternBank) -> Self;
}
// acid_app::audio::renderer
pub fn apply(params: &Parameters, engine: &mut Engine);          // the 303's seven knobs, volume, tempo, swing
pub fn apply_drums(params: &Parameters, drums: &mut Drums);      // tempo and swing — nothing else
impl AudioRenderer {
    pub fn new(sample_rate: f32, channels: usize, state: Arc<SharedState>, max_block: usize) -> Self;
    pub fn render_mono(&mut self, out: &mut [f32]);
    pub fn render_into(&mut self, out: &mut [f32]);
    pub fn bar_line(&self) -> Option<usize>;
    pub fn start_later(&mut self, step: usize, silence: usize);
    pub fn seek(&mut self, step: usize);
    pub fn has_pending_start(&self) -> bool;
}
// inside: enum Machine { Acid(Box<Engine>), Drums { drums: Box<Drums>, kit: Arc<Kit>, volume: Smooth } }
```

`AudioRenderer::engine()` is **gone** (a `Machine` cannot answer `&Engine` for a drum device); `tests/renderer.rs` reads
`state.position` instead. There is no `stop`/`pause`/`resume` on the renderer: the transport arrives through
`Parameters::transport` and the private `settle_transport` decides what a change means — the three states are not three calls.

**The app side as it is now** (`audio/mixer.rs`): one `Slot { state, renderer, mute, waiting }` per published device, slot 0 the
bar-line reference; a waiting slot is skipped entirely and released on the reference's `bar_line`. A paused rack has no bar line,
so a joiner takes the step the resume will use (`seek`). Neither the mixer's structure nor `BusState` changed in task 8 — the
kind reaches the renderer through the state it was already given. (`audio/mixer.rs` did change, but its prose only: the module
and `Slot` docs stopped calling every slot an engine, and `MUTE_MS` is now `acid_core::param::VOLUME_MS`.)

## Traps this task owns

1. **A drum bank's patterns are `hits`/`accents`, not notes.** `Step::hit(Drum, accented)` is the only writer of the masks and of
   `on`; a bank written with `Step::note(...)` would sound nothing on a drum device and a 303 line on a bass. Derive every row
   from `Drum::ALL` (`Drum::index`, `Drum::bit`) — a hand-kept list of sixteen names is the bug this repo has paid for twice.
2. **The five names are the acid side's, and they are checked.** `factory::drum_banks()` must answer exactly the names the
   demo songs use (`Slow Acid`, `Hard Techno Acid`, `Blues`, `Rock`, `Funk`) — **they live in `acid-app::demo::songs()`**, not in
   `acid_core::factory` (which names no bank: it holds `factory_patterns() -> [Pattern; 8]`), and the later rack round pairs them
   by name. `the_demo_names_are_the_acid_names` is the test, and it must read `demo::songs()` rather than restate the five.
3. **Eight patterns each, and every pattern sounds something.** "Sounding something" means at least one hit per bar for every
   pattern, not one hit in the bank: a silent pattern in `BANKS` is a dead cell on the face. The plan's test says "each bank's
   patterns have at least one hit per bar" — write it that way (every pattern × every step, counting `is_hit` over `Drum::ALL`).
4. **`DEMO BANKS` is the drum device's, and the face already routes by kind.** `ui/banks.rs` draws the acid `DEMO BANKS`; a drum
   device must draw the drum banks instead, and `PanelState::showing_kind()` is the one question to ask. The keys and the
   card's metrics are shared (`CARD_*`, `heading`) — do not fork them.
5. **Loading a bank into a drum device goes through the device's own state** (`SharedState::set_pattern`), which is the path the
   renderer reads once per block (`Arc::ptr_eq`). It must not reach into the renderer, and it must not touch the kit.
6. **A demo bank is not a song.** Phase 4 loads the drums' banks per device; the song that loads the acid bank and the drum bank
   together is the rack round's (`SAVE_VERSION` 12). Do not add a document field.
7. **Listening work is a decision, and it is the player's ear.** `EAR_TESTING.md` is how to judge it. If a pattern is written
   from the head and nobody has heard it, say so in the Progress where the patterns are written (task 6 did exactly that for
   the kits' per-voice defaults), rather than implying the ear has passed.
8. **Nothing on the audio path allocates**, and this task adds no audio-path code: a bank is `Pattern`s, published through the
   existing `ArcSwap`. `tests/no_alloc.rs` already covers a drum slot; keep it green.
9. **The book is task 10's**, and its Files line now names page 22, `04`, `14-midi`, `15-keyboard`, `19`, `20`,
   `01-what-it-is`, `SUMMARY.md`, `check.sh` and the images. If task 9 changes what a player hears (a drum device's `BANKS` now
   offers five banks where it offered the acid ones), say in the Progress whether the book moved and why — a face that now lists
   `DEMO BANKS` with five drum grooves is a thing page 22 will describe, and no page describes it today.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` when the book moved.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you — and task 8's five rounds are the
  reason: each round after the first still found something real, including a test that passed without exercising the thing it
  claimed. Save the diff (a new file needs `git add -N` before `git diff` sees it) under `.superpowers/review/`, hand the
  reviewer this task's **Files** and **Interfaces** blocks plus the diff, and act on what it finds in the same commit.
  Regenerate the diff after every fix.
- **Prove each test bites.** `.superpowers/mutate_task8.py` is the current pattern: the full path for a lib test
  (`filter::tests::name`), the **bare** name for an integration test (a test binary's root is its file, so `banks_live::name`
  matches nothing and "passes" with zero run), a list of edits per mutation, and an `"equivalent"` marker the run fails on if it
  is ever caught. **A test that passes without exercising its branch is a hole** — task 8's rounding found one, and the fix was
  to assert the branch had run (`waited > 0`).
- **No `_` arm on a catalogue enum** (`Drum`, `FilterKind`, `DeviceKind`), **no hand-kept second list** (derive from
  `Drum::ALL`/`Drum::index`/`Drum::bit`), **no font glyphs** (except `…` via `ui::text::fit`/`settings::shorten`), and
  **nothing on the audio path allocates**.
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
  The count at task 8 is **860 passed, 0 failed**.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest, and a `cd` inside the chain leaves later commands in the
  wrong directory** — run gates as separate commands.
- **`cargo fmt` collapses blank lines but never adds one**, and a long line it reflows can break a mutation script's `old`
  string. Read the diff after formatting, and re-run the mutation script after it.
- **Editing anchors are fragile around doc comments, module boundaries and script blocks.** An `insert` after the last line of a
  `mod tests` puts the new test *outside* the module; a `replace` whose range ends on a closing brace can eat the next item's
  brace; a bare `);` anchor can belong to a different function than the one you meant (task 8 inserted a block into the wrong
  test that way). Prefer anchors served by `read` immediately before the edit, and check `cargo test -- --list` (or the
  script's own `SKIP` lines) rather than trusting the diff.
- **pi-lens reports type errors as you edit** and sometimes reports a *stale* snapshot (it will insist a call has the wrong
  arity after you have fixed it); `cargo check` and `cargo test` are the authority.
- **`tests/no_alloc.rs` is the audio-path gate**: if you add a `Vec`, a `format!`, a lock or a per-block `Arc::clone` inside
  `render`, it fails there. It counts allocations, so it cannot see a *free* — the renderer's module doc says where one happens
  on purpose.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; `cargo run -p acid-app --bin takka-acid --midi-map` prints
  the note map without opening a device or a window. `cargo run -p acid-app --bin takka-acid` is the app.
- **The screenshots recipe in AGENTS.md is load-bearing**: check `xdotool getdisplaygeometry` says 2000×1400 before trusting a
  grab, because something on this machine pre-starts Xvfb at 1280×800 and a bigger request fails silently.

## The end of this session

When task 9 is committed: add its `## Progress` entry (what shipped, the mutations, the review's findings, every ruling with its
cost, and whether the book moved), **rewrite this file as `plans/phase 4/handover-task-10.md`** for the last task — the book:
page 22, `SUMMARY.md`, `04-the-face`, `19-a-devices-card`, `20-reference`, `01-what-it-is` (its framing now that a drum machine
ships), `check.sh`'s pinned facts and the screenshots — and prepend the one-line
`> **Superseded — kept for the record, not for instructions.**` note to this file, as every handover before it has done.

Carry these into that handover: **`adopt_kit` and `set_kit` are the load and edit paths, and a kit reload through the load path
reseeds the player's mutes** (a wishlist entry, not this round's); **`take_audition` is drained once per block, by the renderer,
and nothing else may**; **`main`'s `kit_rate`/`write_kit()` order is load-bearing** (the write happens at the old rate, before
the new one is adopted); **`SharedState::kind` is the one answer to "what kind is this device" and `Device` has no field for
it**; **`SAVE_VERSION` is 11 and the rack round takes 12**; and **the guide is task 10's, except anything task 8 or 9 makes a
player hear differently** — say in the Progress which it was and why. Do not push; the player decides when the branch goes up.
