# Plan 44 — A device's colour, and the pattern clipboard

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). Steps use checkbox (`- [ ]`) tracking;
> each task ends in a green, reviewed commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md`. This round runs it **inline** — the executor reads the
> staged diff and answers AGENTS.md's three questions — because `pi-subagents`' only reachable model returned `ECONNREFUSED`
> during plan 43 and the `claude-code` agent wants an interactive session. The whole-round review at the end is the same pass,
> run a second time over the seams. If a fresh reviewer becomes reachable, use it instead.

**Goal:** a device can be given a colour scheme of its own — the panel face inverts to amber when the device on the panel asks
for it, so four machines are told apart at a glance — and the pattern on the panel can be copied and pasted, between patterns
in one device, between devices, and between two windows of the app.

**Architecture:** two independent halves in one round, because both are "know which machine you are holding".

- **Colour** is a palette value instead of palette consts: `ui::theme` grows a `Palette` and one const per scheme; the face
  painters take one, the four cards keep the black one. The scheme lives on `device::Device` (in memory, like its name) and
  reaches the panel through the same mirror the strip already uses.
- **Clipboard** is text on the operating system's clipboard: a magic line and a versioned JSON payload, so a pattern travels
  between two running windows and can be read by eye in any editor. Both gestures arrive as egui **events**
  (`Event::Copy`, `Event::Paste`), because egui-winit swallows `Ctrl+C`/`Ctrl+V` and never delivers them as keys — verified in
  `egui-winit 0.37`.

**Tech Stack:** Rust 2024, egui/eframe 0.37, `serde_json` (already a dependency; `Pattern` already derives serde).

**Spec:** this plan is the design of record. It comes from the conversation that asked for the two features and settled four
questions: the face inverts but the cards do not; the strip cells wear their own device's colours; the yellow palette is
drawn by hand rather than computed as a byte inverse; and a paste lands immediately and says so.

## What is wrong today

- **Every device looks identical.** `ui/theme.rs` is a single palette of consts, and every painter reads it directly: 107
  colour references across 13 source files. The panel paints one face in one palette no matter which of the four devices it
  is showing, so the only cues to "which machine am I editing" are the strip's name and its accent border. Picking the
  wrong cell and then turning a knob is a real mistake, not a hypothetical one: it is silent, and it is the device's own
  sound and MIDI route that change, not just a highlight.
- **A pattern cannot leave its device.** The only way a line gets from one machine (or one session) to another is through
  the songs file: save, then load, then copy the steps by hand. There is no copy and no paste, and the clipboard the window
  already has — the name field pastes into it today — holds nothing the instrument can use.

## The design

### Colour

**`ui::theme` gains a palette value and one const per scheme.**

```rust
#[derive(Copy, Clone)]
pub struct Palette {
    pub panel: Color32,        // the face
    pub panel_raised: Color32, // knob bodies, raised key faces, cards
    pub well: Color32,         // display wells, knob recesses, the recess behind the face
    pub silkscreen: Color32,   // legends, frames, engraving
    pub silkscreen_dim: Color32,
    pub led_on: Color32,
    pub led_off: Color32,
    pub led_accent: Color32,
    pub text: Color32,         // display digits
    pub white_key: Color32,    // step keycap face
    pub shade: Color32,        // the scrim under a card — black alpha in both schemes
    pub learn_target: Color32, // cyan in both: it must never read as one of the panel's own lights
}

impl Palette {
    pub fn silkscreen_stroke(&self, width: f32) -> egui::Stroke;
}

pub const BLACK: Palette = …;   // today's colours, unchanged
pub const YELLOW: Palette = …;
pub fn palette(scheme: device::Scheme) -> &'static Palette;
```

The layout consts stay where they are — `DESIGN_SIZE`, `GAP`, `MARGIN`, `SCREW_RADIUS`, `WINDOW_SIZE`, and `legend(size)`,
which is type, not colour. `theme.rs` remains the only file that may name a hex value.

**The yellow palette is hand-drawn, not inverted.** A byte inverse would turn the recessed wells into the brightest shapes on
the face and break every reading rule the panel depends on. The two schemes keep the same *relationships*, with the two
families exchanged:

| | Black (today) | Yellow | rule kept |
|---|---|---|---|
| `panel` | `#0B0B0B` | `#FFC300` | the largest surface |
| `panel_raised` | `#191919` | `#141008` | reads as raised on the face; keys and knobs are dark on amber |
| `well` | `#040404` | `#C99400` | reads as *recessed*: darker than the face in both, by the same perceived step |
| `silkscreen` | `#FFC300` | `#0B0B0B` | maximum contrast against the face |
| `silkscreen_dim` | `#7A5C00` | `#4A3A00` | a legend that is there but not shouting |
| `led_on` | `#FFE066` | `#FFFDF0` | brighter than the face |
| `led_off` | `#2A2208` | `#8A6A00` | darker than the face |
| `led_accent` | `#FF9500` | `#B23A00` | a warm second amber that reads as "this one" |
| `text` | `#F5F0E0` | `#0B0B0B` | contrasting the well |
| `white_key` | `#E8E2D0` | `#E8E2D0` | cream stays cream: a keycap is a physical thing |
| `shade` | `black alpha 90` | `black alpha 90` | darkens whatever is under it |
| `learn_target` | `#3DE8F0` | `#3DE8F0` | never one of the panel's own lights |

The exact hexes are a starting point; the gate is the **per-scheme invariants**, not the numbers (see the tests).

**The scheme is a device's.** `device::Scheme { Black, Yellow }` is a catalogue enum in the module that owns the idea of a
device, with `ALL`, `name()` (`"BLACK"`, `"YELLOW"` — the card's caption) and `next()` (what a click on the row asks for).
`Device` gains `scheme: Scheme`, default `Black`; `Rack::set_scheme(index, scheme)` writes it. There is nothing to publish:
`BusState` carries what the audio thread and the MIDI handlers read, and neither of them can see a colour.

**The panel paints one device, so it paints one palette.** `PanelState` gains `scheme: Scheme`, filled by `main` from the
selected device every frame, exactly as `strip.cells` is. `draw_panel` turns it into a palette and hands it to every painter
that draws on the *face*: `panel`, `knob`, `step_key`, `display`, `transport`, `row_knobs`, `save`, `strip` and the face bits
of `mod.rs` (the help dot, the gear, the switches). **The four cards keep `theme::BLACK`** — help, settings, songs, the device
card and the picker are painted on `PANEL_RAISED` and read as objects laid over the instrument, not as part of its face.
`theme::SHADE` is black in both schemes, so a card sits on amber with a dark halo rather than an amber one.

**The strip is the legend.** `StripCell` gains `scheme`, `main` fills it from the device, and `strip::paint` paints each cell
in *that cell's* palette: a yellow machine's cell is amber with black lettering even while the panel shows a black one. A hand
sees what is where before it clicks. The selected cell's border and the mini MUTE key use the same per-cell palette — a border
in the face's own accent would vanish into an amber cell.

The strip's *band* is a different matter: it is part of the face, painted edge to edge on the face's own colour, so the band,
its rule and the `+` at its right wall follow the **face's** palette while the cells inside it follow their devices'. One amber
face with a dark cell and an amber cell on it is the whole point of the round.

**Where the tests live.** `theme`'s two invariant tests are unit tests in `theme.rs`; the rack's scheme test is in
`tests/rack.rs` beside the other store tests; the colour tests that read shapes back are in `tests/devices_live.rs` and
`tests/device_card_live.rs` (they need frames); and the clipboard's are split the same way — `encode`/`decode` unit-test in
`src/clipboard.rs`, and `dispatch::clipboard` in a new `tests/clipboard.rs` (not `_live`: it drives events and a
`PatternBank`, not frames).

**The way in is the device card.** A `COLOUR` row under `CHANNEL`, one key in the `ROLE` row's shape, captioned with the
*current* scheme (`BLACK` or `YELLOW`); a click asks for the other one: `Request::SetScheme(index, scheme)`. `main` answers
with `rack.set_scheme`. One key rather than two, because there are two schemes and the row already says which one is on.

### The clipboard

**A payload, not a `Pattern`.** `src/clipboard.rs` holds the format:

```rust
pub const MAGIC: &str = "takka-acid";
pub const VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(tag = "kind")]
pub enum Clip {
    /// A pattern, and the slot it came from, for the notice a paste gives.
    Pattern { pattern: Pattern, from: Option<String> },
}

pub fn encode(clip: &Clip) -> String;      // "takka-acid 1\n{ …json… }"
pub fn decode(text: &str) -> Option<Clip>; // None for a foreign clipboard, an unknown version, or a broken payload
```

An envelope with a `kind` tag rather than a bare pattern: the round that copies device settings adds a *variant*, not a second
format, and an older build reading a newer clipboard refuses it by the tag and the version, which is what `decode`'s `None`
already means. Text on the OS clipboard (not an in-process static) so that copying in one window and pasting in another works,
and so a copy can be read back in any editor when something looks wrong.

**Both gestures are events.** `egui-winit` intercepts `Ctrl+C` and `Ctrl+V`, emits `Event::Copy` / `Event::Paste(text)`, and
never delivers the key press, so `ui::shortcuts` gains nothing and the handler is one function beside `escape` — the same
argument for putting it there: `main` is a binary no test can call.

```rust
/// Answer this frame's clipboard events. Returns the text `main` hands to the OS clipboard, if any.
pub fn clipboard(
    events: &[egui::Event],
    typing: bool,          // a text field has the keyboard: the clipboard is the field's, not the pattern's
    panel: &mut PanelState,
    shared: &SharedState,
    bank: &mut PatternBank,
) -> Option<String>;
```

- **Copy** takes the pattern on the panel — `shared.pattern()`, which is the one the engine is playing — and the slot label
  from `panel.pattern_id`, notices `copied A1`, and returns the encoded text. `main` calls `ctx.copy_text` with it.
- **Paste** decodes the event's text. A pattern is put into the slot on the panel: `PatternBank::replace(bank.current(), …)`
  and `shared.set_pattern(Arc::new(…))` so the engine plays it at the next block, `clear::disarm` because an armed CLEAR was
  armed against the pattern that just went, `panel.dirty = true`, and the notice `pasted A1 into C3`. A clipboard that does
  not decode notices `the clipboard has no pattern` — a `Ctrl+V` that silently did nothing would read as a broken app.
- `typing` is `ctx.text_edit_focused()`, the guard the key path already uses: with the card's name field focused, copy and
  paste belong to the field.

**The notice is the confirmation.** A paste replaces a pattern with no undo, and the user's call was: land it, and say what it
replaced. `pasted A1 into C3` names both halves, and it survives the frame — `PanelState::notice` is already the status line's
message and clears itself after a few seconds (`AGENTS.md`: "a message is a notice, not a dialog").

## Tests that make it safe

| Test | What it pins |
|---|---|
| `theme::both_schemes_hold_the_same_reading_rules` | per scheme: the face is the largest surface (Black: `luminance(panel) < 0.02`; Yellow: `> 0.5`), the well is darker than the face, the silkscreen contrasts it, `led_on` outshines the face, `led_off` does not. The two palettes cannot drift apart in legibility. |
| `theme::silkscreen_inverts_between_the_schemes` | `YELLOW.silkscreen ≈ BLACK.panel` and `YELLOW.panel ≈ BLACK.silkscreen`: the swap the user asked for, held to it. |
| `rack::a_device_carries_its_own_scheme` | `set_scheme` writes one device and leaves its neighbours; a new device is `Black`; the scheme survives a selection change. |
| `devices_live::the_face_follows_the_device_it_shows` | through real frames: with a yellow device selected the face is filled `YELLOW.panel`; with a black one selected it is `BLACK.panel`. |
| `devices_live::the_strip_cell_wears_its_own_devices_colours` | cells painted in their own palettes: a yellow cell's face is `YELLOW.panel` while the panel shows the black device. |
| `device_card_live::a_card_stays_black_over_a_yellow_face` | the card's fill is `BLACK.panel_raised` even when the face behind it is amber — the conversation's decision, pinned. |
| `device_card_live::the_colour_row_asks_for_the_other_scheme` | the `COLOUR` key shows the current scheme and asks `SetScheme(index, Yellow)`; and back to `Black`. |
| `clipboard::a_pattern_round_trips` | encode → decode returns the same pattern, every field: `on`, `pitch`, `octave_offset`, `accent`, `slide`, `velocity`. |
| `clipboard::a_foreign_clipboard_is_refused` | plain text, a JSON object without the magic line, a payload with a version this build does not know, and truncated JSON all decode to `None`. |
| `clipboard::copying_notices_the_slot_and_returns_the_text` | `Event::Copy` → notice `copied A1`, returned text decodes back to the pattern on the panel. |
| `clipboard::pasting_lands_the_pattern_and_says_what_it_replaced` | `Event::Paste` → the bank slot and `shared.pattern()` are the clip's, `dirty`, notice `pasted A1 into C3`, and the *other* slots are untouched. |
| `clipboard::a_paste_of_something_else_says_so` | garbage in, notice out, nothing changed, not dirty. |
| `clipboard::a_focused_field_keeps_the_clipboard` | `typing: true` → neither gesture acts, nothing returned, bank untouched. |
| `help_live::the_help_card_names_the_devices_and_what_is_not_saved` (extended) | the card names `Ctrl+C` and `Ctrl+V`, so the only manual is not silent about them. |

**Mutations, run at the gate (task 8):**

1. `theme::palette` returns `&BLACK` for both schemes → `the_face_follows_the_device_it_shows` and
   `both_schemes_hold_the_same_reading_rules` must fail.
2. `clipboard::decode` ignores `VERSION` → `a_foreign_clipboard_is_refused` must fail.
3. `dispatch::clipboard`'s paste skips `shared.set_pattern` → `pasting_lands_the_pattern…` must fail (the bank would move and
   the engine would not, which is the bug `PatternBank`'s `stash_current` exists to prevent in the other direction).

## Tasks

Each task ends with the three gates, the inline review, and a commit.

- [ ] **Task 1 — the palette and the two schemes.** `theme::{Palette, BLACK, YELLOW, palette}`; `silkscreen_stroke` becomes a
  method; the consts keep their names as `BLACK`'s fields so nothing else moves yet. Tests: the two per-scheme invariant tests
  above. No behaviour change — everything still paints `BLACK`.
- [ ] **Task 2 — the face painters take a palette.** `panel`, `knob`, `step_key`, `display`, `transport`, `row_knobs`, `save`,
  `strip` and `mod.rs`'s face bits; the four cards (help, settings, songs, device_card + picker) and `modal` keep
  `theme::BLACK`. `draw_panel` passes `theme::BLACK` for now, so every existing test — including the ones that assert on
  colours — is green before and after, which is the point of splitting it from task 3.
- [ ] **Task 3 — the device's scheme reaches the face.** `device::Scheme`, `Device::scheme`, `Rack::set_scheme`,
  `PanelState::scheme`, `StripCell::scheme`, `draw_panel` uses the mirror, `strip::paint` uses each cell's own palette. Main
  fills the mirror. The three live tests that read colour back off the paint.
- [ ] **Task 4 — the COLOUR row.** `Request::SetScheme`, the card's row and layout, `main`'s arm, and the two card tests.
- [ ] **Task 5 — the clipboard format.** `src/clipboard.rs` with `encode`/`decode` and their tests; no wiring yet.
- [ ] **Task 6 — the gestures.** `dispatch::clipboard`, `main` calling it and flushing `ctx.copy_text`, the notices, the
  `typing` guard, and the six `clipboard` tests. `README.md`'s keyboard/MIDI section gains the two gestures? No — the help
  card is the manual; README's "Features" gains one line under the pattern.
- [ ] **Task 7 — the card and the phase overview.** `help.rs` gains the two gestures (task 1's test extended), and
  `00-phase-2.md` gains this plan's row and its Progress line.
- [ ] **Task 8 — the gate.** Three gates on the workspace, the three mutation checks above, and the review pass.

## Non-goals

- **Saving the scheme.** `SavedState` is frozen at 9 this phase, so a colour, like a name, is gone on restart. The round that
  saves the whole rack saves it (`plans/future/wishlist.md`).
- **An undo.** A paste that replaces an edited pattern loses it; the notice names what went, and that is the whole of the
  safety net, by the user's decision.
- **Copying anything but a pattern.** Device settings, songs and whole banks are the next round's variants of `Clip`; the
  envelope is here so that round adds a variant and not a format.
- **A palette per pattern, or per bank.** A colour belongs to a machine.
- **Two palettes for the cards,** or a theme switcher in settings. The face carries the identity; the cards are furniture.

## Rulings made while writing this plan, and their cost if wrong

| Ruling | Cost if wrong |
|---|---|
| One plan for two features, numbered 44 in `plans/phase 2/` | Two independent halves in one review gate: a finding in one can delay the other. Accepted — both are small and neither depends on the other's code. |
| The yellow palette is hand-drawn, not computed | The user asked for the black and yellow "swapped around"; a hand-drawn set can land a shade away from a literal swap. The invariant tests are what is actually promised. |
| The scheme is in memory only | A colour, like a name, is gone on restart. Stated in `README.md`? No: the save limit already says names and extra devices are not saved, and a colour is part of that sentence's "extra devices" in spirit. If a player expects it to survive, the next round's footnote widens. |
| Paste lands immediately, with a notice | A pattern replaced by mistake is gone. The user chose this over a two-click CONFIRM because the pattern being replaced is the one on screen. |
| The clipboard is text on the OS clipboard | A secret in a clipboard, and a paste from an unrelated app that fails to decode. Both are handled: nothing is written until a copy, and a failed decode says so. |

## Progress

**The round is done.** Three gates on the whole workspace — `cargo test --workspace` → **68 suites, 639 tests, 0 failed**
(the count the next round starts from), `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check`
clean — and `docs/guide/check.sh` still `guide: ok`.

- [x] **Task 1 — the palette and the two schemes** (`1ffead6`). `theme::{Palette, BLACK, YELLOW, palette}`, `silkscreen_stroke`
  as a method, and `device::Scheme` with `ALL`/`name`/`next`. The old colour consts stayed as aliases of `BLACK`'s fields for
  one commit (one source of truth) and were deleted in task 2. Tests: `the_face_is_dark_in_one_scheme_and_bright_in_the_other`,
  `both_schemes_hold_the_same_reading_rules` (well below face, legend contrast ≥ 4.5, dim quieter than full, a lit lamp above the
  face, digits legible on the well), `the_two_families_are_exchanged`, and the rack's catalogue test. Ruling: `device::Scheme`
  landed here rather than in task 3, because `theme::palette` takes it.
- [x] **Task 2 — the face painters take a palette** (`903e6ff`). 107 colour references across 13 files; the face painters take
  `&theme::Palette`, `draw_panel` passed `BLACK`, and the four cards kept `theme::BLACK`. Two shapes the task did not name and
  the work forced: `transport::card_button` (the cards' keys in one place, so "every card is black" is stated once instead of
  at 24 call sites) and `controls::Key` (clippy refused `switch_key` at eight arguments; the five things a row knows together
  became one value). All 626 tests green before and after, and the face was checked against a pre-refactor screenshot.
- [x] **Task 3 — the device's scheme reaches the face** (`e36d668`). `Device::scheme`, `Rack::set_scheme`, `PanelState::scheme`,
  `StripCell::scheme`, per-cell palettes in the strip, and `main` filling both mirrors. Three tests, written first
  (`rack::a_device_carries_its_own_scheme`, `the_face_follows_the_device_it_shows`,
  `the_strip_cell_wears_its_own_devices_colours`). **Ruling that changed the design:** a strip cell painted with
  `panel_raised` is near-black in the amber scheme while that scheme's legends are near-black too, so the cell is now a
  miniature of *its* face (`p.panel` fill, `p.silkscreen` letters). Cost: the black scheme's cells move `#191919` → `#0B0B0B`.
  Mutation: `palette(Yellow) → &BLACK` fails both live colour tests.
- [x] **Task 4 — the COLOUR row** (`668a9a4`). `Request::SetScheme`, `Card::scheme` + `scheme_from` (per frame, beside
  `route_from`), `Rows::scheme` under `CHANNEL`, one key captioned with the scheme it is on. Walked in the app: the face turns
  amber behind a still-black card and the strip's own cell turns amber with black letters.
- [x] **Task 5 — the clipboard format** (`058d221`). `src/clipboard.rs`: `MAGIC`, `VERSION`, a `kind`-tagged `Clip` (one
  variant), `encode`/`decode`, five tests including eight shapes of refusal. No test was watched failing first here (format and
  tests went in together); the gap was closed by two mutations, both caught — the header check loosened to `starts_with`, and a
  payload rebuilt without `from`.
- [x] **Task 6 — the gestures** (`b092772`). `dispatch::clipboard(events, typing, panel, shared, bank) -> Option<String>`,
  `main` handing it the frame's `Copy`/`Paste` events and flushing the return through `ctx.copy_text`, notices `copied A1` /
  `pasted A1 into A2` / `the clipboard has no pattern`, and the `typing` guard. `tests/clipboard.rs`, four tests over real
  events and a real bank. One of my own assertions was vacuous (the "neighbour untouched" slot held the pattern being
  pasted) — the fixture now has three distinct patterns. Mutation (paste skips `shared.set_pattern`) caught. Walked in the app:
  `Ctrl+C` → `copied A1`, `shift+Right` → `A2`, `Ctrl+V` → `pasted A1 into A2` with the steps back.
- [x] **Task 7 — the card and the overview** (`909070d`). `help_live` extended with `Ctrl+C`/`Ctrl+V` first (RED), then the
  card line; `README.md` gained the clipboard sentence and the colour in the restart-cost list; `00-phase-2.md` gained this
  plan's row and Progress line, and lost a paragraph that had been duplicated since an earlier round.
- [x] **Task 8 — the gate** (`f3d5b13`). All three plan-named mutations re-run on the frozen code and all three caught:
  `palette(Yellow) → &BLACK` (both live colour tests), `decode` ignoring `VERSION` (`a_foreign_clipboard_is_refused`), and a
  paste that moves the bank but not the engine (`pasting_lands_the_pattern_and_says_what_it_replaced`).

**Review mode.** Inline, per this plan's header: `pi-subagents`' only model refused the connection during plan 43 and the
`claude-code` agent needs its own session, so there was no fresh reader. The whole-round pass walked the two seams the plan
named:

- **theme ↔ device ↔ ui.** One mapping (`theme::palette`), one catalogue (`Scheme`), the face palette read in exactly two
  places (`draw_panel`, `strip::paint`) and the cards taking `BLACK` everywhere — checked by grep, not by memory.
- **clipboard ↔ dispatch ↔ main.** The payload is a value with a text form; the gesture is one function; `main` is the only
  place that touches the OS clipboard, and it does so in one line each way. The events are *not* keys, and the comment says
  why, because the next hand will expect a `Shortcut` here.

Findings:

- **Critical:** none.
- **Important:** none.
- **Minor — fixed in this round:** the `help` card and `README` lines, the duplicated overview paragraph, and three
  `replace_lines` slips caught by the diff in task 7 (a replaced help line, a half-deleted README bullet, a duplicated
  sentence in the overview).
- **Minor (deferred) — `pasted A1 into A1`.** Copying and pasting the same slot says so without mentioning that nothing
  changed, and the notice names no device. A player who pasted twice will read it once and understand. Cost if wrong: a
  redundant status line.
- **Minor (deferred) — the copy-then-type papercut.** Copy a pattern, then `Ctrl+V` with the card's name field focused, and
  egui pastes the JSON payload into the field; the field's clipboard is the field's, which is the rule, and `Rack::rename`
  trims it into a very long name rather than losing anything. Fixing it means filtering what the field receives, which is a
  change to how the card's field reads the clipboard, not to this round's format. Cost if wrong: one absurd device name.
- **Minor (deferred) — a hand-written payload with no `from`** notices `pasted a pattern into B2`; the wording is untested.
- **Deferred (deliberate):** `main`'s glue — the `Copy` → `ctx.copy_text` call and the `text_edit_focused` gate around the
  event filter — is untested, like every other drain in that binary. `dispatch::clipboard` is where the behaviour is, and it is
  covered; the walk in the app is the evidence for the glue.
- **Declined to judge:** whether amber is the right amber. It was looked at, on a screenshot, by one pair of eyes; the
  palettes' contracts (contrast, recess, lamp order) are tested, the taste is not.

Rulings made while building this round, and what they cost if wrong:

| Ruling | Cost if wrong |
|---|---|
| A strip cell is a miniature of its device's face, not a raised key | The black scheme's cells are one step darker (`#191919` → `#0B0B0B`). The alternative was a cell whose legends are the same colour as its fill. |
| The cards are black by construction — `transport::card_button` — not by every call passing `BLACK` | A card that ever wants its device's colours has to stop using the helper. Twenty-four call sites say `BLACK` today and none of them can forget. |
| `controls::Key` exists | One more type in `controls`; a row that grows a sixth descriptor adds a field. Clippy's arity limit asked for it. |
| Paste lands immediately, with a notice (the user's call) | A pattern replaced by mistake is gone; the notice names the slot it went into. |
| The colour is in memory only | A colour, like a name, is gone on restart. `README.md` says so in the restart-cost list. |
| The clipboard is text on the OS clipboard | A paste of our own payload into a text field (the papercut above), and a secret of a kind: whatever a hand copied is on the system clipboard until something else is. |
