# Plan 44 — A device's colour, and the pattern clipboard

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). Steps use checkbox (`- [ ]`) tracking;
> each task ends in a green, reviewed commit.
>
> **Review:** every task ends with the review gate in `00-phase-2.md`. *Superseded:* this round ran it inline, which
> `AGENTS.md` no longer permits — the reviewer is now a dispatched subagent or the commit waits. Kept as written because it
> records why the rule changed.

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
    pub raised_ink: Color32,   // captions and pointers on `panel_raised`: a key's label, a knob's pointer
    pub well: Color32,         // display wells, knob recesses, the recess behind the face
    pub silkscreen: Color32,   // legends, frames, engraving
    pub silkscreen_dim: Color32,
    pub led_on: Color32,
    pub led_off: Color32,
    pub led_accent: Color32,   // ink for the face
    pub lit_ink: Color32,      // ink for a lit key's raised face — the accent's second role
    pub text: Color32,         // display digits
    pub white_key: Color32,    // step keycap face
    pub keycap_ink: Color32,   // lettering and marks on that cream, which does not change between schemes
    pub shade: Color32,        // the scrim under a card — black alpha in both schemes
    pub learn_target: Color32, // each scheme carries its own; never one of the panel's own lights
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
| `panel` | `#0B0B0B` | `#C99400` | the largest surface, and the **quieter** of the two golds: a whole window of it must not glare |
| `panel_raised` | `#191919` | `#141008` | reads as raised on the face; the keys that are engraved plates and the knobs are dark on amber |
| `key_face` | `#040404` | `#FFC300` | the face of an **unlit key**. The brighter gold lives here, where a hand rests and where the area is small; the face took the quiet one |
| `raised_ink` | `#FFC300` | `#FFE9B0` | legible on `panel_raised`: a key's caption, a knob's pointer. The face's ink will not do, because a raised control is the other family's surface in one scheme |
| `well` | `#040404` | `#9E7600` | reads as *recessed*: darker than the face in both, by the same perceived step |
| `silkscreen` | `#FFC300` | `#0B0B0B` | maximum contrast against the face |
| `silkscreen_dim` | `#7A5C00` | `#3A2E00` | a legend that is there but not shouting — on the **well** as well as the face, because a key's border, the knob's unlit arc and the strip's shelf are drawn on it (3.2:1 in both schemes) |
| `led_on` | `#FFE066` | `#621600` | a lit lamp is **seen**: contrast against the face (and the well its arc is drawn on), not luminance above it |
| `led_off` | `#2A2208` | `#D9A600` | an unlit lamp is **invisible**: nearly the face's own colour, as black's `#2A2208` nearly is on `#040404` |
| `led_accent` | `#FF9500` | `#4E1300` | ink for the **face**: the loud colour, and louder than `silkscreen_dim` on it, because UNSAVED wears it and SAVED wears the dim legend |
| `lit_ink` | `#FF9500` | `#FFC97A` | ink for a **lit key's raised face**, which is a different reading: a colour dark enough to read on the amber face is too dark to read on a near-black key, so the accent's two roles are two values in the amber scheme |
| `text` | `#F5F0E0` | `#0B0B0B` | contrasting the well |
| `white_key` | `#E8E2D0` | `#E8E2D0` | cream stays cream: a keycap is a physical thing |
| `keycap_ink` | `#0B0B0B` | `#0B0B0B` | legible on that cream, which does not change between schemes |
| `shade` | `black alpha 90` | `black alpha 90` | darkens whatever is under it |
| `learn_target` | `#3DE8F0` | `#0B4E55` | never one of the panel's own lights, *and* seen: cyan on a gold face is a hue difference with almost no luminance step (1.8:1) |

On a bright face a lamp is a mark and an unlit lamp is invisible, so the rule a scheme keeps is contrast against the face,
not luminance above it: nothing can be both brighter than `#FFC300` and contrasting against it (white is 1.57:1 on it,
while black's `#8A6A00` off-lamp was 3.15:1 — the *off* lamp shouted). The inverse of the dark face, held by the same three
assertions.

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
`Palette::shade` is black in both schemes, so a card sits on amber with a dark halo rather than an amber one.

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
  from `bank.current()`, notices `copied B2`, and returns the encoded text. `main` calls `ctx.copy_text` with it.
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
| `theme::both_schemes_hold_the_same_reading_rules` | per scheme: the well sits below the face, the silkscreen contrasts the face ≥ 4.5:1 and the dim legend ≥ 2:1 on the face and ≥ 3:1 on the well and quieter than the full one, `led_on` contrasts the face and the well ≥ 3:1 while `led_off` stays under 2:1 against the face (a lamp is a light on a dark face and a mark on a bright one), the digits contrast the well ≥ 4.5:1, `raised_ink` contrasts `panel_raised` ≥ 4.5:1, `lit_ink` contrasts it ≥ 4.5:1, `keycap_ink` contrasts `white_key` ≥ 4.5:1, `learn_target` is neither the silkscreen nor a lamp and is seen on the face ≥ 3:1, `led_accent` outshouts `silkscreen_dim` on the face and is seen on the well ≥ 3:1 (the unlit key's hover ring). Then, once: the two schemes are not the same palette. (The face's own brightness — dark in one scheme, bright in the other — is `the_face_is_dark_in_one_scheme_and_bright_in_the_other`, not this test.) |
| `theme::the_two_families_are_exchanged` | `YELLOW.silkscreen == BLACK.panel` and `YELLOW.panel == BLACK.silkscreen`: the swap the user asked for, held to it exactly. |
| `rack::a_device_carries_its_own_scheme` | `set_scheme` writes one device and leaves its neighbours; a new device is `Black`; the scheme survives a selection change. |
| `devices_live::the_face_follows_the_device_it_shows` | through real frames: with a yellow device selected the face is filled `YELLOW.panel`; with a black one selected it is `BLACK.panel`. |
| `devices_live::the_strip_cell_wears_its_own_devices_colours` | cells painted in their own palettes: a yellow cell's face is `YELLOW.panel` while the panel shows the black device. |
| `device_card_live::a_card_stays_black_over_a_yellow_face` | the card's fill is `BLACK.panel_raised` even when the face behind it is amber — the conversation's decision, pinned. |
| `device_card_live::the_colour_row_asks_for_the_other_scheme` | the `COLOUR` key shows the current scheme and asks `SetScheme(index, Yellow)`; and back to `Black`. |
| `clipboard::a_pattern_round_trips_through_the_clipboard` | encode → decode returns the same pattern, every field: `on`, `pitch`, `octave_offset`, `accent`, `slide`, `velocity`. |
| `clipboard::a_foreign_clipboard_is_refused` | plain text, a JSON object without the magic line, a payload with a version this build does not know, and truncated JSON all decode to `None`. |
| `clipboard::copying_notices_the_slot_and_returns_the_text` | `Event::Copy` → notice `copied B2`, and the returned text decodes back to the pattern the engine holds, labelled with the panel's slot. |
| `clipboard::pasting_lands_the_pattern_and_says_what_it_replaced` | `Event::Paste` → the bank slot and `shared.pattern()` are the clip's, `dirty`, notice `pasted A1 into B2`, and the *other* slots are untouched. |
| `clipboard::a_paste_of_something_else_says_so_and_changes_nothing` | garbage in, notice out, nothing changed, not dirty. |
| `clipboard::a_focused_field_keeps_the_clipboard` | `typing: true` → neither gesture acts, nothing returned, bank untouched. |
| `help_live::the_help_card_names_the_devices_and_what_is_not_saved` (extended) | the card names `Ctrl+C` and `Ctrl+V`, so the only manual is not silent about them. |

**Mutations, run at the gate (task 8):**

1. `theme::palette` returns `&BLACK` for both schemes → `the_face_follows_the_device_it_shows` must fail, and
   `both_schemes_hold_the_same_reading_rules` fails too, on its closing `assert_ne!(palette(Black), palette(Yellow))`:
   every per-scheme rule above it is satisfied twice by the same palette, so that one line is what catches the mutation in
   the unit test.
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
| The scheme is in memory only | A colour is gone on restart for **every** device, the first one included: `SavedState` does not carry it, so it is not "an extra device" — the first device's colour is lost too. `README.md`'s restart-cost list names it in so many words, so nothing is silent about it. If a player expects it to survive, the next round's footnote widens. |
| Paste lands immediately, with a notice | A pattern replaced by mistake is gone. The user chose this over a two-click CONFIRM because the pattern being replaced is the one on screen. |
| The clipboard is text on the OS clipboard | A secret in a clipboard, and a paste from an unrelated app that fails to decode. Both are handled: nothing is written until a copy, and a failed decode says so. |

## Progress

**The round is done.** Three gates on the whole workspace — `cargo test --workspace` → **69 suites, 648 tests, 0 failed**
(the count the next round starts from: 639 for the round, plus nine added by the two fix passes below), `cargo clippy
--workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean — and `docs/guide/check.sh` still `guide: ok`.

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

### Round-2 fixes (review round 1)

Three fresh-context reviewers read the round's eight commits. What they found, applied in one batch:

- **A — the amber scheme's ink was unreadable (P0).** The palette had ink for the face (`silkscreen`), a surface for raised
  controls (`panel_raised`) and cream paper for keycaps (`white_key`), but no ink of its own for the last two: in BLACK the
  face's ink happened to work on all three, in YELLOW it did not. `Palette` gains `raised_ink` (`panel_raised`'s captions
  and pointers: a knob's pointer line, `paint_switch`'s caption) and `keycap_ink` (the keycap's border, note, step number
  and slide arrow). BLACK's two fields are its old colours, pinned by `the_black_scheme_is_the_instrument_it_has_always_been`,
  so the scheme did not move. The reading rules now hold both inks to 4.5:1.
- **B — the learn outline was invisible on amber (P1).** `YELLOW.learn_target` is `#0B7A85` instead of the cyan, which sat
  at 1.07:1 against the amber face — a hue difference with no luminance step. BLACK keeps `#3DE8F0`. Pinned by
  `contrast(panel, learn_target) >= 3.0` in the shared rules.
- **C — SAVED read louder than UNSAVED on amber (P2).** `YELLOW.led_accent` is `#7A1F00` (≈6.5:1 on the amber face) where
  `silkscreen_dim` is 4.5:1, so the dirty label is the loud one. BLACK keeps `#FF9500`. `ui::save`'s doc comment no longer
  says the colour "takes the amber". Pinned by `contrast(panel, led_accent) > contrast(panel, silkscreen_dim)`.
- **D — the LED halos were hard-coded to black's lamp (P2).** `display`'s digit halo and `step_key`'s lamp halo were the
  literal `#FFE066` at alpha 102 and 64; both now derive from the palette's `led_on` at the same alpha, so BLACK's output
  is byte-identical by construction and YELLOW's halo is not a black-scheme lamp bleeding into an amber panel.
- **E — the clipboard re-dispatched on logic-only ticks (P1, real data loss).** `run_logic` leaves the last pass's events
  in `input.events`, and `main` repaints every 200 ms, so a minimised window re-ran the same `Copy`/`Paste` on every tick:
  a copy kept rewriting the OS clipboard, and a paste kept landing — including in a slot a learned pattern controller had
  moved the bank to. `dispatch::take_clipboard` **takes** the clipboard events out of the frame (and takes nothing when a
  text field has the keyboard), and `main` calls it through `input_mut`. `dispatch::clipboard` also answers
  `Event::PasteImage` with the same notice as text that is not ours, and its doc now says plainly that an *empty* clipboard
  produces no event at all (`egui-winit` only sends `Paste` for non-empty text).
- **F — two test gaps.**
  - The copy test seeded the bank slot and the shared pattern with the *same* pattern, so a copy that read the slot rather
    than the engine passed. It now sets the shared pattern to a different one and asserts the clip holds that; the mutation
    (copy from `bank.get(bank.current())`) is caught.
  - `device_card_live::a_card_stays_black_over_a_yellow_face` was named in this plan's test table and never written. It
    exists: with an amber face behind an open card, the card's own surface is `BLACK.panel_raised`. Mutation (paint the
    card from `palette(state.scheme)`) caught.
- **G — small gaps.**
  - the COLOUR key's *painted* caption is asserted to read `YELLOW` after the switch, not just the request it asks for;
  - `rack::a_device_carries_its_own_scheme` re-asserts both devices after `set_scheme(9, …)` — its comment claimed that and
    nothing checked it;
  - `device_card`'s rows-inside-the-card test enumerates `scheme` too (it had been omitted by hand from a hand-kept list);
  - the paste test arms CLEAR first and asserts the paste spends it — the paste's only data-safety behaviour, previously
    untested. Mutation (drop `clear::disarm`) caught.
- **the help card** gains `F3: a device's COLOUR key repaints its face`, with `help_live`'s `COLOUR` needle written first
  and watched failing (RED) before the line.

**The second, final fix pass** actioned what the first one missed, what it introduced, and the two findings the user chose
to fix now:

- **H — the lit key's hover ring was still the accent (P1; the first pass fixed the legend and missed the ring).**
  `transport::transport_button` draws its hover ring on the same face its legend sits on, and RUN or STOP is always lit,
  so on an amber device the ring was `led_accent` at 1.83:1 on `#141008`. It now follows the legend's rule —
  `if lit { p.lit_ink } else { p.led_accent }` — and `contrast(well, led_accent) >= 3.0` pins the unlit half in the shared
  rules. `transport_live::a_hovered_key_rings_itself_in_ink_that_reads_on_its_face` reads both halves back from the
  painted shapes of an amber panel; written first and watched failing on the old code, where the lit STOP key's ring came
  back `#7A1F00`.
- **I — the amber dim legend fell to 2.64:1 on the well (P2, introduced by C).** `YELLOW.silkscreen_dim` is `#5C4A00`:
  5.35:1 on the face — still quieter than the accent's 6.45:1 — and 3.16:1 on the well, where the strip's shelf, the
  greyed `+`, the knob's unlit arc and an unlit key's border are drawn. Pinned by `contrast(well, silkscreen_dim) >= 3.0`
  in the shared rules, with BLACK's 3.2:1 already inside it.
- **J — the lamps read backwards on the amber face (the user's decision: "lit is a mark").** Nothing can be both brighter
  than `#FFC300` and contrasting it (white is 1.57:1; the `#8A6A00` off-lamp was 3.15:1), so on an amber device the *off*
  lamp shouted. The rule for a bright face is the inverse of a dark one: `YELLOW.led_on` is `#861F00` (5.9:1 on the face, 3.5:1 on
  the well, where the knob's lit arc sits) and `YELLOW.led_off` is `#D9A600` (1.38:1 and 1.22:1 — invisible,
  which is what "off" means there, exactly as `#2A2208` nearly disappears on `#040404`). BLACK keeps `#FFE066`/`#2A2208`
  byte for byte, and `leds_read_as_on_and_off_in_the_black_scheme` is unchanged. The shared rules replace "`led_on`
  brighter than the face, `led_off` darker than `led_on`" — which only held for a light on a dark face — with
  `contrast(panel, led_on) >= 3.0`, `contrast(panel, led_off) < 2.0` and `contrast(well, led_on) >= 3.0`. Every
  `led_on`/`led_off` use was grepped: they are lamps (the step key's, the strip's, the knob's lit arc, the display's halo)
  and the greyed `+`, never text a player must read.
- **K — the key path re-dispatched on a logic-only tick (P1, the user's decision to fix it now).** `main` read
  `input.events` through `dispatch::presses`, and egui hands a logic-only tick the last pass's events, so a hidden window
  re-fired the last Space/Enter/arrow every 200 ms: the transport flapped, the selected step toggled, the selection
  walked. `dispatch::take_presses` now **takes** the presses out of the frame, built on the same `as_press` predicate
  `presses` uses, and `main` calls it through `input_mut` with the one `typing` value the clipboard uses. `tests/keys.rs`
  pins it, in a file of its own because it drives events and not frames (`keys_live.rs` is the keyboard through the
  panel); the clipboard's taking tests stay in `tests/clipboard.rs`. Mutation: making `take_presses` a plain read fails
  `a_panel_key_is_taken_and_returned_as_a_press` on the "nothing else leaves the frame" assertion. `Escape` is taken the same
  way, by `dispatch::take_escape` — round 3 found that my first cut had left it a `key_pressed` *read* on the belief that it
  was a state query and idempotent; it is neither (`key_pressed` counts presses over the same event buffer, and `escape`
  closes one card per call, so a hidden window would have closed help and then songs), and `tests/keys.rs` pins the take.

**A ruling asked for and given during the fix pass.** C's darker amber `led_accent` has a second use site it did not account
for: `transport::transport_button` paints a *lit* key's face `panel_raised` and its legend `led_accent`, and RUN or STOP is
always lit — so on an amber device the running key's label fell to 1.83:1 (it had been 3.16:1, already poor). No single
colour clears both uses in the amber scheme (≥4.5:1 on `#FFC300` needs luminance < 0.096; ≥4.5:1 on `#141008` needs
> 0.199), so the accent's two roles are now two values: **`led_accent` is ink on the face (and on cream), `lit_ink` is ink
on a lit key's raised face.** BLACK's two values are identical (`#FF9500`), so its output is byte-identical; YELLOW's
`lit_ink` is `#FFC97A`, 12.5:1 on the raised face. Every other `led_accent` use — the SAVED/UNSAVED label, the step LED and
accent bar, the selected step's border, the strip's caret and cell border — stays on the darker amber, which is correct for
ink on a face. The strip's MUTE key inherits the fix, because it goes through `transport_button` with its cell's palette.
The reviewer who ruled on this asked for it, and it is recorded as their decision, not the executor's.

**Accepted, not fixed:**

- **The notice and the paste target come from `bank.current()`, while the display may already name a pending MIDI swap's
  slot.** With a learned pattern controller holding a new slot for the bar, a paste lands in the slot the engine is still
  playing and says that slot's name, while the display says the one it is going to. Misinformation, not wrong-slot
  destruction: the paste replaces the pattern being heard, which is the one the player was about to lose. Fixing it means
  deciding whether a paste cancels a pending swap, which is a product question this round did not ask.
- **The drag overlay's contrast** (`ui::mod`: the live pitch printed in the cell while a step is dragged) is already wrong
  in the black scheme, and is a pre-existing bug outside this round's colour work.
- **`main::logic`'s remaining wiring.** `TakkaApp` is a binary, so the lines that take the key presses, the Escape and the
  clipboard events out of `ctx` and hand them to `dispatch` cannot be driven by a test. The behaviour lives in `dispatch` and
  is covered — `tests/keys.rs` for the presses and the Escape, `tests/clipboard.rs` for the clipboard — and the walk in the
  app is the evidence for the glue. Making it testable means moving `logic`'s body into a function the binary's tests could
  call, which is a change to the binary's shape and not to this round.
- **Two amber-scheme orderings that round 3 measured and neither fix pass touched.** The untitled song label is painted with
  `gamma_multiply(0.55)`, which *darkens* in gamma space and so raises contrast on a bright face: "SONG untitled" reads
  8.96:1 on amber against a named song's 5.36:1, i.e. "nothing here yet" outshouts the real name (black goes the right way,
  1.61 against 3.15). Dimming with alpha instead would let the face do the darkening. And the ring on an *accented and lit*
  step is 2.81:1 on amber (black 5.08) because the accent and the lit lamp are now the same family of dark red; the accent
  bar on the keycap still carries the accent at 8.02:1, so the step is not unreadable. Both are cosmetic orderings in a
  scheme nobody has played for a week; both want a round that can listen and look, not another fix pass.
- **The greyed `+` at the rack's cap is 1.22:1 on amber** (black 1.30:1), because ruling 1 made `led_off` a colour that
  disappears into the face. It matches black's own intent — a dead key that says "no room" by being nearly unlit — and the
  picker's greyed rows are what actually explain the cap.

### The amber face softened (after the review loop, at the user's request)

A player found the amber scheme too bright to look at. The request: **swap the main yellow with the yellow the buttons use,
so the subtle one is dominant.**

It was not a swap of two values, because the "button yellow" was `well` — one field doing two jobs, the recessed displays
*and* the unlit keys — so moving it under the face would have made the wells brighter than the face and broken "a recess is a
hole in the face". The fix is one new role and a repaint:

- **`Palette::key_face`** — the face of an *unlit* key. Black: `#040404` (its old `well`, so the black scheme is
  byte-identical); amber: `#FFC300`, the gold the face used to be. `transport::transport_button` paints an unlit key with it.
- The amber **face** is the quiet gold it came from, `#C99400`; the **wells** go a step darker again, `#9E7600`, so the
  displays, knob recesses and the strip's shelf still read as recesses.
- The inks were re-derived against a mid-tone face, because every dark pair tightened: `silkscreen_dim` `#5C4A00` →
  `#3A2E00`, `led_on` `#861F00` → `#621600`, `led_accent` `#7A1F00` → `#4E1300` (it must still outshout the dim legend),
  `learn_target` `#0B7A85` → `#0B4E55` (a cue needs 3:1 against the face it is drawn on). All fifteen rules hold in both
  schemes — checked with `theme`'s own luminance formula before a line of Rust was touched, then by the suite.
- Two tests stated the old design and were restated rather than deleted: `the_face_is_dark_in_one_scheme_and_bright_in_the_other`
  is now `the_amber_face_is_a_quieter_gold_than_the_keys_on_it` (the face is no longer the brightest surface — its keys are),
  and `the_two_families_are_exchanged` now runs through `key_face` (`YELLOW.key_face == BLACK.silkscreen`), which is where the
  black scheme's legend gold went. A new rule pins a key's legend against the key's own face: `contrast(key_face, silkscreen)
  >= 4.5`.

Cost if wrong: the keys are now the loudest thing on the face, where before the face was; a hand that reads "bright = the
surface" has to relearn one step. The black scheme is untouched, and the pictures in the guide's amber pages were retaken
(plan 45's record) so the book does not show the old gold.
