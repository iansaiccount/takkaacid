# Plan 46 — The page's themes, and a drop-down to pick them

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). Each task ends green — the workspace
> gates and `docs/guide/check.sh` — and reviewed before its commit, per `00-phase-2.md`.
>
> **Review:** inline per task, plus the whole-round read at the end. The fresh reader that matters here is the *suite*: every
> theme is held to the same fifteen reading rules, so a palette that cannot be read cannot ship.

**Goal:** every palette on `https://www.media.io/color-palette/music-color-palette.html` becomes a theme a device can wear —
**twenty-two** of them, the twenty-one on the page plus the instrument's own black — and the device card's `COLOUR` row becomes
a scrollable drop-down listing each by name with a square of its dominant colour.

**Architecture:** `device::Scheme` becomes the theme catalogue; `ui::theme` keeps **one table row per theme** (its five source
hexes, a polarity, and the rare hand-set override) and a **derivation function** that turns five colours into the eighteen
roles a face needs, chosen once at first use and cached in a `OnceLock` so `palette()` still hands out a `&'static`. The card
grows a popup list with a scrollbar-free wheel scroll, capped to the panel. The player will prune the list after seeing the
themes; the point of this round is to make that possible and cheap.

**Spec:** this plan. Source colours are quoted verbatim from the page, in the table below. The authority for each role's
purpose is `ui::theme`'s field docs and the reading rules in its test module.

## What is wrong today

- **There are two looks, one of them unloved.** `device::Scheme` is `{ Black, Yellow }`; the amber scheme came from plan 44 and
  the player who asked for it does not want it. There is no third, fourth or twenty-second.
- **The `COLOUR` row is a toggle.** One key reading `BLACK`/`YELLOW`, flipping to the other. With twenty-two themes a toggler
  is a roulette wheel: no names, no colours, no way to go straight to the one you want.
- **Five colours are not eighteen roles.** Each page palette is an anchor, two support tones, one or two accents and a light.
  A device needs a face, a recess, a raised key, a key's face, two legend inks, four lamp roles, two keycap roles, a scrim and
  a learn cue. Hand-mapping that per theme is 396 constants with no rule behind any of them — and pruning the list later would
  mean deleting 18 numbers per theme.

## The design

### The catalogue: twenty-two themes

Every palette on the page, in its order, plus the instrument's own **BLACK** first. The colours are the page's, verbatim;
**polarity** is mine, taken from the page's own advice about which colour is the canvas (it says "pale gray as your canvas" for
Indie Dusk, for instance). `D` means a dark face, `L` a light one.

| # | Theme | Pol. | The page's five |
|---|---|---|---|
| — | **BLACK** | D | the instrument's own (unchanged, byte for byte) |
| 1 | **VINYL NOIR** | D | `#0B0B0D` `#1F2933` `#B08D57` `#E6E1D8` `#6B7280` |
| 2 | **STAGE SPOTLIGHT** | D | `#111827` `#F59E0B` `#FDE68A` `#EF4444` `#F9FAFB` |
| 3 | **JAZZ LOUNGE** | D | `#2B1D1A` `#7C2D12` `#D97706` `#F3E8D4` `#1C1917` |
| 4 | **SYNTH NEON** | D | `#0F172A` `#7C3AED` `#22D3EE` `#F472B6` `#F8FAFC` |
| 5 | **ACOUSTIC SUNRISE** | L | `#FDE2C2` `#F59E0B` `#A16207` `#3F6212` `#FFFBEB` |
| 6 | **FESTIVAL CITRUS** | D | `#0EA5E9` `#22C55E` `#F97316` `#FACC15` `#0F172A` |
| 7 | **INDIE DUSK** | L | `#2D1B69` `#4B5563` `#FCA5A5` `#F3F4F6` `#111827` |
| 8 | **OPERA VELVET** | D | `#4C0519` `#9F1239` `#F1C40F` `#F5F3FF` `#1F2937` |
| 9 | **LO FI RAIN** | D | `#0B1320` `#334155` `#94A3B8` `#C7D2FE` `#E2E8F0` |
| 10 | **DRUMLINE CONCRETE** | D | `#111827` `#374151` `#9CA3AF` `#F97316` `#F3F4F6` |
| 11 | **CHOIR PASTEL** | L | `#FCE7F3` `#DDD6FE` `#C7F9CC` `#BAE6FD` `#1F2937` |
| 12 | **BASSLINE EMBER** | D | `#1C1917` `#7F1D1D` `#EA580C` `#FDBA74` `#FAFAF9` |
| 13 | **POP BUBBLEGUM** | L | `#FF4D8D` `#FFB3C7` `#7C3AED` `#FDE047` `#FFF7ED` |
| 14 | **ORCHESTRAL ICE** | D | `#0F172A` `#1D4ED8` `#93C5FD` `#E0F2FE` `#F8FAFC` |
| 15 | **GARAGE GRUNGE** | D | `#0A0A0A` `#3F3F46` `#A3A3A3` `#84CC16` `#F5F5F5` |
| 16 | **DISCO MIRRORBALL** | D | `#111827` `#A855F7` `#F97316` `#FDE047` `#E5E7EB` |
| 17 | **AMBIENT MOSS** | D | `#0B1F17` `#14532D` `#86EFAC` `#D1FAE5` `#F8FAFC` |
| 18 | **K POP CANDY** | L | `#0EA5E9` `#A78BFA` `#FB7185` `#FCE7F3` `#0F172A` |
| 19 | **STUDIO SANDALWOOD** | L | `#3B2F2A` `#8B5E34` `#D6B98C` `#F3EDE2` `#6B7280` |
| 20 | **EDM LASER** | D | `#050816` `#22C55E` `#06B6D4` `#A855F7` `#E2E8F0` |
| 21 | **RETRO CASSETTE** | D | `#2A2E45` `#F97316` `#FB7185` `#FDE68A` `#FAFAF9` |

**BLACK stays first and byte-identical** — it is the regression every scheme is measured against, pinned by
`the_black_scheme_is_the_instrument_it_has_always_been`. **The amber scheme is retired.** The list is expected to shrink: the
player will review the twenty-two and ask for removals, and a removal is one variant, one table row and one `name()` arm.

### Deriving eighteen roles from five colours

**One table row per theme and one function, not 396 hand-typed numbers.** The row is
`(name, polarity, [five hex], overrides)`; the function is `derive(polarity, five) -> Palette`, run once at first use and
cached in a `OnceLock<[Palette; THEMES]>`, so `palette(scheme)` still returns a `&'static Palette` and the painters are
unchanged. The rule, in the order it applies:

1. **Sort the five by luminance** (`C1..C5`). Every page palette is built from an anchor, so its own ordering is the input.
2. **The face is the anchor, and polarity decides which end.** `D`: `panel = C1`. `L`: `panel =` the lightest of the two
   lightest (`C5`, or `C4` when `C5` is off-white and `C4` is the cream). Everything else is relative to it, not to a fixed
   end, which is what the amber scheme taught: a rule that says "brighter than the face" is unwinnable on a bright face.
3. **The surfaces around it.** `well` is the face moved 20 % *away* from the light end (a recess is always the darker/denser
   of the two, in both polarities); `panel_raised` is the face moved 8 % toward the other family; `key_face` 35 %.
4. **The legends are the far end.** `silkscreen` is the source colour with the greatest contrast against the face (in `D`
   that is `C5`, in `L` it is `C1`); `text = silkscreen`; `silkscreen_dim` is `silkscreen` mixed into `panel` until it clears
   3:1 against the **well** as well as the face — the knob's unlit arc, a key's border and the strip's shelf all sit there.
5. **The lamps are a direction.** `led_on` is the most chromatic of `C2..C4` that clears 3:1 against the face **and** the well;
   `led_off` is the face moved 10 % toward `silkscreen`, so "off" nearly disappears in either polarity; `led_accent` — the
   loud ink for UNSAVED — must out-contrast `silkscreen_dim` against the face; `learn_target` is the remaining chromatic
   colour (or a tint of it) at 3:1. On a bright face this yields dark marks, exactly as the amber scheme concluded.
6. **The physical parts do not vary.** `white_key` cream, `keycap_ink` near-black, `shade` black alpha 90 — a keycap is a
   physical thing and the cream is the instrument's. `raised_ink`, `lit_ink` and `keycap_ink` are chosen against the surface
   they are drawn on (a raised face, a lit key's face, the cream), not against the face.
7. **`overrides` is the escape hatch.** A theme whose five colours cannot satisfy a rule has the offending role set by hand in
   its row — visible in the table, one field, with the rule named in a comment. Expected to be needed for two or three of the
   twenty-two (Orchestral Ice's accents are all blues; K Pop Candy has no dark accent that is not a neutral).

**Ruling:** derivation rather than a committed 396-number table. `palette()` gains a `OnceLock` and loses `const`-ness; the
round gains a rule that can be read, reviewed and re-applied when the page changes, and pruning a theme becomes deleting one
row. Cost if wrong: a theme's face is a rule's idea of the palette rather than a designer's, and the `overrides` will grow if
the rule is too crude.

### The drop-down

The `COLOUR` row stops being a toggler and becomes the opener: one key captioned with the current theme's name (`text::fit`
cuts a long one with `…`) and the small square of its face colour.

A click opens a popup list over the card, one row per `Scheme::ALL`, twenty-two of them:

- a **14 px square** of that theme's `panel`, bordered with a 1 px stroke in that theme's own `silkscreen_dim` so a near-black
  theme's square is still a square on a dark card;
- the theme's **name**, in the card's black type (a theme belongs to a device, not to the card);
- the **current theme's row** marked with a filled dot in `led_accent`, so "which one am I on" needs no reading.

**Placement and scroll.** Twenty-two rows at the card's row height are taller than the panel, so the list is capped: anchored
to the `COLOUR` key, it opens downward when the space below holds at least six rows, otherwise upward; its height is the space
available, and the **wheel scrolls it** — the same wheel the knobs use, and the same "a list may scroll" the settings card's
port list already states. The rows scroll under a fixed frame; the wheel moves by rows, not pixels, so a row is never half-drawn.

**Dismissal.** Its own shield closes it on a click outside; `Escape` gives it up **first**, before the device card, because it
is the topmost thing on the face while it is up (the picker, the device picker, then this list, then the card).

A row click asks `device::Request::SetScheme(index, theme)` — the request the toggler already sends, so `main`'s answer is
unchanged. The list is the card's own state (`device_card::Picker`, beside `armed_remove`) plus a scroll offset, because
nothing on it is a reading of the machine.

### What this changes elsewhere

- `Scheme::next()` goes: with twenty-two entries a toggler is not the model. `ALL`, `name()` and `swatch()` stay — `name()` in
  `device`, `swatch()` in `theme` (it needs the palette), so the catalogue names no view type.
- `Palette` keeps its eighteen public fields untouched, so **no painter changes**.
- The strip already paints each cell in its device's colours; twenty-two themes need nothing there.
- `README`, the guide's `19-a-devices-card` and `20-reference` describe the list and name the themes; the guide's amber
  pictures are replaced by a themed device with the list open.

## Tests that make it safe

| Test | What it pins |
|---|---|
| `theme::every_theme_holds_the_reading_rules` (the existing rule test, over all twenty-two) | all fifteen rules per theme — the gate the derivation is written against, in both polarities |
| `theme::the_black_scheme_is_the_instrument_it_has_always_been` | BLACK unchanged, field for field, with twenty-one derivations beside it |
| `theme::every_theme_has_its_own_face` | no two themes share a `panel` colour: a drop-down of identical squares is a lie |
| `theme::every_theme_is_derived_from_its_row` | each `Palette` equals `derive(row)` — an override that bit, or a table edit that drifted, fails; the source five stay the truth |
| `theme::a_light_face_gets_dark_lamps_and_a_dark_face_light_ones` | polarity is real: for every `L` theme `led_on` is darker than the face, for every `D` theme lighter |
| `device_card_live::the_colour_row_opens_a_list_of_themes` | a click opens the list; it is a card for the one-card rule (opening settings puts it away) |
| `device_card_live::the_list_names_every_theme_and_shows_its_colour` | one row per `Scheme::ALL`, the name painted, and a filled `Shape::Rect` of that theme's `panel` read back off the paint |
| `device_card_live::a_row_in_the_list_asks_for_that_theme` | the row click sends `SetScheme(index, theme)` for a row below the first, so an off-by-one fails |
| `device_card_live::the_wheel_scrolls_the_list_and_the_last_theme_is_reachable` | two wheel notches move the list, and the twenty-second row is clickable once scrolled to |
| `device_card_live::escape_gives_up_the_list_before_the_card` | `Escape` closes the list, leaves the card; a second closes the card |
| `device_card_live::a_click_outside_the_list_closes_it_and_does_not_press_the_panel` | the shield |
| `device_card_live::the_colour_caption_is_the_current_theme_cut_to_fit` | `text::fit` with `…` for `DISCO MIRRORBALL` at the row's width, and no glyph outside the bundled font |

**Mutations, run at the gate:** (1) give two rows the same `panel` via an override → `every_theme_has_its_own_face` fails;
(2) drop a theme from `ALL` but not from the table → the row test's count fails; (3) make a row's swatch draw `palette(Black)`
→ the row test fails; (4) remove the polarity branch → `a_light_face_gets_dark_lamps…` fails; (5) stop the wheel from moving
the offset → the scroll test fails.

## Tasks

Each ends with the three workspace gates and `bash docs/guide/check.sh`, reviewed, committed.

- [ ] **Task 1 — the catalogue.** `Scheme` becomes twenty-two variants with `name()`; `ALL` in the table's order (BLACK
  first); `next()` deleted; every exhaustive `match` updated. The suite fails to compile where a decision is owed — the point
  of a catalogue.
- [ ] **Task 2 — the derivation.** The `Row` table (five hexes, polarity, overrides), `derive()`, the `OnceLock`, and the
  nudges until all twenty-two pass the rule test; then the four new theme tests (own face, equals-its-row, polarity, BLACK).
  This is the bulk of the round and deliberately one task: a palette alone is not a usable face.
- [ ] **Task 3 — the drop-down.** The list state (open, scroll offset), geometry (down or up, capped, rows at the card's row
  height), the swatch, the current-row dot, the wheel, the shield, the row hit-testing, the request, the caption with `…`, and
  the `Escape` order in `ui::dispatch::escape`. The six live tests.
- [ ] **Task 4 — documents.** `README` (themes rather than an inversion), the guide's `19-a-devices-card` (`COLOUR` is a
  list), `20-reference` (the names), `check.sh`'s fact list (two theme names). Pictures: a themed device with the list open,
  one other theme on the panel, the strip showing two devices in different themes.
- [ ] **Task 5 — the gate.** Three gates, `guide: check`, the five mutations, and the round's review.

## Non-goals

- **Pruning the list.** The player will review the twenty-two and name the ones to remove; that is a one-row-each follow-up, not
  this round. Shipping all of them is what makes the review possible.
- **Saving the theme.** `SavedState` is frozen at 9; a theme is gone on restart, like the name beside it.
- **Custom hexes.** The list is the list; a colour picker is a different round with a different test problem.
- **A theme per rack or per pattern.** A theme belongs to a machine.
- **A general GUI list widget.** The drop-down scrolls with the wheel because that is what this face has; a scrollbar, a drag
  thumb or keyboard navigation inside the list are not this round.

## Rulings made while writing this plan, and what they cost if wrong

| Ruling | Cost if wrong |
|---|---|
| All twenty-one page palettes ship, including the light ones | Twenty-two rows to prune, and six light faces whose lamps read as marks rather than lights — a look the player may dislike. The alternative (shipping ten) hides the ones they might have loved. |
| BLACK survives as the first entry; amber is retired | A player who liked amber has no amber; it was asked out by the player who asked for it. |
| The palette is *derived*, with per-theme overrides | A theme is a rule's idea of the page's palette; the overrides are where taste had to intervene, and they are visible in the table. |
| Polarity is a hand-set bit per theme | Six themes are declared light on my reading of the page's "canvas" advice; a wrong bit gives a face whose legends and lamps are inverted. `a_light_face_gets_dark_lamps…` proves the bit is applied, not that it was the right bit. |
| The list scrolls by rows with the wheel | No keyboard access to the list beyond `Escape`, and a mouse-less player can reach only what fits. Accepted: the card's other rows are pointer controls too. |
