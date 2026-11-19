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

**Files:** create `rust/crates/acid-app/src/wav.rs`; register it in `lib.rs`; test in the same file.
**Interfaces (produces):** `pub fn decode(bytes: &[u8], target_rate: f32) -> Result<Arc<[f32]>, WavError>` — mono, at
`target_rate`, from PCM 8/16/24/32-bit integer or 32-bit float, mono or stereo folded down, linearly resampled;
`pub enum WavError { NotRiff, NotWave, Unsupported { what: &'static str }, Truncated, TooLong }` — five variants, corrected
from the four first written here: a cut file must be an error, and `Truncated` is the only honest way to say so (ruling 1 in
Progress). `Unsupported` also names three refusals this list did not: a `fmt ` chunk shorter than 16 bytes, a WAVE missing its
`fmt ` or `data` chunk, and a frame that is not packed.
- [ ] Tests: `a_sixteen_bit_mono_file_decodes_to_the_same_samples`, `a_stereo_file_is_folded_to_mono`,
      `a_forty_four_one_file_is_resampled_to_forty_eight` (length scales, peak preserved within a tolerance),
      `a_float_file_decodes`, `eight_and_twenty_four_bit_decode`, `adpcm_is_refused_by_name` (the error names the format),
      `a_file_that_is_not_riff_is_refused`. Build the fixtures as byte arrays in the test, not as files.
- [ ] Run; expect failures. Implement the parser (RIFF/WAVE chunks, `fmt `, `data`; ignore unknown chunks).
- [ ] Run; PASS. Commit: `feat(wav): a hand-rolled decoder for a kit's samples`.

### Task 6 — kit documents and the factory kits

**Files:** modify `rust/crates/acid-app/src/persist.rs` (kit document, save/load/import, name validation), create
`rust/crates/acid-app/kits/{acoustic,808,909}/` (the WAVs), `kits/CREDITS.md` and `kits/refresh.py` (the script that fetched
them and generated the credits), `rust/crates/acid-app/src/kits.rs` (the compiled-in factory kits) and `src/ui/kits.rs` (the
browser and the copy seam); tests in `tests/kits.rs` and `tests/kits_live.rs` (both new). The task also touches, because a kit
is now part of the document and of the card: `src/wav.rs` (`encode`), `src/state.rs` (`adopt_kit`), `src/device.rs`
(`DeviceKind::has_kit`), `src/ui/device_card.rs` (the `KIT` row), `src/ui/mod.rs` (the card, `put_cards_away`),
`src/ui/dispatch.rs` (`Escape`), `src/ui/drum_panel.rs` (the edit seam), `src/ui/banks.rs` (the shared card metrics),
`src/main.rs` (the rate seam, the load/import/write paths), `src/lib.rs`, `tests/persist.rs` and `tests/device_card_live.rs`,
and the book — `19-a-devices-card.md`, `05-the-three-cards.md`, `20-reference.md`, `04-the-face.md`, `SUMMARY.md` and
`check.sh` — with the `SAVE_VERSION` queue corrected in the plan, the wishlist and the spec. **Corrected after the fact**, as
tasks 1, 3, 4 and 5 corrected theirs: the plan named only `persist.rs`, `kits/`, `kits.rs` and `tests/kits.rs`.
**Interfaces (consumes Task 5; produces):**
```rust
// in acid-app/src/persist.rs
pub struct KitDocument { pub name: String, pub voices: [KitVoiceDoc; 16] }
pub fn kits_dir() -> PathBuf;                       // <data>/kits
pub fn save_kit(kit: &Kit, dir: &Path, rate: f32) -> Result<PathBuf, KitError>;   // a WAV records its own rate
pub fn load_kit(dir: &Path, name: &str, rate: f32) -> Result<LoadedKit, KitError>; // LoadedKit { kit, problems }: a missing
                                                                                  // WAV costs one voice, not the kit (Review Focus 2)
pub fn import_kit(from: &Path, name: &str, dir: &Path, rate: f32) -> Result<ImportedKit, KitError>;  // lands beside a
                                                                                  // taken name; a broken voice lands silent
                                                                                  // with its file named (ImportedKit.problems)
pub fn kit_names(dir: &Path) -> Vec<String>;        // factory names then the player's, sorted
pub fn free_kit_name(dir: &Path, wanted: &str) -> String;   // factory names are reserved, and the rule is free_name's
pub fn free_name(wanted: &str, taken: impl Fn(&str) -> bool) -> String;  // the one copy rule: songs, kits, and a panel's copy
pub fn kit_name(typed: &str) -> Option<String>;     // song_name's rule, named for what it is here
// in acid-app/src/kits.rs, beside the factory banks' code
pub fn factory_kits(rate: f32) -> Vec<Kit>;         // compiled in, read-only, cached per rate
pub fn factory_names() -> Vec<&'static str>;  pub fn is_factory(name: &str) -> bool;  pub fn default_name() -> &'static str;
pub fn factory_sample_bytes() -> usize;             // the spec's ~2 MB, as a number a test holds a build to
// in acid-app/src/wav.rs
pub fn encode(samples: &[f32], rate: f32) -> Vec<u8>;   // 16-bit mono, the other half of `decode`
// in acid-app/src/state.rs
pub fn adopt_kit(&self, kit: Kit);                  // the load path; `set_kit` is the edit path and seeds no mutes
```
**Corrected after the fact** (three deviations, each ruled on in the Progress): `save_kit` takes the rate a WAV must record and
answers `KitError`; `load_kit` answers `LoadedKit` with a `problems` list so a broken voice does not cost the kit; and
`import_kit` answers `ImportedKit { folder, name, problems }` — a fourth correction, made after the follow-up — because an
import with a broken voice **lands** and names the file to replace rather than being refused whole.
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
`rust/crates/acid-core/tests/drums.rs` (new). **Corrected after the fact**, as tasks 1 and 3–6 corrected theirs: the commit also
touches `pattern.rs` (trap 12's `drum_bit` → `Drum::bit()` move, which had to land in the same commit), `envelope.rs` (its time
setters became idempotent — the first review's Important), `tests/envelope.rs`, `tests/sequencer_feel.rs` (a slide fixture that
was vacuous) and the six `tests/*.rs` files that render a `Voice` and so need `use acid_core::sequencer::Performer;`, plus the
spec's §"The clock, and the performer".
**Interfaces (consumes Tasks 1–3; produces):**
```rust
pub trait Performer: Sized { fn step(&mut self, step: &Step); fn release(&mut self);
                            fn render(&mut self, out: &mut [f32]); fn arm(&mut self); }
impl Performer for Voice;                       // `prev_slide` moves here from `Sequencer`
impl Performer for DrumMachine;                 // NOT `Drums` — see the Progress, ruling 1: the performer is a field
pub struct Drums;                               // a Sequencer and a private DrumMachine: the kit, sixteen voice states, the mask
impl Drums {
    pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self;
    pub fn set_kit(&mut self, kit: Arc<Kit>);
    pub fn set_mute_mask(&mut self, mask: u16);
    pub fn audition(&mut self, drum: Drum);      // a hit with no step, served at the next render
    pub fn render(&mut self, out: &mut [f32]);
    pub fn bar_line(&self) -> Option<usize>;  pub fn start(&mut self);  pub fn start_at(&mut self, step: usize);
    pub fn start_later(&mut self, step: usize, silence: usize);  pub fn stop(&mut self);  pub fn pause(&mut self);
    pub fn resume(&mut self);  pub fn set_reverse(&mut self, on: bool);  pub fn set_transpose(&mut self, semitones: i32);
    pub fn set_pattern(&mut self, pattern: Arc<Pattern>);  pub fn set_bpm(&mut self, bpm: f32);
    pub fn set_swing(&mut self, swing: f32);  pub fn position(&self) -> usize;   // added: the clock it owns, and its bar
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

**Files:** modify `rust/crates/acid-app/src/demo.rs` (the five drum banks, as `Demo::drums` and `demo::drum_banks`),
`rust/crates/acid-app/src/device.rs` (`Device::load_bank`, `Device::publish_pattern`), `rust/crates/acid-app/src/main.rs`
(the `DEMO BANKS` row routed by kind), `rust/crates/acid-app/src/ui/banks.rs` (the request's doc), and the book —
`docs/guide/src/12-songs-and-demos.md` and `check.sh`; tests in `rust/crates/acid-app/tests/drum_banks.rs` (new).
**Corrected after the fact**, as every task before it corrected its own lines: the plan named `acid-core/src/factory.rs`,
`ui/banks.rs` for a listing change and `tests/banks_live.rs`; the banks live in `acid-app` beside the songs (ruling 1),
the card's rows are the same five names on either face so it needs no listing change at all (ruling 5), and a test file
that never drives `draw_panel` may not carry the `_live` suffix.

**Interfaces (produces):** `demo::drum_banks() -> Vec<(&'static str, PatternBank)>` and
`demo::load_drum_bank(&mut Rack, index) -> Option<&'static str>`, with the five names the demo songs use
(`Slow Acid`, `Hard Techno Acid`, `Blues`, `Rock`, `Funk` — in `acid-app::demo::songs()`, not in `factory`), eight patterns
each, every pattern sounding something, and a demo's tempo and shuffle landing with its grooves.
- [ ] Tests: `every_demo_variation_has_a_drum_bank_of_eight_patterns`; `the_demo_names_are_the_acid_names` (the two lists agree,
      so the later whole-rack song can pair them); each bank's patterns have at least one hit per bar.
- [ ] Write the patterns (listening work: a four-on-the-floor for the acid demos, a backbeat for Rock, a shuffle for Funk).
- [ ] Run; PASS. Commit: `feat(factory): a drum bank for every demo variation`.

### Task 10 — the book

**Files:** create `docs/guide/src/22-the-drum-machine.md`; modify `src/SUMMARY.md`, `04-the-face.md`, `20-reference.md`,
`19-a-devices-card.md`, `14-midi.md` and `15-keyboard.md` (both say a device's one voice is played by the keyboard, which is
no longer true of a drum device — task 8 added the one sentence each; re-read them with the drum machine in mind),
`01-what-it-is.md` (the intro's framing: task 8 made a drum machine audible, so "what it is not" needs the page that says what
it now is), `docs/guide/check.sh` (facts), `12-songs-and-demos.md` (task 9 added the drum-bank paragraph, which wants the
cross-reference to page 22 from where a player arrives at `DEMO BANKS`), and `src/img/README.md` + the screenshots; plus,
**corrected after the fact** as every task before it corrected its own lines: `05-the-three-cards.md` and
`18-a-rack-of-devices.md` (the two other pages a reader arrives from), `rust/crates/acid-app/src/ui/kits.rs` and
`ui/banks.rs` (two defects the pictures exposed — see the Progress), and `rust/crates/acid-app/src/ui/mod.rs` (the call
site a new argument reaches).
- [ ] Page 22: what the drum machine is, the grid, the lit plate, the knobs' relabelled meaning, `SHUFFLE`, the kit browser and
      kits, the factory banks. Facts pinned in `check.sh`: `KICK`, `LP | BP | HP`, `MY KITS`, `drum machine` (the plan said
      `Drum machine`; the guide's own title is "The drum machine", and `grep -F` is case-sensitive, so the pin is the
      sentence the pages actually say).
- [ ] Screenshots at 1960 × 1340 (the AGENTS.md recipe, checking the display size first): the drum face, the kit list.
- [ ] `./docs/guide/check.sh`; PASS. Commit: `docs(guide): the drum machine, its card and its kits`.

## Non-goals

As the spec's: no rack-wide save (that phase takes `SAVE_VERSION` **12** — task 6 took 11), no polyphony within a voice, no MIDI pads or per-voice CC,
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
- [x] **Task 5 — the WAV decoder.** `acid-app::wav` is a hand-rolled RIFF/WAVE reader: `decode(bytes, target_rate) ->
      Result<Arc<[f32]>, WavError>` walks `RIFF` → `WAVE` then the chunks in order, steps over what it does not know (the
      `LIST`/`INFO` block most encoders write before the samples), and returns one channel at the engine's rate — 8/16/24/32-bit
      integer and 32-bit float, mono or stereo averaged, resampled linearly. Every slice goes through one checked `take` and
      every sample through one `exact::<N>` → `from_le_bytes`, so a hostile or half-copied file is a refusal and never a panic;
      the 24-bit path sign-extends through `<< 8 >> 8`. `std` only, no new dependency, nothing on the audio path,
      `SAVE_VERSION` still 10. 26 tests in `wav.rs`'s own `mod tests`, every fixture built as a byte array. **771 tests**,
      clippy and fmt clean, `./docs/guide/check.sh` ok.
      **Mutations run, 30 of 30 caught, all reverted** (`.superpowers/mutate_task5.py`): the 16-bit scale divisor; stereo summed
      instead of averaged; a nearest-neighbour resampler; the float arm read as an integer; the 8-bit 128 offset dropped; the
      24-bit sign extension dropped; 24-bit called four bytes wide; 32-bit scaled by 2^15; ADPCM refused without its name;
      `WAVE_FORMAT_EXTENSIBLE` read as PCM; the RIFF magic unchecked; the form type unchecked; the odd-chunk pad byte not
      stepped over; a short chunk trimmed instead of refused; the cap doubled; three channels read as mono; the frame-alignment
      check disabled; both missing-chunk refusals renamed; `fmt ` shorter than 16 bytes read as far as it goes; an empty `data`
      chunk treated as missing; the file's own rate unchecked; the target rate unchecked; `Truncated`'s words changed; an unread
      width and an unknown tag each taken as 16-bit; `fmt ` demanded to be exactly 16 bytes; the RIFF header's size trusted;
      the rate ratio inverted; and the `is_finite` half of the rate guard dropped. Two runs are part of the record rather than
      the count: swapping the form type from `WAVE` to `RIFF` **did not bite**, because `AVI ` differs from both words — an
      equivalent expression is not a mutation, and it became `if false`; and three entries stopped matching after the review's
      edits, which the script reports as `SKIP`, never as a pass.
      **Review: OK with notes, every finding actioned.** Its two Important findings were both real. (1) Trap 3's `cbSize`
      requirement was claimed in a doc comment and guarded by `body.len() < 16` with no test able to fail on it, so the normal
      shape of a WAV that has been through a DAW was untested: `a_fmt_chunk_with_an_extension_decodes` is that test, its helper
      refactored out of the existing `fmt ` builder, and the `!= 16` mutation proves it bites. (2) The plan's Interfaces line
      named four variants while the code ships five — corrected above. Of its Minor findings: the frame size was computed twice
      and is now one `Format::frame()` call; the `Sample` doc promised `-1.0..=1.0` for the float path, which passes the file's
      numbers through as recorded (the prose now says so — clamping is task 7's renderer's decision); a decimating resample had
      no test (`a_ninety_six_file_is_resampled_down_to_forty_eight`, where an inverted rate ratio shows up in the peak and not
      only in the length); the `is_finite` half of the target-rate guard could not fail (a `f32::NAN` case now sits beside the
      zero and negative ones); "the RIFF header's size is ignored" was claimed and untested (a fixture now lies about it); two
      player-facing refusals carried backticks the status line would draw literally (`no format chunk`, `no sample data`); and
      "allocates, once" undercounted the load-time allocations, which the module doc now says plainly. Its report-only note that
      `exact`'s `map_err` arm is unreachable by construction is accepted and recorded here, so the 30/30 is not read as covering
      it.
      **Rulings.** (1) `WavError` gains a fifth variant, `Truncated`: trap 4 requires a cut file to be an error, and the four
      variants could only say it by lying in a `what` string — the one line a player reads for why their kit did not load. Task 6
      consumes this enum, so the variant is cheaper now than after its notice table exists. If wrong, one arm in task 6's notice
      and one test. (2) A `data` chunk that is present and empty decodes to silence; one that is absent is refused. If wrong, a
      file cut exactly before its `data` header loads as a silent voice instead of saying so. (3) The `fmt ` block alignment is
      checked ("a frame that is not packed"), which the plan did not ask for: a frame wider than its samples puts every frame
      after the first out of step, which plays as noise rather than as a file that failed to load. Zero is accepted as "not
      filled in". If wrong, one `Unsupported` arm and one test. (4) A float file's samples are passed through as recorded — not
      clamped, not checked for NaN or infinity — because what a hot or non-finite sample should sound like is the renderer's
      question, and the spec puts the ceiling in task 7's limiter. If wrong, a float WAV can poison a voice's filter state and
      the fix is one clamp plus a test. (5) `TooLong`'s cap is 1 << 20 samples (21.8 s at 48 kHz) against the spec's ≤0.6 s
      voice, checked against the *output* length before either `Vec` exists. It bounds the output, not the mono fold, which
      reserves at most the input file's own duration in samples — an 8-bit file can reserve four times its own byte count
      before returning ≤1 M samples, which is bounded by the file the player handed us and not by a header. If wrong, one
      constant. (6) A `target_rate` that is not finite and positive is refused through `Unsupported` rather than a panic, so a
      caller's bug lands in the same vocabulary as a file's. If wrong, a nonsense rate reads as a file problem, and separating
      it costs a sixth variant. (7) Trap 3's open question is answered by refusing `WAVE_FORMAT_EXTENSIBLE` by name: it usually
      wraps plain PCM in a sub-format field this decoder does not read, and guessing would be the "notice names the wrong
      format" failure the handover warns about. If wrong, a DAW export that writes extensible for ordinary 24-bit material is
      refused by name; reading the sub-format is a wishlist entry, not this round's.
      **Invisible to the guide, deliberately:** a decoder behind a kit nothing loads yet changes nothing a player can reach, so
      no page in `docs/guide` becomes wrong. Task 6 is the one that moves the book — the kit row, the browser, the factory kits.
- [x] **Task 6 — kit documents and the factory kits.** A kit is a folder: `acid-app::persist` holds `KitDocument { name,
      voices: [KitVoiceDoc; 16] }` with `KitVoiceDoc` in `KitVoice`'s field order and `file`/`source`/`credit` appended (trap 14),
      plus `kits_dir`, `kit_name`, `kit_folder`, `free_kit_name`, `save_kit`, `load_kit`, `import_kit` and `kit_names`. The folder
      is `<data>/kits/<name>/kit.json` and one `<VOICE>.wav` per sampled voice — the name from `Drum::label`, never a second
      list — written through `write_through`, which is now the one copy of the `.tmp`-and-rename rule the save file already used.
      `wav::encode` is the half the decoder was missing (16-bit mono at the rate it is handed). `acid-app::kits` compiles in the
      three kits that ship — `808`, `909`, `Acoustic` — from 48 CC0 WAVs: 2.02 MiB, every voice ≤ 0.6 s and 16-bit mono, one URL
      and licence per file in `kits/CREDITS.md`, and `kits/refresh.py` the script that fetched them from the same table that
      generated that page. The decode is cached per rate behind a `Mutex` (`factory_sample_bytes` also answers the spec's ~2 MB,
      and a test holds a build to it) so no frame decodes two megabytes. `ui/kits.rs` is the browser — `FACTORY` then `MY KITS`,
      the factory rows marked read-only, `IMPORT` and `CLOSE` — and **the one seam a kit edit goes through**; `ui/device_card.rs`
      gains a `KIT` row on a drum device's card only. `SAVE_VERSION` is **11** with `SavedState::kit` appended after every
      existing field, and the rack round's number moved to **12** in the plan, the wishlist and the spec.
      **810 tests**, clippy and fmt clean, `./docs/guide/check.sh` ok. New: 19 in `tests/kits.rs`, 11 in `tests/kits_live.rs`,
      4 encode tests in `wav.rs`, 3 in `tests/persist.rs` (the version-10 upgrade, a session naming its kit, and a session on the
      default kit naming none — the version pin itself is renamed, not added), and one unit test in `device_card.rs` for the kit
      row at 0.65× and 1.6×. `a_kit_written_by_hand_loads_its_own_wav` is the player's own door: a folder with a document and a
      WAV nothing in this build wrote.
      **Mutations run, 34 of 34 caught, all reverted** (`.superpowers/mutate_task6.py`; 42 of 42 with the follow-up, one retired
      when the import reversed and nine added): a voice's file named by its slot; a
      decoded voice thrown away; a clipped filter index off by one; a missing WAV not reported; a document allowed to name a
      path; a save allowed a factory name; a free kit name ignoring the factory three; a save forgetting provenance; an import
      overwriting; any file counted as a kit; the save file not naming the kit; `SAVE_VERSION`
      back at 10; `upgraded` inventing a kit; the factory kits decoded at a hard-coded rate; every name a factory name; a factory
      edit in place; a copy named by a fixed suffix; a kit edit leaving the session saved; a factory row not marked read-only;
      the two lists not sorted apart; the card's `KIT` key inert; a kit row on every card; a sample stored through a wrapping
      cast; an encoded file claiming a rate it was not given; a copy named against an empty list; a loaded kit not seeding the mute
      mask; a card opening with the kit browser left up; the default kit's own word taken as a usable name; an edit recording the
      first device rather than the edited one; a kit list running under the footer keys; F2 opening settings under the browser; and
      a key reaching the step editor behind the browser. Two runs are part of the record: removing the clamp in `encode` is
      **equivalent** — `as i16` saturates, so the test cannot bite it, which is task 5's lesson again — so the mutation is the
      cast through `u16` that would wrap; and the script's first run reported a filter that matched nothing, because an
      integration test's path is its bare name and `kits::name` runs zero tests and exits 0. The script now strips the module
      path for a test binary and still refuses a filter that matches nothing.
      **Review: BLOCK on one Critical, every finding actioned.** The Critical was real and is why the review is not a formality:
      `Kits`' `refresh` started `false`, so on the frames the panel is usable the copy rule named a factory kit's copy against
      an **empty** list — the first dial turn of `808` could take a folder's name, and the next save would overwrite the
      player's kit with no notice at all. `Kits::default` now asks for a read ("a list nobody has read is not up to date"),
      `a_fresh_panel_asks_for_its_kit_list` pins that and `a_copy_lands_beside_a_name_the_library_already_has` proves the copy
      steps to `808 (3)` past a taken name. Four of its Important findings were behaviour, not prose: a kit's `mute` flags never
      seeded the mute mask (`SharedState::adopt_kit` is the load path and `set_kit` stays the edit path, with
      `a_loaded_kit_seeds_the_mute_mask_and_an_edit_does_not` saying why they must be two); `write_kit` wrote device 1's kit
      while the browser loaded and the panel edited the **selected** device's, so an edit on a second machine was reported saved
      and then dropped (it now writes the edited one, and the kit is in the library either way); the two-click load guard had no
      test (`loading_a_kit_over_an_unsaved_edit_takes_two_clicks`) and no guide sentence (page 19 has one); and the book promised
      that a *mute* edit copies a factory kit, which it does not — the sentence says a dial and the filter key now. The browser
      was also the one card the one-card rule did not know: `PanelState::put_cards_away` is the single spelling every open path
      calls, page 05 counts five cards and names the browser in its `Escape` order, and page 20 lists `KIT` and the browser's
      words. Of its minor findings: the unfailable `attack_ms > 0.0` compares against the value before the drag; the tautological
      `assert!(… || true)` and its `STEPS_PER_PATTERN` companion are gone; the hand-written kit's comment no longer claims to be
      the smallest document (a missing field is refused *whole*, which is the point); the field-order pin is strictly ascending
      and the doc says a reorder is a diff's problem rather than a migration's; `DeviceKind::has_kit` is written out so a third
      kind has to answer; the browser imports the banks card's `CARD_*` and `heading` instead of keeping a second set of numbers;
      the unused `default_factory_kit` is deleted; a duplicated comment in `wav.rs` is gone; and the spec's source list now says
      the TicTokMen set was rejected. `save_kit` still never deletes a WAV for a voice that lost its sample — no UI can clear a
      sample yet, so it is recorded here as a deferred minor rather than paid.
      **Second review: BLOCK on two Importants, both fixed.** Its Critical count was zero — the first round's fix holds — but two
      survivors of the first review were real. (1) `put_cards_away` was called by every *pointer* path and by none of the keyboard
      ones: F2 opened the settings card **underneath** the panel-sized browser (a key that looked dead), and while the browser was
      up `selection::dispatch`'s modal guard did not name it, so `Enter` edited a step nobody could see and marked the session
      unsaved. Both F1/F2 arms now go through `put_cards_away`, the guard names every card, and
      `opening_a_card_puts_the_kit_browser_away` drives the keys and the invisible `Enter`, with a mutation behind each. (2)
      `kit_dirty` was a `bool`, so `write_kit` wrote whatever device happened to be **selected** at save time: edit a kit on the
      second drum machine, click back to the bass, quit — the write aimed at device 1, the factory early-return cleared the flag,
      and the copy was never written while the session said it had been. It is `Option<usize>` now — the number of the device
      owed the write — and `an_edit_records_which_device_is_owed_the_write` pins the panel half; the `main` half is a binary and
      has no test, which is recorded here rather than implied. Of its minors: a failed kit load is no longer overwritten by its
      own `kit: X` confirmation (`load_kit_now` answers whether it loaded); the problems of a whole kit arrive in **one** notice
      naming every silent file instead of the last one; `Init` is a reserved name (`kits::is_reserved`), because a folder called
      `Init` would load, play and then be forgotten — `SavedState::kit: None` means the default kit, whose name is `Init`; the
      two assertions that could not fail are gone (the list one now asks the function for two sorted blocks with no duplicate);
      `04-the-face` no longer names the device card twice; `default_name() == "808"` is pinned; and the browser caps its rows
      above the footer, where forty kits used to paint over `IMPORT` and `CLOSE`. That last one the songs card still gets wrong,
      which is recorded as a deferred minor below rather than fixed here.
      **Rulings.** (1) `load_kit` answers `LoadedKit { kit, problems }` rather than the plan's `Result<Kit, KitError>`: Review
      Focus 2 and the plan's own test want a kit to load around a missing file, and one error channel cannot say both "there is
      no kit" and "this voice is silent". If wrong, every caller matches a struct for nothing. (2) `save_kit` and `import_kit`
      take the rate and answer `KitError` rather than `io::Result`: a WAV records its own rate, so a save without one writes the
      wrong pitch, and one error type for a kit's whole life beats two vocabularies for the same folder. If wrong, two call sites
      match an enum where they used `?`. (3) The factory kits are built from two CC0 libraries — Sonic Pi's sample set and VCSL —
      and the TicTokMen set is rejected because its README says only "free use". The names `808`/`909` are therefore presets, not
      transcriptions: a 909 whose hats are freesound one-shots is a claim the files do not all support, and the caveat is in
      `CREDITS.md`'s own words. If the ear rejects it, it costs `refresh.py` rows or three names — no code. (4) The per-voice
      defaults (tune, decay, cutoff) are set by intent and **nobody has listened**: `kits.rs` says so where they are written. If
      wrong, constants only. (5) The default kit is factory #1 (`808`) and `SavedState::kit: None` means it, spelled once in
      `kits::default_name`. If wrong, a session that never ran an audio device comes back on `808`, and reordering `FACTORY`
      silently moves the default. (6) **Superseded by the follow-up below:** an import that finds one broken voice was refused
      whole, and the player's partner asked for the opposite — a kit that lands with the file named is one they can repair, and a
      refusal gives them nothing to repair. (7) A kit load over
      unsaved edits takes two clicks, which the plan did not ask for: the edit is not on disk, and one click would throw it away.
      If wrong, one extra click. (8) A kit loads into the device whose card opened the browser (the selected one) and is written
      from the edited one (`PanelState::kit_dirty` carries that device's number, so the write cannot drift to whatever is
      selected when the save happens), while the **document** still names the saved device's kit — so a kit edited on a second
      drum machine
      lands in the library and is named by the rack round, exactly as that device's patterns are lost until then. If wrong, the
      rack round's `SavedDevice` list is where it is repaired, and no player loses a folder in the meantime.
      **Third review: BLOCK on four P1s, all fixed — it closed the two Importants and found what the fixes opened.** (1) The
      rate-change arm adopted the new rate *before* writing the pending kit, and `save_kit` stamps the rate it is handed into
      every WAV: a device switch would have labelled old samples with the new rate, and the reload would then play the player's
      kit sharp for ever. `write_kit()` now runs first, at the rate the samples are at, and the move is one line with the reason
      next to it. (2) `kit_dirty` held a rack *position*, so a device removal shifted it and the write landed on a neighbour or
      on nothing — the same "reported saved, never written" harm by a second route, and against the field's own doc. It holds
      the device **number** now, the identity `focused` and `edit_slots` already use, found with `devices().iter().find(…)`;
      `an_edit_records_which_device_is_owed_the_write` asserts the number. (3) A drum device still on the default kit made the
      reload loop ask the library for `Init` and report a kit that never existed: `load_kit_now` answers the default kit for that
      name, and the startup load only runs for a first device that has a kit at all. (4) Page 05 said three cards spend the
      editing keys; it says **any** card now, which is what the guard does. Of its P2 notes, taken: `tests/modal_live.rs` sweeps
      **every** open card (the browser and the device card included, dim and click-outside), where it used to sweep three and
      claim "every"; a failed kit write leaves the session unsaved instead of reporting it kept; the two-click guard asks the
      *selected* device's number rather than the panel's; the missing-WAV notice sentence is pinned whole; and the F1 arm has
      its own mutation, so "a mutation behind each" is now true of F1, F2 and the invisible `Enter`. Left recorded, not paid:
      `shift`+note and the MIDI queue still write a step behind a card (`ui::selection`'s arrows and `Enter` are the ones
      guarded — pre-existing, and the one fix is a `PanelState::a_card_is_up()` asked by all three writers).
      **Follow-up after the commit (asked for by the player's partner), four changes, all paid.** (1) **An import with a broken
      voice now lands instead of being refused** — ruling (6) reversed: `import_kit` answers `ImportedKit { folder, name,
      problems }`, the copy's document keeps the source's file name for a voice that did not decode (so the kit is a kit *with a
      hole* rather than one that never had the voice), and the status line says `imported X, but KICK: KICK.wav is missing - put
      that file in its folder and load it again`. A test proves the whole arc: the import lands, the hole is named, the right WAV
      dropped in under that name makes it load whole. (2) The three deferred minors are paid. `write_folder` deletes a
      `<VOICE>.wav` the document no longer names (only names this module writes are ever touched), so a kit folder holds exactly
      its samples — and `save_kit` keeps a name **only while its file is absent**, which is what tells "the player deleted this
      voice" (delete the file and the name) apart from "this kit has a hole" (keep the name, so it stays repairable). The songs
      card caps its rows above the footer keys the way the kit browser does. And `PanelState::a_card_is_up()` is now the one
      spelling of "the face is covered": the step editor's arrows and `Enter`, the computer keyboard's step writes (a note key
      behind a card auditions instead) and a MIDI note waiting to be placed all ask it, which closes the class the second review
      found one instance of. **816 tests**, clippy and fmt clean, `./docs/guide/check.sh` ok, and the mutation run is **42 of 42**
      — the follow-up's nine are a copy that does not keep the file name, an import that forgets its problems, a save that keeps a
      stale WAV, a save that forgets a name, a save that deletes a file it cannot decode (the review's Important finding, fixed),
      an import that lands a document naming a path,
      a song list under the footer, a key writing behind a card, and a MIDI note landing behind one. The follow-up's own review
      found one Important — that the deletion rule, as first written, deleted a WAV the app could not *decode* — and it is fixed
      with `a_save_keeps_a_wav_that_will_not_decode`: only a file this build can play back is one the player deleted.
- [x] **Task 7 — the machine.** `acid-core::sequencer` holds the `Performer` trait (`step`, `release`, `render`, `arm`) and
      `Sequencer::render<P: Performer>(&mut self, performer: &mut P, out: &mut [f32])` renders through it, so the bar, `REVERSE`,
      swing, pause, the bar line and the sample-accurate start are one implementation for every machine and only "make this step
      sound" moves. All three render paths — the wait before a late start, the tail after a stop, the block loop — go through the
      trait, and the 303's slide history moved out of the `Sequencer` into `Voice`, where `Performer::arm` clears it, called on
      the step a start begins with. `Voice`'s inherent `render` is **gone**: rendering a voice *is* its `Performer` role, so
      `accent.rs`, `velocity.rs` and the four `voice_*.rs` test files gained one `use acid_core::sequencer::Performer;` line each
      and nothing else.
      `drums.rs` gained `Drums` — a `Sequencer` and the sixteen voices — with `new`, `set_kit`, `set_mute_mask`, `audition`,
      `render`, `set_pattern`, `set_bpm`, `set_swing`, `position`, `bar_line`, `start`, `start_at`, `start_later`, `stop`,
      `pause`, `resume`, `set_reverse` and `set_transpose`. A voice is a fractional sample position advanced by
      `2^((TUNE + transpose)/12)`, linearly interpolated between two samples, through the envelope and the new filter; an accent
      takes the 303's gain *and* its decayed filter pulse; a hit on a hat folds the other hat away through its release. `filter.rs`
      gained `MultimodeFilter`, the topology-preserving state-variable filter: `LP | BP | HP` from one pair of integrators, cutoff
      and resonance mapped by `param::cutoff_hz`/`param::resonance` exactly as the 303's are. `Drum::bit()` now owns the mask bit
      — trap 12's fourth caller — and `pattern::drum_bit` was deleted onto it in the same commit. `tests/sequencer_feel.rs`'s
      slide fixture was strengthened (step 0 now slides, so the slide history is actually set): it had been vacuous.
      **848 tests, all green**, clippy and fmt clean, `./docs/guide/check.sh` ok. New: 26 in `tests/drums.rs` — the twelve the
      task named, plus `the_rack_transpose_moves_every_voice`, `a_muted_hat_still_chokes_an_open_one`,
      `a_kit_swap_changes_the_sound_and_keeps_the_hit_going`,
      `a_new_machine_takes_its_mutes_from_its_kit_and_a_kit_swap_does_not`, `a_kit_with_no_samples_is_silence_and_a_hit_is_harmless`,
      `a_hostile_sample_is_silence_not_a_poisoned_voice`, `an_enormous_sample_does_not_outlive_itself`,
      `a_cut_off_turn_is_heard_on_the_next_block`, `a_resonant_voice_at_the_edges_stays_finite`,
      `a_late_start_waits_out_the_silence_it_is_given`, `a_pause_keeps_the_step_and_a_resume_carries_on_from_it`,
      `a_start_strikes_rather_than_gliding_from_the_previous_run`, `start_at_lands_on_the_step_it_names` and
      `reverse_reads_the_bar_from_its_other_end` — five unit tests in `filter.rs`, and
      `restating_a_time_does_not_restart_the_ramp` in `tests/envelope.rs`.
      **Mutations run, 42 of 42 caught (and 2 equivalent, below), all reverted** (`.superpowers/mutate_task7.py`): a retrigger that does not restart; an
      envelope never gated; a stop that releases nothing; a release that cuts instead of folding; the kit's release time ignored;
      the end of a sample read past; a non-finite sample reaching the filter; the accent's gain dropped; its filter pulse dropped;
      the voice's filter kind ignored; TUNE ignored; the transpose ignored; the choke group ignored; a muted voice's stroke
      skipped (so it chokes nothing); the mask never read (twice, one test each); a kit swap resetting every voice's place;
      loading a kit seeding no mutes; an edit reseeding them; the audition drained and thrown away; the bar line dropped; the
      silence before a late start skipped; a resume starting from the top; the performer not armed; a rest keeping the slide
      history; the mask bit counted backwards; the low face answering with the high and the high with the low; resonance never
      reaching the damping; the cutoff unclamped; `reset` leaving the integrators; a release re-stated every block (twice, the
      envelope's own test and the drum tail); an overflowing filter state left in the voice; an audition surviving the stop;
      `start_at` starting at the top; the direction never reaching the clock; one voice past the rails; and a choke that cuts
      nothing. The first run of thirty reported **five MISSED**, and each was a hole rather than a script problem: a peak-only
      accent assertion that the brightness lift could satisfy; a kit-swap test whose second block held no hit; a restart test
      whose own bar ended by striking step 0 and clearing the slide; a rest test whose previous step did not slide; and a
      `next()` guard that the empty-sample path never reached. All five tests were rewritten or retargeted, and the record is
      here because "the tests bite" is only true of the run that says so.
      **Review: OK with notes, every finding actioned.** The Important finding was real and is the reason this entry is long:
      `DrumVoice::shape` re-states the kit at the top of every block, and `Envelope::set_release_ms` re-armed the ramp whenever
      it was called, so a release restarted every block never finished: a 20 ms release became a 20 ms *time constant*, and its tail
      ran nowhere near to its end — the smaller the block, the longer it ran, and a block as long as the release hid the whole bug,
      which is why every test passed with every tail rendered in one gulp. Fixed at the source: all
      three of `Envelope`'s time setters are now no-ops when the value has not moved (`restating_a_time_does_not_restart_the_ramp`
      proves it, and both tail tests now render in device-sized blocks, so the release is measured at the granularity a device
      asks for rather than in one gulp). Of its
      Minors: the non-finite guard promised a repair it did not do, so the check moved to the *output* and resets the filter's
      state (an enormous but finite sample can overflow a resonant filter's own state — `an_enormous_sample_does_not_outlive_itself`),
      and the now-redundant input guard was deleted rather than kept as a second rule; a hit asked for but not yet served now goes
      with the stop that silences the machine; the two tautological `pattern.steps[0]` assertions are gone and the misleading
      kit-swap message says what its assertion proves; the per-sample `tan` now only runs while an accent's pulse is worth hearing;
      a resonant voice at both cutoff clamps has a test; and `MultimodeFilter::kind()` — unused — is deleted.
      **Second review: OK with notes, no Critical and no Important, all six Minors actioned.** It confirmed the Important is fixed
      at the source and that both tail tests would fail without it, that one guard is the right count and the reset is
      load-bearing, and that `hush` is needed because `Sequencer::stop` only closes a gate it opened. Its first finding was that
      `hit`'s accent write came *after* `shape`, so a plain hit could keep the previous accent's cutoff for a block. It is now
      written first — but the mutation that reorders it back **is equivalent**, and the record says so: every hit is followed by a
      render, and `DrumMachine::render` re-shapes the voice at the top of every chunk, so `shape` always runs again before a
      sample is produced. What that finding had assumed could be reached is not reachable through `Drums`' API, and the reordering
      is kept because it makes the precondition local rather than depending on the render loop. The same is true of the
      accent-then-plain case could not fail and was replaced by `a_cut_off_turn_is_heard_on_the_next_block`.
      **Third review: OK with notes — no Critical, one Important and six Minors, all actioned.** The Important was that a branch
      the round above called load-bearing was held by nothing: the accent's spent pulse was drawn to zero in an arm whose other
      line re-applied the plain cutoff, so the zeroing was that arm's only real job and no test could fail it. The arm is now just
      `self.accent_pulse = 0.0` — `shape` owns the plain cutoff — which makes it load-bearing, and the cut-off test was rebuilt
      around it: an **accented** hit, then nine and a half of the pulse's time constants in device-sized blocks so it is spent, and
      the baseline window taken
      *after* it. Measuring during the accent was the first version's hole: the accent is far brighter than either cutoff, so the
      assertion held without the dial turn landing at all. The arm is held by the zeroing-removed mutation, and the test's other
      mutation makes `shape` never re-apply the plain cutoff — the `> 1e-4` sweep arm is a cost fence, not the thing the test
      pins, because it multiplies `self.cutoff_hz`, which `shape` keeps current. Of its Minors: the spec's rewritten sentence said
      the sixteen voices implement the trait (they do not; `DrumMachine`
      does) and the Interfaces line now names the four `Drums` methods ruling 2 added; `shape`'s doc no longer claims the filter's
      setters are no-ops when unchanged (they hold no ramp state, which is the real reason re-stating is safe); `Kit::set_mute_mask`
      no longer claims to be the only other place a mask bit is decoded (`DrumMachine::render` reads it too); the ladder's module doc
      no longer claims it skips a rebuild for an unchanged cutoff, which `Voice` never gives it; and the script can now apply a
      mutation of several edits at once, so `a_kit_with_no_samples_is_silence_and_a_hit_is_harmless` — which no *single* mutation
      could fail, because either guard alone is enough — is held by removing both.
      **The script tells equivalent mutations apart from caught ones.** Two are recorded as equivalent and the run fails if either
      is ever caught: moving the accent's write after `shape` (equivalent because every hit is followed by a render, and
      `DrumMachine::render` re-shapes first), and `shape` setting the plain cutoff even while an accent's pulse is running
      (equivalent because the per-sample arm rewrites it to the same swept value). Those two are in the record so that a caught
      count cannot be read as "every mutation tried was caught".
      **Rounds four, five and six found only bookkeeping, corrected in this commit:** a duplicated doc line on `DrumVoice::shape`, an
      underived literal in the cut-off test (now `9.5 × ACCENT_DECAY_MS`), four prose statements that outran the code (the ~280 ms
      figure was a tail's duration rather than its time constant; `Drums`'s Interfaces comment still described the pre-split struct;
      the spec cited four tasks where five corrected their own lines; the test file's header claimed *every* test drives `Drums` when one
      drives `Sequencer` and `Voice`), and two Progress sentences that did not match the script. A seventh pass found the two figures
      above — a ~280 ms tail that was a small-block limit, and "six" tasks where five corrected their own lines — and both are
      fixed here by stating the mechanism instead of a number.
      **Rulings.** (1) The trait's implementor on the drum side is a private `DrumMachine`, not `Drums`: `Sequencer::render`
      borrows the performer for the block while holding the clock, so the performer must be a field, and implementing the trait on
      `Drums` as well would give the voices two ways to be rendered. The spec is corrected in the same commit. If wrong, it is one
      wrapper type and task 8's `Machine` calls `drums.render` either way. (2) `Drums` also carries `set_pattern`, `set_bpm`,
      `set_swing` and `position`, which the Interfaces line did not name: without them a drum machine cannot be given anything to
      play or compared with the 303's clock. (3) `Drums::new` seeds the mask from its kit and `set_kit` does not, mirroring
      `SharedState::adopt_kit` (the load path) against `set_kit` (the edit path) — if wrong, one dial turn undoes a mute.
      (4) `stop` and `pause` silence the machine themselves, because an audition sounds with no transport under it and the clock
      closes only a gate it opened; the clock's own close at the next render starts the same ramp from the same level. (5) A
      sample that leaves the filter's state out of range plays as silence and resets the filter rather than silencing the voice for
      the session — task 5's deferred ceiling question, answered here. If wrong it costs one branch. (6) An accented drum takes the
      *whole* of the 303's accent lift, because a drum face has no `ACCENT` knob; if wrong, one constant and one test. (7) The mask
      silences a voice's output, not the sequencer: a muted closed hat still cuts an open one, which is `a_muted_hat_still_chokes_an_open_one`
      and a deliberate reading of trap 5. (8) `the_envelope_shapes_the_tail` reads the plan's "a long release passes the ADSR's
      release point" as a 200 ms release measured in windows *through* it, since a stop is the only gate-off this machine has.
      (9) Envelope time setters are idempotent — the fix for the review's Important — which also means a knob landing on its own
      current value no longer restarts anything; if that was ever wanted, it is `gate_on`/`gate_off`'s job, not a setter's.
      **Deferred, deliberately:** `tests/no_alloc.rs` with a drum slot rendering is task 8's line, and no test here proves the
      audio path allocates nothing *with drums in it* — the code allocates nothing (one `Arc` field read, no `Vec`, no `format!`,
      no lock), but the gate that would catch a regression is task 8's.
      **Invisible to the guide, deliberately:** nothing a player can reach changes until task 8 gives the renderer a `Machine` — a
      drum device's face still draws the 303's sound under the grid — so no page in `docs/guide` is now wrong and the drum face's
      page is task 10's.
- [x] **Task 8 — the renderer holds a machine.** `audio::renderer::AudioRenderer` owns one `Machine`: `Acid(Box<Engine>)` or
      `Drums { drums: Box<Drums>, kit: Arc<Kit>, volume: Smooth }`, chosen once at open from the device's own kind. The public
      surface is kept except `engine()`, which is gone: four `tests/renderer.rs` assertions now read `state.position`, the step
      the panel's chase reads, because a `Machine` cannot answer `&Engine` for a drum device and nothing outside that file asked.
      Per block: `push` (the 303's seven knobs and volume into its engine, a drum machine's tempo and swing), `settle_transport`
      (one three-state decision, dispatched to either machine's `start`/`resume`/`pause`/`stop`), the direction and the transpose
      on change, `service_drums` (the kit by `Arc::ptr_eq`, the mask by one `u32` load, a `HIT` drained once), `service_note` (the
      acid-only MIDI gate), the pattern by `Arc::ptr_eq`, then the render loop, then `state.position`.
      `apply_drums` and `apply_volume` are the two free functions beside `apply`; the second is the 303's master VOLUME applied to
      a drum block — same `param::master_volume` taper, same `param::VOLUME_MS` ramp, exact silence at zero — because a drum
      machine has no engine to hold its gain. The two `let … else` service functions are exhaustive two-armed `match`es, so a
      third kind fails to compile at every decision in this file rather than falling into nothing.
      **848 → 860 tests** (26 in `tests/mixer.rs`, up from 14: the four the task names for that file — its fifth is the extension
      to `tests/no_alloc.rs` — plus `a_hit_is_heard_with_the_transport_stopped`,
      `a_drum_device_added_to_a_paused_rack_joins_the_step_it_is_holding`, `a_303_joining_a_drum_machine_finds_its_bar_line`,
      `a_paused_drum_device_keeps_its_step_and_a_stopped_one_plays_the_bar_again`, `a_drum_device_plays_the_bar_backwards_when_the_rack_asks`,
      `a_pattern_published_to_a_playing_drum_device_is_heard`, `a_note_on_a_drum_device_is_silent_and_on_a_303_is_not` and
      `the_volume_knob_sets_a_drum_devices_level`), clippy and fmt clean, `./docs/guide/check.sh` ok. `SAVE_VERSION` untouched at
      **11**: this task adds no document field.
      **One ruling, and it is the one the plan left open.** The renderer is built from a `SharedState` and nothing else, and
      `BusState.slots` carries no `DeviceKind`, so the kind had to arrive somewhere. It lives on `SharedState` as an immutable
      `pub kind: DeviceKind`, set at construction by `SharedState::for_kind` / `with_tempo_for` (and the older `new`/`with_tempo`
      mean the 303, which is what a state built without a device is), and **`Device`'s own `kind` field is deleted**:
      `Device::kind()` reads the state's, so the face and the sound cannot answer the question differently. `Device::on` and
      `Device::adopted` lost their `kind` parameter, `Rack::add` builds the state with the kind, and `Rack::from_saved` keeps the
      303's. **If wrong:** the alternative was `BusState.slots: Vec<(Arc<SharedState>, DeviceKind)>`, which keeps the kind on
      `Device` and costs a wider diff on the audio path (the mixer, the MIDI router's two loops, every `BusState` literal in a
      test) for the same answer; moving back is that diff. The cost of this one is a save file with no kind, which the next
      entry pays.
      **A second ruling, on the drum device's VOLUME.** The knob is drawn in the top strip of **every** face, so a drum device
      must hear it; the engine holds the 303's gain and a drum machine has none, so `apply_volume` is the renderer's. It is
      seeded at the knob's value at open (so a device opened with the knob down does not ramp down for its first 10 ms), ramps
      otherwise, and is exact silence at zero. **If wrong:** a second gain somewhere else in the drum path is the fix, and the
      one test that pins it (`the_volume_knob_sets_a_drum_devices_level`, with the open-at-the-knob ratio and the ramp's shape)
      would move with it.
      **Mutations: 22 of 22 caught, all reverted** (`.superpowers/mutate_task8.py`): a drum device built on the 303's state; the
      kit never handed over; a pattern published mid-play lost; the mask never read; the audition drained and thrown away; the
      master volume never applied; volume 0 fading instead of silencing; a gain that steps instead of ramping; a device opened at
      full gain whatever its knob says; the published step stuck at 0; a drum device handing out no bar line; a late start
      started at once; the tempo, the shuffle, the transpose and the direction each never reaching the machine; `stop`, `pause`
      and `resume` each not reaching it (or reaching the wrong verb); a kit swapped mid-tail allocating; a key on a drum device
      striking a voice; `Device::kind` always answering the bass. Three runs are part of the record: the first reported
      **9/12** and the three misses were holes — a
      published step of 0 passes every `drum == rack` equality, a drum device's own `bar_line` is never asked for when it is not
      slot 0, and the rack transpose had no test at all — which cost `furthest > 0`,
      `a_303_joining_a_drum_machine_finds_its_bar_line` and a transpose in the sample comparison; the second, after the first
      review, reported **17/19** because the paused/stopped silence was asserted on one block instead of a bar, which a machine
      that kept clocking satisfied; the third reported **19/20** until a pattern publish got its own test.
      **Review: five rounds, and every round after the first found something real.** Round one: Important — the drum arm's
      `stop`/`pause`/`resume`/`set_reverse` and `apply_drums`' swing were unproven (paid with the transport and reverse tests,
      swing in the sample comparison, and four mutations); Minors — `Machine::of` took the kind beside the state (now reads
      `state.kind`), the volume ramp was claimed and not measured (now `rising < 0.9 * settled` on a held envelope), and three
      comments outran the code. Round two: Important — `Machine::Drums => drums.set_pattern` was reached by no test, so a grid
      edit could be inaudible with the suite green (paid with `a_pattern_published_to_a_playing_drum_device_is_heard` and its
      mutation); Minors — the two service functions would have silently skipped a third kind (now exhaustive matches), `mixer.rs`
      still called every slot an engine with its own limiter, the drum gain was a second copy of the engine's 10 ms (now
      `acid_core::param::VOLUME_MS`, used by both), one assertion's message claimed more than it proved, and page 01 still denied
      a drum machine. Round three: BLOCK, narrowly, on the executor's own test — the waiting half of
      `a_drum_device_sounds_through_the_mixer` never ran, because a drum device added before the mixer is released on the first
      block's own downbeat, and its control rack was never set playing, so "the kit must be heard" would have passed with silent
      drums (paid by adding the drum device *after* a bar, setting the control rack playing, and asserting the wait was seen:
      `waited > 0`); Minors — "a `Machine` is two words" was false, the initial-gain claim had no test, `MUTE_MS` still held a
      literal, "engine" survived in two places, the drum gain takes no idle snap, and a rate change reseeds the player's mutes.
      Round four: BLOCK, narrowly, on two things this commit itself left — (1) a drum device's MIDI notes went **silent** where
      the old renderer (which held an `Engine` whatever the kind) played them, and the Progress claimed no page was wrong, while
      `14-midi` and `15-keyboard` both say a device's one voice is played by the keyboard; (2) `service_note`'s drum arm was the
      one drum decision with neither a test nor a mutation. Paid with `a_note_on_a_drum_device_is_silent_and_on_a_303_is_not`
      (the same key on both, so the silence is a decision and not a missing message), a mutation that turns the guard into a MIDI
      pad, one sentence on each of the two pages, both pages and `01-what-it-is` in task 10's Files, and this entry corrected.
      Its Minors: three more "engine"s (the waiting-slot doc, the `no_alloc` message, and a test name, now
      `a_steady_rack_rebuilds_no_machine`); the handover said the mixer did not change when its prose and `MUTE_MS` did; the
      handover and the plan named `factory` for the five demo names, which live in `acid-app::demo::songs()`; the wishlist's
      `is_playing_something` was not the condition `Engine::set_volume` actually snaps on (`!playing && !voice.is_active()`),
      and its "no test could tell it from the ramp" was simply false (the volume test's ratio would); and the phase overview still
      pointed at a superseded handover.
      Round five: OK with notes, ready to land — no behavioural finding, four stale numbers in this record and the handover
      (859 for 860, 21 for 22, "three rounds" where there were four) and one reason that was still wrong: the wishlist said the
      volume test's ratio would see the idle snap, but that test's rack is *playing*, which is exactly when the snap does not fire.
      All four are corrected here and in the handover, and the wishlist now names the test that would see it (transport stopped, a
      `HIT` in the same block as the knob move, the hit's opening level compared with the settled one) instead of misnaming an
      existing one.
      **Rulings, with what each costs if wrong.** (1) The kind lives on `SharedState` and `Device` has no field for it — above.
      (2) The drum device's master gain is the renderer's, seeded at the knob, ramped at `param::VOLUME_MS` — above. (3) The
      machine is `Machine::{Acid(Box<Engine>), Drums { … }}` with both arms boxed, where the plan's Interfaces line says
      `Machine::{Acid(Engine), Drums(Drums)}`: clippy's `large_enum_variant` is right that a 1.5 KB drum machine should not sit
      inline in a value whose job is to say which machine this is, and both are built once at open. If wrong, two boxes.
      (4) The plan's surface list names `stop`, `pause` and `resume`, which `AudioRenderer` never had: the transport arrives
      through `Parameters::transport` and `settle_transport` is private. The list is corrected here rather than the code.
      (5) `AudioRenderer::engine()` is deleted rather than made to return `Option<&Engine>`: nothing outside one test file used
      it, and that test now asserts the published step — the seam the panel reads. If wrong, one accessor.
      **Deferred, deliberately, and recorded in the wishlist:** a sample-rate change re-decodes one kit through the *load* path,
      so `adopt_kit` reseeds the mask and the player's shift-clicked mutes come back — pre-existing, audible only now, and its
      fix is `main`'s, which has no test. The drum gain also cannot take `Engine::set_volume`'s idle snap, because `Drums` has
      no "nothing is sounding" answer; that is a wishlist line with what would have to move.
      **The guide moved by three passages, and no more.** `01-what-it-is.md` said "if you want a polysynth **or a drum machine**,
      this is the wrong door" — false the moment a `DRUMS` device sounds like drums — so the clause is gone and the sentence now
      scopes the 303 clone ("the 303 clone is one line at a time … if you want a polysynth, this is the wrong door"). `14-midi.md`
      and `15-keyboard.md` both said a device's one voice is played by the keyboard and the MIDI gate: true of a 303 and no
      longer true of a drum device, whose renderer now answers a note with silence (`service_note`'s drum arm), where before this
      commit a drum device still held an `Engine` and a note played. Each page gets one sentence saying a drum machine takes no
      notes this phase, and both are in task 10's **Files** line. The rest of page 01's framing (its "one device is one voice"
      line, and "not a sampler" beside a machine that plays samples) is task 10's, and `01-what-it-is.md` is in that list too.
      **The other drum-face pages are not wrong yet:** a drum device still opens on an empty grid (its masks are zero, so the
      machine sounds nothing), the card, the browser and the picker are task 6's and task 4's, and no page claimed what a `DRUMS`
      device sounds like or which banks its `BANKS` offers — the drum face's page is task 10's page 22.
- [x] **Task 9 — a drum bank for every demo variation.** `acid_app::demo` gained the drum side of the five demos: each
      `Demo` carries `drums: [Pattern; 8]`, five authored `groove`/`turnaround` pairs are built through one `bar` table of
      `(Drum, steps, accented steps)` strikes, and eight variations are derived from each pair (`groove`; `groove` with a
      downbeat crash; hats out; kick out; a shaker on the last sixteenth of each beat; `turnaround`; `turnaround` without
      the snare; `turnaround` with sixteenths into the bar). `demo::drum_banks() -> Vec<(&'static str, PatternBank)>` is
      that same list read from the drum side — eight patterns in `A1`..`A8`, the other seven banks empty, built by one
      `bank_of`. Every strike goes through `Step::hit`, so a groove cannot draw a cell the machine does not sound.
      `demo::load_drum_bank(&mut Rack, index) -> Option<&'static str>` is what a `DEMO BANKS` row does on a drum machine:
      the bank through `Device::load_bank` (which opens it on `A1` and publishes it with `Device::publish_pattern`, the one
      spelling of that rule, used by `adopted` too), then the demo's own tempo on the rack's cell and its shuffle on the
      device's own. `main`'s `LoadDemo` arm branches on `Device::kind`: a 303 loads the whole demo song exactly as before,
      a drum machine loads that demo and nothing else. Tests: four in `tests/drum_banks.rs` (new) — the three the task
      names, plus `a_demo_arrives_on_a_drum_machine_as_patterns_a_tempo_and_a_shuffle`. **864 tests** (860 + 4), clippy and
      fmt clean, `./docs/guide/check.sh` ok, `SAVE_VERSION` untouched at **11** — a bank is patterns and adds no document
      field.
      **Mutations run, 11 of 11 caught, all reverted** (`.superpowers/mutate_task9.py`): a demo renamed while its drum bank
      is not; every demo given the slow acid groove; a groove written with `Step::note` (a grid that draws what the machine
      never plays); a bank filling the second bank as well; two of a bank's eight variations being the same bar; the demo's
      tempo never reaching the rack; its shuffle never reaching the machine; a loaded bank opening on its second pattern; a
      loaded bank never reaching the state; a bank arriving with the default kit; and a device never publishing the bank it
      was built on. The last one was **MISSED on the first run** against the test I guessed for it, which is why the script
      names `mixer::two_devices_sum_to_something_louder_than_one` — `adopted`'s publish is held by the 303's sum test, not
      by the drum device's, whose two patterns happen to coincide. The script now also **fails on a `SKIP`**: task 5's
      record is right that a skip is not a pass, and its exit code used to say otherwise.
      **Rulings, with what each costs if wrong.** (1) **The banks live in `acid-app`, beside the songs, not in
      `acid_core::factory`**: the names are acid-app's, acid-core cannot depend on acid-app, and `factory::drum_banks()`
      plus a test holding two lists equal would leave exactly the hand-kept second list this repo has paid for twice. As a
      field of `Demo` there is one list, so a rename renames both sides and no test has to say so; the plan's **Files** and
      **Interfaces** lines are corrected here. If wrong, the grooves are ~200 lines of tables to move and a name comparison
      to add; nothing else depends on the home. (2) **`the_demo_names_are_the_acid_names` pins the five names as literals**
      and compares the two accessors, but the equality half is structural now (one list) and cannot fail — the pin and the
      "no two demos share a groove" loop are what bite, which the first two mutations show. If wrong, the test is a rename
      reminder and a copy-paste guard rather than a drift guard. (3) **A demo's tempo and shuffle travel with its grooves.**
      A bank is patterns only — the kit, the knobs and the session's name stay where they are — but a *demo* is more than
      its patterns: a rack has one tempo cell and a drum device is built at swing 0.0, so without them a funk groove plays
      at the slow acid's 118 and the blues arrives straight. The first review's Important finding was exactly that the
      shuffle was missing (see below). If wrong, two lines in `load_drum_bank`. (4) **`Device::publish_pattern` is
      extracted** so `adopted` and `load_bank` cannot spell the publish rule two ways, and it is named `publish_pattern`
      rather than `publish` because `Rack::publish` — the `BusState` write `AGENTS.md` names — lives in the same module. If
      wrong, one duplicated pair of `set_pattern` calls. (5) **The card itself is unchanged in what it draws.** The spec
      says a drum device's `DEMO BANKS` lists the same five names as the acid side, and with one list it does — so the kind
      is asked once, in `main`'s written-out match, and `ui/banks.rs` changes by its docs only. That is the plan's Files
      line corrected again: it named `ui/banks.rs` for a listing change the "same five names" rule makes invisible, and the
      load went behind `demo::load_drum_bank` rather than four statements in a binary no test can see. (6) **The tests live
      in `tests/drum_banks.rs`, not `tests/banks_live.rs`** as the plan said: nothing in them drives `draw_panel`, and
      `_live` is the repo's word for a test that does — the one exception to the naming in `tests/`, and a suffix is a
      claim like any other. If wrong, a file name.
      **Untested, deliberately, and why:** `main`'s arm is a binary with no tests (the same limit task 6 recorded for its
      kit paths), so what a `DEMO BANKS` row loads is a function the test calls directly (`demo::load_drum_bank`), with
      four mutations behind it — tempo, shuffle, the opening pattern and the published state. What is left in `main` is
      three calls the acid arm already makes (`save`, `pattern_swap::adopt`, the notice) and the request's routing. The
      card's drum rows are the acid rows by the spec's own rule, so there is no widget whose behaviour differs for a live
      test to drive; `tests/songs_live.rs` keeps the row's request.
      **The drum grooves are listening work and nobody has heard them.** The tables are written from the head against
      `EAR_TESTING.md`'s marks — a four-on-the-floor for the two acid demos, a ride-led shuffle for Blues (its swing is the
      clock's, and the demo sets it), a backbeat for Rock, sixteenth-note syncopation with a clave on the last sixteenth of
      two for Funk — and every pattern's sounding is a test's, but the ear has not passed any of them. If a groove is wrong
      it is a table, not code; `kits.rs`'s per-voice defaults carry the same caveat from task 6.
      **Review: BLOCK on two Importants, both fixed in this commit, and every Minor actioned.** (1) **The drum side was
      losing the demo's shuffle** while `demo.rs`'s own comment and this record claimed the drums lean where the walk leans:
      swing is per-device, a drum machine is built at 0.0, and the arm set only the tempo, so Blues arrived straight and
      Funk lost its 0.12 — a lie in the code, the record and (after my first draft) page 12. `load_drum_bank` now writes
      `demo.state.swing`, page 12 says "tempo and shuffle", and a mutation holds it. (2) **The seam stopped at the bank.**
      Everything about a demo that is not a pattern was re-derived in the binary, which is where (1) hid; the demo's
      identity now lives in `demo::load_drum_bank` with the whole arc under test. Of its Minors: `check.sh` had the
      "# 4. the facts" comment line twice (my edit re-added the header instead of replacing the wrapped line); `publish`
      collided in name with `Rack::publish` and is `publish_pattern`; page 12's first draft said the fresh patterns make
      the name go amber, which oversells a save that still writes the first device only (the page now says the grooves last
      the session); the funk clave's comment said "the and of two" over a table that puts it on the last sixteenth; and this
      record claimed task 10's Files line named page 12 when it did not — it does now, because page 22 wants the
      cross-reference from the page a player arrives at `DEMO BANKS` from. The review also confirmed the ruling on where
      the banks live ("right home, and the alternative is strictly worse") and found the unlistened-grooves caveat honest.
      **The guide moved by one page and one fact.** `12-songs-and-demos` said a `DEMO BANKS` row loads a demo "as a **new**
      song", which is no longer true on a drum machine — the same five rows now load that demo's drum bank at its tempo and
      shuffle and leave the kit and the session's name alone — so the page gains one paragraph saying so, and `check.sh`
      pins `drum bank` so a rewrite cannot drop it — proven by replacing the phrase on the page and watching `check.sh` stop
      with `the guide never says: drum bank`, then restoring it. Nothing else in the book describes the BANKS row; the drum face's own
      page is task 10's page 22.
      **Second review: OK with notes — no Critical and no Important, and it checked each of the first round's eight findings
      as genuinely gone rather than trusting the record.** Of its notes, two were fixed here. (1) `drum_variations`' doc said
      "Every one of them keeps the downbeat", which is false of the kick-out break on the slow acid groove (its step 0 holds
      the kick and nothing else) — the same class as round one's first finding, a comment claiming a property the table does
      not have, and no test could hold it because no test asserts a downbeat. The sentence now says "all but the kick-out
      break", which is also the reason the break is worth having. (2) Page 12's paragraph did not say the demo's tempo lands
      on the **rack's** cell, so loading a drum bank on the second device also retunes the 303s beside it — and that tempo,
      unlike the grooves, is what a save keeps. The page says both now, and says where the grooves themselves are kept (the
      first device alone). Two notes are left standing and recorded rather than paid: `banks::paint`'s two-click guard prints
      `unsaved session: click again to replace it` on a drum face, where a demo row replaces that device's eight patterns and
      not the session — a sentence a player reads, so it is the player's call and the wishlist carries it with what would have
      to move; and `demo::drum_banks()` has no production caller (the app reaches a bank through `load_drum_bank`, and the
      test holds the two equal across `A1..A8`), which is the documented pairing seam for the rack round rather than a second
      list. Its second-round limits are worth recording too: the reviewer had no shell, so it verified the diff and the
      assertions by reading and re-ran nothing — the gates below are the executor's, run after the fixes.
      **Third review (the delta only): OK with notes — no Critical, no Important, and every finding prose.** It caught four
      claims the new handover made and the code does not: "no hover tips on drum controls" (the drum `SHUFFLE` and the shared
      keys keep the 303's tips; only the seven dials, `LP | BP | HP` and `HIT` have none), a factory kit's first edit copying
      "silently" (it says so on the strip and the browser's header reads `NOW: <name> - READ ONLY`), "two CC0 libraries and one
      Freesound collection" (`CREDITS.md` and ruling 3 both say **two** collections), and `STEP` being "inert" on a drum face
      (it lights and toggles; only the writing it opens is gated). Its remaining notes: the wishlist said `banks::paint`
      already receives the `SharedState` (`kits::paint` does; `banks::paint` would grow the argument), the deferral named the
      wrong round, `drum_variations`' second phrasing still read as "the kick-out break never keeps the downbeat" (it can, in
      four of the five banks — the comment now says why the slow acid's cannot), and page 12's bare "unlike the grooves" is
      qualified. Two structural notes were acted on as well: `handover-task-9.md` now carries its superseded note and
      `00-phase-4.md`'s bullet points at `handover-task-10`, which the phase overview requires of every handover ("only the
      newest is instructions") — they travel in this commit rather than a follow-up, so the diff that landed is the diff that
      was read. Nothing in the third round touched code: what it moved was prose in a handover, a wishlist entry and a page,
      and the gates below were re-run after it.
- [x] **Task 10 — the book, and two defects the pictures exposed.** `22-the-drum-machine` is a new page: the grid and its
      gestures, the seven dials' relabelled meaning with `LP | BP | HP`, `HIT` and `SHUFFLE`, the kit browser and what a kit
      is, the five demo drum banks, and a "what is not here" section (no notes, no step or performance row, one hit per voice,
      no per-voice hover help). It is in `SUMMARY.md` under *Making a sound*, and the pages a reader arrives from — `04`'s new
      "The second face", `12`'s drum-bank paragraph, `19`'s `KIT` section, `05`'s kit-browser entry and `18`'s picker
      paragraph — link it. `01-what-it-is` now opens on both machines ("one voice, or sixteen") and answers "not a sampler"
      for a machine that plays samples; `20-reference` carries the face's whole vocabulary. `check.sh` pins `KICK`,
      `LP | BP | HP`, `MY KITS` and `drum machine`. Two pictures at 1960 × 1340 (`drum-face.png`, `kit-browser.png`), each
      with its own row in `img/README.md` and the session that took them recorded there. **870 tests** (867 at the end of task 9, plus three the reviews added), clippy and
      fmt clean, `./docs/guide/check.sh` ok. `SAVE_VERSION` untouched at 11.
      **The pictures found two real defects, both fixed here with a test and a mutation behind each.** (1) The kit browser
      drew `MY KITS` **on top of** `808`: the caption was measured straight after `FACTORY` and only moved when a player's
      kit appeared, so a first run — three factory kits, no others — put it across the first row. Found by looking at the
      grab the page was going to publish, not by the suite. `rows` now places it after the factory rows when the player has
      none, and `each_caption_sits_above_its_own_rows_and_never_on_one` holds it. (2) Task 9's deferred minor: the two-click
      guard said `unsaved session: click again to replace it` on a drum face, where a demo row replaces that device's eight
      patterns and not the session. `banks::paint` now takes the device's kind and `armed_hint`/`guard_message` answer per
      machine (`click again to replace the grid`), with the song rows and `NEW` keeping the session's sentence — a song *is*
      a session on either face — and `the_guard_names_what_a_second_click_would_replace` pinning both. Of the plan's other
      deferred items, three are left standing and each is in the wishlist with what would have to move: per-voice hover help
      (the tip table is a 40-variant catalogue and nine new player-facing sentences are the player's to hear first), the
      rate-change reload reseeding the player's mutes, and the drum gain's missing idle snap. The version-refusal wishlist
      entry is behaviour on the one path every file goes through and belongs to the round that also writes the rack document.
      **Mutations run, 7 of 7 caught** (`.superpowers/mutate_task10.py`): the kit browser's caption measured after `FACTORY`
      again; the kit list's cap removed; the drum face's warning naming the session again; and each of the four new pins
      taken out of the guide (`check.sh` stops with `the guide never says: …` for each). The first run reported **3 of 7**,
      and the four misses were the script's own bug rather than a weak pin: it walked `src/*.md` and missed
      `src/img/README.md`, where `KICK` and `MY KITS` are also said, so the check kept passing. `guide_pages` now walks the
      tree, and the record keeps that run because "the pin bites" is only true of the run that says so.
      **The guide is the deliverable, so its pins were proven, not assumed:** removing `drum bank` from page 12, then each of
      the four new phrases from every page, stops `./docs/guide/check.sh` with the phrase it stopped saying.
      **The pictures are the round's other test, and one of them is why the kit browser is in this commit.** Both are at
      1960 × 1340 on a display checked with `xdotool getdisplaygeometry` first (`2200 1500`), from a fresh
      `TAKKA_ACID_DATA_DIR`, pointer parked away from the panel before each grab, and the state each one records is the
      state `img/README.md` says.
      **Invisible to the guide: nothing.** This task *is* the guide, and the two defects above were prose's subject before
      they were code's: a page describing the kit browser's two lists would have described a picture that contradicted it.
      **Whole-branch review (phase 4 against `main`): BLOCK on one Critical and three Importants, all fixed in this
      commit.** The Critical was this task's own: `04-the-face` printed "The second face" **twice** — the first paragraph's
      continuation was deleted when it was rewritten and the opening sentence was left behind, so the page the new page hangs
      from rendered a sentence starting twice and stopping mid-clause. `check.sh` cannot see a duplicated line; the reviewer
      can, which is the difference between the two tools. The Importants: (1) page 22 said the drum filter "is the same one
      the 303's is", and `filter.rs` says in its own module doc that it is deliberately **not** the 303's ladder but a
      state-variable filter that *maps* cutoff and resonance the same way — the one sentence that explains `LP | BP | HP` was
      the one contradicting the engine. (2) `LEARN` is drawn on every face and picks nothing on a drum machine's dials; worse,
      a *drag* while it was choosing turned the voice and, on a factory kit, copied the whole kit under a free name because
      the hand meant to teach. The dials and the filter key now stand still while a learn is choosing (the 303's own idiom,
      without a target to pick), `a_learn_in_progress_turns_no_drum_dial` holds it, and the wishlist carries the two ways to
      finish the job — a `CcTarget` per voice, or hiding `LEARN` there, a face change and so the player's. (3) The factory
      kits' sample tables are positional and nothing bound a row to a voice: swapping the 808's kick and snare rows left
      every existing check green. `every_factory_voice_plays_the_file_named_after_it` now decodes the WAV in the repository
      whose name is the voice's own label and compares it to the compiled-in voice, and the mutation that swaps those two
      rows is caught. Of its minors, all actioned: page 22's kit folder is written "when the session is saved"; `20-reference`
      and `ui/dispatch`'s two comments now count **five** cards inside the panel and name the kit browser in the escape order;
      the kit is in the three enumerations that describe what is on disk (`20`'s `state.json` line and its files table, `18`'s
      restart paragraph, `12`'s "one song, one rack"); `1u32 << drum.index()` became `u32::from(drum.bit())`; and
      `Kit::set_mute_mask` is **deleted** — it had no production caller, and `SharedState::drum_mute`'s doc claimed the kit
      and the state were "the only pack and unpack" when nothing unpacks into a kit any more. The retired `Arc<Kit>` freed on
      the audio thread is now a wishlist line with what would have to move, because it ships knowingly and the record did not
      say so. The review's second-round caveat is worth keeping: it had no shell, so the gates below are the executor's, run
      after every fix.
      **Mutations after the review, 9 of 9 caught** (the same script, two entries added): a learn in progress turning a drum
      dial again (a two-edit mutation, because either half of the guard alone already stops it — the lesson of task 7's
      accent ordering), and the 808's kick and snare rows swapped. **870 tests** (867 + the learn test, the sample-binding
      test).
      **Second review of the delta (after the whole-branch fixes): BLOCK, narrowly, on three prose findings and six minors —
      all actioned.** The three Importants were book text this commit would have shipped: page 22 said `LEARN` on a drum face
      could "only bind the `VOLUME` knob" when the shared top strip also lets it bind the tempo and pattern readouts (the
      wishlist entry written in the same commit had it right, so the page was the one liar); `18`'s new kit sentence sat
      beside a list that still counted kits as memory-only and a "They are gone on restart" that contradicted it, so the
      paragraph now separates what the file keeps, what the folder keeps and what the rack forgets; and `02-install` still
      said the data directory holds **two** things, with no kit in its `state.json` line — it says three now, with a `kits/`
      row, and `17-trouble`'s version of the same sentence took the same clause. Of the minors: the drum `SHUFFLE` got the
      303's `learn_pick` treatment (it is the one knob on that face with a real target, `CcTarget::Swing`) and its guard is
      recorded as **equivalent**, because picking ends `choosing` on the press frame so the swing is only ever turned by a
      drag already past the pick in either spelling; the filter key's half of the learn guard got its own assertion (mutating
      `&& !choosing` away now fails, which it did not before); the two-click guard's wording got a **live** test
      (`a_drums_demo_row_is_armed_against_its_grid_not_the_session` in `tests/songs_live.rs`, a frame-driven click on the demo
      row, with the unit test in `banks.rs` kept for the exhaustive match); `ui/mod.rs`'s "four cards" comment is five;
      `check.sh`'s duplicate `MY KITS` pin is dropped, so the round added **three** new pins, not four; the phase's test count
      is **870** everywhere it is stated; and `handover-phase-5` now points at the wishlist entry and the `SavedDevice` shape
      the record actually has, rather than at a section and a field list it invented.
      **Mutations at the end: 10 of 10 caught, 1 recorded as equivalent, 0 skipped.** Three were added after the reviews (the
      dial guard as a two-edit mutation, the filter key's guard, the 808 row swap), and the equivalent is the `SHUFFLE`'s,
      written down so a caught count cannot be read as covering it.
