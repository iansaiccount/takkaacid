# Plan 07 — the drum face's keys, underlines and tooltips

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, each commit reviewed by a
> dispatched subagent before it lands (this repo's rule). This plan is one of two written together; plan 06 is the other.

**Goal:** the drum face's keys can be learned by eye — the letter that presses a key is underlined on it — the filter is
reachable from the keyboard, and every control on the face says what it does on hover. The guide, the in-app help card and the
pictures agree with the face.

**Spec:** none. The player's words: *"Make sure we update the documentation and the tooltips please - we are missing tooltips
for everything. Underline the H on the hit button, and lets me the L, B and H keys work for the filter too, underlining those
keys on the filter. That will give us a much nicer keyboard experience on the drums."* One correction follows from plan 05 and
is in **Rulings**: `H` cannot be both `HIT` and `HP`, since the player kept `h` for the hit, so the high side takes `P`.

## What is wrong today

- **No mnemonic is printed.** `HIT` is pressed by `h` (plan 05) and the filter's three sides have no key at all; nothing on the
  face says either. A player learns the keys from the guide or not at all.
- **The filter is pointer-only.** `drum_panel::face_key` answers the voice walk, the step walk (plan 06), the write and the
  hit; the filter's `LP | BP | HP` segments are clicked. `L`, `B` and `P` are free letters on a drum face (they are not among
  the 303's notes), so they cost nothing to take.
- **Nine controls on the face have no tip.** The seven dials, the filter key and `HIT` print a caption and nothing else.
  `SHUFFLE` already has one (`Control::Shuffle`), and the 303's knobs, the panel's keys, the strip and the cards all have
  theirs — which is why the gap is invisible: `ui::tips::Control` is a positional catalogue, and a control with no variant
  simply cannot have a tip.
- **The help card lists only the 303's keys.** `ui/help.rs`'s `KEYBOARD` block is fifteen lines about the acid face; the drum
  face's walk (03), hit (05), step write (06) and filter letters (this round) are absent, so the card is now several keys
  behind the face it describes. (It is the wishlist's "The help card knows only the 303's keys", which this round retires.)

## The design

- **The filter takes `L`, `B` and `P`.** `FaceKey::Filter(FilterKind)` for `l`, `b` and `p` with no shift, applied through
  one writer `drum_panel::set_filter(state, shared, kind)` that the segment click also calls — silent, like the click, and
  **carrying the click's `!choosing` guard**: a `LEARN` in progress must not turn a voice under a hand that meant to teach the
  controller, which is what `tests/drum_live.rs::a_learn_in_progress_turns_no_drum_dial` exists to prevent.
- **The mnemonic is underlined on the control it presses.** `HIT` underlines `H`; the filter segments underline `L`, `B` and
  `P`. One helper, `ui::text::with_mnemonic(painter, rect, label, font, colour, letter, ink)`, lays the label out with
  `layout_no_wrap`, finds that letter's glyph, paints the text and strokes a `1 * scale` line under the glyph in `ink`.
  `transport::transport_button` gains a `mnemonic: Option<char>` parameter — its four callers pass `None` except `HIT` — and
  the filter segments, which `drum_panel` paints itself, use the same helper. The underline is an egui shape under ASCII text
  from the bundled font — no glyph, the rule AGENTS.md keeps.
- **Nine tips, one catalogue.** `ui::tips::Control` gains `DrumAttack`, `DrumDecay`, `DrumSustain`, `DrumRelease`,
  `DrumCutoff`, `DrumResonance`, `DrumTune`, `DrumFilter` and `DrumHit`, **appended** so no existing position in `TIPS`
  moves (`ALL` grows 40 → 49). Each needs a `title` that is **not already taken** — `TIPS` already holds `CUT OFF`,
  `RESONANCE` and `DECAY` for the 303's knobs, and `the_titles_are_distinct_so_two_controls_cannot_be_confused` walks for
  duplicates — so the dials' titles read `VOICE …`. The two mappings that cross the catalogues live beside `for_knob`, as it
  does: `Control::for_dial(Dial) -> Control` and `Control::for_drum_key(DrumKey) -> Control`, both exhaustive. `draw_dials`
  calls `tips::hover` on each dial, on each filter segment and on `HIT` — the same one-line call `SHUFFLE` already makes. The
  grid's 256 cells and its row labels get none: a tip per cell would fight the click that is the cell's whole job, and the
  labels' gestures are the guide's paragraph.
- **The help card gains the drum face's keys inside its line budget.** `help_content_height` is `16 + 4·24.7 + 13·L` design
  px against a card of `662` (`ui/help.rs`, `theme::DESIGN_SIZE`), and today's `L = 38` leaves **four lines** of slack
  (`ui/help.rs::the_card_fits_inside_the_panel_at_both_scale_extremes` is the test that says so). So the drum answers are
  **folded into lines that already exist** where they can be — `left / right` becomes "move the selection, on either face",
  `shift+up / down` gains "on a drum device, the voice above or below", `enter` gains "on a drum device, write the selected
  voice" — and only **two** lines are added: `h: strike the selected voice (HIT)` and `l / b / p: the filter's low, band,
  high`. Five new lines would not fit, and that test would fail at every scale.
  | title | control | `what` | `shortcut` |
  |---|---|---|---|
  | `VOICE ATTACK` | `ATTACK` | "How fast the voice reaches full level after a hit." | — |
  | `VOICE DECAY` | `DECAY` | "How fast a hit falls from its peak towards SUSTAIN." | — |
  | `VOICE SUSTAIN` | `SUSTAIN` | "The level the voice holds while its gate is open. A drum arrives at 0, so a hit falls unless you raise it." | — |
  | `VOICE RELEASE` | `RELEASE` | "How long the voice takes to fade once the gate closes." | — |
  | `VOICE CUT OFF` | `CUT OFF` | "The filter's corner, in the 303's own knob positions." | — |
  | `VOICE RESONANCE` | `RESONANCE` | "How much the corner is emphasised." | — |
  | `VOICE TUNE` | `TUNE` | "The voice's pitch, in semitones either way." | — |
  | `VOICE FILTER` | `LP / BP / HP` | "Which side of the voice's filter the sample passes: low, band or high." | "L / B / P: pick a side" |
  | `HIT` | `HIT` | "Strikes the selected voice, stopped or running. Writing a step is `enter`." | "H" |

## Tests that make it safe

- `tests/drum_live.rs::l_b_and_p_choose_the_filter_side` — on a drum face, `l` sets the selected voice's filter to `Low`, `b`
  to `Band`, `p` to `High`; no audition is requested; the same voice's filter set by a segment click matches; and with
  `panel.learn == learn::Step::Choosing` a letter writes nothing, the same guard the click keeps.
- `tests/drum_live.rs::the_hit_and_the_filter_carry_underlines` — the painted shapes include a line under `H`'s glyph on the
  `HIT` key and under `L`, `B` and `P` on the three segments, drawn in the palette's ink (so a second scheme repaints them),
  and each line sits inside the glyph's own box — a line anywhere in the key would not be the mnemonic.
- `tests/drum_live.rs::every_drum_control_explains_itself` — hovering each of the seven dials, each filter segment and `HIT`
  paints its tip's text (the `tests/tips_live.rs` pattern: one frame per second, so egui's hover delay passes, with
  `tooltips` on).
- `ui::tips`'s own tests — `Control::ALL.len() == 49` and every entry has a `Tip` (the existing walk), the existing
  `the_titles_are_distinct_so_two_controls_cannot_be_confused` still passes with the nine new titles, and
  `Control::for_dial` is answered for every `Dial` reached through `Knob::ALL.map(Dial::for_knob)`,
  `Control::for_drum_key` for every `DrumKey`.
- `tests/help_live.rs` — the card paints the two new lines and the folded ones, **and** `ui::help.rs`'s own
  `the_card_fits_inside_the_panel_at_both_scale_extremes` still passes: the line budget is the constraint, not a wish.
- `tests/keys.rs`'s ownership test gains `l`/`b`/`p`: taken on a drum face, left in the frame on an acid one.
- `drum_panel::tests::the_face_keys_are_the_walk_and_the_hit` gains the filter arms, and its `face_key(Key::B, false)` line
  becomes `Some(FaceKey::Filter(FilterKind::Band))` — the assertion plan 06 leaves saying `None`.

Mutations: `H` on the filter instead of `P` (the collision returns); a filter key that does not write (a silent no-op); an
underline drawn in a literal colour (the palette test); a tip missing from `TIPS` (the `ALL` walk); `for_dial` answering the
wrong control for one dial.

## Tasks

### Task 1 — the filter's letters

**Files:** `rust/crates/acid-app/src/ui/drum_panel.rs` (`FaceKey::Filter`, `set_filter`, the segment click),
`rust/crates/acid-app/src/ui/dispatch.rs` (the `Drums` arm), `tests/drum_live.rs`, `tests/keys.rs`.

**Interfaces:** produces `pub fn set_filter(state: &mut PanelState, shared: &SharedState, kind: FilterKind)`,
`FaceKey::Filter(FilterKind)`.

1. Write `l_b_and_p_choose_the_filter_side` (including the learn case); run — expected FAIL.
2. Implement `set_filter` with the click's `!choosing` guard inside it, the filter arms and the dispatch arm; the segment
   click calls `set_filter` so there is one writer and one guard; add the letters to `tests/keys.rs`. Run — expected PASS.
3. Review, commit.

### Task 2 — the underlines

**Files:** `rust/crates/acid-app/src/ui/text.rs` (`with_mnemonic`), `rust/crates/acid-app/src/ui/transport.rs`
(`transport_button`'s new parameter and its three other callers), `rust/crates/acid-app/src/ui/drum_panel.rs` (the hit key and
the segments), `tests/drum_live.rs`.

**Interfaces:** produces `pub fn with_mnemonic(painter: &egui::Painter, rect: Rect, label: &str, font: egui::FontId,
colour: egui::Color32, letter: char, ink: egui::Color32)`; `transport_button(ui, id, rect, label, lit, p, mnemonic:
Option<char>) -> Response`.

1. Write `the_hit_and_the_filter_carry_underlines`; run — expected FAIL.
2. Implement the helper, the parameter and the two call sites; run — expected PASS, and the face still paints only palette
   colours.
3. Review, commit.

### Task 3 — the nine tips

**Files:** `rust/crates/acid-app/src/ui/tips.rs` (`Control`, `ALL`, `TIPS`, `for_dial`, `for_drum_key`),
`rust/crates/acid-app/src/ui/drum_panel.rs` (the `hover` calls), `tests/drum_live.rs`, `tests/tips_live.rs` if a shared
pattern moves.

1. Write `every_drum_control_explains_itself` and the two mapping tests; run — expected FAIL.
2. Add the variants, the copy above, the mappings and the `hover` calls; run — expected PASS.
3. Review, commit.

### Task 4 — the help card

**Files:** `rust/crates/acid-app/src/ui/help.rs` (the `KEYBOARD` lines and the height), `tests/help_live.rs`.

1. Fold the drum answers into the existing `left / right`, `shift+up / down` and `enter` lines, and add only the two new
   lines (`h`, `l / b / p`) — four lines of slack exist and five new lines do not fit. Add the lines; let
   `help_content_height` follow; `the_card_fits_inside_the_panel_at_both_scale_extremes` must stay green.
2. A live test asks for the painted lines; run — expected FAIL, then PASS.
3. `help-card.png` is retaken with the rest in Task 5. Review, commit.

### Task 5 — the book and the pictures

**Files:** `docs/guide/src/22-the-drum-machine.md`, `15-keyboard.md`, `20-reference.md`, `docs/guide/check.sh`,
`docs/guide/src/img/drum-face.png`, `help-card.png`, `img/README.md`, `plans/phase 4/00-phase-4.md`,
`plans/future/wishlist.md` (retire "The help card knows only the 303's keys").

1. Page 22: the filter's `L`/`B`/`P`, the underlines, and the tips; its *No per-voice hover help yet* bullet is deleted
   outright — it is now false — and its `HIT` sentence ("with the transport stopped") is amended to match the tip ("stopped or
   running"). Page 15 and page 20: the letters and the drum keys.
2. `check.sh` pins the phrases worth keeping (the filter letters, the underlines' words); the existing `walk it up and down the
   grid` and `strikes the selected voice` pins must survive the rewrites.
3. Retake both pictures at the guide's size: `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` first and
   the window must measure `1960x1340`; record each command in `img/README.md`, and look at each grab: the underlines on
   `HIT` and the filter, and the help card's new lines.
4. List the round in `00-phase-4.md`, retire the wishlist entry; run `./docs/guide/check.sh`.
5. Review, commit.

## Non-goals

Tips on the grid's cells and row labels; a mnemonic on the 303's keys (it has none to press); a settings switch for the
underlines; per-voice controllers and MIDI pads; a tip for the playhead (there is none to hover); the step editor's own keys,
which are plan 06's.

## Rulings, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| The filter's letters are `L`, `B` and `P` | `H` is the hit's key (plan 05, kept by the player), so `HP` cannot take it. `P` is the last letter and reads as the odd one out on the face; the alternative is moving `HIT` off `h`, which the player has twice chosen to keep. |
| One tip for the filter key, not one per segment | "Everything" means the nine controls; three tips for one three-way key would say the same thing three times. If a side needs its own sentence, it is two more `Tip`s and two more `hover` calls. |
| The underline is ink under the glyph, not a font style | It is an egui shape, so it takes the palette and the scale like everything else, and no glyph is involved. |
| The help card is in this round | The player asked for the documentation and the tooltips together, and the card is the list of keys a player reads on the face. It is the round's largest single item; if it must wait, it is Task 4 alone, and the wishlist entry stays. |

## Progress

- [ ] Task 1 —
- [ ] Task 2 —
- [ ] Task 3 —
- [ ] Task 4 —
- [ ] Task 5 —
