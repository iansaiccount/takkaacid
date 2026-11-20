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

- [x] **Task 1 — the filter's letters.** `FaceKey::Filter(FilterKind)` and `drum_panel::filter_cell`'s
      letters: plain `l`, `b`, `p` pick low, band and high, silent, and the segment click now goes through the same
      `set_filter(state, shared, kind)`, which holds the click's `!choosing` guard so there is one writer and one guard (the
      click's own `&& !choosing` is gone). `dispatch`'s `Drums` arm returns `Handled::Selection` for a filter letter — a spent
      press, not a sound — and a card refuses it with the other edits, since a card covers the segments it clicks.
      Tests: `drum_live::l_b_and_p_choose_the_filter_side` (the three letters write their three sides, no audition, the click
      leaves the same filter, and with `learn == Choosing` no letter writes — the kit's name and `dirty` are watched too, so a
      write of the side the voice already has, which would still copy a factory kit, cannot pass), plus the filter keys in
      `tests/keys.rs`'s ownership test (taken on a drum face, left in the frame on an acid one) and
      `a_card_refuses_the_voice_keys` (a refused `b` writes nothing). `drum_panel::tests::the_face_keys_are_the_walk_the_step_and_the_write`
      gained the three filter arms — the plan names this test `…_the_walk_and_the_hit`; `the_face_keys_are_the_walk_the_step_and_the_write`
      is the name it actually has — and its `Key::B` `None` assertion is now `Filter(Band)`, with `Key::A` carrying the
      "other letters are silent" claim. **Mutations: 4 of 4 caught** (`.superpowers/mutate_filter_keys.py`): `H` on the filter
      instead of `P` (named by the live test and the unit test), a filter letter that writes nothing, the dispatch arm that
      reaches no writer, and the `LEARN` guard dropped — each fails a named test.
      **Review: PASS on one Important and one Minor, both actioned, plus two notes taken.** The Important was the two `Handled`
      doc comments that the new return made untrue: `Selection` now says "the face's current edit wrote — a step, or the
      selected voice's filter" and `Refused` names the filter beside the walk, move and write. The Minor was the filter's card
      refusal being unpinned — pinned in `a_card_refuses_the_voice_keys` above. Of the notes: a redundant `paint()` before the
      click in the new test is gone (`click` settles its own frame); the `learn` loop now watches the kit's name and `dirty`;
      and `tests/keys.rs`'s pre-existing "`B` auditions a note" comment was wrong (B is no note on either face) and is corrected
      while in the file. The deferral of the book is deliberate and stale at three sentences — page 22's "the rest of the
      letters stay silent here", page 15's "the letters are silent this phase", and page 20's keyboard table — and **the push
      depends on Task 5**, which owns those pages. Gates: `cargo test --workspace` **893**, clippy `-D warnings`, `fmt --check`.
- [x] **Task 2 — the underlines.** `ui::text::with_mnemonic` paints a centred label and one `1 * scale` line under one glyph,
      in the ink it is given; `transport_button` grew `mnemonic: Option<char>` (its four callers pass `None` except `HIT`, whose
      `H` is marked) and the filter's three segments are painted by the same helper. The line's weight and its drop below the
      text follow the font's own point size against the legend's 13 px: one design pixel at 1.0x for `HIT`, a lighter mark on
      the 11 px segment captions. The letter and the key that presses it are **one table** (`FILTER_KEYS`), `face_key`'s filter
      arm and the printed letter both derived from it, because two matches answering one question is the drift this round is
      about: the first cut took the label's head, and `HP`'s head is `H` — `HIT`'s key. The unit test caught it before the
      picture did, which is why `filter_letter` exists as a mapping rather than a `label().chars().next()`.
      Tests: `drum_live::the_hit_and_the_filter_carry_underlines` (a line under `H` on `HIT` and under `L`/`B`/`P` on the
      three segments, each inside its glyph's own advance box and inside its key, in the palette's ink in **both** schemes) and
      `drum_panel::tests::the_filter_letters_are_the_keys_that_press_them` (the pair, both ways, and that each label prints
      the letter under it). **Mutations: 3 of 3 caught** (`.superpowers/mutate_underlines.py`): a mark in a literal colour, no
      mark at all, and a mark spanning the whole label instead of the glyph. Task 1's four were re-run after the table landed
      and still catch: 4 of 4.
      **Review: BLOCK on one Important and two Minors, all actioned, plus four notes.** The Important was that the test's
      second-scheme half was vacuous — `any line in the frame in the light ink` is satisfied by the tempo stepper's `-`/`+`
      bars, so the literal-colour mutation survived. It now re-runs the per-case glyph-box search against the light frame in
      that scheme's own ink, which is what catches the mutation above (proven by re-running it). Of the Minors: the
      `font.size / 13.0` weight was documented as "one pixel at 1.0x whatever the control", which is false for the 11 px
      segment caption — the doc now says what the arithmetic does, and the picture was checked at the guide's size before the
      claim stood; and the letter↔key pair was a second hand-kept mapping beside `face_key`, so both now derive from
      `FILTER_KEYS` and the walk test's three duplicate assertions are one pointer to the pairing test. Of the notes: the test
      measures the glyph with egui's own fonts, independent of the helper's code but not of its method (recorded — the risk it
      pins is *which* glyph, which is the `HP` bug above); `with_mnemonic` now `debug_assert!`s when a label does not print its
      letter; `text::fit` measures through the new `advance` so there is one spelling; and the test slices the letter with
      `len_utf8` like the helper. The book is still Task 5's — the same three sentences Task 1 recorded.
      Gates: `cargo test --workspace` **895**, clippy `-D warnings`, `fmt --check`.

- [x] **Task 3 — the nine tips.** `ui::tips::Control` gained `DrumAttack` … `DrumHit`, **appended** (`ALL` 40 → 49, `TIPS`
      49, the nine entries last), with the plan's copy word for word — the table's `` `enter` `` is rendered as `enter`, because
      a tooltip is plain text and backticks would print. Titles read `VOICE ATTACK` … `VOICE TUNE` and `HIT`, which is what
      keeps them out of the 303's way: `CUT OFF`, `RESONANCE` and `DECAY` were already taken, and
      `the_titles_are_distinct_so_two_controls_cannot_be_confused` still passes with all nine. `Control::for_dial` and
      `Control::for_drum_key` sit beside `for_knob`, both exhaustive; `SHUFFLE` answers `Control::Shuffle` through
      `for_drum_key`, so the device's swing keeps the one set of words it had rather than gaining a second. `draw_dials` calls
      `tips::hover` on each dial's own `knob::knob` response, on each of the filter's three segments (one tip, the plan's
      ruling) and on `HIT`; the grid's cells and its labels still get none.
      Tests: `drum_live::every_drum_control_explains_itself` — a frame per second with the pointer settled, so egui's delay
      passes, for all seven dials, all three segments and `HIT` — and `tips::tests::the_drum_face_has_its_own_help_for_every_control`
      (`ALL.len() == 49`, the appended block in order, the seven dial titles reached through `Knob::ALL.map(Dial::for_knob)`,
      the key titles, and every one of the nine reached by some drum control). **Mutations: 6 of 6 caught**
      (`.superpowers/mutate_tips.py`): `DrumHit` wearing the filter's tip, `for_dial` answering the wrong control, `for_drum_key`
      answering the wrong control, a drum title colliding with a 303 title, the filter key's hover call removed, and a dial's
      hover ignoring the `TOOLTIPS` switch.
      **Review: OK, one Minor actioned and four notes.** The Minor was that nothing exercised the `state.tooltips` argument at
      the four new call sites — passing a literal `true` survived the suite — so the live test now also hovers the `ATTACK` dial
      with the switch off and asserts the tip never arrives, and that mutation is in the script above (6 of 6 is one more than
      the five the plan named). Of the notes: the live test is circular about *which* control a dial wears (`for_dial` both
      paints and is asserted), which is recorded rather than fixed — the mapping is pinned non-circularly by literal titles in
      `tips.rs` and literal dial labels in `drum_panel`'s own test, and what the live test uniquely proves is that the tip is
      painted on that rect; `ui::tips` now names `ui::drum_panel`'s types while `drum_panel` names `tips`, a mutual module
      dependency the plan mandates and the note records; and the hover loop is one local closure rather than a fourth copy of
      `tips_live`'s. **The book is now one sentence false:** page 22's *No per-voice hover help yet* bullet is untrue as of
      this commit — `check.sh` does not pin it, so the gate stays green, but the push depends on Task 5, which deletes it.
      Gates: `cargo test --workspace` **897**, clippy `-D warnings`, `fmt --check`.

- [ ] Task 4 —
- [ ] Task 5 —
