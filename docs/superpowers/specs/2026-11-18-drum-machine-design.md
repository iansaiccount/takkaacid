# Design — the drum machine (phase 4)

2026-11-18. Choices marked **decided** were the player's; **call** are mine, with the reason. Revised after the builder check:
the editor is the **16 × 16 grid on the main panel**, not the card, and the seven knobs are **repurposed per voice**.

## Intent

A second machine in the rack, beside the 303 clone: **a 16-voice sample drum machine** — a grid of sixteen sounds against the
sixteen steps — with per-voice ADSR, filter and tuning, sharing the 303's bar, patterns, banks, transport, transpose and reverse.
Three kits ship (acoustic, 808, 909), a player can build their own from their own WAVs, and each of the demo variations gets a
matching drum bank so a loaded song is a groove from the first bar.

**Why.** The rack is already the instrument. A drum machine makes it a groovebox rather than four basses, and it exercises the
seam the plans have pointed at since phase 2: a device whose engine is not the 303.

## Goals

- A drum **pattern** is a bar of sixteen steps on the 303's grid, in the same 64-pattern bank type.
- **Sixteen voices** — a fixed catalogue — each with ADSR, a multimode filter (low / band / high) with cutoff and resonance,
  tuning, a mute, a choke group, and a sample.
- The **seven knobs stay where the 303 puts them**, relabelled for the selected voice; the **grid sits under them**, taking the
  rest of that band plus the performance and step bands the drum machine does not use.
- The **selected voice is unmistakable**: its row's label is a lit plate in the accent, its row carries a faint band, and the
  knobs are captioned with the voice's name — `KICK`, `SNR`, … — so what is being edited is readable at a glance.
- **`SHUFFLE` stays.** It is the device's swing — the same clock control the 303 has, and it swings the drums exactly as it
  swings a bass line — so it keeps a knob on the drum face even though the rest of the performance row goes. It sits apart from
  the voice's own dials, because it is not the voice's.
- **Audition**: the selected voice can be played while editing, without the transport running.
- **Three factory kits** and **custom kits** from the player's own WAVs, browsed like a bank.
- **Factory drum banks**: an eight-pattern bank for every demo variation the acid side ships (Slow Acid, Hard Techno Acid, Blues,
  Rock, Funk), loadable from a drum device's `BANKS`. Later, a song loads the acid bank and the drum bank together.
- The drum machine shares the 303's clock: same bar line, pause, `REVERSE`, rack transpose, swing.
- Kits and patterns are data, so a new sound or a later fill is a kit or a bank, not code.
- **Themed like any device.** Every surface of the drum face — the grid's cells, its labels, the lit plate, the knobs — paints
  from `theme::Palette` (never a hard-coded colour, and never `theme::BLACK` the way `card_button` does, because a card is always
  black and a *face* is not). A drum device carries a `scheme` like any other, its COLOUR row picks from all `Scheme::ALL`, and a
  test paints the face in the black scheme and a light one and asserts the grid's shapes change with the palette.

## Non-goals

- **The rack-wide session document.** A session still saves one device; `SavedState` gains only the kit's name. The whole rack is
  the next phase's first item (and takes `SAVE_VERSION` 12 — the kit's name took 11, as this spec asked).
- **Polyphony within a voice.** One hit per voice at a time; a second hit retriggers it.
- **MIDI pads and per-voice CC.** The device keeps its port and channel; notes on a drum channel are a later round, and the
  303's learn/CC map does not apply.
- **More than sixteen voices.** A later fill needing a seventeenth voice is a third mask.
- **Loading a whole song (acid + drums) in one gesture.** The drums' factory banks load per device this phase; the song that loads
  both waits for the rack document.
- **Exporting a kit or a bank** as an archive.

## Anti-goals

- A drum machine that drifts from the 303's bar.
- A second pattern type that makes the bank browser, the clipboard and the save file branch.
- Anything that allocates or locks on the audio path.
- A face with dead controls. On a drum device the 303-only controls are gone, not inert.

## Constraints

`AGENTS.md` in full: catalogues with no `_` arm, no hand-kept second list, no font glyphs (except `…`), the save file is a
contract, a message is a notice, nothing on the audio path allocates (`tests/no_alloc.rs`), no lock across the callback, and a
fresh review per commit. The book is part of the work.

**Licensing:** a bundled sample must be CC0 or public domain, with its origin in `kits/CREDITS.md` — redistribution is a
different right from "royalty-free in your music". Where a voice has no CC0 sample it ships as a synthesised stand-in and the book
says so. Two sources ship: `smpldsnds/sonic-pi-samples` (CC0 per its README, one freesound URL per file) and `sgossner/vcsl`
(CC0 per its README, the acoustic kit and the non-machine voices). `smpldsnds/sfzinstruments-tictokmen-retrodrums1` was
considered and **not used**: its README calls its recordings "free use" without naming a licence, which fails the test this
paragraph sets.

## The pattern

`Step` gains two `u16` masks, `#[serde(default)]` so a version-9 file upgrades to zeroed masks:

```rust
pub hits: u16,     // which of the sixteen voices this step strikes
pub accents: u16,  // which of those strikes are accented; a bit without its `hits` bit means nothing
```

Names matter: `Step::accent: bool` already exists and is the 303's own, so the mask cannot share the word. **One writer** for
both worlds — a drum edit calls `Step::hit(voice, accented)`, which sets the mask bit, the accent bit and `on`, so `on` can never
drift from the mask; a drum step is "on" because a voice is. The 303 reads `note`, `octave`, `velocity`, `accent`, `slide`, and
ignores the masks. `SAVE_VERSION` → 10; `persist::upgraded` keeps upgrading rather than refusing, and `tests/persist.rs` pins the
bytes and the upgrade.

## The clock, and the performer

The bar, `REVERSE`, swing, pause, the bar line and the sample-accurate start stay in `acid_core::sequencer`. Only "make this step
sound" differs, so that becomes a small trait:

```rust
pub trait Performer: Sized {
    fn step(&mut self, step: &Step);
    fn release(&mut self);
    fn render(&mut self, out: &mut [f32]);   // the sequencer renders its performer directly
    fn arm(&mut self);                        // clears per-run state (the 303 clears its slide history)
}
```

`Voice` implements it (taking the `prev_slide` state `Sequencer` holds today), a private `DrumMachine` over the drum machine's
sixteen voices implements it, and
`Sequencer::render` keeps its shape. Every render path — the delay in `wait_out_start`, the tail after a stop, the block loop —
already goes through the performer, which is why `render` is on the trait.

**Corrected after the fact**, as tasks 1 and 3–6 corrected their own lines: `Drums` does not implement the trait itself. A
`Drums` owns the clock, and `Sequencer::render` borrows the performer for the length of a block while it holds that clock, so the
performer has to be a **field** of the machine — a private `DrumMachine` with the kit, the sixteen voice states and the mask.
Implementing the trait on `Drums` as well would give the voices two ways to be rendered, which is the second-list bug this repo
has paid for twice. `Drums` is the machine; `DrumMachine` is what the clock drives.

## The drum machine

`acid_core::drums::Drums` owns a `Sequencer`, the current `Kit`, and sixteen voice states:

```rust
pub struct Kit { pub name: String, pub voices: [KitVoice; 16] }
pub struct KitVoice {
    pub sample: Arc<[f32]>,                 // mono, at the engine's rate, already resampled
    pub tune: f32,                          // semitones, ±12
    pub attack_ms: f32, pub decay_ms: f32, pub sustain: f32, pub release_ms: f32,
    pub filter: FilterKind,                 // Low | Band | High
    pub cutoff: f32, pub resonance: f32,    // knob positions, mapped like the 303's
    pub mute: bool,                         // the kit's starting mask
}
```

A hit resets the voice's sample position, gates the existing `Envelope`, and gives the hit its accent: louder (the 303's accent
gain) and brighter (the filter opens further, as `ENV MOD` does). A voice renders its sample at a fractional position advanced by
`2^(tune/12) × rate_ratio`, through a **new multimode state-variable filter** and the envelope. A hit on a voice in a non-zero
choke group releases the others in that group — the open hat cuts itself off under a closed hat.

Machine output chain: performer → (no drive this phase) → the mixer's master limiter. `Drive` has its place in the chain and its
sound is unspecified; it waits.

## The voice catalogue

`acid_core::drums::Drum`, sixteen variants in grid order:

```
Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom, MidTom, HighTom, Crash, Ride, Cowbell, LowConga, HighConga, Clave, Shaker
```

`Drum::ALL`, `index()`, `label()` (`KICK`, `SNR`, `RIM`, `CLAP`, `CHH`, `OHH`, `LOT`, `MID`, `HTOM`, `CRS`, `RDE`, `CWB`, `LCO`,
`HCO`, `CLV`, `SHK`), `choke_group()` (`ClosedHat`/`OpenHat` share one, the rest zero) — all written out, no `_` arm. Sixteen
voices is exactly one `u16`.

## The face

A drum device's panel, design size 980 × 670, with the strip, top band, switches row and transport row untouched. The knobs stay
where the 303 puts them; the grid goes underneath, and takes the performance and step bands the drum machine does not use:

```
  114 ┌ knobs: (ATT)(DEC)(SUS)(REL)(CUT)(RES)(TUNE)  [LP|BP|HP] [HIT] ┐   the 303's band, its top half
  232 │ KICK │▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪│  grid: 16 rows × 16 steps,
      │ SNR  │▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪ ▪│  ~15 px a row, from under the
      │ ...  │                               │  knobs to the switches row
  520 └──────────────────────────────────────────────────────────────────┘
      │ switches row: BANK PATTERN COPY PASTE SAVE BANKS STEP │
      │ transport row: REVERSE TRANS- [-0] TRANS+ TBC TBC LEARN │
```

- **The knobs are the 303's seven, in the 303's place**, relabelled and repurposed for the selected voice: `ATTACK`, `DECAY`,
  `SUSTAIN`, `RELEASE`, `CUT OFF`, `RESONANCE`, `TUNE` — with the waveform switch becoming the three-way filter key `LP | BP | HP`
  and a `HIT` key beside it that auditions the selected voice (audible with the transport stopped). They occupy the top ~110 px
  of their band; the 303's band is 216, so the grid starts at 232 and runs to the switches row — 288 px, which is what the grid
  would have had if it had taken the whole knob band anyway. The knobs give up height, not the grid.
- **`SHUFFLE` keeps a dial** at the right-hand cluster, between the filter key and `HIT`, set apart from the voice's seven because
  it is the device's swing and not the voice's: the same `shared.swing` the 303's performance row writes, so a shuffle set on the
  drums is the shuffle the rack's other machines hear.
- **The selected voice is unmistakable.** Its label is a **lit plate**: accent fill, dark caption — the same drawing a lit key
  uses. Its row carries a faint accent band behind the cells, and the knob group is captioned with the voice's name (`KICK`)
  above `ATTACK`, so the row and the dials say the same thing. A muted voice's label prints dim and its row's cells lose their
  accent (they stay readable, so a muted voice can be unmuted by eye).
- **The grid** is the editor: a row per voice, a cell per step. A click toggles that voice on that step; `SHIFT`-click makes it
  accented (the 303's own accent gesture). An unaccented strike is a dim filled square, an accented one is accent, a rest is the
  well. Every fourth column is framed in silk, so the beat is findable. The row's label selects the voice; `SHIFT`-clicking the
  label toggles its mute.
- **What is gone on a drum device**: the step row (the grid's columns are the steps), the performance row (`OCT`, `CLEAR`,
  `RANDOM`, `SHUFFLE`, density, scale — all 303-only). The switches row keeps every key: banks, patterns, copy/paste and save are
  the drum machine's too.
- **Themed.** Nothing on this face is drawn in a fixed colour: the plate, the row band, the cells, the beat frames, the labels and
  the knobs all read `theme::Palette`, so the device's `COLOUR` row themes the drum machine exactly as it themes a 303 — including
  the light schemes, where the plate inverts (dark fill, light caption) the way a lit key already does.

The device card for a drum machine keeps its rows (name, ports, channel, colour, mute, remove, close) and gains the **kit's name
and a `KIT` key** that opens the kit browser — the card is where a device is set up, and the panel is where it is played. The
browser is the banks card's sibling: `FACTORY` / `MY KITS`, the factory three marked read-only.

## Persistence

- **Kits**: `<data>/kits/<name>/kit.json` plus one WAV per voice (`KICK.wav`). The JSON holds the voice table, each entry's
  `file`, `source` and `credit` for provenance. `save_kit`, `load_kit`, `import_kit` are plain path functions (testable without a
  display); `import_kit` lands beside a taken name, as `import_song` does.
- **Factory kits** are compiled in (`include_bytes!`): about 2 MB for three kits at 16 voices × ≤0.6 s × 16-bit mono. They are
  **read-only**: editing one silently makes a copy under a free name (`free_song_name`'s rule), so the browser has two sections
  (`FACTORY` / `MY KITS`) like the banks card, and a saved kit name is never ambiguous.
- **Factory drum banks** live in code beside the acid factories: one eight-pattern bank per demo variation, so a drum device's
  `BANKS` → `DEMO BANKS` lists the same five names as the acid side.
- **The session** gains `kit: Option<String>` (`#[serde(default)]`): the kit the saved device plays; absent means the default.
- **Kit names** are validated like song names, so a name can never become a path.

## What's left to the builder

- The multimode filter's topology and how cutoff/resonance map from knob positions (copy the 303's feel from `knob.rs`).
- The grid's exact cell and label sizing at 0.65× and 1.6× scale, and the hit target at the smallest.
- The factory kits' per-voice defaults and which CC0 file fills which voice; the factory drum banks' patterns. Both are listening
  work (`EAR_TESTING.md`).
- The synth stand-ins for voices with no CC0 sample.

## Risks

- **The grid's height.** Sixteen rows in 288 px is ~15 px a row at the design size with 3 px gaps — the smallest thing this face
  has drawn. The mockup at 1:1 is the test; if it is too tight the lever is a slimmer knob area (each 10 px off the knobs is
  0.6 px on every row).
- **Two masks in the save file.** Old files upgrade; a version-9 build reading a version-10 file loses the drum steps silently,
  which `SAVE_VERSION` is there to prevent.
- **A sixteen-bit cap**, and a third mask if a fill needs a seventeenth voice.
- **Bundled samples and licensing**: a wrong CC0 claim is a legal problem; `CREDITS.md` and the review are the guard.
- **A long sample is cut** by a stop or a retrigger. That is correct for a drum machine and the first thing to check on the ear.
