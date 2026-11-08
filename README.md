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

One voice, sixteen steps, and the four knobs that make the sound: the oscillator's wave, the filter's cutoff and
resonance, the envelope's decay, and how much the accent pushes through. It is not a sampler and not a workstation; it is
the box that plays one line well.

- **Engine** (`rust/crates/acid-core`, no UI, no allocation in the audio callback): saw, square, sine and triangle
  oscillator; a resonant ladder low-pass filter; an envelope with accent-modulated decay; glide/slide; a soft limiter and
  a drive stage; 40–300 BPM with swing.
- **Panel** (`rust/crates/acid-app`, egui/eframe): painted to look like a 303 — silkscreen legends, lit keys, a chasing
  step row — with the seven tone knobs, a performance row, and a transport row.

## Features

**The pattern.** Sixteen steps, each with a note (C…C over several octaves), an octave, a velocity, and two switches:
accent (a brighter, louder hit) and slide (the note glides into the next instead of retriggering). Sixty-four patterns sit
in eight banks of eight; the pattern readout in the top strip names the one playing (`A1`…`H8`).

**The row.** Seven tone knobs in the order the panel draws them — waveform, tuning, cutoff, resonance, envelope mod,
decay, accent — plus the volume knob in the top-left corner. Every one of them can be taught to a MIDI controller.

**The performance row.** Transpose the line an octave either way, shuffle the clock, fill the pattern with a randomised
acid line (density decides how many steps, scale decides which notes), and pick the scale from a catalogue rooted on C.

**Transport.** Run/stop (with the step LED chasing the audio, not the wall clock), tempo in 40–300 BPM, swing, and the
MIDI LEARN key.

**MIDI.** Pick an input port from the panel; notes with velocity play the voice and — in step mode or with shift held —
write into the pattern. Every control can be taught a continuous controller: the seven knobs, volume, tempo, the pattern
selector and the shuffle. Dial-like controls use **pickup** — a fader parked somewhere else has to reach the knob before
it may turn it, so a song load never slams the patch. The pattern selector additionally waits for the bar: cycling through
patterns with a knob is visible on the readout at once and heard at the end of the pattern, and an external MIDI clock can
drive the tempo.

**Keeping your work.** The session autosaves to `state.json` (atomically, through a temporary file), and named songs live
in the same data directory; the SONGS card saves, loads, renames and deletes them.

**Help.** Everything hoverable explains itself, and a help card lists the gestures and the whole MIDI map.

## Installation

**What you need:** a Rust toolchain new enough for edition 2024 (1.85 or later; developed on 1.99), and on Linux the ALSA
development headers, because the audio and MIDI crates talk to it directly:

```bash
sudo apt install build-essential pkg-config libasound2-dev     # Debian / Ubuntu
sudo pacman -S base-devel alsa-lib                              # Arch
```

**Build and run** from the repository root:

```bash
cd rust
cargo run --release -p acid-app --bin takka-acid
```

The binary is `rust/target/release/takka-acid` afterwards, so you can run it without cargo.

**Where it keeps things.** The session and your songs live in `$XDG_DATA_HOME/takka-acid`, or `~/.local/share/takka-acid`
when `XDG_DATA_HOME` is unset. Point `TAKKA_ACID_DATA_DIR` somewhere else to keep a separate rig or to test.

**Two flags worth knowing.** `takka-acid --midi-map` prints what the app makes of every MIDI note number without opening a
window or an audio device — the fastest way to answer "is it my keyboard or the app?". And the app picks its MIDI input
from the panel's port picker, remembering the choice.

**Working on it.** Every change is gated by:

```bash
cd rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

`EAR_TESTING.md` explains the other half of testing: the suite pins RMS, peak, brightness and timing, but whether it sounds
like acid is a judgement only your ears make — including how to render the voice to WAVs and A/B them.

## Where it started: `Prompt.txt`

The project began with a five-line brief, kept verbatim in [`Prompt.txt`](Prompt.txt). It asked for a plan, in markdown, for
a Roland TB-303 clone in Rust called *takka-acid-rust*, broken into logical major steps with a file per step — ten or more
of them — with the code under `rust/` and the plans under `plans/`, and "a great user interface … a black and yellow version
of the traditional TB-303 design". Everything else in this repository — every feature, every plan, every test — grew out of
that one prompt.

## Plans: the evolution of the product

`plans/` is not documentation written after the fact: it is the record of how the instrument was built. Each round of work
has a numbered plan, from `00-main-plan.md` (the original breakdown) through the engine, the panel, the sequencer, MIDI,
persistence, and the polish rounds, up to `32-learn-polish-and-pattern-controller.md`.

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
  crates/acid-core/    the engine: oscillator, filter, envelope, sequencer, patterns, bank
  crates/acid-app/     the panel, shared state, MIDI, persistence, the binary
  crates/acid-app/tests/   the live tests: they drive real egui frames
AGENTS.md              the conventions the codebase is held to
docs/                  design notes and review material
```

## Licence

MIT.

## Demo songs and saving

Five demos ship inside the binary — **Slow Acid**, **Hard Techno Acid**, **Blues**, **Rock** and **Funk** — each a patch
and a bank of eight variations of its line. They are read-only: the SONGS card lists them in a `DEMO SONGS` section of
their own, and picking one loads it as a *new* song called "New song", unsaved, which you rename and save under your own
name. Your own songs sit below them under `MY SONGS`, and replacing a session you have not saved takes a second click.

Saving is manual by default: the `SAVE` key writes the session (and the song file, when the session came from one), and the
lamp beside the top strip says `SAVED` or `UNSAVED`. The `AUTO` key beside `TIPS` turns the old autosave back on for those
who want it; the choice is kept in the session file. A first run opens on the Slow Acid demo, named "New song", with the
lamp already saying what needs saying.
