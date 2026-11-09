# AGENTS.md — takka-acid

A TB-303-style acid bass synth: a Rust audio engine and an egui/eframe panel. Two crates in one workspace:

- `rust/crates/acid-core` — the engine. Oscillator, filter, VCA, sequencer, pattern and bank types. No UI, no egui.
- `rust/crates/acid-app` — the panel, the seam between the two threads (`state::SharedState`), MIDI in, and persistence.
  Panel behaviour is tested by driving `draw_panel` through real egui frames in `rust/crates/acid-app/tests/*_live.rs`.

`plans/` holds one plan per round of work, numbered; `EAR_TESTING.md` is how to judge the sound by ear.

## Gates

Every change ends here, all three green:

```bash
cd rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
```

Run it with `cargo run -p acid-app --bin takka-acid`, and `--midi-map` to print what the app makes of every note
number without opening an audio device or a window.

## Planning

Write the plan into `plans/NN-topic.md` before touching code, in the shape plan 30 and plan 31 use: what is wrong today,
the design, the tests that make it safe, a numbered task list, and non-goals. Progress one task at a time and keep a
`## Progress` section at the end of the plan current — a task, the test or mutation check that proves it, and any ruling
you made with what it costs if wrong. A later session resumes from that section and `git log`, not from memory.

`.superpowers/` is git-ignored scratch for review ledgers; nothing in it is part of the project.

## Code conventions

**One catalogue per kind of control.** A control's identity — its slot, caption, learn target and the atomic it
drives — lives in one module: `crate::knobs::Knob` for the seven tone knobs, `ui::scale::Scale`, `ui::switches::Switch`,
`ui::tips::Control`, `midi::cc::CcTarget` for what a controller can drive. The one mapping that crosses two of them, a
knob's hover help, is `ui::tips::Control::for_knob`, kept with the tip table so the catalogue names no view type. Derive
orders and lengths from those catalogues (`Knob::ALL.len()`, `LEARNABLE` built from `Knob::ALL`) instead of writing a second
positional list; the two bugs this repo has already paid for were both a hand-kept second list.

**No `_` arm on a catalogue enum.** A `match` over one of those enums is written out in full, so adding a variant fails to
compile where a decision is needed. `_` is for genuinely open input (a `u8` controller number), never for a catalogue.

**One mapping per question.** If two functions answer "which atomic does this knob drive" or "which slot does this control
teach", one of them is a bug waiting for a reorder. Put the mapping in the module that owns the type it produces.

**Test behaviour through the panel.** A panel feature gets a test in `rust/crates/acid-app/tests/*_live.rs` that pushes
egui `RawInput` frames through `draw_panel` and asserts on the `SharedState` or the `PanelState` — not on a private helper.
A widget's `egui::Id` is part of its behaviour: a drag in progress is routed by id, so renaming one silently breaks it.

**The save file is a contract.** `persist::SavedState` field names and order are what older builds read; a change to them
is a `SAVE_VERSION` bump, and `tests/persist.rs` pins the bytes. Panel state that `main` owns travels through
`SavedState`, not through a side channel.

**Dialogs live in `main`; paths live in `persist`.** A file dialog is `rfd` and belongs in the `main` binary, where it is
allowed to block; the work it leads to is a plain path function in `persist` (`export_song`, `import_song`, `save_song`),
so it can be tested without a display. `import_song` never overwrites: it lands beside a taken name. `save_target` is the
only place that decides whether a save writes in place or makes a new file, so the rule cannot drift between the button and
the handler.

**A message is a notice, not a dialog.** `PanelState::notice` says something on the status line for a few seconds and
clears itself; nothing the panel says should ever need dismissing, and nothing it says should die with the card that
caused it.

**Two threads, two rules.** The panel writes atomics on `SharedState`; the audio callback reads them and allocates
nothing. Never hold a lock across the audio path, and never read the engine's pattern from the UI thread without going
through the `ArcSwap`.

**No font glyphs.** Everything on the face is an egui shape or ASCII text from the bundled font. A symbol drawn with a
character from a system font renders as a box on some machines.
