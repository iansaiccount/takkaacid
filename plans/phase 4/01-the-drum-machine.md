# Plan 01 — the drum machine

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, each commit reviewed by a
> dispatched subagent before it lands (this repo's rule, which is stricter than the skill's default).

**Goal:** a second machine in the rack — sixteen sample voices against the 303's sixteen steps, with per-voice ADSR, filter and
tuning, three factory kits, custom kits from the player's own WAVs, and a factory drum bank for each demo variation.

**Architecture:** the drum machine is a `Machine` inside the existing per-device `AudioRenderer`; the clock, bar, `REVERSE`,
swing, pause and bar line stay in `acid_core::sequencer`, which gains a `Performer` trait so a voice and a drum bank share one
clock by construction. A drum step is the 303's `Step` with two `u16` masks added. A kit is a folder of WAVs and a `kit.json`,
held on `SharedState` as an `ArcSwap<Kit>` and compiled-in for the three factory kits.

**Tech Stack:** Rust, `acid-core` (no UI, no deps beyond serde), `acid-app` (egui/eframe, cpal, midir, arc-swap, serde_json).
No new dependency: the WAV decoder is hand-rolled.

**Spec:** [`docs/superpowers/specs/2026-11-18-drum-machine-design.md`](../../docs/superpowers/specs/2026-11-18-drum-machine-design.md)

## Global Constraints

- `AGENTS.md` in full: one catalogue per kind of control with **no `_` arm** on a catalogue enum, no hand-kept second list, no
  font glyphs except `…` via `ui::text::fit`/`settings::shorten`, `persist::SavedState` is a contract (`SAVE_VERSION` bump + a
  `tests/persist.rs` pin), a message is a notice, nothing allocates or locks on the audio path (`tests/no_alloc.rs`), and three
  threads with no lock across the callback.
- **Every surface of the drum face paints from `theme::Palette`.** No `theme::BLACK`, no literal colours.
- **Bundled samples are CC0 or public domain**, each with source URL and licence in `kits/CREDITS.md`.
- The book is part of the task that changes behaviour, not a later one.

## Review Focus

The input classes the spec implies and no task's happy-path test would otherwise catch. Each has its test named in the task that
owns the code.

1. **A cell clicked while the line runs** — the write must land on the step the pointer is over, not the step the chase moved to
   between frames (the panel's pattern is refreshed from `shared.pattern()` each frame).
2. **A kit whose WAV is missing, renamed or unreadable** — a notice naming the file and the voice, the kit loads without it, and
   nothing panics or silently plays the previous file.
3. **A long sample under `STOP`, and a hit retriggered** — the tail is cut where the stop says and the retrigger restarts the
   sample, rather than the old tail bleeding into the next bar.
4. **Scale extremes, 0.65× and 1.6×** — cells stay hittable and labels do not collide; the layout is derived from the band, not
   from constants in design pixels.
5. **All sixteen voices muted, and all sixteen struck on one step** — the first is silence with the pattern still "on", the second
   does not clip: the mixer's ceiling holds it.

## What is wrong today

There is one machine kind (`DeviceKind::Acid`), its engine is `acid_core::Engine` owned by `audio::renderer::AudioRenderer`, a
`Step` holds one note (`on/note/octave/velocity/accent/slide`), and a session saves one device's bank and knobs. Nothing plays a
sample, no kit exists, and the device card has no kit row. The wishlist's "second device type" entry names the seam this round
takes: the trait object belongs in the renderer, and the pattern is the hard part.

## The design, in the shape a task needs

The spec is the authority; this is the part a planner must hold: `Step { hits: u16, accents: u16 }` with `Step::hit` as the only
writer of all three (mask bits and `on`); `Drum` as a sixteen-variant catalogue; `Kit { name, voices: [KitVoice; 16] }` with
`KitVoice { sample: Arc<[f32]>, tune, attack_ms, decay_ms, sustain, release_ms, filter: FilterKind, cutoff, resonance, mute }`;
`Machine::{Acid(Engine), Drums(Drums)}` in the renderer; `SharedState` gaining `kit: ArcSwap<Kit>`, `drum_mute: AtomicU32` and
`audition: AtomicI32`; kits at `<data>/kits/<name>/` with `kit.json` + one WAV per voice; and a drum face whose grid is the editor
and whose seven knobs are the selected voice's.

## Tasks

### Task 1 — the voice catalogue

**Files:** create `rust/crates/acid-core/src/drums.rs`; modify `rust/crates/acid-core/src/lib.rs`; and
`docs/superpowers/specs/2026-11-18-drum-machine-design.md` for one label (`HighTom` is `HTOM`, because the knob row's audition
key is `HIT`).
**Interfaces (produces):**
```rust
pub enum Drum { Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom, MidTom, HighTom, Crash, Ride, Cowbell,
                LowConga, HighConga, Clave, Shaker }
impl Drum {
    pub const ALL: [Drum; 16];
    pub fn index(self) -> usize;                  // written out, no `_` arm
    pub fn label(self) -> &'static str;           // "KICK" … "SHK", ASCII
    pub fn choke_group(self) -> u8;               // 1 for ClosedHat/OpenHat, 0 otherwise
}
pub enum FilterKind { Low, Band, High }
impl FilterKind { pub const ALL: [FilterKind; 3]; pub fn index(self) -> usize; pub fn label(self) -> &'static str; }
```
- [ ] Test `the_sixteen_voices_are_labelled_numbered_and_chase_in_one_group` — order matches `ALL`, every label ASCII and ≤4
      chars, `choke_group` non-zero for exactly `ClosedHat` and `OpenHat`, and `index()` round-trips.
- [ ] Run it; expect a compile error naming the missing type. Then implement both catalogues.
- [ ] Run `cargo test -p acid-core`; expect PASS. Commit: `feat(core): the drum voice catalogue`.

### Task 2 — the pattern's masks

**Files:** modify `rust/crates/acid-core/src/pattern.rs`, `rust/crates/acid-app/src/persist.rs`, `rust/crates/acid-app/tests/persist.rs`.
**Interfaces (consumes Task 1; produces):**
```rust
pub struct Step { /* existing fields */ pub hits: u16, pub accents: u16 }
impl Step {
    pub fn hit(&mut self, drum: Drum, accented: bool);  // sets the hit bit, the accent bit, and `on`
    pub fn clear_hit(&mut self, drum: Drum);            // clears both bits, and `on` when no hit remains
    pub fn is_hit(&self, drum: Drum) -> bool;
    pub fn is_accented(&self, drum: Drum) -> bool;
}
```
- [ ] Tests in `pattern.rs`: `a_hit_sets_the_mask_the_accent_and_on`, `clearing_the_last_hit_rests_the_step`,
      `a_drum_edit_leaves_the_303_fields_alone`, `a_pattern_without_masks_is_a_303_pattern` (all masks zero, `on` unchanged).
- [ ] Test in `tests/persist.rs`: `a_version_nine_file_upgrades_with_empty_drum_masks` — a pinned version-9 document loads, every
      `hits`/`accents` is zero, and the 303's steps are byte-identical after a round trip; plus the byte pin for `SAVE_VERSION` 10.
- [ ] Run them; expect failures. Implement the fields with `#[serde(default)]`, the four methods, and `SAVE_VERSION = 10` with an
      upgrade that leaves the masks zero.
- [ ] `cargo test --workspace`; expect PASS. Commit: `feat(pattern): a step carries the drum masks, and the save file moves to 10`.

### Task 3 — the kit, in memory

**Files:** create `rust/crates/acid-core/src/drums.rs` additions (`Kit`, `KitVoice`, `default_kit`); modify
`rust/crates/acid-app/src/state.rs`.
**Interfaces (produces):** `Kit { pub name: String, pub voices: [KitVoice; 16] }`, `KitVoice` as the spec defines it, and on
`SharedState`: `pub kit: arc_swap::ArcSwap<Kit>`, `pub drum_mute: AtomicU32`, `pub audition: AtomicI32`. `fn kit(&self) ->
arc_swap::Guard<Arc<Kit>>` mirrors `pattern()`.
- [ ] Test: `a_default_kit_has_sixteen_voices_and_no_samples` (empty `Arc<[f32]>`, default envelope/filter values, every voice
      unmuted), and `the_drum_mute_mask_starts_from_the_kit` (the mask is the kit's `mute` flags packed in catalogue order).
- [ ] Implement; run; PASS. Commit: `feat(drums): a kit lives on the device's shared state`.

### Task 4 — the face

**Files:** modify `rust/crates/acid-app/src/device.rs` (`DeviceKind::Drums` + `Device::on`), `ui/mod.rs` (route by kind),
`ui/device_card.rs` (the card's rows for a drum device); create `rust/crates/acid-app/src/ui/drum_panel.rs`; add to
`tests/drum_live.rs` (new).
**Interfaces (consumes Tasks 1–3; produces):** `drum_panel::draw(state, shared, ui, &layout, s, p)`, and `PanelState::drum:
DrumView { selected: Drum }` (panel state; not saved).
- [ ] `ui/drum_panel.rs` draws: the seven relabelled knobs and the device's `SHUFFLE` dial, the `LP | BP | HP` key, `HIT`, the
      sixteen-row grid with beat frames, the selected voice's lit plate and row band, a muted voice dim, and the voice's name over
      the knobs. Everything from `p: &Palette`.
- [ ] `draw_panel` branches on `device.kind`: a drum device draws `drum_panel` and **not** the knob, performance or step rows.
- [ ] Live tests in `tests/drum_live.rs`: `the_grid_has_sixteen_labelled_rows_and_sixteen_cells`; `a_click_arms_a_voice_on_a_step`
      (the `Step`'s mask and `on`); `a_shift_click_accents_it`; `clicking_a_label_selects_that_voice_for_the_knobs`;
      `a_shift_click_on_a_label_mutes_the_row_and_the_mask`; `the_face_paints_from_the_palette` (the same frame under `Scheme::Black`
      and a light scheme, asserting the grid's fill colours differ); `the_selected_row_is_plated` (accent fill under the label);
      `a_click_lands_on_the_step_under_the_pointer_while_the_line_runs` (the frame refreshes the pattern from `shared.pattern()`
      mid-bar; the write must land where the pointer is, not where the chase moved to — Review Focus 1);
      `the_grid_stays_hittable_at_both_scale_extremes` (0.65× and 1.6×: every cell's rect is inside the band, wider and taller than
      8 px, and no label's rect overlaps the cell beside it — Review Focus 4).
- [ ] Run; expect failures. Implement; run; PASS. Commit: `feat(panel): the drum machine's face — a grid of sixteen voices`.

### Task 5 — the WAV decoder

**Files:** create `rust/crates/acid-app/src/wav.rs`; test in the same file.
**Interfaces (produces):** `pub fn decode(bytes: &[u8], target_rate: f32) -> Result<Arc<[f32]>, WavError>` — mono, at
`target_rate`, from PCM 8/16/24/32-bit integer or 32-bit float, mono or stereo folded down, linearly resampled;
`pub enum WavError { NotRiff, NotWave, Unsupported { what: &'static str }, TooLong }`.
- [ ] Tests: `a_sixteen_bit_mono_file_decodes_to_the_same_samples`, `a_stereo_file_is_folded_to_mono`,
      `a_forty_four_one_file_is_resampled_to_forty_eight` (length scales, peak preserved within a tolerance),
      `a_float_file_decodes`, `eight_and_twenty_four_bit_decode`, `adpcm_is_refused_by_name` (the error names the format),
      `a_file_that_is_not_riff_is_refused`. Build the fixtures as byte arrays in the test, not as files.
- [ ] Run; expect failures. Implement the parser (RIFF/WAVE chunks, `fmt `, `data`; ignore unknown chunks).
- [ ] Run; PASS. Commit: `feat(wav): a hand-rolled decoder for a kit's samples`.

### Task 6 — kit documents and the factory kits

**Files:** modify `rust/crates/acid-app/src/persist.rs` (kit document, save/load/import, name validation), create
`rust/crates/acid-app/kits/{acoustic,808,909}/` (the WAVs) and `kits/CREDITS.md`, `rust/crates/acid-app/src/kits.rs` (the
compiled-in factory kits); tests in `tests/kits.rs` (new).
**Interfaces (consumes Task 5; produces):**
```rust
// in acid-app/src/persist.rs
pub struct KitDocument { pub name: String, pub voices: [KitVoiceDoc; 16] }
pub fn kits_dir() -> PathBuf;                       // <data>/kits
pub fn save_kit(kit: &Kit, dir: &Path) -> io::Result<PathBuf>;
pub fn load_kit(dir: &Path, name: &str, rate: f32) -> Result<Kit, KitError>;   // a missing WAV is a `KitError::Missing{voice,file}`
pub fn import_kit(from: &Path, name: &str, dir: &Path, rate: f32) -> io::Result<PathBuf>;  // lands beside a taken name
pub fn kit_names(dir: &Path) -> Vec<String>;        // factory names then the player's, sorted
// in acid-app/src/kits.rs, beside the factory banks' code
pub fn factory_kits(rate: f32) -> Vec<Kit>;         // compiled in, read-only
```
- [ ] Tests: round trip through `save_kit`/`load_kit` with a one-voice sine written by hand; `import_kit` never overwrites (second
      import of the same name lands as `name (2)`); a kit with a missing WAV loads every other voice and reports the missing one by
      voice and file (Review Focus 2); a name that is a path (`../x`, `/abs`, `a/b`) is refused; `factory_kits` has three kits,
      sixteen voices each, every voice audible, and names that are reserved (a player kit may not take `808`).
- [ ] **The browser and the card's row.** `ui/kits.rs` draws the list as the banks card's sibling — `FACTORY` then `MY KITS`, the
      factory three marked read-only, `IMPORT` beside them — and `ui/device_card.rs` gains the kit's name and a `KIT` key that opens
      it. Live tests: `the_kit_browser_lists_the_factory_kits_first`; `a_factory_kit_is_marked_read_only_and_selecting_it_loads_it`;
      `editing_a_factory_voice_copies_the_kit_under_a_free_name` (no write to the factory folder, and the session names the copy).
- [ ] Source the samples (CC0 only) into `kits/`, record each file's URL and licence in `kits/CREDITS.md`, and set the per-voice
      defaults so each kit sounds like itself (listening work; `EAR_TESTING.md`).
- [ ] Run; PASS. Commit: `feat(kits): a kit is a folder, and three of them ship`.

### Task 7 — the machine

**Files:** modify `rust/crates/acid-core/src/sequencer.rs` (the `Performer` trait), `voice.rs` (impl), `filter.rs` (a multimode
filter); create `rust/crates/acid-core/src/drums.rs` additions (`Drums`, the voice state); tests in
`rust/crates/acid-core/tests/drums.rs` (new).
**Interfaces (consumes Tasks 1–3; produces):**
```rust
pub trait Performer: Sized { fn step(&mut self, step: &Step); fn release(&mut self);
                            fn render(&mut self, out: &mut [f32]); fn arm(&mut self); }
impl Performer for Voice;                       // `prev_slide` moves here from `Sequencer`
impl Performer for Drums;
pub struct Drums;                               // owns a Sequencer, the current Kit, sixteen voice states
impl Drums {
    pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self;
    pub fn set_kit(&mut self, kit: Arc<Kit>);
    pub fn set_mute_mask(&mut self, mask: u16);
    pub fn audition(&mut self, drum: Drum);      // a hit with no step, served at the next render
    pub fn render(&mut self, out: &mut [f32]);
    pub fn bar_line(&self) -> Option<usize>;  pub fn start(&mut self);  pub fn start_at(&mut self, step: usize);
    pub fn start_later(&mut self, step: usize, silence: usize);  pub fn stop(&mut self);  pub fn pause(&mut self);
    pub fn resume(&mut self);  pub fn set_reverse(&mut self, on: bool);  pub fn set_transpose(&mut self, semitones: i32);
}
pub struct MultimodeFilter;  // one state-variable filter, `FilterKind`, cutoff/resonance in knob positions
```
- [ ] `Sequencer` keeps everything it has; `render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32])`.
- [ ] Tests in `tests/drums.rs`: `a_hit_plays_its_sample_once_and_stops` (a 1 kHz buffer renders and then falls silent);
      `the_envelope_shapes_the_tail` (a long release passes the ADSR's release point);
      `a_stop_cuts_a_long_sample_where_the_stop_says` (a two-second sample stops inside the release, not at the buffer's end —
      Review Focus 3); `a_retrigger_restarts_the_sample` (the second hit's first sample is the buffer's first, not the old tail);
      `accent_is_louder_and_brighter` (peak and spectral centroid both rise); `the_filter_has_three_faces` (the same noise reads
      brighter in High than Low, quieter in Band); `tuning_moves_the_pitch` (+12 doubles the zero-crossing rate);
      `an_open_hat_is_choked_by_a_closed_one` (after the closed hit, the open voice's output falls below a threshold within 10 ms);
      `audition_sounds_with_the_transport_stopped`; `a_muted_voice_is_silent_and_keeps_its_step`;
      `every_voice_muted_is_silence_with_the_steps_still_on` (Review Focus 5's first half);
      `the_bar_is_the_303s` (the drum `Sequencer` reports the same position and bar line as a 303 engine started on the same frame).
- [ ] Run; expect failures. Implement. Run `cargo test -p acid-core`; PASS. Commit: `feat(core): the drum machine — sixteen
      voices on the 303's clock`.

### Task 8 — the renderer holds a machine

**Files:** modify `rust/crates/acid-app/src/audio/renderer.rs` (`Machine`), `audio/mixer.rs` if the slot type changes,
`state.rs` if the kit/mask/audition need wiring, `main.rs` (the kit reload on a rate change, the audition key's request);
tests in `tests/mixer.rs`, `tests/no_alloc.rs`.
**Interfaces (consumes Task 7; produces):** `AudioRenderer` holds `Machine::{Acid(Engine), Drums(Drums)}` and keeps its public
surface (`render_mono`, `bar_line`, `start_later`, `seek`, `stop`, `pause`, `resume`, `has_pending_start`).
- [ ] Tests: `a_drum_device_sounds_through_the_mixer` (a bar of the stub kit is audible and its bar line matches the 303's in the
      same rack); `no_alloc` still passes with a drum slot rendering; `all_sixteen_voices_struck_at_once_do_not_exceed_the_ceiling`
      (Review Focus 5's second half: the mixer's limiter holds the sum); `every_voice_muted_is_silence` (through the mixer, with the
      steps still on); `a_kit_swapped_while_a_voice_sounds_keeps_the_renderer_legal` (the `Arc` outlives the block, no panic, no
      allocation).
- [ ] Run; expect failures. Implement, including `apply_drums` and the audition drain. Run; PASS. Commit:
      `feat(audio): a device's machine is a machine, not always the 303`.

### Task 9 — factory banks for the demo variations

**Files:** modify `rust/crates/acid-core/src/factory.rs` (five drum banks), `rust/crates/acid-app/src/ui/banks.rs` (the drum
device's `DEMO BANKS`), `rust/crates/acid-app/src/audio/renderer.rs` if a bank needs its kit; tests in `tests/banks_live.rs`.
**Interfaces (produces):** `factory::drum_banks() -> Vec<(&'static str, PatternBank)>` with the five names the acid factories use
(`Slow Acid`, `Hard Techno Acid`, `Blues`, `Rock`, `Funk`), eight patterns each, every pattern sounding something.
- [ ] Tests: `every_demo_variation_has_a_drum_bank_of_eight_patterns`; `the_demo_names_are_the_acid_names` (the two lists agree,
      so the later whole-rack song can pair them); each bank's patterns have at least one hit per bar.
- [ ] Write the patterns (listening work: a four-on-the-floor for the acid demos, a backbeat for Rock, a shuffle for Funk).
- [ ] Run; PASS. Commit: `feat(factory): a drum bank for every demo variation`.

### Task 10 — the book

**Files:** create `docs/guide/src/22-the-drum-machine.md`; modify `src/SUMMARY.md`, `04-the-face.md`, `20-reference.md`,
`19-a-devices-card.md`, `docs/guide/check.sh` (facts), and `src/img/README.md` + the screenshots.
- [ ] Page 22: what the drum machine is, the grid, the lit plate, the knobs' relabelled meaning, `SHUFFLE`, the kit browser and
      kits, the factory banks. Facts pinned in `check.sh`: `KICK`, `LP | BP | HP`, `MY KITS`, `Drum machine`.
- [ ] Screenshots at 1960 × 1340 (the AGENTS.md recipe, checking the display size first): the drum face, the kit list.
- [ ] `./docs/guide/check.sh`; PASS. Commit: `docs(guide): the drum machine, its card and its kits`.

## Non-goals

As the spec's: no rack-wide save (that phase takes `SAVE_VERSION` 11), no polyphony within a voice, no MIDI pads or per-voice CC,
no seventeenth voice, no archive export of a kit.

## Progress

- [x] **Plan written** from the approved spec, 2026-11-18. Self-review found three gaps, all fixed before saving: the kit browser
      had no task (it is now the second half of task 6, with its tests); two of the Review Focus lines had no test (the mid-bar click
      and the scale extremes, now in task 4); and the long-sample/retrigger and all-muted modes had none (now in tasks 7 and 8).
      Nothing has been built.
- [x] **Task 1 — the voice catalogue.** `acid-core::drums` holds `Drum` (sixteen voices in grid order, `ALL`, `index`, `label`,
      `choke_group`) and `FilterKind` (`LP | BP | HP`); every `match` is written out, so a seventeenth voice fails to compile in
      `index`, in `label`, in `choke_group` and in `ALL`. Two tests: the voices numbered, round-tripping, ASCII and exactly the hat
      pair choking; and the filter's three faces printing in the order the face draws them. 709 tests.
      **One label differs from the spec, deliberately:** `HighTom` prints `HTOM`, not `HIT` — the knob row's audition key is
      `HIT`, and one face cannot carry two. The spec is corrected in the same commit.
      **Mutation run:** giving `Kick` a choke group fails `the hats share a choke group and nothing else does`
      (`left: [Kick, ClosedHat, OpenHat]`); changing `Snare`'s caption to `SNX` fails the label pin (`left: [..., "SNX", ...]`).
      Both reverted.
      **Review: OK with notes.** Its Important finding was the plan's gap, not the code's: the sixteen captions were guarded only
      by ASCII-and-length assertions, so a renamed or duplicated label — including restoring `HIT` — passed every gate. One
      assertion now pins them in grid order, mirroring what the filter test already did, and the mutation above proves it fails.
      Also taken: the `ClosedHat` doc said the choke both ways round and now names the direction (a hit on it cuts an open hat);
      and this task's **Files** line now names the spec, which the commit edits. Nothing else in the round is affected.
