# Hover Tooltips for the Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task.

**Goal:** hovering a knob or a button explains what it does and which key does the same thing, and a small button in the
top strip turns the help off for players who know the panel.

**Architecture:** one `ui::tips` module owns the text — a `Tip` (title, description, shortcut) per control, looked up by a
stable id — and a single `tips::hover` call attaches it to a widget's `Response`. Panels keep drawing themselves; they ask
the module for the text instead of carrying prose. The on/off state lives in `PanelState` and is saved with the session.

**Tech Stack:** Rust 2024, egui/eframe 0.37 (native hover tooltips), serde.

## Scope

- **In:** the seven knobs, the eight switches (BANK · PATTERN · MODE · CLEAR · SAVE · SONGS · RESET · STEP), the transport
  keys, the SAVE lamp, the status line (MIDI/audio pickers), the SONGS card's buttons, and the new TOOLTIPS toggle itself.
- **Out:** the sixteen step keys and the keyboard gesture map — they are the panel's instrument, not its controls, and a
  tooltip on every key would be noise. Their gestures stay on the F1 card.

## Global Constraints

- Legends stay ASCII; tooltips say "shift+left / right" in the same words the help card uses, so the two never disagree.
- Nothing here may run on the audio thread or allocate per sample; hover text is built only while a control is hovered.
- `TAKKA_TRACE_INPUT`-style opt-in, not opt-out: tooltips are **on** by default and the toggle turns them off.
- Persisted: `SavedState` gains `tooltips: bool` with `#[serde(default = "yes")]`, `SAVE_VERSION` → 4 (a reader that
  predates the field keeps them on).

## Review Focus

1. **A tooltip while dragging a knob** — hovering must not fight the drag; the text appears on hover, and the drag keeps
   working underneath.
2. **The toggle in a saved session** — off stays off across a restart, and the button itself still explains what it does
   (a disabled help must not be unexplainable).
3. **A shortcut that does not exist** — many knobs have no key. Their tip says what they do and omits the shortcut line
   rather than inventing one.
4. **Two controls with the same name** — `CLR`/`CLEAR`, `TAP`/`TAP TEMPO`: the tip must name the control the pointer is
   actually over.

## Tasks

### Task 1: the text, as data

**Files:** create `rust/crates/acid-app/src/ui/tips.rs`; test in the same file.

- [x] `pub struct Tip { pub title: &'static str, pub what: &'static str, pub shortcut: Option<&'static str> }`.
- [x] `pub enum Control { Waveform, Tuning, Cutoff, Resonance, EnvMod, Decay, Accent, Bank, Pattern, Mode, Clear, Save,
      Songs, Reset, Step, Run, Stop, Tap, Help, Tooltips, Status, … }` with `fn tip(self) -> Tip`.
- [x] Tests: every variant returns a non-empty ASCII title and body; a shortcut, where present, is ASCII and matches the
      string the help card uses for the same gesture.
- [x] `pub fn hover(response: Response, enabled: bool, tip: &Tip)` — one place that formats the multi-line text, so all
      controls behave the same.

### Task 2: the switch and its state

**Files:** `rust/crates/acid-app/src/ui/mod.rs` (the top strip), `src/ui/switches.rs` or the transport row, `src/persist.rs`.

- [x] `PanelState::tooltips: bool` (default true); a small `?` button beside the SAVE lamp toggles it, with its own tip.
- [x] `SavedState::tooltips` with `#[serde(default = "default_true")]`; `SAVE_VERSION` 4; round-trip test plus "a v3 file
      has them on".
- [x] Live test: clicking the toggle flips `PanelState::tooltips` and marks the session dirty; a tip is shown when on and
      suppressed when off.

### Task 3: attach the tips

**Files:** `src/ui/mod.rs` (knobs, switches, SAVE lamp), `src/ui/transport.rs`, `src/ui/songs.rs`, `src/main.rs` (status
line).

- [x] Each control calls `tips::hover(response, state.tooltips, &Control::X.tip())`.
- [x] The transport keys and the switches share one helper so the tip follows the button that was actually drawn.
- [x] Final live test: with tooltips on, hovering each knob and each switch produces a response that egui reports as
      having a tooltip (`Response::show_tooltip_text` or the response's `widget_info`), and with them off it does not.

## Result

Shipped as `ui::tips` with 18 controls and an on-by-default `TIPS ON` / `TIPS OFF` key in the top strip, left of the
SAVED lamp. `SAVE_VERSION` is 4; the field is `#[serde(default = "default_true")]`, so a v3 file opens with the hints on.

Where the build differs from the plan above, and why:

- **A text key, not a `?` glyph.** The house rule is that no control face may be a font glyph; `TIPS ON` / `TIPS OFF`
  also says what it does when the hints are already off.
- **`Control` has 18 variants, not 23.** BANK, PATTERN and MODE are engravings painted with no interaction, so a tip for
  them could never be hovered. HELP and TAP are not buttons either (F1 / shift+? open the card; tempo is tapped by
  dragging it). Every control a pointer can land on is covered: 7 knobs, CLEAR, SAVE, SONGS, RESET, STEP, TIPS, RUN,
  STOP, TEMPO -, TEMPO +, LEARN.
- **`hover(&Response, &PanelState, Control)`, with `text_for` split out.** An egui tooltip is a floating area, so a test
  cannot read it back. `text_for` is the decision the tests pin (hint vs. silence); the live test covers the key that
  flips it, and `switch_key` is the one place a button's response meets its tip.
- **The final hover test from Task 3 was not written** for the same reason: egui does not expose a tooltip's text to a
  headless run. What is tested instead: the table matches the panel order entry for entry (`tip()` is an exhaustive
  `match`, so no control can reach a player without text), the text is ASCII and a whole sentence, the shortcut lines
  are the ones the help card shows, the flag silences `text_for`, the key flips it both ways, and flipping it dirties
  the session so the choice survives a restart.
