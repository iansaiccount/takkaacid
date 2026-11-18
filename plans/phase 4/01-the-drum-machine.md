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
Arc<Kit>` mirrors `pattern()`.
- [ ] Test: `a_default_kit_has_sixteen_voices_and_no_samples` (empty `Arc<[f32]>`, default envelope/filter values, every voice
      unmuted), and `the_drum_mute_mask_starts_from_the_kit` (the mask is the kit's `mute` flags packed in catalogue order).
- [ ] Implement; run; PASS. Commit: `feat(drums): a kit lives on the device's shared state`.

### Task 4 — the face

**Files:** modify `rust/crates/acid-app/src/device.rs` (`DeviceKind::Drums` + `Device::on`), `state.rs`
(`SharedState::set_kit`), `ui/mod.rs` (route by kind, `showing_kind`), `ui/dispatch.rs` (the keyboard's step editor is the
303's), `ui/strip.rs` (`StripCell.kind`), `main.rs` (the strip fill); create `rust/crates/acid-app/src/ui/drum_panel.rs`; add
to `tests/drum_live.rs` (new) and `tests/rack.rs`. `ui/device_card.rs` needed no row: every row it draws (ROLE, PORTS,
CHANNEL, COLOUR, MUTE, REMOVE) is kind-agnostic and the kit row is task 6's — corrected here, as tasks 1 and 3 corrected
their own lines.
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
- [x] **Task 2 — the pattern's masks.** `Step` carries `hits` and `accents`, two `u16` appended after `velocity` so the file's
      field order only grows, both `#[serde(default)]`. The four methods are the interface the plan asked for, and the bit comes
      from `Drum::index` through one private `drum_bit`, so the mask is not spelled a second time anywhere. `SAVE_VERSION` is 10;
      `upgraded` needs no rewrite — the serde default *is* the upgrade — and its doc now says so. Five tests in `pattern.rs` and
      three in `tests/persist.rs`: the four the task named, plus `an_accent_with_no_hit_under_it_is_not_an_accent`,
      `the_save_version_is_pinned_at_ten` and `a_drum_step_survives_the_save_file`. **717 tests, all green**, clippy and fmt
      clean, `./docs/guide/check.sh` ok.
      **Mutations run:** dropping `self.on = true` from `hit` fails three of the pattern tests; dropping `on = false` from
      `clear_hit` fails `clearing_the_last_hit_rests_the_step`; `is_accented` without the `hits` mask fails the disagreement test;
      `skip_deserializing` on `accents` fails the new round trip *while `a_version_nine_file_upgrades_with_empty_drum_masks` still
      passes*, which is the hole below. All reverted.
      **Review: OK with notes, every finding actioned.** Important: no test pinned a *non-zero* mask through the save file, and
      every other persist test holds zeros, so a mask written and never read would have passed the whole suite —
      `a_drum_step_survives_the_save_file` is that test, and the mutation above proves it bites. Two Minor findings were behaviour,
      not prose: `is_accented` now reads `hits & accents`, so a hand-edited accent bit with no hit under it answers as nothing
      instead of as an accent the doc promised it was not; and `clear_hit` rests the step only when it actually removed a hit, so
      a stray clear cannot empty a note the 303 put on a step. Two doc comments claimed a version refusal this build does not
      implement — both now say what the number is (a promise to a later build), and the missing refusal is a wishlist entry with
      what would have to move, because it is a behaviour change to the one path every file goes through. Of the review's
      report-only notes, the test helper's mask-name list carries the "a seventeenth voice is a third mask" warning;
      `ui/randomise.rs` writing `on` under `..*step` stands — RANDOM is not on a drum face and the masks survive the reroll.
      **Two rulings.** (1) `Step::hit(voice, false)` *writes* the accent bit rather than leaving it, so the two masks cannot
      disagree through the API — if wrong, a shift-click that accents and then a plain click would still sound accented.
      (2) `clear_hit` rests the step only when it removed a hit, where the plan said "and `on` when no hit remains"; the two differ
      only on a maskless 303 step, which a drum gesture must not silently empty — if wrong, a drum device can leave a step `on` with
      no voice on it, which the face draws as a rest anyway.
      **Invisible to the guide, deliberately:** nothing a player can reach changes until task 4 draws the grid, and what an old file
      reads back as is unchanged, so no page in `docs/guide` is now wrong. `clipboard.rs` is touched only because its "one of
      everything" `Step` literal stopped compiling — the fixture is meant to force that decision — and the masks are carried by the
      clipboard already, since a `Clip::Pattern` is the whole `Pattern`. The wishlist's rack entry said the rack round takes
      `SAVE_VERSION` 10; it takes 11 now that this one took 10.
- [x] **Task 3 — the kit, in memory.** `acid-core::drums` holds `Kit` (`name`, `voices: [KitVoice; Drum::ALL.len()]`),
      `KitVoice` in the spec's field order, `KitVoice::default()`, `default_kit()` and `DEFAULT_KIT_NAME`. `Kit::mute_mask()`
      and `Kit::set_mute_mask()` are the one pack and unpack of the sixteen mute bits, every bit taken from `Drum::ALL` and
      `Drum::index()` rather than from a positional list. `SharedState` gained `kit: ArcSwap<Kit>`, `drum_mute: AtomicU32` and
      `audition: AtomicI32`, built in `with_tempo` from the one `default_kit()` that goes into the swap, plus `kit()`,
      `request_audition` and `take_audition`. **721 tests**, clippy and fmt clean, `./docs/guide/check.sh` ok. Tests:
      `a_default_kit_has_sixteen_voices_and_no_samples` and `a_kit_packs_its_mutes_in_catalogue_order` in `drums.rs`;
      `the_drum_mute_mask_starts_from_the_kit` and `nothing_is_auditioned_until_a_voice_asks` in `state.rs`'s `mod tests`.
      **Mutations run, all caught, all reverted:** the mute pack in reverse catalogue order; `set_mute_mask` inverted; the
      default voice's `cutoff` given the resonance constant; the default voice muted; the audition slot seeded `0` instead of
      `-1`; the audition stored one past the voice's slot; the starting mask seeded `u32::MAX`. The first mutation run
      reported **0 of 7 caught** and that run is part of the record: `--exact` with a bare test name matches no test, so every
      run "passed" with zero run. The script now refuses a filter that matches nothing.
      **Review: OK with notes, both actioned.** Minor 1: the state test's comment claimed the mask is the kit's own mutes
      "rather than one kept beside it", which the test cannot show — the default kit mutes nothing, so a pack and a bare `0`
      are both `0 == 0`; the comment now says what the assertions do show and points at `drums.rs` for the pack. Minor 2:
      this task's Interfaces line still said `Guard<Arc<Kit>>` while the code returns an owned `Arc`, so the plan contradicted
      its own commit; the line is corrected here, as task 1 corrected the spec.
      **Rulings.** (1) `kit()` returns `Arc<Kit>` through `load_full()`, not the plan's `Guard<Arc<Kit>>`: it is the shape
      `pattern()` returns next door, both are lock- and allocation-free, and task 8 wants an owned `Arc` for `Drums::set_kit`,
      which a guard would be cloned into anyway. If wrong, task 8 pays one refcount bump per block it could have skipped and
      one call site changes. (2) `Kit::set_mute_mask` is written here although task 7 owns `Drums::set_mute_mask`: the unpack
      belongs beside the pack, or it is the second hand-kept list this repo has paid for twice. (3) `request_audition` and
      `take_audition` are added though the Interfaces line named only the field: they keep the `-1` sentinel and the
      `Drum::index()` cast in one place, as `take_pattern_request` does — **task 8 must consume them**, or the renderer spells
      the sentinel again. (4) The default voice is attack 2 ms, decay 300 ms, sustain 0.0, release 20 ms, cutoff 1.0 (wide
      open), resonance 0.0 (flat), tune 0.0, low-pass, and the kit is named `Init` — **a word the card prints and task 10's
      page will name**, chosen here because neither spec nor plan settles it. If wrong it costs one constant, one pinned
      assertion and one guide sentence. (5) `voices: [KitVoice; Drum::ALL.len()]` instead of the spec's literal `16`, so a
      seventeenth voice cannot leave a voice table one short.
      **One hole recorded rather than hidden:** the state's `drum_mute` could be seeded from a literal `0` instead of from the
      kit and every test would still pass, because the only kit a constructor can see mutes nothing. The pack is proven in
      `drums.rs`; proving the seed needs a kit-taking constructor, which is task 6/8's seam.
      **A debt noted, not paid:** `1u16 << Drum::index()` now appears in `pattern::drum_bit` and twice here. Every copy comes
      from the catalogue, so no reorder can desynchronise them, and task 2's ruling deliberately left `Drum::bit()` out. If
      task 7's `Drums::set_mute_mask` needs a fourth, `Drum::bit()` earns its place — and `pattern::drum_bit` must move onto
      it in the same commit, or there are two.
      **Invisible to the guide, deliberately:** nothing a player can reach changes. A device still starts silent until task 6
      supplies samples, the face is task 4's, and `persist::SavedState` is untouched, so no page in `docs/guide` is now wrong.
- [x] **Task 4 — the face.** `ui/drum_panel.rs` is the drum machine's panel: the seven 303 slots repurposed left to right as
      `ATTACK DECAY SUSTAIN RELEASE CUT OFF RESONANCE TUNE` (one written-out `Dial::for_knob`, no `Dial::ALL` beside
      `Knob::ALL`), the `LP | BP | HP` key, the device's `SHUFFLE` dial, `HIT`, and the sixteen-row grid with a label gutter,
      beat frames, a lit plate and row band for the selected voice, and a dim muted row. `draw_panel` routes on
      `PanelState::showing_kind()` (from the strip, `StripCell.kind` filled by `main`, `.get(selected)` never an index) and a
      drum device draws `drum_panel` in place of the knob, performance and step rows. `DeviceKind::Drums` is the second
      catalogue entry (`DRUMS`, same cap); `SharedState::set_kit` is the clone-on-write publish; the keyboard's step editor,
      the computer-key/audition path and MIDI note placement are all gated off for a drum device. **745 tests**, clippy and
      fmt clean, `./docs/guide/check.sh` ok. Tests: 18 in `tests/drum_live.rs` — the nine the task named plus
      `every_fourth_column_is_framed`, `turning_attack_edits_the_selected_voice`, `the_filter_key_sets_the_voices_filter`,
      `the_hit_key_auditions_the_selected_voice`, `the_shuffle_dial_writes_the_devices_swing`,
      `a_drum_device_does_not_draw_the_303_rows`, `a_drum_device_ignores_the_keyboard_and_the_note_queue`,
      `every_dial_reads_and_writes_its_own_field`, `the_seven_dials_are_labelled_in_the_order_the_face_draws_them`; four unit
      tests in `drum_panel.rs`; and `a_new_drum_device_is_named_and_opens_on_an_empty_grid` in `tests/rack.rs`.
      **Mutations run, 26 of 26 caught, all reverted** (`.superpowers/mutate_task4.py`): the cell writing to the chase's column
      instead of the pointer's; a click arming `Kick` whatever the row; `shift` dropped from a hit; the label not selecting; the
      mute bit hard-coded to `Kick`; an accented strike painted as `led_on`; the plate as `panel_raised`; `ROW_INSET` widened
      until a 0.65x cell is under 8 px; a drum dial also writing the 303's `tuning` (caught by reading *all* seven atomics, not
      one); the filter key leaving the voice on low; `HIT` auditioning `Kick`; beat frames on every fifth column; the drum dial
      reading the 303's `CUT OFF` instead of the voice; the step row drawn on a drum face; the tuning dial reversed; the dial
      order permuted; the grid stopping at the knob band; rows pitched for fifteen voices; the key catalogue reordered; one dial
      writing another's field; the selected row unbanded; a muted row keeping its accent; `SHUFFLE` not writing `swing`; the
      computer keyboard, the note queue and the keyboard step editor each re-enabled for drums.
      **One claim the mutations cannot prove, recorded rather than implied:** the mid-bar test's second assertion — that the
      write starts from `shared.pattern()` rather than the frame's mirror — does not bite alone, because `draw_panel` refreshes
      the mirror at the top of every frame, so the mirror cannot be stale by the time a cell is handled. The test's real bite
      is the column (the mutation above), and the fresh read stays as belt-and-braces.
      **Review: OK with fixes, every finding actioned.** Its two Important findings were both real and are fixed in this
      commit: (1) the book said the picker had one kind — `05-the-three-cards`, `18-a-rack-of-devices` and `20-reference` now
      name `ACID` and `DRUMS`, `img/picker.png` is retaken at 1960x1340 with both rows and its `img/README.md` row updated, and
      `DRUMS` is pinned in `check.sh` so the next kind cannot repeat it silently (task 10's Files list does not name `05` or
      `18`, which is why this could not wait); (2) the three input gates this task added had no test that would fail if they
      were removed — `a_drum_device_ignores_the_keyboard_and_the_note_queue` is that test, with three mutations behind it. Of
      its Minor findings: `cell_rect` no longer allocates a `Vec` per cell (`row_cells` builds a row's `[Rect; 16]` once, and the
      tests' `cell_rect` indexes it, which also settles the clamp-vs-panic split); the ATTACK test now watches all seven 303
      atomics instead of `cutoff`; the row band, a muted row's lost accent and the `SHUFFLE` write each have a test; the grid
      test counts each label instead of accepting it once anywhere (the selected voice's name is also the caption, so a dropped
      gutter label would have hidden); and `Dial::ALL` is deleted. The review's finding that the plan's Files line named
      `ui/device_card.rs` and omitted `state.rs`, `ui/dispatch.rs` and `tests/rack.rs` is corrected in the task text above.
      **Rulings.** (1) A drum knob's value lives in the kit, clone-on-write through `SharedState::set_kit`, never in a second
      copy in the 303's atomics — one truth, and a `Kit` clone (~1.5 KB) only on the frames a hand is turning a dial, on the UI
      thread. If wrong, the audio thread still reads one coherent kit, but a future per-voice parameter that the engine reads
      from an atomic would have to move; the alternative (borrowing the 303's atomics as scratch) is the two-sources-of-truth
      bug `AGENTS.md` exists to prevent. (2) The selected voice is **panel-wide** (`PanelState::drum.selected`), as the plan's
      Interface says, not per device: two drum devices share the last voice, one click to correct, and per-device would be a
      `PanelEdit` field plus a test. (3) A new drum device opens on the 303's starter pattern with both masks zero — an empty
      grid — and, until task 8 gives the renderer a `Machine`, it still *sounds* that 303 line under the grid. If wrong, the
      fix is task 9's factory drum bank for the opening pattern and task 8's machine for the sound, not a face change.
      (4) The dial slots are the 303's seven, repurposed left to right in the spec mockup's order; if the player meant a
      different order it is seven arms of one `match` plus the pinned test. (5) The drum dials and the `LP|BP|HP`/`HIT` keys
      carry no hover tips yet: `ui::tips` is the 303's table, and adding drum variants belongs with task 10's page. If wrong, a
      player hovers a drum knob and learns nothing — no wrong text, which the 303's tips would have been. (6) `STEP` stays drawn
      on the switches row (the spec keeps every key) but is inert on a drum face, because the keyboard step editor is gated;
      if wrong it is a lit key that does nothing until the grid grows a cursor, and the fix is one `if`. (7)
      `ui/device_card.rs` is untouched: none of its rows is 303-specific and the kit row is task 6's.
      **The guide moved with the face**, unlike tasks 1–3: the picker's prose, alt text and screenshot are this commit's,
      because the picker's claim was false the moment `DRUMS` joined `DeviceKind::ALL`. The drum face itself has no page yet;
      task 10 writes page 22.
