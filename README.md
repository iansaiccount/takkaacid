# takka-acid

A TB-303-style acid bass synth, written in Rust: a real-time audio engine and an egui panel that looks like the machine
it is named after.

## How it was built

This instrument was written end to end by a coding agent — **pi**, running the **`deepseek-flash`** model — against a
one-page prompt, and the whole `plans/` directory it produced is in this repository (see [Plans](#plans-the-evolution-of-the-product)).
The agent's tooling is worth naming because it shaped the code: **pi-lens** (LSP navigation and diagnostics, plus
tree-sitter/ast-grep rules), **pi-lean-ctx** (a token-compressing read/shell cache), **pi-subagents** (fresh-context
review agents), **pi-web-access** (source lookup), **pi-hashline-edit-pro** (anchored edits), **pi-clear** and
**pi-extension-no-timeout**, with a small local **pi-retry** extension. Every round of work ended with the same three
gates: `cargo test --workspace`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

## What it is

An acid device is one voice, sixteen steps, and the four knobs that make the sound: the oscillator's wave, the filter's cutoff
and resonance, the envelope's decay, and how much the accent pushes through. It is not a sampler and not a workstation; it is
the box that plays one line well — and one window holds up to four of them. A **drum device** is the other machine: sixteen
sample voices against the same sixteen steps, with its own kit
([`docs/guide/src/22-the-drum-machine.md`](docs/guide/src/22-the-drum-machine.md)).

- **Engine** (`rust/crates/acid-core`, no UI, no allocation in the audio callback): saw, square, sine and triangle
  oscillator; a resonant ladder low-pass filter; an envelope with accent-modulated decay; glide/slide; a soft limiter and
  a drive stage; 40–300 BPM with swing.
- **Panel** (`rust/crates/acid-app`, egui/eframe): painted to look like a 303 — silkscreen legends, lit keys, a chasing
  step row — with the seven tone knobs, a performance row, and a transport row.
- **Drums** (`rust/crates/acid-core::drums`, with the kits in `acid-app`): sixteen sample voices per machine, each with its own
  envelope and a three-way filter; a kit is a folder of WAVs, and three ship in the binary.

## The drum machine

The second kind of device is a **drum machine**: sixteen sample voices against the same sixteen steps, with its own kit and its
own grid. Add one with the strip's `+` → `DRUMS`. The whole of it is
[`docs/guide/src/22-the-drum-machine.md`](docs/guide/src/22-the-drum-machine.md); this is the summary.

- **The grid is the editor.** One row per voice — `KICK`, `SNR`, `RIM`, `CLAP`, `CHH`, `OHH`, `LOT`, `MID`, `HTOM`, `CRS`,
  `RDE`, `CWB`, `LCO`, `HCO`, `CLV`, `SHK` — and one cell per step, every fourth column framed so the beat is findable without
  counting squares. A click strikes that voice on that step, a `SHIFT`-click accents it (louder and brighter), a click on a
  struck cell takes it off, and a `SHIFT`-click on an accented cell takes the accent off. Clicking a row's **label** selects the
  voice — its plate lights, its row carries a faint band and its name is printed over the dials — and `SHIFT`-clicking the
  label **mutes** the row: dim, with its accents gone, so a muted voice can still be found and unmuted by eye.
- **The dials are the selected voice's**, in the 303's own seven slots: `ATTACK`, `DECAY`, `SUSTAIN`, `RELEASE`, `CUT OFF`,
  `RESONANCE`, `TUNE`. Beside them sit a three-way **`LP | BP | HP`** filter key, the **device's** `SHUFFLE` (not one of the
  voice's dials: it swings the drums the way the 303's swings a bass line, and it is the one drum-face control `LEARN` can
  teach), and **`HIT`**, which plays the selected voice, stopped or running, so a kit can be tuned without writing anything into
  the bar.
- **The keyboard.** `up` / `down` walk the selected voice; `left` / `right` move the selected step, a lit column across the
  sixteen rows; `enter` writes the selected voice into that step, silently, and a second `enter` takes it off; `shift+enter` is
  the accent gesture. The selection stays where it is after a write, because a step here holds sixteen voices and a hand writes
  several before moving on. `h` is `HIT`'s key, `l` / `b` / `p` pick the filter's sides; the letter that presses a control is
  underlined on it, and every control in the dial row explains itself on hover.
- **A voice's sound is a kit.** Three ship under `FACTORY` — `808`, `909` and `Acoustic` — each read-only: editing one copies it
  under a free name. `MY KITS` starts empty and holds your own; a kit is a folder of WAVs plus a `kit.json` under
  `$XDG_DATA_HOME/takka-acid/kits/<name>/`, written when an edited kit is saved or a folder of WAVs is imported, so a kit can be
  copied, edited by hand and handed on. The face prints `KIT <name>` above the dials and the device card's `KIT` row opens the
  browser. `rust/crates/acid-app/kits/CREDITS.md` records where every sample came from (all CC0).
- **Drum banks.** On a drum device, `BANKS` loads that demo's **drum bank** instead of a song: eight grooves into `A1`…`A8`, at
  the demo's tempo and shuffle, with this machine's kit and the session's name left alone. A drum bank is patterns, not a song.
- **Its own transpose.** The transport row's `TRANS -` / `TRANS +` move **the device on the panel** — the readout between them is
  that device's and a second machine keeps its own — so one drum machine can be a semitone up while the 303 beside it is not.
- **What it does not do yet.** No letter plays a note on a drum device and a MIDI note on its channel is silent; MIDI pads and
  per-voice controllers are a later round, so `LEARN` leaves the voice's dials, the filter key and `HIT` alone. A second strike
  on the same voice retriggers it rather than layering a second copy.

## Up to four machines

The window holds up to four devices. Each one is a whole machine — its own pattern bank, its own shuffle, its own MIDI inputs
and its own name — and a strip above the panel is the whole rack: a cell per device with its name and its play lamp, the
cell the panel is showing outlined, a `M` key at the right end of each — that device's mute — and a `+` at the far right
that adds one.

- **Adding, picking, muting.** The strip's `+` opens a picker of the kinds a device can be (two today, `ACID` and `DRUMS` —
  [the drum machine](docs/guide/src/22-the-drum-machine.md)), and every
  row of it is greyed once the rack holds four. Click a cell, or press `Ctrl+1` to `Ctrl+4`, to put that device on the
  panel. The `M` at a cell's right end mutes that device without stopping it — a 10 ms fade, not a cut — so a device that
  was running mutes late and unmutes back in time.
- **One card per device.** `F3` opens the card for the device on the panel: its name, whether it plays what it hears or only
  answers controllers, which ports it hears and on which channel, its mute, its removal (`REMOVE` asks twice), and the theme
  it wears — one of twenty-two, from the list its `COLOUR` row opens.
- **One transport, one clock.** The deck's `PLAY` / `PAUSE` / `STOP` and the tempo belong to the rack and every device follows
  them; a device added
  while the transport is running waits for the bar line and comes in with everyone at the top of it, rather than starting on
  whatever step the rack happens to be on. Swing and the performance
  row are per device.
- **MIDI learn is per device**, so the same controller number can drive a knob on two devices at once, and each device has
  its own ports and channel.
- **What a restart costs.** `NAMES AND EXTRA DEVICES ARE NOT SAVED YET`: the session file still holds one device — the first
  one's patterns, patch, swing and learned controllers, plus the first MIDI port the app had open — so extra devices, the
  ports, channel and role any device was given, the theme it wears, and every name typed for one, are gone on restart. "Save the whole rack" in `plans/future/wishlist.md` is the round that ends this.

## Features

**The pattern.** Sixteen steps per pattern, 64 patterns in eight banks of eight (`A1`…`H8`). Each step holds a note, an
octave, a velocity, an accent and a slide. `Ctrl+C` copies the pattern on the panel and `Ctrl+V` pastes it — into another
slot, another device's pattern, or another window of the app, because the clipboard is text.

**The sound.** Seven tone knobs — waveform, tuning, cutoff, resonance, envelope mod, decay, accent — plus a volume knob.
Every one of them can be taught to a MIDI controller.

**The drums.** A second kind of device: sixteen sample voices, a kit behind each, a per-voice envelope and a three-way filter,
and the same sixteen-step sequencer, transport and themes as the acid device
([the drum machine](#the-drum-machine), above).

**The performance row.** Transpose an octave either way, shuffle the clock, or roll a randomised acid line from a density
and a scale.

**Transport.** Run/stop, tempo in 40–300 BPM, swing, and a step LED that chases the audio rather than the wall clock.

**MIDI.** Which ports are open is the settings card's; which of them a device hears, and on what channel, is on that
device's card; notes with velocity; every control learnable, with pickup on the dials; a pattern selector that waits for
the bar; and an external clock for the tempo.

**Keeping your work.** A session file and a library of named songs, with a songs card to save, copy, import, export, load
and delete them.

## Installation

**What you need.** Everything the app actually asks for, including the parts this list used to miss:

| Needed | Why |
|---|---|
| Rust 1.85 or newer (the workspace is edition 2024; developed on 1.99) | builds it |
| `build-essential` and `pkg-config` | linking |
| `libasound2-dev` (the ALSA headers) | the audio and MIDI crates talk to ALSA directly |
| A Wayland or X11 session | egui needs a surface to draw on |
| A working OpenGL 3.3+ or GLES driver | the panel is drawn on the GPU |
| `xdg-desktop-portal` **and a backend** (`-hyprland`, `-gtk`, …), plus a D-Bus session | `IMPORT` and `EXPORT` are the only file dialogs, and `rfd` is built with `xdg-portal` |
| PipeWire or ALSA for audio out | `cpal`'s default host |
| `mdbook` 0.4 or newer (built with 0.5.4) | building the player's guide only |
| `grim`, `slurp` and ImageMagick | regenerating the guide's screenshots only |

On Debian or Ubuntu the packages in the second and third rows are one command; on Arch, another:

```bash
sudo apt install build-essential pkg-config libasound2-dev
sudo pacman -S base-devel alsa-lib
```

**Build and run** from the repository root:

```bash
cd rust
cargo run --release -p acid-app --bin takka-acid
```

The binary is `rust/target/release/takka-acid` afterwards, so you can run it without cargo.

**Where it keeps things.** The session and your songs live in `$XDG_DATA_HOME/takka-acid`, or `~/.local/share/takka-acid`
when `XDG_DATA_HOME` is unset. Point `TAKKA_ACID_DATA_DIR` somewhere else to keep a separate rig or to test.

**Two flags worth knowing.** `takka-acid --midi-map` prints what the app makes of every note from 21 to 108 without opening a
window or an audio device — the fastest way to answer "is it my keyboard or the app?". And the app picks its MIDI input
from the settings card, remembering the choice.

**Working on it.** Every change is gated by:

```bash
cd rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

`EAR_TESTING.md` explains the other half of testing: the suite pins RMS, peak, brightness and timing, but whether it sounds
like acid is a judgement only your ears make — including how to render the voice to WAVs and A/B them.

## The player's guide

Playing the instrument — installing it, the face, the knobs, writing a line, songs, MIDI, and what to do when it makes no
sound — is `docs/guide`, an mdBook with screenshots of the real panel. `mdbook serve docs/guide` reads it in a browser,
`mdbook build docs/guide` writes the site into `docs/guide/book/`, and `bash docs/guide/check.sh` is its test: the book
builds, every page is reachable from the table of contents, every image and every cross-reference points at something that
exists, and the facts that must not drift are still in it.

## Where it started: `Prompt.txt`

The project began with a five-line brief, kept verbatim in [`Prompt.txt`](Prompt.txt). It asked for a plan, in markdown, for
a Roland TB-303 clone in Rust called *takka-acid-rust*, broken into logical major steps with a file per step — ten or more
of them — with the code under `rust/` and the plans under `plans/`, and "a great user interface … a black and yellow version
of the traditional TB-303 design". Everything else in this repository — every feature, every plan, every test — grew out of
that one prompt.

## Plans: the evolution of the product

`plans/` is not documentation written after the fact: it is the record of how the instrument was built. Each round of work
has a numbered plan, from `00-main-plan.md` (the original breakdown) through the engine, the panel, the sequencer, MIDI,
persistence and the polish rounds, to the drum machine and the rounds after it (`plans/phase 1/` to `plans/phase 4/`, where
the numbering restarts with each phase).

Every plan has the same shape, and reading two or three of them in order shows the product changing:

- **What is wrong today** — the problem in the user's terms, with the code that causes it named.
- **The design** — the shape of the fix, including the types and signatures that decide it.
- **The tests that make it safe** — the table of tests, what each one pins, and the mutation checks that proved the tests
  can fail.
- **Task list** — the work in commit-sized pieces, each ending green.
- **Non-goals** — what the round deliberately does not do.
- **`## Progress`** — filled in as the work happened: what landed, which test proved it, and the *rulings* the
  implementer made where the plan was silent, each with what it costs if wrong. That section is how a later session (or a
  human) resumes without re-deriving decisions.

The plans are included on purpose: they are the audit trail of an agent-built product. They show the mistakes (a knob
catalogue that had to be centralised after six copies drifted, a MIDI map whose array length panicked), the reviews that
caught what the tests missed, and the rulings that were taken instead of guessed at. `plans/notes` holds the scratch notes
that fed them.

## Repository layout

```
plans/                 one plan per round, numbered, with their progress sections
Prompt.txt             the original brief
EAR_TESTING.md         how to judge the sound by ear, and render WAVs to compare
rust/
  crates/acid-core/    the engine: oscillator, filter, envelope, sequencer, patterns, bank, drums
  crates/acid-app/     the panel, shared state, MIDI, persistence, the binary
  crates/acid-app/tests/   the live tests: they drive real egui frames
AGENTS.md              the conventions the codebase is held to
docs/                  design notes and review material
  guide/               the player's guide (mdBook): its pages, screenshots and check.sh
```

## Licence

MIT.

## Credits

The twenty-one themes besides the instrument's own black are built from the palettes published on the music colour palette
page at <https://www.media.io/color-palette/music-color-palette.html>. Those palettes are that page's work; the derivation,
the mixed faces and anything that reads badly are this project's. What a theme is, and how the list chooses one, is in
[`docs/guide/src/19-a-devices-card.md`](docs/guide/src/19-a-devices-card.md).

## Demo songs and saving

Five demos ship inside the binary — **Slow Acid**, **Hard Techno Acid**, **Blues**, **Rock** and **Funk** — each a patch
and a bank of eight variations of its line. They are read-only, and picking one loads it as a new song called "New song",
unsaved. Saving is manual by default: `SAVE` writes, and the name is amber while the session is unsaved and plain silk once it
is on disk; a name that has changed writes a new file rather than moving the old one.

The whole of it — the card, its buttons, and exactly what a save does to a name — is
[`docs/guide/src/12-songs-and-demos.md`](docs/guide/src/12-songs-and-demos.md).
