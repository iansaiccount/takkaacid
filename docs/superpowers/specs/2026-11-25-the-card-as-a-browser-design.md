# The card as a browser — phase 8 round 03

**Status:** approach chosen by the player on 2026-11-25 (one filterable list, A of A/B/C). This document is the design; the
plans argue from it, and where a plan disagrees the spec wins and the plan is wrong.

## Intent

The `BANKS` card keeps a player's work and loads it back, and it was built one document at a time: `SONGS`, then `DEMO BANKS`,
then `PATTERN BANKS`, then `RACKS`, each a heading, each its own list, each with its own save row or none. It has **no
scrolling** — it draws what fits and stops — and **no way to find anything** except by reading down four lists.

Round 02 moved the songs under the field that saves them and the review found the cost of doing so: with the songs first, a
library of twelve songs stops the card drawing `SAVE RACK`, and thirteen stops `SAVE BANK`. A door a player cannot reach is not
a list that shows what fits; it is the card's own invariant broken. The player's answer, in their words, was the scroll "and
ideally a search filter. We can select from one or all of banks, songs or racks and partial text search", and then "let's rethink
the card completely to redesign the user journey from best practice, knowing what we know now".

So this round replaces the four stacked lists with **one list you can search, filter by kind, and scroll** — and answers "where
do I save?" with the strip that follows the kind you are looking at. It is the third redesign of this card and the first one
aimed at a library big enough to need finding.

## What exists today

- **One painter, four lists.** `ui::banks::paint(ui, panel, scale, songs, session, kind) -> Option<Request>` draws, top to bottom:
  the title and the `NOW:` line; the song's name field with `SAVE` and `SAVE A COPY`; `DEMO BANKS`; `PATTERN BANKS` and its
  `SAVE BANK` row; `SONGS`; `RACKS` and its `SAVE RACK` row; then the footer `NEW` `IMPORT` `EXPORT` `CLOSE`.
- **No scroll.** Each list breaks when it runs past `list_limit = card.bottom() - padding - row_height - gap`, and the bank and
  rack rows stop one row short so their own save rows keep their place. Nothing scrolls, and there is no wheel handling in the
  card at all.
- **State.** `ui::banks::Banks` holds `names`, `bank_names`, `rack_names`, `demos`, the three name fields, `open` (the song file the
  session is on), `message`, `delete_armed`, `load_armed`. `ui::SongState` holds `name`, `file`, `bank_file`, `rack_file`, and the
  card is dropped when it closes, so anything that must outlive it lives on `SongState`.
- **Requests.** `Load`, `LoadDemo(index)`, `LoadBank(name)`, `LoadRack(name)`, `Delete`, `DeleteBank`, `DeleteRack`, `Save`,
  `SaveCopy`, `SaveBank`, `SaveRack`, `New`, `Import`, `Export`, `Close` — every one answered by `main`, the painter writes nothing.
- **Two scroll idioms already ship.** `egui::ScrollArea::vertical()` in the settings card (`ui/settings.rs:524`), and a
  hand-rolled clamped window in the theme list (`ui/device_card.rs`: `themes.top`, `theme_rows(rows, panel, s, top)`, the wheel
  read only while the pointer is over the list, the floor clamped in the painter and the ceiling in the geometry function).
- **The card's tests find rows by arithmetic.** 19 `*_centre` helpers across `tests/banks_live.rs` and `tests/songs_live.rs`,
  mirroring `paint`'s layout in design pixels. Round 02's review recorded that the two files are two copies of one card's
  arithmetic, and that the two `songs_top()` do not even mean the same thing.
- **What the book says.** `12-songs-and-demos` describes the four lists in the card's order; `20-reference` lists the card's keys;
  `check.sh` pins `PATTERN BANKS`, `SAVE BANK`, `SAVE RACK`, `banks/<name>.json`, `racks/<name>.json`, and round 02's
  "the first list on the card".

## Goals

1. **One list.** Every document the card knows — songs, pattern banks, racks, demos — is a row in one list, in a fixed kind order,
   each row carrying its kind as a word.
2. **Search.** A field at the top narrows the list to the rows whose name contains the typed text, case-insensitively, as the
   player types. Clearing it restores the list.
3. **A kind filter.** Chips `ALL` `SONGS` `BANKS` `RACKS` `DEMOS`; `ALL` shows every kind, a chip shows one. The filter and the
   search compose: the list is `filter(search(all rows))`.
4. **Scrolling.** The list's band is fixed; the rows scroll under it with the wheel, clamped at both ends, and a row the list
   cannot show is reachable. A door is never unreachable: `SAVE BANK` and `SAVE RACK` are in the strip, not under a list that can
   run out of room.
5. **The save strip follows the filter.** `ALL`, `SONGS` and `DEMOS` show the song's field with `SAVE` and `SAVE A COPY`; `BANKS`
   shows the bank's field with `SAVE BANK`; `RACKS` shows the rack's field with `SAVE RACK`. **Every key names its own verb**, so
   no key's meaning depends on where it sits.
6. **One geometry, one module.** The card's arithmetic moves into `tests/common/`, which both live test files include, because
   every helper changes in this round anyway.
7. **The book says what the card does**, with a picture of it.

## Non-goals

- **Fuzzy search, ranking, or matching anything but the name.** Partial text, case-insensitive, nothing else: a search that
  guesses is a search a player cannot predict.
- **Sorting controls, multi-select, renaming from the list, dragging a row.** Each is its own round; this one makes the list
  findable.
- **Keyboard navigation of the list** (arrows, Enter to load). The card has never taken keys beyond `Escape` and the typed field;
  giving the list a caret is a face question of its own.
- **Changing what a document holds, or how a file is written.** No `SAVE_VERSION` bump, no new field on `SavedState`, no change to
  `songs/`, `banks/` or `racks/` bytes.
- **`IMPORT`/`EXPORT` for banks or racks**, and any file dialog beyond the two the song already has.
- **The demos becoming editable**, or a demo row gaining an `x`.
- **The device card's or the strip's layout.** This round is the `BANKS` card.

## Anti-goals

- **A second geometry for the card's rows.** One function answers "where is row `n`", and the tests ask it.
- **An id renamed without a reason.** The per-kind row ids (`song-{i}`, `bank-{i}`, `rack-load-{i}`, `demo-{i}`) and the arm keys
  (`song:{name}`, `bank:{name}`, `rack:{name}`, `demo:{index}`), and their row ids `song-{i}`, `bank-{i}`, `rack-load-{i}`,
  `demo-{i}`, stay: they are how a click and an arming are routed. The one key that moves is the song cross's, and it moves to
  `song:{name}` so a song and a rack of one name cannot share a guard — the collision round 01's review found.
- **A `_` arm over the kind catalogue.** Filtering, tagging, arming and deleting all match on the kind, written out in full.
- **A search that rebuilds the world each frame.** The row list is built once per frame from borrowed state; no clone of a bank,
  no `Arc` of a pattern, nothing that the painter does not need to paint.
- **A key whose label depends on the filter.** `SAVE BANK` says `SAVE BANK` on every chip it appears under.
- **A scroll that hides the strip or the footer.** They are fixed; only the rows move.

## Approaches considered

**A — one list you filter (chosen).** Search field, kind chips, one scrolling list, and a strip whose field and keys follow the
chip. Heads and their separate save rows go; the kind moves onto the row and into the chips.

**B — keep the four sections, add a filter and a scroll.** The shape players know survives, but there are then two filters over
four lists (which sections draw, and which rows inside each), four bands to scroll or one to clip, and the same test cost — the
geometry changes either way. It half-solves the finding problem: a player still reads down a list per kind.

**C — split the verbs into two cards.** A browser (`Open…`) and a save card (`Save as…`). The cleanest verbs, and the answer a
face with many documents reaches; here it costs a second card on a face whose rule is one at a time, and a new door to find. If
the card ever grows a second job — an arrangement, say — this is the shape to come back to.

A was chosen because it makes "find the thing" one question (type, or pick a kind), and gives the save field for the kind you are
looking through without a second card.

## The design

### The card, top to bottom

```
BANKS                                          NOW: New song - UNSAVED
[ search…                                  ]
 ALL   SONGS   BANKS   RACKS   DEMOS
──────────────────────────────────────────────────────────────────────
  SONG   Acid 1                                              x
  SONG   Studio                                              x
  BANK   Easy                                                x
  RACK   Studio                                              x
  DEMO   Fast Acid
──────────────────────────────────────────────────────────────────────
[ song name ..................... ]  SAVE      SAVE A COPY
NEW    IMPORT    EXPORT                                             CLOSE
```

- **The title and `NOW:` are unchanged**, top-left and top-right, because the session's state is the card's subject.
- **The search field** spans the card's width under the title, `TextEdit::singleline`, id `banks-search`, hint `search`.
- **The chips** are one row of keys, id `banks-filter-{chip}`; the selected one is drawn lit, like a switch. `ALL` is where the
  card opens.
- **The list** is one band between the chips and the strip. Its rows are `row_height` tall with `CARD_GAP` between, and it scrolls.
- **The strip** is one row: a field (`name` / `bank name` / `rack name`, id `banks-name` / `banks-bank-name` / `banks-rack-name`)
  and the keys for the filtered kind.
- **The footer** is unchanged: `NEW` `IMPORT` `EXPORT` `CLOSE`, with `NEW` arming twice while the session is unsaved.

### The kinds, one catalogue

```rust
// acid-app/src/ui/banks.rs

/// What a row in the card's list is. The catalogue the chips, the tags and the saving match on, in the order `ALL` draws them.
pub enum Kind { Song, Bank, Rack, Demo }

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Song, Kind::Bank, Kind::Rack, Kind::Demo];
    /// The word on the row's tag: `SONG`, `BANK`, `RACK`, `DEMO`.
    pub fn tag(self) -> &'static str;
    /// The arm key's stem: `song`, `bank`, `rack`, `demo`. Every kind has one, because every kind can be **loaded**, and a
    /// load that is armed must arm on its own row.
    pub fn stem(self) -> &'static str;
    /// Whether this kind is a file the player owns, and so has a delete cross. A demo is factory content: no cross, and no
    /// delete arm key. **A second question, so a second function** — the stem says which list a key belongs to, this says
    /// whether there is a key at all.
    pub fn is_file(self) -> bool;
}

/// The chips. `All` is a view over every kind; the rest are one kind each.
pub enum Filter { All, Songs, Banks, Racks, Demos }

impl Filter {
    pub const ALL: [Filter; 5];
    /// The chip's word: `ALL`, `SONGS`, `BANKS`, `RACKS`, `DEMOS`.
    pub fn caption(self) -> &'static str;
    /// Which kinds this chip shows — one, or all four.
    pub fn kinds(self) -> &'static [Kind];
}
```

**One row, one struct, built once a frame:**

```rust
pub struct Row {
    pub kind: Kind,
    /// The index into the kind's own list (`songs.names`, `songs.bank_names`, …), which is what a `Request` carries.
    pub index: usize,
    pub name: String,
}

/// The rows the card draws for this state, in `Kind::ALL` order and each kind's own order (which is sorted by `persist`).
pub fn rows(songs: &Banks) -> Vec<Row>;
```

`rows` applies the chip and the search — `songs.filter.kinds()` and `name.to_lowercase().contains(&search.to_lowercase())` — and
nothing else. It is a pure function of `Banks`, so a test can ask it what the card would draw without a frame.

### The scroll

The theme list's idiom, not `ScrollArea`: a **window** over the row list, held as `Banks::top: usize`.
- **A change of filter or search re-tops the list.** `top` returns to 0 whenever the chip or the typed text changes, because the
  row set is a different list and a *clamped* old position would leave the best match scrolled off the top of the band: 120 songs
  at `top = 80`, type `acid`, 40 matches, and the ceiling clamp alone would put the player at row 22. Deleting a row only
  re-clamps — the list re-flows under the hand that clicked, and jumping the view would be a surprise, not a correction.
```rust
/// The rows the list can show in `band`, and the window's first row after clamping the request into it.
pub fn window(band: Rect, scale: f32, rows: usize, top: usize) -> (usize, usize); // (visible, top)
```

- The painter clamps the **floor** (`top` can never be negative) and the geometry function clamps the **ceiling** (a window never
  starts past the last page), exactly as `theme_rows` does, so there is one place that decides what a window is.
- The wheel scrolls the list **only while the pointer is over it** — the rule the theme list already keeps, so a wheel over the
  rest of the card keeps doing whatever the card does with a wheel.
- A row's rect is `row_rect(band, scale, n - top)`; nothing is drawn or hit-tested outside the band, because `ui.interact` outside
  it would let a click on the strip hit a row.
- **The strip and the footer do not scroll.** A door is fixed furniture; only the rows move.
- **The list shows that there is more of it, and only when there is.** A 4 px track sits at the card's right edge inside the list band,
  with a thumb whose length is `visible / total` of the track, drawn **only when the rows exceed the band** — a library that fits
  looks exactly as it does today, with no line down the card. The track's width is **reserved whether or not the thumb is drawn**,
  so a row's crosses sit at the same x at every list size and one geometry answers for both cases. The thumb is a filled rounded
  rect in the silkscreen colour, dimmed: a shape, never a glyph, and it drags nothing this round — the wheel is the input.

`ScrollArea` was the alternative and is rejected here: it owns an offset the painter cannot see, and every row's own keys would
then have to be placed inside its scroll layer — where a test's arithmetic and the real positions part company.

### The strip

| chip | the strip |
|---|---|
| `ALL` | `[ song name ]` `SAVE` `SAVE A COPY` |
| `SONGS` | `[ song name ]` `SAVE` `SAVE A COPY` |
| `BANKS` | `[ bank name ]` `SAVE BANK` |
| `RACKS` | `[ rack name ]` `SAVE RACK` |
| `DEMOS` | `[ song name ]` `SAVE` `SAVE A COPY` |

`ALL` and `DEMOS` show the song's door because the session is what is always open, and saving it must never need a chip change;
`BANKS` and `RACKS` show their own because that is the thing the player is looking at. The fields keep their `save_target` rules
and their `document_save_name` fallback, unchanged.

**Cost, stated:** a player who has clicked `BANKS` and then wants to save the *song* clicks `ALL` or `SONGS` first. That is one
click, and the chips are on screen; the alternative — two save rows on screen at once — was rejected for the vertical budget and
for putting two verbs on one row.

### The empty list

When the filter and the search leave no rows, the band draws one dim line, centred: `NOTHING MATCHES` when a search is typed, and
`NO SONGS YET` / `NO BANKS YET` / `NO RACKS YET` when a chip alone finds nothing. Four short sentences, drawn as text, no glyph.
A demonstration of absence is cheaper than a player wondering whether the card is broken.

### What a click does

Unchanged in meaning, and re-routed through `Row`:

- **A row's name** loads it: `Request::Load(name)` / `LoadBank(name)` / `LoadRack(name)` / `LoadDemo(index)`, armed twice while the
  session is unsaved. Its arm key is `{stem}:{name}` — or `demo:{index}`, because a demo has no name of its own to key on.
- **A row's `x`** deletes it after a second click, for the three kinds where `Kind::is_file()` is true. Its arm key is the same
  `{stem}:{name}`. **The song's cross gets its stem too** (`song:{name}`), where today it arms on the bare name: that bare key is
  what a rack of the same name collided with in round 01's review, and naming it removes the collision at its root rather than
  leaving one kind out of the rule.
- **The strip's keys** ask what their label says.
- **The footer** is what it is.

### The tests' geometry, in one place

`tests/common/mod.rs` gains `card.rs`-shaped helpers — the card's rect, its padding, the title, the search field, the chips, the
list band, `row_centre(rows, top, n)`, the strip and the footer — and both `banks_live.rs` and `songs_live.rs` include it
(`mod common;`). Round 02's review recorded the duplication and the two `songs_top()`s that disagree; this round changes every one
of those helpers anyway, which is the moment to have one copy.

## Tests that make it safe

- **`rows`** — `ALL` draws kinds in catalogue order and each kind in its own order; a chip narrows to its kind; a search narrows
  by substring, case-insensitively, and composes with the chip; an empty search restores; a search that matches nothing is empty;
  a demo row has no stem and no cross.
- **`window`** — a band that fits everything scrolls nowhere; a band that fits `n` rows of `m > n` stops at `m - n`; a wheel past
  either end clamps; a filter or a search change puts the window back to row 0 while a delete only re-clamps; a resize (a shorter
  band) re-clamps rather than drawing past the strip.
- **the thumb** — a list that fits draws no track and no thumb; a list that overflows draws both, with the thumb's length and
  offset in proportion; a row's cross centre is at the same x whether or not the thumb is drawn (the reserved width).
- **the card (live, through `draw_panel`)** — typing narrows the list and the row that remains loads what its tag says; a chip
  narrows and the strip changes to that kind's field and keys; a wheel over the list scrolls it and a wheel over the strip does
  not; a row's cross still arms on its own key (`song:Studio` armed does not arm `rack:Studio`); the empty list says so; every key
  is inside the card at the design size and at `scale` 0.65.
- **the reachability that round 02's review found** — a library of thirty songs plus a bank and a rack still draws `SAVE BANK` and
  `SAVE RACK` (they are in the strip, so the assertion is that the strip exists and the keys answer), and the bank row is
  reachable by scrolling to it.
- **the book** — `check.sh` says the card's chips and the search exist, and that the words `PATTERN BANKS` are gone from the face.

## Risks

- **The vertical budget.** Title, search, chips, list, strip and footer in 638 px leaves the list about 430 px — some eighteen rows.
  The chips at 0.65 scale and a longer sentence in the empty state are the two things that could not fit; the round's picture is
  where that is judged.
- **A player who knows the old card.** The four headings go, and with them "scroll down to `RACKS`". The chips say the same words
  in one row, and `ALL` opens on the same order, so the news is one click wide — but it is news.
- **Search state surviving a close.** `Banks` is dropped when the card closes, so a typed search is forgotten. That is right for
  a search and wrong for a filter? Keeping the chip on `SongState` would be a second place the card remembers something; this
  round forgets both, and the book says so.
- **Two test files, one module.** `mod common;` in two integration tests is a second crate-internal file the naming has to keep
  straight (each test crate gets its own copy of the module). No behaviour, but a reader should know why the module exists.
## What is left to the builder

- The exact band arithmetic and how many rows fit at each size; the round's picture and its tests pin it.
- Whether the wheel scrolls by one row or a page — one row is the suggestion, matching the theme list's step.
- The words of the four empty-list lines, within the shapes above.
- Whether the search field clears itself when the card closes (it does, with the card) and whether `Escape` clears it before
  closing the card — `Escape` closing the card is the existing rule, and this round does not change it.
- The chip row's exact widths; equal fifths of the card is the suggestion.
