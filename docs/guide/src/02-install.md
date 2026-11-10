# Installing and running

## What you need

On Linux, with a Rust toolchain:

| Needed | Why |
|---|---|
| Rust 1.85 or newer (the workspace is edition 2024) | builds it |
| `build-essential` and `pkg-config` | linking |
| `libasound2-dev` — the ALSA headers | the audio and MIDI libraries talk to ALSA directly |
| A Wayland or X11 session | the panel needs a surface to draw on |
| A working OpenGL 3.3+ or GLES driver | the panel is drawn on the GPU |
| `xdg-desktop-portal` **and a backend** (`-hyprland`, `-gtk`, …), plus a D-Bus session | `IMPORT` and `EXPORT` are the only file dialogs in the app, and they go through the portal. Without it those two buttons have no dialog to open; everything else works |
| PipeWire or ALSA for sound out | it has to play somewhere |

Only if you are working on **this book**, not on the instrument:

| Needed | Why |
|---|---|
| `mdbook` 0.4 or newer (this book is built with 0.5.4) | building the guide |
| `grim`, `slurp` and ImageMagick | regenerating its screenshots |

On Debian or Ubuntu the first three are one command; on Arch, another:

```bash
sudo apt install build-essential pkg-config libasound2-dev
sudo pacman -S base-devel alsa-lib
```

## Build and run

From the repository root:

```bash
cd rust
cargo run --release -p acid-app --bin takka-acid
```

The binary lands at `rust/target/release/takka-acid` afterwards, so you can start it without cargo.

## Where it keeps things

The session and your songs live in `$XDG_DATA_HOME/takka-acid`, or `~/.local/share/takka-acid` when `XDG_DATA_HOME` is
not set. Two things are in there:

- `state.json` — the session: the pattern on screen, the patch, the tempo, the switches in settings, your MIDI choice.
- `songs/` — one file per song you have named.

Point `TAKKA_ACID_DATA_DIR` at another directory to keep a separate rig, or to experiment without touching the real one:

```bash
TAKKA_ACID_DATA_DIR=/tmp/acid-test cargo run --release -p acid-app --bin takka-acid
```

Saving is **manual by default**: nothing is written until you press `SAVE`. Autosave is a switch in the settings card
for players who want it, and [`12-songs-and-demos`](12-songs-and-demos.md) explains exactly what a save does to a name.

## One flag worth knowing

```bash
takka-acid --midi-map
```

`--midi-map` prints a table of every MIDI note number, the note the app makes of it, and the frequency it plays, then
exits — no window, no audio device. When a controller plays the wrong octave, or a key seems dead, this is the fastest
way to find out whether the app or the keyboard is wrong. [`17-trouble`](17-trouble.md) comes back to it.
