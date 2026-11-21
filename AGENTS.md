# AGENTS.md — takka-acid

A TB-303-style acid bass synth: a Rust audio engine and an egui/eframe panel. Two crates in one workspace:

- `rust/crates/acid-core` — the engine. Oscillator, filter, VCA, sequencer, pattern and bank types. No UI, no egui.
- `rust/crates/acid-app` — the panel, the seam between the two threads (`state::SharedState`), MIDI in, and persistence.
  Panel behaviour is tested by driving `draw_panel` through real egui frames in `rust/crates/acid-app/tests/*_live.rs`.

`plans/` holds one plan per round of work, numbered, and one directory per phase: `phase 1/` is the finished rounds,
`phase 4/` is the round in flight, `future/wishlist.md` is where an idea goes before it earns a round. Each phase directory
opens with a `00-` overview that says what the phase is and which plan to take next. `EAR_TESTING.md` is how to judge the
sound by ear.

## Gates

Every change ends here, all three green, and the book's own check with them when the change touches it:

```bash
cd rust
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check

./docs/guide/check.sh   # from the repo root, when the guide moved
```

Run it with `cargo run -p acid-app --bin takka-acid`, and `--midi-map` to print what the app makes of every note
number without opening an audio device or a window.

## Documentation

The book is `docs/guide`: an mdBook whose pages are `src/NN-topic.md`, numbered in reading order, with the table of contents in
`src/SUMMARY.md` and the screenshots in `src/img/`. It is the only documentation — the README summarises, the plans record
decisions, and neither is a substitute for the page a player reads.

**Functionality is documented before it is pushed.** No push leaves the book describing behaviour the app no longer has. A
change is finished when the page that describes it says what it now does — in the change's own commit where the change is a
feature, and in any case in the same push, never in a follow-up branch nobody pairs with the work. A guide that contradicts
the panel is a bug, and a stale screenshot is a contradiction the eye believes. If a change is invisible to the guide, say why
in the plan's `## Progress` — a new internal invariant, a refactor with no behaviour, a fix to something the book never
claimed.

Editing the book:

- **A new page** is `src/NN-topic.md` with the next number, a line in `src/SUMMARY.md` under the part it belongs to, and a
  cross-reference from the page a reader would arrive from — for example `` [`19-a-devices-card`](19-a-devices-card.md) ``.
- **Changed behaviour** edits the page in place. Prose names controls as the face spells them, in backticks: `COLOUR`,
  `MUTE OFF`, `CHANNEL`. A claim worth pinning is added to the fact list in `docs/guide/check.sh`, which fails when the guide
  stops saying it — that list is why a removed feature cannot leave a sentence behind.
- **A screenshot** goes through `docs/guide/src/img/README.md`, which records per image the state and the command that took
  it. A new picture gets a row there; a retaken one gets its row updated. Both files travel with the code that changed them.

Taking screenshots — no display, real app, the design size at 2×:

```bash
cd rust && cargo build -p acid-app --bin takka-acid
Xvfb :86 -screen 0 2000x1400x24 &
DISPLAY=:86 WINIT_X11_SCALE_FACTOR=2 LIBGL_ALWAYS_SOFTWARE=1 WAYLAND_DISPLAY= \
  TAKKA_ACID_DATA_DIR=/tmp/takka-guide ./target/debug/takka-acid &
```

`TAKKA_ACID_DATA_DIR` must be a fresh directory so the picture shows a first run and nothing of the photographer's own.
The window is 1960 × 1340 (980 × 670 logical, doubled): `xdotool search --name takka` finds it, `xdotool mousemove X Y` and
`xdotool click 1` drive it with X and Y **doubled** from logical coordinates, and `import -window <id> NAME.png` grabs it.

**Check the display before you trust a grab.** On this machine something pre-starts `Xvfb` on display numbers at **1280 × 800**,
and a request for a bigger screen on a taken display fails quietly ("Server is already active") while the app and the grab both
use the small one — a whole round's screenshots were 1280 × 800 for that reason. So: pick a display, run
`DISPLAY=:N xdotool getdisplaygeometry`, and only launch the app if it says `2000 1400` or more; `xdotool search --name takka`
followed by `xdotool getwindowgeometry` must then say `1960x1340`. If it does not, the picture is the wrong size and no amount
of cropping will fix it.
A crop is a second grab with `-crop`, never a cut-out of a finished image. Rect coordinates come from the layout
(`ui/layout.rs`, `ui/strip.rs`, `ui/device_card.rs`), not from reading a screenshot; and park the pointer away from the panel
before a grab, or a hover tooltip lands in the book.

Then `./docs/guide/check.sh`, which proves every page and every image a page names exists and every pinned fact is still said.



## Review at the pull request

**The branch is reviewed, not every commit.** A task's last step is: run the gates, keep the task's `## Progress` line current,
and commit. The review happens once, on the branch, when it is ready for a pull request — a subagent reads the whole diff against
the plans that produced it, and what it finds is fixed on the branch before the PR merges. A change the player asks to have
reviewed is reviewed when they ask, wherever it sits.

**The reviewer is a subagent.** Dispatch one as a separate agent that did not write the code and has not seen the session that
did. The executor reviewing its own branch is not a review: it reads what it meant to write, and the questions below are exactly
the ones an author cannot answer about their own work. There is no fallback to self-review — if the harness offers no subagent
tool, say so and stop before opening the PR rather than pretend the branch was gated: record the blocked review in the plan's
`## Progress` and take it to the human partner, because an unreachable reviewer is a blocked round, not a licence to review your
own work.

Give it the plans' **Files** and **Interfaces** blocks and the branch's diff against the base, not the session's reasoning, so it
judges the change against what was claimed. It reports; it does not edit — a round has one writer, and the executor applies what
the review found, so that what merges is what was read. It answers:

- **Does the branch do what the plans said, and only that?** Extra work smuggled into a branch is a finding.
- **Does it break a convention in this file?** A second hand-kept list, a `_` arm on a catalogue enum, a font glyph, a lock or an
  allocation on the audio path, a change to the save file, an `egui::Id` renamed.
- **What does it leave untested, and is that deliberate?** A missing test is a finding; a deferred one must be in the plan's
  Progress with the reason.

**Act on the findings yourself, then carry on.** A review is not a question for the player: the executor fixes what it found,
records it, and merges once the gates are green again. Stop and ask only when a finding needs a decision that is the player's and
not the code's: a face they will see, a word they will read, a feature that is not this round's, or a finding whose two fixes
cost materially different things. Everything else — a duplicated mapping, a missing test, a doc that lies, a `_` arm, a gap in a
catalogue — is the executor's to settle. A subagent that could not be reached, timed out, or answered about the wrong diff is a
review that did not happen: re-dispatch it, and do not open the PR on the strength of the gates alone.

A review nobody acted on is worse than no review, because it looks like one: the plans' `## Progress` records what the branch
review found and what was done about it, including a finding the executor disagrees with, which is written down with the reason
it stood rather than dropped.

## Planning

Write the plan into `plans/<the phase in flight>/NN-topic.md` before touching code, in the shape plan 30 and plan 31 use: what
is wrong today, the design, the tests that make it safe, a numbered task list, and non-goals. Progress one task at a time and
keep a `## Progress` section at the end of the plan current — a task, the test or mutation check that proves it, what the review found,
and any ruling
you made with what it costs if wrong. A later session resumes from that section and `git log`, not from memory.

`.superpowers/` is git-ignored scratch for review ledgers; nothing in it is part of the project.

## Code conventions

**One catalogue per kind of control.** A control's identity — its slot, caption, learn target and the atomic it
drives — lives in one module: `crate::knobs::Knob` for the seven tone knobs, `ui::scale::Scale`, `ui::switches::Switch`,
`ui::tips::Control`, `midi::cc::CcTarget` for what a controller can drive, and `device::DeviceKind` for the kinds the
picker offers and the strip's `+` greys at. The one mapping that crosses two of them, a knob's hover help, is
`ui::tips::Control::for_knob`, kept with the tip table so the catalogue names no view type. Derive orders and lengths
from those catalogues (`Knob::ALL.len()`, `LEARNABLE` built from `Knob::ALL`) instead of writing a second positional
list; the two bugs this repo has already paid for were both a hand-kept second list.

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

**Three threads, two rules.** The panel — and the MIDI handlers, on midir's thread — write atomics on `SharedState`; the
audio callback reads them and allocates nothing. Never hold a lock across the audio path, and never read the engine's
pattern from the UI thread without going through the `ArcSwap`. `device::BusState` is the one thing both readers share:
`Rack::publish` writes it, the mixer loads it once per block and each port's `Router` loads it once per message, and
neither may lock or allocate around it.

**No font glyphs.** Everything on the face is an egui shape or ASCII text from the bundled font. A symbol drawn with a
character from a system font renders as a box on some machines.

One exception, and one only: `…` (U+2026) marks a line or a device name that was cut, because `...` costs three characters
of a label that has none to spare. It comes from the bundled font, and `ui::text::fit` plus `settings::shorten` are its only
users.
