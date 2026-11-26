# Cards that explain themselves — hover help on every card

**Status:** approach chosen by the player on 2026-11-25 ("B — every card"). This document is the design; the plan argues from
it. It was written after the conversation that chose the scope, and a builder check read it before the player did.

## Intent

The `BANKS` card's keys say what they are and nothing about what they would do. `SAVE BANK` is the sharpest case: whether it
rewrites a file or makes a new one, and *which* bank it is talking about, depend on state the key does not show — what is in
the field beside it, and which bank the selected device's bank came from. A player has to remember that, or guess, or click
and find out.

The panel already answers this question everywhere else. Knobs, the transport row, the tape deck, the strip, the settings
switches and the whole drum face explain themselves on hover, from one catalogue, in one voice, obeying one switch. **No card
does.** The device card's `COLOUR`, `KIT`, `REMOVE` and `CLOSE` are silent, the kit browser's two keys are silent, and every
key on the `BANKS` card is silent — the single exception is a device's `MUTE`, which borrows the strip's tip. Guide page 13
already tells the player that turning `TOOLTIPS ON` off makes "the switches and keys go quiet"; on the cards that sentence is
currently describing something that was never said.

So: every card explains itself on hover, in the panel's existing voice, and the keys whose effect depends on hidden state say
what they would actually do — naming the document they would write.

## What exists today

- **One catalogue.** `ui::tips` holds `Tip { title, what, shortcut }` (all `&'static str`), `Control` (49 variants), `TIPS`
  (positionally coupled to `Control::ALL` by an index table), `Control::tip()` (one exhaustive `match`), and three mappings that
  cross into other catalogues (`for_knob`, `for_dial`, `for_drum_key`) so the tip table names no view type.
- **One switch, one entry point.** `text_for(wants_help, control) -> Option<String>` and
  `hover(&Response, wants_help, control)`, which uses egui's own `Response::on_hover_text` — so the hint has the usual hover
  delay, follows the panel's scale, and stays out of a drag's way.
- **Who is covered.** `row_knobs`, `controls` (the transport row), `tape` (deck keys), `strip` (cell, `M`, caret, `+`),
  `settings` (the two switches), `drum_panel` (nine), the two readouts in `ui::mod`, and one `MUTE` on a device's card.
- **Who is not.** `banks::paint` — nine keys, none tipped, and it is not even given the flag: its signature is
  `paint(ui, panel, scale, songs, session, kind)`. `kits::paint` and `device_card::paint` do hold `PanelState`, so they can
  read `state.tooltips` already.
- **The words are compile-time.** Every `Tip` field is `&'static str`, and `TIPS` is a `const` array — so "which bank" cannot
  be written into the catalogue. It has to be computed beside the key and added to the text.
- **Tooltips are testable.** `tests/drum_live.rs` already hovers a control across five frames, advancing `input.time`, and
  reads the tip's galley out of the painted shapes; `tests/tips_live.rs` carries the same `painted_text` walk.
- **The one function that already answers "which file".** `persist::save_target(current: Option<&str>, typed: &str) ->
  Option<SaveTarget>` decides in place vs new file from two strings and needs no directory. `main` calls it; the card does not.

## Goals

1. **Every key on every card says what it does on hover**, in the catalogue's voice: a title, one sentence, and the key when it
   has one. `BANKS`, the device card and the kit browser.
2. **A key that writes a document names it.** The strip's four doors say which name they would write and whether that rewrites a
   file or makes a new one; a row's cross says which file it deletes; `EXPORT` says which session it writes.
3. **The hint obeys `TOOLTIPS`**, like every other one.
4. **One catalogue still.** A card key's identity and words live in `ui::tips`; nothing on a card spells its own help a second
   time, and no second table appears.
5. **The document-facing facts come from the functions that already decide them**, not from a second copy of the rule.

## Non-goals

- **A tip on a field.** `NAME`, the strip's save field and the search field stay silent: a hint that follows the pointer into a
  field you are typing in is in the way, and each field's caption and hint already say what it wants. `device_card.rs` says the
  same thing in so many words ("a field has no tooltip") and stays true.
- **A tip on the chips.** A chip's effect is the list redrawn under the pointer; a hint that explains the obvious is noise on a
  card whose subject is finding things.
- **A tip on an engraving.** `title`, `NOW:`, `FACTORY`, `MY KITS` are painted words, not controls a pointer can land on — the
  catalogue's own rule.
- **A second catalogue, or a per-card table.** The card keys join `Control`, appended, as every control since `Tempo` has.
- **Changing the tooltip's look, delay or placement**, or adding a switch for cards.
- **Keyboard shortcuts for card keys.** None of them has one; inventing one for a tip to name would be a lie.
- **A picture of a tooltip.** A hint needs the pointer on the panel, and this book's pictures park it away — so the guide
  describes this in prose and `img/README.md` gains no row.

## Anti-goals

- **A tip whose text is a second opinion about a rule.** If the tip says "in place when the name has not changed" in its own
  words *and* derives that from its own comparison, the day `save_target` changes the tip lies. The tip asks `save_target`.
- **A tip that reads the disk.** The card never touches the files (`main` owns them), and it cannot know a name is taken
  without a directory read. So no tip promises `Easy (2)`.
- **A tip nobody can reach.** Every entry must be attached to a real response on a real card, and a test must hover it.
- **Words that break the catalogue's rules**: a duplicate title, a non-ASCII character, a sentence shorter than three words.

## Constraints

- **No `_` arm.** `Control::tip()` is one exhaustive `match`; `ALL` and `TIPS` are index-coupled and a test pins the coupling.
  New entries are **appended**, and the drum face's length assertion and its `&Control::ALL[40..]` slice must move with them.
- **No font glyphs.** Titles and sentences are ASCII; the catalogue's own test enforces it.
- **The save file is untouched.** No `SavedState` field, no `SAVE_VERSION` bump, no change to `songs/`, `banks/` or `racks/`.
- **`Request` gains nothing, and `main.rs` is not edited.** The flag reaches the card through `ui::mod`.
- **Three threads.** The card is UI-thread only; a `String` per hovered key per frame is a paint-time cost, not an audio-path
  one, and is the same order as the row names the card already clones.
- **An `egui::Id` is behaviour.** No id changes; a tooltip hangs off a `Response`, not an id.

## Approaches considered

**Scope** (put to the player, who chose B):

- **A — the `BANKS` card only.** The nine keys. Cheapest, ends where the complaint started, and leaves the other two cards
  silent — so a player who learns to hover `SAVE BANK` hovers `COLOUR` and gets nothing.
- **B — every card** (`BANKS`, the device card, the kit browser). One helper, one voice, the gap in the panel closed. Chosen.

**Mechanism** (settled here, since the shape follows from the catalogue):

- **A second catalogue for card keys.** Rejected: it is the hand-kept second list `AGENTS.md` names, and the two would drift on
  the first rename.
- **Static sentences only.** Rejected: it answers "what does this key do" but not the question the player actually asked.
- **A computed field on `Tip`.** Rejected: `Tip` is `Copy` and `TIPS` is a `const` array; a `String` there would end both.
- **A detail line passed at the call site.** Chosen: `Tip` stays static and shareable, the catalogue stays a table, and a
  card that needs a computed line supplies it where it computes everything else.

## The design

### The catalogue grows by seventeen

Appended to `Control` and `TIPS`, grouped per card. Titles are the catalogue's own words, in capitals; a title must be distinct
from all 49 existing ones, which is why the card's song key is **`SAVE SONG`** and not `SAVE` (the panel's `SAVE` key already
owns that title), and why **one `CLOSE`** serves all four cards.

| Control | title | what the sentence must say |
|---|---|---|
| `SongSave` | `SAVE SONG` | Writes the session's song now: in place when the field names it, as a new file when it names another. A blank field keeps the song you are on. |
| `SongCopy` | `SAVE A COPY` | Always a new file under the field's name; the song you are on is left where it is. |
| `BankSave` | `SAVE BANK` | Writes the selected device's whole bank under the field's name; the bank's own name rewrites that file, and a blank field keeps it. |
| `RackSave` | `SAVE RACK` | Writes every machine in the session under the field's name, as one rack file. |
| `CardNew` | `NEW SONG` | Throws the session away and starts from the starter phrase; asks twice while the session is unsaved. |
| `SongImport` | `IMPORT` | Copies a song file from your own disk into the library and loads it; a taken name lands beside, never over. |
| `SongExport` | `EXPORT` | Writes the session out to a file you choose, in the library's own format. |
| `CardDelete` | `DELETE` | Deletes the file this cross belongs to; takes two clicks, because a delete has no undo. |
| `CardClose` | `CLOSE` | Puts this card away; Escape and a click outside do the same. Shared by all four cards. |
| `DeviceRole` | `ROLE` | Whether the machine plays notes as well as answering a controller: `PLAY + CTRL` or `CTRL ONLY`. |
| `DevicePorts` | `PORTS` | Which of the machine's own ports it listens on; `ALL PORTS` takes every one. One tip on every port row. |
| `DeviceChannel` | `CHANNEL` | Which MIDI channel it answers: `CH ANY`, or one to sixteen. |
| `DeviceColour` | `COLOUR` | The theme the device wears; a click opens the list of twenty-two. |
| `DeviceKit` | `KIT` | The drum kit the machine plays; a click opens the kit browser. |
| `DeviceRemove` | `REMOVE` | Takes the machine out of the rack; asks once more before it goes. |
| `KitsImport` | `IMPORT KIT` | Copies a kit folder from your own disk into the library and loads it. |
| `PickerKind` | `DEVICE KIND` | One kind per row: adds a machine of that kind to the rack, up to four. The resolved line names the kind. |

That is **17** new variants — `Control` 49 → **66**, `TIPS` likewise. One `MUTE` and one `CLOSE` serve several cards each, and
nothing is added for a control that already has words.

**Fields take no tip.** `NAME`, the strip's save field and the search field stay silent: a hint that follows the pointer into a
field you are typing in is in the way, and each field's caption and hint already say what it wants. So there is no `DeviceName`
entry, and `device_card.rs`'s "a field has no tooltip" comment stays true rather than being rewritten to mean the opposite.

**The picker is a card too.** `device_card.rs` calls it one, and its `+` on the strip already explains itself — so its `CLOSE`
takes the shared `CardClose`, and its one key per kind (`ACID`, `DRUMS`) takes **one** `PickerKind` whose resolved line names the
kind, rather than one control per kind: the rows differ in their noun, not in their job.

### The resolved line

`ui::tips` gains the one formatter and nothing else:

```rust
/// A control's help with one computed line after the sentence that says what it does, and before its key.
pub fn text_with(wants_help: bool, control: Control, detail: Option<&str>) -> Option<String>
```

- `text_for(wants_help, control)` becomes `text_with(wants_help, control, None)` — same words, so no existing surface changes.
- `hover(response, wants_help, control)` becomes `hover_with(response, wants_help, control, None)`.
- `hover_with(response, wants_help, control, detail: Option<&str>)` is the new entry point: `on_hover_text` with the composed
  text.

Composition is `title`, `what`, the detail when there is one, then the shortcut. `None` and `Some("")` are the same thing.

### Where each detail comes from

Each detail is built **beside the key that uses it**, in the module that owns that card — the rule `AGENTS.md` gives for a
mapping (`persist`'s answer, `banks`'s words).

**The strip's four doors** (`ui::banks`). One function, because it is one question:

```rust
/// What the strip's key would do with this name, in the words its hover help uses.
///
/// `save_target` is the function that decides whether a save writes in place or makes a new file, and the click's path runs
/// through the same one — so the hint cannot describe a rule the save does not follow.
fn save_detail(door: Door, typed: &str, current: Option<&str>) -> String
```

| the case | the line |
|---|---|
| the door is the song's, `save_target` says in place | `Writes Acid 1 in place.` |
| `save_target` says a new file | `Makes a new file, Acid 2.` |
| the copy key (always a new file) | `Makes a new file, Acid 2.` |
| a blank field with nothing open, on `SAVE` | `No name yet: the key opens the field.` |
| a blank field with nothing open, on `SAVE A COPY` | `Type a name first: this key is dim until you do.` |

`SAVE A COPY`'s door is the song's, so it asks `save_target` too and overrides only the in-place case, which it cannot produce. It
has one more state than `SAVE`: with a blank field and nothing open it is **dim and does nothing** — it never opens the field the
way `SAVE` does — so its line says that instead of promising a click that will not happen. A blank field with a song open gives it
that song's name as a new file, and the landing-beside is `main`'s (`Acid 1 (2)`), which no hint names.

A bank's and a rack's door ask it with `session.bank_file` / `session.rack_file`.

**A row's cross** (`ui::banks`): `Deletes the song Studio.` / `... bank ...` / `... rack ...` — the kind's word and the file's
name, from the row that already holds both.

**`EXPORT`** (`ui::banks`): `Writes the session, Acid 1.` when the session has a name, and `The session has no name yet.` when it
does not. It names the **session**, not the field's text: `EXPORT` writes the session and `main` only *suggests* a file name from
the field, so a player who types into an untitled session's field can light the key while the header still reads
`NOW: untitled`. Naming the field's text there would contradict the card's own header, so the hint and the header agree and the
lighting rule is allowed to disagree with neither.
**`REMOVE`** (`ui::device_card`): `Takes ACID 2 out of the rack.` — the device's own name.

**A picker row** (`ui::device_card`): `Adds an ACID machine.` — the kind that row offers.

Everything else takes no detail: their sentences are complete without one.

### The flag reaches the cards

- `banks::paint` gains one parameter, `wants_help: bool`, after `session`. `ui::mod` copies `state.tooltips` out before it borrows
  the card, exactly as it copies `bank_file` and `rack_file` for the same reason.
- `kits::paint` and `device_card::paint` already hold `PanelState`; they read `state.tooltips` where they already read
  `state.device_card`.

### What a player sees

```
BANKS                                        NOW: Acid 1 - SAVED
[ search                                   ]
 ALL   SONGS   BANKS   RACKS   DEMOS
 ─────────────────────────────────────────────────────────────
  SONG   Acid 1                                           x
  BANK   Easy                                             x
  DEMO   Slow Acid
 ─────────────────────────────────────────────────────────────
[ Acid 1                          ]  [ SAVE ]  [ SAVE A COPY ]
```

with the pointer resting on `SAVE`:

```
SAVE SONG
Writes the session's song now: in place when the field names it, as a new file when it
names another. A blank field keeps the song you are on.
Writes Acid 1 in place.
```

and on the `x` of the `Easy` row:

```
DELETE
Deletes the file this cross belongs to; takes two clicks, because a delete has no undo.
Deletes the bank Easy.
```

## Tests that make it safe

- **The catalogue's own tests** keep holding: `ALL`/`TIPS` the same length, a distinct title for every control, ASCII, three
  words or more. Their pinned numbers move with the append: `Control::ALL.len()` 49 → 66, and the drum face's
  `&Control::ALL[40..]` becomes `&Control::ALL[40..49]` so it keeps meaning "the nine drum controls".
- **`text_with`** (unit): `None` gives exactly `text_for`'s text; a detail lands on its own line after the sentence and before
  the key; `Some("")` gives no line.
- **The `BANKS` card** (live, `banks_live.rs`): resting the pointer on `SAVE` shows the tip's sentence *and* `Writes Acid 1 in
  place.`; on `SAVE BANK` with the field naming another bank, `Makes a new file, Easy 2.`; on `SAVE BANK` with a blank field
  and `bank_file` set, the bank's own name **in place**; on a row's cross, the file's name; with `tooltips = false`, nothing at
  all. The hover helper advances `input.time` and runs several frames, as `drum_live.rs` does, because egui waits for the hand
  to settle.
- **The device card** (live, `device_card_live.rs`): `COLOUR` and one port row answer, and the port rows all answer with the
  same tip.
- **The kits** (live): `IMPORT KIT` answers.
- **The picker** (live): one kind row answers with the kind's name in its line, and its `CLOSE` answers.

## Risks

- **A tip can cover the thing it describes.** egui places tooltips away from the pointer, but the card is small; the strip's
  keys are 22 px tall and the hint is three lines. **Judged by eye and recorded, not tested**: the plan's last task takes an
  offscreen grab on `Xvfb` with the pointer resting on `SAVE BANK` and one on `CLOSE` (the two shapes of the strip's and the
  footer's rows), the executor reads it, and the PR carries what it saw. The guide cannot hold that picture — its own recipe parks
  the pointer away from the panel — so the player judges the overlap where they will really meet it, on their own screen.
- **The catalogue's order is load-bearing.** Two numbers in the existing tests encode positions; appending is what keeps them
  honest, and the append is why the drum face's length and slice must be edited in the same commit.
- **A sentence that goes stale.** Every card key whose behaviour changes will need its sentence changed too, and no test can say
  a sentence is *true*. The mitigation is that the *facts* (which file, in place or not) are computed, and only the rule prose is
  hand-written.
- **`hover_with` runs for every key, every frame.** Seventeen `format!`s on a card whose rows already clone their names. If it
  ever shows up in a profile, the fix is to build the detail inside a closure; that is left alone until it does.

## What is left to the builder

- The exact prose of the seventeen sentences and the computed lines, within the notes in the table above, in the catalogue's
  voice: a title in capitals, one plain sentence, no second person, no glyph.
- The order the new variants are appended in — grouped per card, `BANKS` first, is the suggestion.
- Whether `save_detail` lives on `Door`, on `Banks`, or as a free function in `ui::banks`. It is one function either way; the
  module is what matters.
- How the live tests park the pointer so a tooltip appears without a preceding drag: `drum_live.rs`'s `hover` helper is the
  precedent to copy, and it may as well move into `tests/common/mod.rs`, where the card's geometry already lives.
- Whether the phase overview gains a line for this round, or it takes its own phase.

## What the book must say

- **`12-songs-and-demos`**: a paragraph on the card explaining itself, with the resolved line named and one example, and the
  rule that a hint obeys `TOOLTIPS`.
- **`13-settings`**: the switch's sentence already claims the keys go quiet; it becomes true, and gains the cards by name.
- **`20-reference`**: the device card's and the kit browser's key lists gain "each key explains itself on hover".
- **`check.sh`**: one new pin for the sentence that says the card explains itself, on one line, as a pin must be.
- **No picture.** A hint needs the pointer on the panel and the guide's pictures park it away, so `img/README.md` gains nothing;
  the plan's `## Progress` says so in the guide's own terms.
