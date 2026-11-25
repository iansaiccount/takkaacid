# Plan 03 — the card as a browser

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of
> the branch at the pull request (`AGENTS.md`).

**Goal:** the `BANKS` card becomes one searchable, kind-filtered, scrolling list with the save strip following the chip, so a
library of any size still finds its document and still reaches `SAVE` — the answer to round 02's review, which found that twelve
songs took `SAVE RACK` off the card.

**Spec:** [`2026-11-25-the-card-as-a-browser-design.md`](../../docs/superpowers/specs/2026-11-25-the-card-as-a-browser-design.md),
approved by the player on 2026-11-25 (`search...` confirmed as the field's hint). It is the design; this plan argues from it and
where they disagree the spec wins and the plan is wrong. The round's own choices are listed under *Rulings this plan makes*.

**Architecture:** one `Kind` catalogue (song, bank, rack, demo) answers what a row is; `rows()` turns `Banks` into the list a
frame draws; one public `Rects` object answers where everything on the card is, and the painter and both live test files read it
instead of re-spelling the arithmetic. The scroll is the theme list's idiom — a window with its floor clamped by the painter and
its ceiling by the geometry — not `egui::ScrollArea`.

**Tech Stack:** Rust, egui 0.37 / eframe 0.37. No new dependency.

## Global Constraints

- **The save file is a contract.** No field on `persist::SavedState`, no `SAVE_VERSION` bump, no change to the bytes in `songs/`,
  `banks/` or `racks/`. A task that needs one has the wrong design.
- **No `_` arm over a catalogue enum.** `Kind`, `Filter` and `DeviceKind` matches are written out in full, so a fifth kind or
  chip fails to compile where a decision is needed.
- **One mapping per question.** Which list a kind reads, what its tag says, what its arm key is: one function each, on the
  catalogue. Nothing else in the card may spell any of them a second time.
- **No font glyphs.** Everything drawn is an egui shape or ASCII text from the bundled font. `…` stays `ui::text::fit`'s and
  `settings::shorten`'s alone — the search field's hint is plain `search`, not `search…`.
- **An `egui::Id` is behaviour.** `song-{i}`, `song-delete-{i}`, `bank-{i}`, `bank-delete-{i}`, `rack-load-{i}`,
  `rack-delete-{i}`, `demo-{i}`, `song-name`, `song-save`, `song-save-copy`, `bank-name`, `bank-save`, `rack-name`,
  `rack-save`, `song-new`, `song-import`, `song-export`, `song-close` are all **kept**. The new ids are `banks-search`,
  `banks-filter-{n}`.
- **No lock, no allocation on the audio path.** This round touches no engine code and no `SharedState` atomic.
- **The three threads, the notices, and `main`'s arms are not this round's.** `Request` gains no variant and loses none; `main`
  is not edited.
- **Gates, every task:** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`,
  and `./docs/guide/check.sh` from the repo root once the book moves.

## Review Focus

- **A library too big for the card still reaches every door.** `SAVE BANK` and `SAVE RACK` are furniture in the strip, not rows
  under a list, so the round-02 failure cannot recur. The test that proves it is thirty songs plus a bank and a rack, not twelve.
- **A search that leaves nothing typed by mistake.** An empty list must say which kind of nothing it is (`NOTHING MATCHES` versus
  `NO BANKS YET`) and the strip must still work, or the card looks broken rather than empty.
- **The two files' geometry.** Round 02 recorded 31 helpers across two files mirroring one painter, and two `songs_top()` that
  meant different things. If any test still spells the card's arithmetic after this round, the round's biggest stated goal failed.
- **A wheel over the strip and a wheel over the list.** The wheel may move the rows only while the pointer is over the band;
  the strip and the footer are fixed furniture and must not move under the pointer.
- **A resize at `scale` 0.65.** Every rect inside the card, no row drawn past the band, no key clipped: the design's own risk is
  the vertical budget, and the smallest window is where it bites.

---

## What is wrong today

- **Four stacked lists, and no way to find anything.** `ui::banks::paint` draws `SONGS`, `DEMO BANKS`, `PATTERN BANKS` and
  `RACKS` in turn (`banks.rs:191`, `:256`, `:295`, `:390`); a player finds a document by reading down four lists.
- **Nothing scrolls.** Each list breaks at `list_limit = card.bottom() - padding - row_height - gap` (`banks.rs:186`), and the
  bank and rack rows stop a further row short so their own save rows still get a place.
- **Round 02's review measured what that costs once the songs are first** (`02-the-card-in-order.md`, Progress): **12 songs** →
  `RACKS`, its rows and `SAVE RACK` are not drawn; **13** → `PATTERN BANKS` and `SAVE BANK`; **21** → `DEMO BANKS` is a heading
  with no demos. `banks.rs:292`'s own comment promises the opposite.
- **The card's geometry is two hand-kept copies.** `tests/banks_live.rs` carries 18 helpers (`card`, `padding`, `row_height`,
  `heading`, `field_y`, `songs_top`, `song_row_centre`, `demos_top`, `row_centre`, `section_top`, `demo_row_centre`, `banks_top`,
  `bank_row_centre`, `bank_save_centre`, `racks_top`, `rack_row_centre`, `rack_save_centre`, `list_limit`, plus three cross
  helpers) and `tests/songs_live.rs` 13 of its own, and the two `songs_top()` do not mean the same thing. That is the hand-kept
  list `AGENTS.md` warns about, and round 02's review recorded it.
- **The song's cross arms on its bare name** (`banks.rs:227`), where a bank and a rack arm on `bank:{name}` / `rack:{name}`. One
  kind out of the rule, and the rule exists because of the collision round 01's review found.
- **Five `check.sh` facts describe the four headings**, and page 12's four paragraphs are written as four sections.

## The design

**The catalogue**, in `ui::banks`, and the only place any of it is answered:

```rust
/// What a row in the card's list is. The catalogue the chips, the tags and the saving match on, in the order `ALL` draws them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Song, Bank, Rack, Demo }

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Song, Kind::Bank, Kind::Rack, Kind::Demo];
    /// The word on the row's tag: `SONG`, `BANK`, `RACK`, `DEMO`.
    pub fn tag(self) -> &'static str;
    /// The arm key's stem: `song`, `bank`, `rack`, `demo`. Every kind has one, because every kind can be loaded, and a load
    /// that is armed must arm on its own row.
    pub fn stem(self) -> &'static str;
    /// Whether this kind is a file the player owns, and so has a delete cross. A demo is factory content: no cross, no arm.
    pub fn is_file(self) -> bool;
    /// The list this kind's rows come from, in its own order — which `persist` sorted.
    fn names<'a>(self, songs: &'a Banks) -> &'a [String];
}

/// The chips. `All` is a view over every kind; the rest are one kind each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Filter { #[default] All, Songs, Banks, Racks, Demos }

impl Filter {
    pub const ALL: [Filter; 5] = [Filter::All, Filter::Songs, Filter::Banks, Filter::Racks, Filter::Demos];
    /// The chip's word: `ALL`, `SONGS`, `BANKS`, `RACKS`, `DEMOS`.
    pub fn caption(self) -> &'static str;
    /// Which kinds this chip shows — one, or all four.
    pub fn kinds(self) -> &'static [Kind];
}

/// What a row's click arms on: `song:Acid 1`, `bank:Easy`, `rack:Studio`, `demo:3`. A demo has no file name of its own, so it
/// keys on its slot.
pub fn arm_key(kind: Kind, index: usize, name: &str) -> String;
```

**The row list, built once a frame:**

```rust
pub struct Row { pub kind: Kind, pub index: usize, pub name: String }

/// The rows the card draws, in `Kind::ALL` order, each kind in its own order, narrowed by the chip and then by the search.
pub fn rows(songs: &Banks) -> Vec<Row>;
```

`rows` applies `songs.filter.kinds()` and `name.to_lowercase().contains(&songs.search.to_lowercase())` and nothing else. A pure
function of `Banks`, so a test asks it what the card would draw without a frame.

**The card's arithmetic, one object:**

```rust
/// The width the thumb's track occupies at the band's right edge, reserved whether or not the thumb is drawn, so a row's cross
/// sits at the same x at every list size.
pub const CARD_TRACK: f32 = 4.0;

/// Where everything on the card is, in design pixels times `scale`. One function fills it; the painter draws from it and the
/// tests click from it, so the card's arithmetic is written once.
pub struct Rects {
    pub card: Rect,
    pub search: Rect,
    pub chips: [Rect; Filter::ALL.len()],
    pub band: Rect,
    pub strip_field: Rect,
    pub strip_save: Rect,
    /// `SAVE A COPY`: the song's strip has two keys, a bank's and a rack's have one.
    pub strip_copy: Option<Rect>,
    pub footer_new: Rect,
    pub footer_import: Rect,
    pub footer_export: Rect,
    pub footer_close: Rect,
    scale: f32,
}

impl Rects {
    /// Row `n` of the window, and the delete cross at its right end.
    pub fn row(&self, n: usize) -> Rect;
    pub fn cross(&self, n: usize) -> Rect;
    /// Where the kind's tag word and the row's name begin.
    pub fn tag(&self, n: usize) -> egui::Pos2;
    pub fn name(&self, n: usize) -> egui::Pos2;
}

/// Place every rect of the card inside `panel`. Pure arithmetic (1): no painting, no egui context.
pub fn card_rects(panel: Rect, scale: f32, filter: Filter) -> Rects;

/// The rows the band can show, and the window's first row after clamping the request into it — the one place that decides what
/// a window is. The painter clamps the floor, this clamps the ceiling.
pub fn window(band: Rect, scale: f32, count: usize, top: usize) -> (usize, usize);

pub struct Thumb { pub track: Rect, pub thumb: Rect }

/// The track and its thumb, or `None` when the list fits and has nothing to say about it.
pub fn thumb(band: Rect, scale: f32, count: usize, visible: usize, top: usize) -> Option<Thumb>;
```

**The face**, top to bottom, from those rects: the title and `NOW:` (unchanged); the search field, `TextEdit::singleline`, id
`banks-search`, hint `search`; one row of five chips, id `banks-filter-{n}`, the selected one lit; the band, whose rows are
`row_rect` tall with `CARD_GAP` between and scroll under the wheel; the strip, whose field and keys follow the filter; the footer,
unchanged. The band does not draw outside itself and nothing is hit-tested outside it.

**The strip**, by chip — every key names its own verb:

| chip | field (`song-name` / `bank-name` / `rack-name`) | keys |
|---|---|---|
| `ALL`, `SONGS`, `DEMOS` | `song-name` | `SAVE` `song-save`, `SAVE A COPY` `song-save-copy` |
| `BANKS` | `bank-name` | `SAVE BANK` `bank-save` |
| `RACKS` | `rack-name` | `SAVE RACK` `rack-save` |

The fields keep `save_name` / `document_save_name` and their `save_target` rules untouched; a blank field still saves the
document the session came from.

**The empty band** draws one dim line, centred: `NOTHING MATCHES` when a search is typed, and `NO SONGS YET` / `NO BANKS YET` /
`NO RACKS YET` when a chip alone finds nothing — from one function, `empty_line(filter, searching)`, matched in full.

**The vertical budget**, from the constants (`CARD_PADDING` 12, `CARD_ROW` 22, `CARD_GAP` 2) at `scale` 1.0 in a 638 px card:
title 18 → search 22+12 → chips 22+12 → **band 460 → 19 rows** → strip 22+12 → footer 22+12. Nineteen, not eighteen: the
design's estimate was close and this plan pins the arithmetic a test asserts (`>= 16`).

## Tests that make it safe

- **`rows`** (unit): `ALL` draws kinds in catalogue order and each kind in its own order; a chip narrows to its kind; the search
  narrows by substring, case-insensitively, and composes with the chip; an empty search restores; a search that matches nothing
  is empty; a demo is not a file.
- **`arm_key`** (unit): four stems, and a demo keys on its slot.
- **`empty_line`** (unit): the sentence each chip shows, and `NOTHING MATCHES` when a search is typed.
- **`window`** (unit): a band that fits everything scrolls nowhere; a band that fits `n` of `m > n` stops at `m - n`; a shorter
  band re-clamps rather than drawing past the strip.
- **`thumb`** (unit): a list that fits draws neither track nor thumb; one that overflows draws both, with the thumb a
  proportional share of the track, at the top at the top and at the bottom at the last page.
- **`card_rects`** (unit): every control inside the card at `scale` 1.0 **and** 0.65; the band between the chips and the strip;
  the band holds at least sixteen rows; a row and its cross stay inside the band and clear of the reserved track; only the song's
  strip has a `SAVE A COPY`.
- **the card (live, through `draw_panel`)**: each kind's row asks its own request; typing narrows the list to the row that loads;
  a chip narrows and the strip changes to that kind's field and keys; a wheel over the list scrolls it and a wheel over the strip
  does not; a song's cross and a rack's cross arm under `song:{name}` / `rack:{name}` and not each other; the empty band leaves
  the strip working.
- **the reachability round 02's review found**: thirty songs plus a bank and a rack — `SAVE BANK` and `SAVE RACK` answer from
  their chips, and a bank row far down the list is reached by scrolling to it.
- **the book**: `check.sh` pins what the card now is and `gone`-guards `PATTERN BANKS`, `DEMO BANKS` and the four-headings
  claims.

## Rulings this plan makes

Each is a choice the design left to the builder. Cost if wrong, stated.

| Ruling | Cost if wrong |
|---|---|
| The card's geometry is **`Rects`, public in `ui::banks`**, and `tests/common/` is a thin layer that calls it — not the design's "helpers move into `tests/common`". The anti-goal "one function answers where row `n` is" outranks the design's sentence about where the helpers live, and `device_card::theme_rows` is the precedent. | The library gains a public geometry type. A reader may think the tests could own it; the round's own anti-goal says they may not. |
| **An empty band says `NO DEMOS YET` and `NOTHING HERE YET`** for the two chips the player cannot reach empty (seven demos ship in the app, so `ALL` always has rows and `DEMOS` always has seven). Written out because a `_` arm is forbidden; see Task 1's `empty_line`. | Two sentences in the book's vocabulary that a player should never read. The alternative is a `_` arm or a panic. |
| **The strip's field keeps its per-document ids** (`song-name`, `bank-name`, `rack-name`) rather than the design's single `banks-name`. The fields are one control per document, and the anti-goal forbids renaming an id without a reason. | Three ids where the design drew one, and a grep that finds the bank's field by `bank-name` rather than by position. |
| **The chips' ids are index-based** (`banks-filter-{n}`), like `song-{i}` and `device-picker-kind-{index}`, rather than the design's `banks-filter-{chip}`. | A chip's id changes if the catalogue is reordered — as a row's already does. |
| **The search and the chip are forgotten when the card closes**, because `Banks` is dropped with it. Keeping the chip on `SongState` would be a second place the card remembers something. | A player who closes the card while browsing banks reopens on `ALL`. The design's Risks says so and the book will too. |
| **A filter or search change sets `top` to 0; a delete only re-clamps.** The design's rule, made mechanical: the painter compares the typed text before and after the frame and re-tops on any change. | Typing one character jumps the list to the top, which is what the player asked for; a delete that re-flows a list kept at `top` could hide the row under the pointer, which is why it does not re-top. |
| **`rows()` clones every name it returns.** The painter mutates `Banks` while it draws, so the list cannot borrow from it. The painter already cloned each row's name. | One small `String` per row per frame — the same order of allocation the card has always made. |
| **The wheel reads `smooth_scroll_delta`**, like the theme list's, so both lists behave alike when a hand spins a wheel — and the card's tests wheel past both ends the way `device_card_live`'s do. | A test cannot assert one notch equals one row; it asserts the ends, as the theme list's test already does. |

---

## Tasks

### Task 1 — the catalogue and the rows

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs` (the enums, `arm_key`, `empty_line`, `rows`, and the three new `Banks`
fields); tests in `banks.rs`'s own `#[cfg(test)] mod tests`.

**Interfaces:** produces `Kind` (`ALL`, `tag`, `stem`, `is_file`, `names`), `Filter` (`ALL`, `caption`, `kinds`, `Default = All`),
`Row { kind, index, name }`, `rows(&Banks) -> Vec<Row>`, `arm_key(Kind, usize, &str) -> String`,
`empty_line(Filter, bool) -> &'static str`; `Banks` gains `pub search: String`, `pub filter: Filter`, `pub top: usize`.
Consumes nothing new.

- [ ] **Step 1: Write the failing tests** in `banks.rs`'s `mod tests`, with a `library()` fixture of two songs, one bank, one
      rack and two demos — one whose name contains `acid` and one `Studio` song beside a `Studio` rack. The tests, by name:
      `the_all_chip_lists_every_kind_in_catalogue_order`, `a_chip_shows_its_own_kind_and_each_kind_keeps_its_order`,
      `the_search_narrows_by_substring_without_caring_about_case`, `the_search_and_the_chip_compose`,
      `only_the_kinds_a_player_owns_have_a_cross`, `a_row_arms_under_a_key_that_names_its_kind`,
      `the_empty_list_says_what_is_missing`. Assert exact counts (`ALL` 6, songs 2, banks 1, racks 1, demos 2), exact arm keys,
      and that each `tag()` is its `stem()` uppercased — the row's word and the key's word are one catalogue's.
- [ ] **Step 2: Run** `cargo test -p acid-app --lib banks` **and expect FAIL** (`cannot find type Kind in this scope`).
- [ ] **Step 3: Implement** the catalogue, `rows`, `arm_key` and `empty_line` as *The design* gives them; add the three `Banks`
      fields with doc comments saying what forgetting them at close means. `names` is a full match over `Kind`; `rows` is the
      chip and then the search and nothing else.
- [ ] **Step 4: Run** the file, then `cargo test --workspace` — nothing else may change behaviour, because nothing calls the new
      functions yet.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): one catalogue for what the card can list"`.

### Task 2 — the window and the thumb

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs`; tests in its `mod tests`.

**Interfaces:** produces `window(band: Rect, scale: f32, count: usize, top: usize) -> (usize, usize)` and
`thumb(band: Rect, scale: f32, count: usize, visible: usize, top: usize) -> Option<Thumb>` with
`Thumb { track: Rect, thumb: Rect }`, plus `CARD_TRACK: f32 = 4.0`. Consumes `CARD_ROW`, `CARD_GAP`.

- [ ] **Step 1: Write the failing tests**: `a_list_that_fits_scrolls_nowhere` (a ten-row band over ten and over three rows is
      `(10, 0)` / `(3, 0)`, and `thumb` is `None`); `a_list_that_overflows_stops_at_its_last_page` (a four-row band over ten:
      `(4, 0)`, `(4, 3)`, and `top = 99` clamps to `(4, 6)`); `a_shorter_band_re_clamps_the_window` (the same `top = 6` in a
      six-row band is `(6, 4)`); `the_thumb_is_proportional_and_inside_the_track` (its width is `CARD_TRACK * scale`, it sits in
      the band, its height is `visible / count` of the track, at the top when `top = 0` and at the bottom at the last page).
      A local `band_of(rows: usize) -> Rect` helper builds the band from the two constants.
- [ ] **Step 2: Run** `cargo test -p acid-app --lib banks` **and expect FAIL** (`cannot find function window`).
- [ ] **Step 3: Implement** both: `window` computes `visible = ((band.height() + gap) / (row + gap)).floor()` clamped to `count`,
      then `top.min(count.saturating_sub(visible))`; `thumb` returns `None` when `count <= visible` and otherwise places the track
      at the band's right edge minus its width and the thumb by `top / (count - visible)` of the free travel.
- [ ] **Step 4: Run** the file, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): a window over the card's list, and the mark that says there is more"`.

### Task 3 — the card's arithmetic, made public

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs`; tests in its `mod tests`.

**Interfaces:** produces `Rects` with its fields and `row` / `cross` / `tag` / `name`, and
`card_rects(panel: Rect, scale: f32, filter: Filter) -> Rects`. Consumes `Kind`, `Filter`, `Filter::ALL`, `CARD_*`.

- [ ] **Step 1: Write the failing tests**: `the_card_puts_every_control_inside_itself_at_both_sizes` (at `scale` 1.0 in
      980 × 670 and at 0.65, every rect in `Rects` is inside `rects.card`, and the chips neither overlap nor leave it);
      `the_band_sits_between_the_chips_and_the_strip` (search below the title, chips below the search, band below the chips,
      band above the strip, footer below the strip and inside the card); `the_band_holds_a_library_of_sixteen_at_the_design_size`
      (`window(...).0 >= 16`, and the last visible row's bottom is inside the band);
      `the_strip_offers_a_copy_key_only_to_the_song` (written out over `Filter::ALL`, no `_` arm);
      `a_row_and_its_cross_are_inside_the_band_and_clear_of_the_track` (each row in the band, each cross right of its row's
      centre and left of `band.right() - CARD_TRACK * scale`, rows in order without overlapping).
- [ ] **Step 2: Run** `cargo test -p acid-app --lib banks` **and expect FAIL** (`cannot find function card_rects`).
- [ ] **Step 3: Implement** `card_rects` to *The design's* vertical budget, keeping the title's `y += 18.0 * scale` and the
      footer at `card.bottom() - padding - row_height` exactly as the painter has them, and the strip one `padding` above the
      footer; `row`/`cross`/`tag`/`name` are the only places a row's position is computed. Widths: the strip's field 0.40, its
      keys 0.20 and 0.22 for the song and 0.30 for a bank or a rack, as today, and the five chips equal fifths of the card's
      width less `CARD_GAP` between them. `Rects.chips` is `[Rect; Filter::ALL.len()]` — a length derived from the catalogue, as
      `LEARNABLE` is built from `Knob::ALL`, so a sixth chip cannot leave a rect without a slot.
- [ ] **Step 4: Run** the file, then the workspace. The painter still draws the old card and must compile unchanged.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the card's arithmetic in one place, and a test that asks it"`.

### Task 4 — the face, and the tests that find it

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs` (`paint`); create `rust/crates/acid-app/tests/common/mod.rs`; modify
`rust/crates/acid-app/tests/banks_live.rs` and `rust/crates/acid-app/tests/songs_live.rs`.

**Interfaces:** consumes `Kind`, `Filter`, `Row`, `rows`, `arm_key`, `empty_line`, `Rects`, `card_rects`, `window`. Produces
`tests/common/mod.rs`: `screen()`, `panel()`, `scale_of()`, `rects() -> banks::Rects` (the `ALL` strip), `run_on`, `click`,
`row_centre(n)`, `cross_centre(n)`, `chip_centre(filter)`, `search_centre()`, `save_centre()`, `copy_centre()`,
`footer_new_centre()`, `footer_import_centre()`, `footer_export_centre()`, `footer_close_centre()`, `open_card(..)`. Every one of
them is a call into `banks::Rects` — the module owns no card arithmetic.

- [ ] **Step 1: Write `tests/common/mod.rs`** and convert `banks_live.rs` and `songs_live.rs` to `mod common;`, deleting all 31
      helpers. `open_card(songs: Banks, dirty: bool) -> PanelState` is the one constructor — the test passes the library it
      wants and the module opens the card over it, so the two files' different fixtures (`banks_live`'s one-of-each, `songs_live`'s
      a list of names) are one function with different arguments rather than two shapes.
- [ ] **Step 2: Write the failing live tests** in `banks_live.rs`, replacing the section tests:
      `every_kind_of_row_loads_what_its_tag_says` (a click on the song's row asks `Load`, the bank's `LoadBank`, the rack's
      `LoadRack`, the demo's `LoadDemo`), `a_chip_narrows_the_list_and_the_strip_follows_it` (`BANKS` then the strip's
      `SAVE BANK`; `RACKS` then `SAVE RACK`; `ALL` then `SAVE`), `the_search_narrows_the_list_to_the_row_that_loads`
      (click the search field, push `Event::Text("easy")`, then click the only row and read `Request::LoadBank`; if focus proves
      unreliable under `run_ui`, the test sets `card.search` before the frame instead and its comment says why — the narrowing
      itself is `rows()`'s and already unit-tested, and this test's job is that the field is wired to it),
      `a_search_that_matches_nothing_leaves_the_strip_working` (search `zzz`, then `SAVE` still asks for the typed name),
      `the_song_row_and_the_rack_row_arm_under_their_own_keys` (arming `song:Studio` leaves the rack's cross unarmed and vice
      versa — it replaces `a_rack_cross_and_a_song_cross_do_not_share_their_arm` and now expects `song:Studio`),
      and `a_library_of_thirty_songs_still_leaves_the_doors` (thirty songs, one bank, one rack: the `BANKS` chip's `SAVE BANK`
      and the `RACKS` chip's `SAVE RACK` both answer; it replaces `a_full_bank_list_still_leaves_save_bank_on_the_card` and
      `a_full_rack_list_still_leaves_save_rack_on_the_card`, and with them `bank_rows_drawn`, `rack_rows_drawn` and
      `list_limit`).
- [ ] **Step 3: Rewrite `paint`** to draw `Rects`, the search field, the five chips, the band's rows from `rows(songs)` through
      `window`, the empty line, the strip by filter and the unchanged footer; re-top on a chip or search change; delete
      `section`, `list_limit`, `bank_rows_limit`, `rack_rows_limit` and the four `section(...)` calls. Keep `load_row`,
      `guarded_load`, `save_name`, `document_save_name`, `now_line`, the four guard sentences and every id.
- [ ] **Step 4: Run** `tests/banks_live.rs`, then `tests/songs_live.rs`, then the workspace — the songs file's shape-walking
      test that asserts no song row is painted over the footer must now assert the band's window instead of a cap, and its
      `demo_row_centre` is a `Rects::row`.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the card is one list you can search, filtered by kind"`.

### Task 5 — the wheel, the thumb, and a library too big for the card

**Files:** modify `rust/crates/acid-app/src/ui/banks.rs` (`paint`); modify `rust/crates/acid-app/tests/banks_live.rs` and
`rust/crates/acid-app/tests/common/mod.rs`.

**Interfaces:** consumes `window`, `thumb`, `Thumb`, `CARD_TRACK`. Produces `common::wheel(ctx, panel, shared, dy: f32)`.

- [ ] **Step 1: Write the failing live tests**: `the_wheel_scrolls_the_list_and_stops_at_the_ends` (thirty songs; a wheel of
      `-60.0` over the band moves `songs.top` above 0; forty more clamp it to `count - visible` through `window`; forty the other
      way return it to 0 — the shape `device_card_live`'s theme test uses); `a_wheel_over_the_strip_does_not_scroll_the_list`
      (a wheel over `save_centre()` leaves `top` at 0); `a_delete_re_clamps_the_window_without_re_topping_it` (`top = 20` with
      thirty songs, delete one, and the window is clamped rather than reset); and
      `a_list_that_fits_draws_no_track_or_thumb` (three songs, and `thumb(...)` over the card's band is `None`).
- [ ] **Step 2: Run `cargo test -p acid-app --test banks_live` and expect FAIL** (the wheel does nothing; `songs.top` stays 0).
- [ ] **Step 3: Implement** the wheel in `paint`: read `smooth_scroll_delta.y` only while the pointer is inside `rects.band`,
      step by `(scrolled / ((CARD_ROW + CARD_GAP) * scale)).round()`, clamp the **floor** to 0 there and let `window` clamp the
      ceiling; draw `thumb(..)`'s track and handle when it says so, in the silkscreen colour dimmed, as rounded rects — a shape
      and not a glyph.
- [ ] **Step 4: Run** the file, then the workspace.
- [ ] **Step 5: Gates and commit** — `-m "feat(panel): the list scrolls, and it says when there is more of it"`.

### Task 6 — the book, the pins and the picture

**Files:** modify `docs/guide/src/12-songs-and-demos.md`, `docs/guide/src/20-reference.md`, `docs/guide/src/22-the-drum-machine.md`,
`docs/guide/src/18-a-rack-of-devices.md`, `docs/guide/check.sh`, `docs/guide/src/img/README.md`; retake
`docs/guide/src/img/banks-card.png`.

- [ ] **Step 1: Find every claim the round makes stale:** `grep -n "PATTERN BANKS\|DEMO BANKS\|RACKS\|four lists\|four sections"
      docs/guide/src/*.md`, and read each hit rather than rewriting it blind. Page 12's `## The four lists`, page 20's browser
      line, page 22's "under `DEMO BANKS`" and page 18's "`BANKS`'s `RACKS` section" are the four already known.
- [ ] **Step 2: Rewrite page 12** as the browser: the search field, the five chips in one row, one list whose rows carry their
      kind, the strip following the chip, the empty band's sentences, and the one click the design accepted (looking at `BANKS`
      and wanting to save the song means clicking `ALL` or `SONGS`). Keep the paragraphs about what a song, a bank and a rack
      *are* — those did not change — and keep the name rules and `## The buttons`.
- [ ] **Step 3: Page 20's browser line** names the card's keys as the face now spells them, and page 22 and page 18 name the
      `DEMOS` and `RACKS` chips where they named headings.
- [ ] **Step 4: Move the pins.** `PATTERN BANKS` and `DEMO BANKS` leave the must-say list for `gone` guards; add pins for the
      new card's facts, each on **one line** because `grep -F` does not flatten: the chips' row, the search field, and the
      sentence about the strip following the chip. `SAVE BANK`, `SAVE RACK`, `banks/<name>.json`, `racks/<name>.json`,
      `click again to replace the rack` and `the first list on the card` are read again as the page now says them: the last of
      them is no longer true of a searchable card and moves to a `gone` guard if page 12 stops saying it.
- [ ] **Step 5: Retake `banks-card.png`.** `DISPLAY=:N xdotool getdisplaygeometry` must say at least `2000 1400` **before** the
      launch, `xdotool getwindowgeometry` must say `1960x1340`, `TAKKA_ACID_DATA_DIR` must be fresh, the pointer parked away
      from the panel, and the card showing its search field, the five chips and the seven demo rows under `ALL` — demos are
      factory content, so this picture has rows in it and still shows a first run. Record the row in `img/README.md` with the
      state and the command.
- [ ] **Step 6: `./docs/guide/check.sh` → `guide: ok`**, the three gates, then commit — `-m "docs: the card is a browser"`.

## Non-goals

- **Fuzzy search, ranking, or matching anything but the name.** Partial text, case-insensitive, nothing else.
- **Sorting controls, multi-select, renaming from the list, dragging a row.**
- **Keyboard navigation of the list** — no arrows, no `Enter`. `Escape` still closes the card, unchanged.
- **What a document holds or how a file is written.** No `SavedState` field, no `SAVE_VERSION` bump, no change to `songs/`,
  `banks/` or `racks/` bytes.
- **`IMPORT`/`EXPORT` for banks or racks**, or any file dialog beyond the song's two.
- **The demos becoming editable**, or a demo row gaining an `x`.
- **The device card, the picker or the strip.** This round is the `BANKS` card.
- **`main`.** `Request` keeps every variant it has, so the binary is not edited; the live tests still stop at the request.

## Review focus for the pull request

1. **One geometry.** `tests/common/mod.rs` owns no card arithmetic: every helper is a call into `banks::Rects`. A helper that
   recomputes a position is a finding, and so is a `Rects` field the painter does not use.
2. **No `_` arm over `Kind` or `Filter`** — including in `empty_line`, `names`, `arm_key` and the strip's match. Adding a fifth
   chip must fail to compile.
3. **Every id kept, and the two new ones.** A renamed `song-{i}` silently breaks a drag-or-arm that a test may not catch; the
   diff should show none.
4. **The doors survive a full library.** The thirty-song test is the round's reason for existing; a version of it that clicks a
   chip it does not then check is not the test.
5. **Nothing outside the band is clickable, and nothing is drawn past the footer** — the shape-walking test in `songs_live.rs`
   is this round's guard on both.
6. **The book, the face and the pins agree.** A page that still says `PATTERN BANKS`, or a picture of the old card, is a finding
   `AGENTS.md` names as a bug.

## Progress

*(One line per task, written as it is done: what landed, the test that proved it, and any ruling with what it costs if wrong.
The branch review's findings and what was done about each go here too, before the pull request.)*
- **Task 1 — the catalogue and the rows.** `ui::banks` now holds `Kind` (`ALL`, `tag`, `stem`, `is_file`, `names`), `Filter`
  (`ALL`, `caption`, `kinds`, `Default = All`), `Row`, `rows(&Banks)`, `arm_key` and `empty_line`; `Banks` gains `search`,
  `filter` and `top`, each with a doc comment saying what forgetting it at close means. `Filter::kinds` derives its five views
  from `Kind::ALL` by slicing, so a sixth kind cannot leave a chip answering for nothing. Proved by seven unit tests in
  `banks.rs`'s `mod tests` over a `library()` of two songs, one bank, one rack and two demos — `Studio` a song *and* a rack,
  `Fast Acid` a demo a lower-case query finds — watched fail first as `cannot find type Kind in this scope`,
  `cannot find type Row` and `cannot find function rows`: exact counts (`ALL` 6, songs 2, banks 1, racks 1, demos 2), exact
  arm keys including `demo:3`, the chip's word beside its count, and every `tag()` equal to its `stem()` uppercased. Suite
  **1007/1007**, clippy `-D warnings`, `fmt --check`.
  **Ruling: `empty_line` has six sentences, not the plan's five.** The design's four (`NO SONGS YET`, `NO BANKS YET`,
  `NO RACKS YET`, `NOTHING MATCHES`) plus the two this plan added (`NO DEMOS YET`, `NOTHING HERE YET`) is six, and both
  additions are needed: `ALL` empty and `DEMOS` empty are two different chips. The plan's number was its own arithmetic, not a
  decision. Cost if wrong: none — the two sentences the ruling names are both there and both unreachable in the shipped app.
  **Ruling: `Kind::names` is private and takes no explicit lifetime.** The design wrote `fn names<'a>(self, songs: &'a Banks)`
  and `-D warnings` refuses the elidable lifetime. It is a `pub fn` in neither the design nor the plan, and nothing outside
  `rows` asks a kind for its list. Cost if wrong: a later caller wants it and widens it, which is one word.
- **Task 2 — the window and the thumb.** `window(band, scale, count, top) -> (visible, top)` is the one place that decides
  what a window is — the painter clamps the floor, this clamps the ceiling — and `thumb(band, scale, count, visible, top) ->
  Option<Thumb>` draws the track and handle only when the list overflows, with `CARD_TRACK: 4.0` the width reserved at the
  band's right edge whether or not it is drawn. Proved by four unit tests over a local `band_of(rows)` built from `CARD_ROW`
  and `CARD_GAP`, watched fail first as `cannot find function window` / `cannot find value CARD_TRACK`: a ten-row band over
  ten and over three rows is `(10, 0)` / `(3, 0)` with `thumb` `None`; a four-row band over ten is `(4, 0)`, `(4, 3)` and
  `(4, 6)` at `top = 99`; the same `top = 6` is `(6, 4)` in a six-row band and `(10, 0)` in a ten-row one; and the handle is
  `CARD_TRACK` wide and `visible / count` of the track, at the top at `top = 0`, at the bottom on the last page. Suite
  **1011/1011**, clippy `-D warnings`, `fmt --check`.
  **Ruling: the handle's share is asserted to a hundredth of a pixel, not to the bit.** A `Rect`'s height is `max.y - min.y`,
  so placing the handle re-rounds its height at the y it sits at: `94 × 0.4` reads back as `37.600006` against the direct
  `37.600002`. The code is the proportional one the design asked for; the test's equality was the wrong instrument. Cost if
  wrong: a defect inside a hundredth of a pixel of the proportion would pass, and the painter cannot draw one.
- **Task 3 — the card's arithmetic, made public.** `Rects` (card, search, `chips: [Rect; Filter::ALL.len()]`, band,
  `strip_field`, `strip_save`, `strip_copy`, the four footer keys, a private `scale`) with `row` / `cross` / `tag` / `name` as
  the only places a row's position is computed, and `card_rects(panel, scale, filter)` placing it — the band is what the fixed
  furniture leaves between the chips and the strip, 460 px at the design size, nineteen rows. Proved by five unit tests, watched
  fail first as `cannot find type Rects` / `cannot find function card_rects`: every rect inside the card at scale 1.0 **and**
  0.65 with the chips neither overlapping nor leaving it; search under the title, chips under the search, band above the strip,
  strip above the footer, footer inside the card, the field and its keys on one row; `window` giving `>= 16` rows with the last
  one inside the band; `strip_copy` present exactly for `ALL`/`SONGS`/`DEMOS`, written out over `Filter::ALL`; and each row and
  its cross inside the band, the cross right of its row's centre and clear of the reserved `CARD_TRACK`. Suite
  **1016/1016**, clippy `-D warnings`, `fmt --check`.
  **Ruling: a `Door` catalogue, so the strip is answered once.** `card_rects` needs the chip only to know the strip's shape
  (`SAVE A COPY` or not, and the key's width), and `paint` then needs the field's id, its hint, the key's words and the request.
  One match over `Filter` per caller would be five answers to one question, which the plan's own Global Constraints forbid. So
  `Filter::door(self) -> Door { Song, Bank, Rack }` is the one mapping, and `Door` owns `field_id`, `field_hint`, `save_label`,
  `has_copy`, `request` and `copy_request`. It is a three-variant catalogue, not a fourth `Kind`: a demo has no door, and this
  way no arm is dead and no `_` is needed. Cost if wrong: one public type the design did not name, and the book must describe
  the strip as the chips describe it — which the `Door` doc comment already does.
