# Rack documents — phase 8

**Status:** design agreed with the player on 2026-11-25, and revised once against a builder check (the seven questions it
raised and what each changed are recorded at the end). The plans argue from this document; where a plan disagrees with it,
the spec wins and the plan is wrong.

## Intent

A **song** already holds the whole rack. A **bank** is already a file. What has no document of its own is the middle amount:
the machines as a preset — build four of them, keep them, recall them in another session. That is the round this phase is
for, and it is the wishlist's *Load and save a rack of its own*, whose keys have been drawn on the card and refusing since
phase 5.

**The phase's premise was corrected before it was planned.** The handover that opened phase 8 said the session still rebuilds
a rack from one device. It does not: phase 5's round 01 put every machine in the session file (`SavedState::devices`,
`Rack::from_saved`, `Rack::replace_from_saved`, `main::session_for_disk` over `persist::rack_to_saved`), and
`tests/rack.rs` pins it (`a_saved_rack_of_three_devices_opens_as_three`). So "the rack document" here is **not** the song
carrying every machine — that is done — it is the rack as a **file of its own**. Anything that reads the old handover's
sentence as a task would rebuild what exists.

Two edits ride behind the document, because both change what a rack *is* and both were deferred until the rack had a shape:
**duplicate a device** and **drag to reorder devices**.

## What exists today

- **The song format.** `persist::SavedState` (`SAVE_VERSION` **14**) holds `devices: Vec<SavedDevice>` — `number`, `name`,
  `kind`, `scheme`, `inputs`, `bank`, `knobs`, `kit`, `swing`, `cc_map` — with the flat `bank`/`knobs`/`kit`/`swing`/`cc_map`
  as device 1's mirror for older builds. `sanitize` repairs the list (one device minimum, `MAX_DEVICES` truncation,
  duplicate numbers, duplicate and blank names, unknown `kind`/`scheme` captions, out-of-range `cc_map`).
- **The rack side.** `Rack::from_saved` / `replace_from_saved` build N devices from a document, reset the selection to 0 and
  republish `BusState`; every device is built on the rack's one **tempo cell**. `replace_from_saved` then **writes** the
  document's `bpm` into that cell (`device.rs:476`, `set_tempo_bpm`) — so a document with no tempo cannot go through it
  unchanged. `Rack::add` refuses at `MAX_DEVICES` (4), does not take the selection, and numbers from a high-water mark that
  is never reused.
- **The bank document**, which is the shape to copy: `BankDocument { version, kind, bank }` in `banks/`, with
  `bank_path`, `list_banks`, `save_bank`, `load_bank`, `delete_bank`, `free_bank_name`, `document_name` for the name, and
  `save_target` deciding in place versus new file. The card's `PATTERN BANKS` section is rows with an `x`/`SURE` cross and
  its own name field under them. `bank_file` — which file the selected device's bank came from — lives on
  `ui::SongState`, **not** on `banks::Banks`, because the card value is dropped when the card closes.
- **The rack's keys.** `RACKS` on `BANKS` holds `SAVE RACK` and `LOAD RACK`; `ui::banks::Request::{SaveRack, LoadRack}` and
  `main`'s drain answer both with a notice naming "the rack round" (`main.rs:333`). There is no `save_rack`, no `racks/`,
  and no row list. Page 12 says both keys do nothing yet.
- **One function written twice.** `list_songs` and `list_banks` are the same read-dir, strip `.json`, sort — and a rack list
  would be the third.
- **Four sentences that are already stale.** `ui/help.rs:69` ("four devices; names and extra devices are not saved yet"),
  `ui/tips.rs:400` (`ADD DEVICE`: "Names and the rack are not saved yet"), `05-the-three-cards`'s picker caption "and what
  is not saved" and the prose under it, and `ui/device_card.rs`'s module doc. Phase 5 deleted the footnote and the book
  sentence it came from; these four outlived it and contradict the panel.
- **The strip.** Cells are `ui.interact` ids by **index** today — `device-cell-{index}`, `device-mute-{index}`,
  `device-caret-{index}` — each `Sense::click()`; `Request::{Select, SetMute, Open}` carry that index. `StripCell.number`
  exists, and the panel's per-device editor slots are already keyed by it.
- **The recall has no library seam.** `main::load_song_now` is where a song's load does its work: it cannot be called from
  `tests/*_live.rs`, which is the lesson phase 7's branch review learned the hard way (the P0 that wrote over a refused
  file). `demo::load_drum_bank` is the precedent for the other answer: the decision lives in the library and `main` only
  performs the side effects.

## Goals

1. **A rack is a document.** `RackDocument` in `racks/`: the machines with their patches, banks, kits, routes, names and
   colours. Saved under a name, listed on the card, loaded, and deleted — the bank's four verbs.
2. **A recall replaces the rack.** The file's machines become the instrument. The song's name and the rack's tempo stay.
3. **The rack file carries no transport and no song.** No `bpm`, no `song`, no panel state (`midi_input`, `tooltips`,
   `autosave`, `density`, `scale`). A recall changes the machines and nothing else.
4. **The refusal reaches a third and fourth reader.** A rack file with a `version` newer than this build is refused with its
   version named, exactly as the session and the song already are.
5. **One name rule and one file-list rule.** A rack name validates with `document_name`, a new name lands beside a taken one
   with `free_name`, and the three libraries list through one function.
6. **Duplicate a device**, from the source's own saved shape.
7. **Drag a device to a new slot**, with identity intact.
8. **The book stops saying a rack is not saved**, and says what a rack file is.

## Non-goals

- **Merging a rack into the open one.** The player chose replace. Append would invent a second meaning of "rack" — a bag of
  devices — that the strip, the selection and the file all key on position for, and it needs its own rules at four devices.
- **A rack file that carries tempo.** Tempo is the song's, one cell shared by every machine; a rack that retuned the rack
  it was recalled into would be a transport change dressed as a preset.
- **Remembering the rack file's name across a restart.** The session file has no field for it, and adding one is a
  `SAVE_VERSION` bump for a convenience. A bank file's name is not remembered either; the card's two libraries behave the
  same, and the book says so.
- **A `SAVE A COPY` key on `RACKS`.** The save rule is `save_target`'s and the name field's: a different name is a new file.
  A footer key whose meaning depends on the heading above it is the key phase 5 refused for banks.
- **`IMPORT` / `EXPORT` dialogs for a rack.** `racks/` is the folder, as `banks/` is for banks.
- **Reordering by keyboard**, and a reorder that changes a device's number. Numbers are identity.
- **An arrangement, stems, a second rack, or several racks open at once.** All wishlist, all later.
- **A `SAVE_VERSION` bump.** A rack is a third *kind* of document at the same version — the bank's precedent. Nothing in the
  session, song or bank formats moves.

## Anti-goals

- **A third copy of the directory listing.** `list_songs`, `list_banks` and `list_racks` go through one private
  `list_documents(dir, subdir)`.
- **A second device-sanitising pass.** A rack document is repaired by the same `sanitize` the session uses; a hand-written
  repair for racks is the hand-kept second list this project has paid for twice.
- **A second copy of "what a device is"** for a rack document or for duplicate. Both go through
  `rack_to_saved`/`saved_device_from` and `saved_device_to_shared`.
- **A second naming rule.** A copy's name comes from `persist::distinct_name` and nothing else; `ACID 1 (2) (2)` is what
  that rule says a copy of `ACID 1 (2)` is called, and a stem-stripping variant would be a rule `rename` and `sanitize`
  would then need too.
- **Renaming an `egui::Id`.** `rack-save` is already on the face; the strip's `device-cell-{index}` stays as it is. The
  new ids are `rack-name`, `rack-load-{index}`, `rack-delete-{index}`.
- **A drag routed by a stale index.** The drag's *source* is the device's number; the destination is resolved against the
  cells as they are at release.
- **Any lock or allocation on the audio path.** Nothing here touches the callback; a load republishes `BusState` once,
  through the seam `Rack` already owns.

## Approaches considered

**How a recall meets the open session.** *Replace* (chosen by the player) or *append*. Replace is one sentence, one arm,
and the same thing a song load already does: `Rack::replace_from_saved`. Append is a new mechanic with rules for
renumbering, name collisions and a full rack, and it makes "the rack" mean the open list rather than the instrument.

**What the file's shape is.** A dedicated `RackDocument { version, devices }` (chosen) or the song's own `SavedState` with
the panel fields left at their defaults. The second is smaller to write and dishonest to read: `SavedState::bpm` has a
default, so every rack file would claim a tempo it does not own, and a reader could not tell "this preset says 130" from
"this preset says nothing". A struct with two fields says what it is.

**Where the tempo's absence is enforced.** A tempo-less document has two ways into a rack: hand `replace_from_saved` a
`SavedState` whose `bpm` is the rack's current one, or split the seam so a document's devices and a document's tempo are
applied separately. *Split* is chosen: `Rack::replace_devices_from` makes "this document brings machines, not a clock" a
statement in the code rather than a value passed carefully at one call site, and `replace_from_saved` keeps its behaviour
for songs by calling it and then setting the tempo.

**Where the version check lives.** `load_document` parses a `SavedState`; a rack is not one, but the *rule* is the one
phase 7 built. `load_document` splits into `checked_version(bytes, path)` — the probe and the comparison, `LoadError::Newer`
and all — plus a parse, and four readers go through the check (the session, a song, an import, a rack). A fourth copy of
`version > SAVE_VERSION` is the bug phase 7's round was about.

**Where a recall's work lives.** In `main` beside `load_song_now` (untestable, and the phase-7 lesson), or in the library
with `main` performing the side effects (`demo::load_drum_bank`'s precedent). *The library* is chosen: the panel resets,
the rack replacement, the dirty flag and the bank-file clearing become a function a test can call, and `main` keeps only
what needs the audio device and the MIDI system — decoding kits and saying what is missing.

**How the card grows.** `RACKS` becomes a section shaped like `PATTERN BANKS` (rows, then a name field and its key),
not two keys beside the footer and not a dialog.

## The design

### The document

```rust
// acid-app/src/persist.rs

/// Where the player's racks live: one file each, beside `songs/`, `banks/` and `kits/`.
pub const RACKS_DIR: &str = "racks";

/// A rack as a document of its own: every machine, and nothing about the song it was built in.
pub struct RackDocument {
    /// `SAVE_VERSION`, because a rack is a third kind of saved document and not a third version.
    #[serde(default)]
    pub version: u32,
    pub devices: Vec<SavedDevice>,
}

impl RackDocument {
    /// The rack as it stands. `rack_to_saved` already merges each device's live pattern and runs `sanitize`, so a rack
    /// written by this build and one arriving have passed the same gate. Its `bpm` is dropped on the floor: a rack
    /// document has no tempo.
    pub fn of(rack: &Rack) -> Self;
    /// The state a rack load's rack half takes: the devices, the flat mirror `sanitize` rebuilds from them, and every
    /// other field at its default. The `bpm` here is nobody's: `replace_devices_from` does not read it.
    pub fn into_saved(self) -> SavedState;
}

pub fn rack_path(dir: &Path, name: &str) -> PathBuf;
pub fn list_racks(dir: &Path) -> Vec<String>;
pub fn save_rack(dir: &Path, name: &str, document: &RackDocument) -> Result<()>;
pub fn load_rack(dir: &Path, name: &str) -> Result<RackDocument>;
pub fn delete_rack(dir: &Path, name: &str) -> Result<()>;
pub fn free_rack_name(dir: &Path, wanted: &str) -> String;

/// The version rule, alone: the probe and the comparison, for every document kind that has a `version`.
pub fn checked_version(bytes: &[u8], path: &Path) -> Result<u32, LoadError>;
```

`save_rack` goes through `document_name` and `write_atomic`, as `save_bank` does. `load_rack` goes through
`checked_version` and then parses; the devices are repaired by `into_saved`'s `sanitize`, the same pass a song takes.
`list_documents(dir, subdir)` is the private lift of the two identical listing bodies; `list_songs`, `list_banks` and
`list_racks` are its three callers.

### The rack's two halves

```rust
// acid-app/src/device.rs
impl Rack {
    /// Replace every machine with the ones `saved` describes, keeping this rack's tempo cell, transport and bus, resetting
    /// the selection to 0. A document's `bpm` is **not** read: this is the seam a document with no tempo goes through.
    pub fn replace_devices_from(&mut self, saved: &crate::persist::SavedState);

    /// The rack as a song load applies it: the machines, and the document's tempo.
    pub fn replace_from_saved(&mut self, saved: &crate::persist::SavedState) {
        self.replace_devices_from(saved);
        self.set_tempo_bpm(saved.bpm);
    }
}
```

`replace_from_saved`'s observable behaviour does not move — `replacing_the_rack_publishes_in_order_selects_zero_and_keeps_the_tempo_cell`
still asserts 140 after a replace — so the song path is untouched and the rack path gains a seam it can use.

### Where a recall lives

```rust
// acid-app/src/rack_recall.rs

/// What a recall changed, and what `main` still has to do about it: the kits to decode at the running device's rate, and
/// the ports the new rack names that this machine has not got.
pub struct Recalled {
    pub kits: Vec<(usize, String)>,
    pub unopened_ports: Vec<String>,
}

/// Replace the session's rack with a document's, and put the panel back on slot 0. Everything here is a decision a test
/// can make; decoding a kit and speaking to MIDI are `main`'s and stay there.
pub fn recall(rack: &mut Rack, panel: &mut PanelState, document: &RackDocument) -> Recalled;
```

`recall` does, in this order: `rack.replace_devices_from(&document.into_saved())`; `panel.close_device_card()`;
`pattern_swap::adopt`, `pending_pattern = None`; `songs.bank_file = None`; `dirty = true`; and answers the two lists
`main` needs. `main`'s `load_rack_now` is then: read the document (a notice and no change if it will not read), `save()`,
`recall(..)`, load the kits, notice the ports, set `songs.rack_file` and the card's fields, notice "loaded rack {name}".

### The card

`RACKS` becomes a section shaped like `PATTERN BANKS`:

| what | behaviour |
|---|---|
| the rows, from `racks/*.json` | a click loads that rack: twice while the session is unsaved (`rack:{name}` is the arm key) |
| each row's `x` | one click turns it into `SURE`; the second deletes the file |
| the name field | what `SAVE RACK` writes under |
| `SAVE RACK` | in place when the field names the rack the session came from, a new file otherwise, landing beside a taken name |

`ui::SongState` gains `rack_file: Option<String>` — beside `file` and `bank_file`, because it must outlive the card.
`ui::banks::Banks` gains `rack_names: Vec<String>` and `rack_name: String`, and `banks::Session` gains `rack_file`, the
same way it carries `bank_file`. `Request::SaveRack` and `LoadRack` take the name; `DeleteRack(String)` joins them. Ids:
`rack-save` (kept), `rack-name`, `rack-load-{index}`, `rack-delete-{index}`. Opening the card prefills `rack_name` from
`songs.rack_file`, as it prefills the song's field from `songs.name`.

`rack_file` is set by a rack save and a rack load; cleared by `NEW`, by a song load, and by deleting that rack file — the
three places `bank_file` is cleared. It is **not** cleared by adding, removing, duplicating or reordering a device: a rack
file is the name this rack is kept under, and editing it is what makes the session dirty, exactly as a song's name survives
its edits. It lives one session, by the non-goal above.

### What a recall does to a machine

Everything the file holds comes back as written: number, name, colour, kind, bank, patch, kit, shuffle, learned controllers
and ports. Everything a **restart** forgets, a recall forgets too, because a recall builds every machine anew exactly as a
launch does: **mute comes back off, `REVERSE` forwards, and transpose in tune.** Which MIDI ports the app has open is the
session's and does not change. A rack file with more than four machines is truncated to the first four, silently, because
only a hand-edited file can hold more and a song is repaired the same way.

### Duplicate a device

```rust
impl Rack {
    /// Another machine of the source's kind, carrying everything the file would keep of it — bank, patch, kit, colour,
    /// routes, shuffle, learned controllers — under a name made distinct by the rule every named thing in this app uses.
    /// Numbered from the high-water mark, placed last, published; it does **not** take the selection.
    pub fn duplicate(&mut self, index: usize) -> Result<usize, AddError>;
}
```

The copy is `saved_device_from(&source)` with `number` from the high-water mark and `name` put through
`persist::distinct_name` against the rack's names, then `saved_device_to_shared` on the rack's tempo cell: the file's own
mapping of "what a device is", used once more rather than written again. At `MAX_DEVICES` it answers
`AddError::AtCapacity`, the same refusal `add` gives; an index that is not there answers the same.

The names the rule gives: duplicating `ACID 1` twice gives `ACID 1 (2)` then `ACID 1 (3)`; duplicating `ACID 1 (2)` gives
`ACID 1 (2) (2)`. That is `distinct_name`, and it is the rule on purpose.

The face is a key on the device card beside `REMOVE`, labelled `DUPLICATE`, which refuses with "the rack is full" at four.
The panel then shows the copy — the rack does not move the selection itself, so the panel asks, as it already does after
`add`. `DUPLICATE` needs no `tips::Control` entry, as `REMOVE` has none.

### Drag to reorder

A reorder is the order of `Rack::devices`, and nothing else:

```rust
impl Rack {
    /// Move the device at `from` so it sits at `to`. Numbers, names and everything else travel with it; the bus is
    /// republished in the new order. Out of range answers false and changes nothing.
    pub fn move_device(&mut self, from: usize, to: usize) -> bool;
}
```

- **Identity.** A device is its `number`. A reorder does not renumber or rename anything; the selection follows the device
  that moved, so dragging the third cell to the front leaves that machine on the panel.
- **The file.** `SavedState::devices` is already strip order, so a reorder needs no format change and no `SAVE_VERSION`
  bump: the next save writes them in the new order and a restart brings them back in it.
- **The gesture.** The cell's existing interaction becomes `Sense::click_and_drag()` on the same `device-cell-{index}` id —
  one interaction, so a click and a drag do not compete for the hit test, and no id is renamed. A press that moves becomes
  a drag; `PanelState` gains `strip_drag: Option<StripDrag { number, over }>`, remembered by the dragged device's
  **number**, with the drop slot resolved from the cells as they are at release. On release the strip asks
  `Request::MoveDevice { number, to }` through `device_request` like every other strip press; `main` resolves the number
  to an index against the rack it owns and calls `move_device`. A number that is no longer in the rack moves nothing.
- **What it looks like.** The lifted cell is a stroke and the destination a marker line — shapes, never a glyph.
- **Not this round:** a keyboard reorder; a drag between windows or racks; a drag that adds or removes.

### The book

Page `12-songs-and-demos`'s `RACKS` paragraph is rewritten for a section that works, and says what a recall does to the
session — the machines replaced, the song and the tempo kept, mute and reverse reset — and that the rack's file name is
remembered for the session, not across a restart. Page `18-a-rack-of-devices` gains the rack file beside the session file.
Page `20-reference` names the new keys. Page `05`'s picker caption and prose, the help card's line, the `ADD DEVICE` tip and
`device_card.rs`'s doc lose "not saved yet". `check.sh` pins what the new pages must keep saying (a rack is a document, a
recall replaces the rack) and pins the stale claim gone. `banks-card.png` is retaken: `RACKS` with a name field and a row.

## Tests

- **`tests/persist.rs`** — a rack document round-trips every device field; its bytes hold `version` and `devices` and no
  `bpm`, `song`, `midi_input`, `density` or `scale`; a newer rack file is `Err(Newer { version, .. })` through
  `checked_version`; `into_saved`'s `sanitize` truncates a five-machine rack to four and repairs duplicate numbers, a blank
  name and an unknown `kind`; `save_rack` refuses a blank and a path-shaped name; `list_documents` gives the same answer as
  the two functions it replaces.
- **`tests/rack.rs`** — `replace_devices_from` replaces the machines, resets the selection and leaves the tempo cell's value
  where it was, while `replace_from_saved` still applies the document's tempo (the existing test, unchanged);
  `RackDocument::of(&rack).into_saved()` then `replace_devices_from` gives back the rack it was taken from, device for
  device; `duplicate` copies bank, patch, kit, colour, shuffle and routes, gives a fresh number, a distinct name, and leaves
  the selection alone; duplicating a copy is `ACID 1 (2) (2)`; at four devices `duplicate` is `AtCapacity` and the rack is
  unchanged; `move_device` reorders without touching numbers and moves the selection with the device, and out of range
  changes nothing.
- **`tests/rack_recall.rs`** (new, a library test) — `recall` on a one-machine session with a two-machine document leaves
  the document's two with their names, colours, patches and banks, selection 0, `dirty`, `bank_file` cleared, and answers
  the kits and the ports the new rack wants.
- **`tests/racks_live.rs`** (new) — the card saves the session's rack to `racks/<name>.json` and the row appears; saving
  under the same name rewrites **that** file and one under a new name lands beside a taken one; the rack row asks
  `Request::LoadRack(name)` once and, while the session is unsaved, arms first and asks on the second click; a delete takes
  two clicks and removes the file; the section is drawn when the card has room and skipped when it has not.
- **`tests/strip_live.rs`** (extended) — a drag from cell 2 to cell 0 asks `MoveDevice` with the dragged device's number and
  moves it; the dragged device stays on the panel; a click on a cell still selects it (the two senses on one id); a release
  whose number is gone moves nothing.
- **`tests/duplicate_live.rs`** (new) — `DUPLICATE` on the device card adds a machine and shows it; at four it refuses with
  the notice and the rack is unchanged.
- **`./docs/guide/check.sh`** — the pins above.
- **Mutation checks** where a test cannot be watched fail honestly: the drag's number-to-index resolution (make it use the
  index as written and the "number gone" case must fail), and `replace_devices_from`'s tempo (make it call
  `set_tempo_bpm(saved.bpm)` and the rack test must fail).

## Risks

- **A recall replaces four machines in one click.** The two-click arm while the session is unsaved is the guard, and the
  notice names what loaded. A player who wanted a merge is told no by the non-goal, which is the player's own ruling.
- **A recall unmutes every machine.** That is what a restart does and it is the loudest thing this round can do; carrying
  mute across by number would be a second identity mechanism and is not this round's.
- **A rack file's name is forgotten at restart**, so a `SAVE RACK` under a typed name that is taken lands beside it. It is
  the bank rule, and the book says it.
- **The rack file is a fourth document shape at one version number.** If a later build changes `SavedDevice`, the rack file
  moves with the session file and the version bump covers both — which is why it must not get a version of its own.
- **Two machines on one MIDI port after a duplicate.** Both play the same notes, which is what duplicating a patch is for;
  the card shows both, and the second is a click away from its own port.
- **A drag is the first pointer gesture on the strip.** Click and drag share one interaction id; the dragged device is
  remembered by number, so the release cannot move the wrong machine, but a reader of `strip.rs` must see that.
- **The card's density.** `RACKS` grows rows, a field and a key inside a card that already holds four sections. The section
  is not drawn when the budget runs out (`list_limit`), and `banks-card.png` is where a player sees that happen.

## What the builder check changed

A fresh builder read the first draft against the code and raised seven questions. All seven are answered above; each was a
real gap:

1. **`rack_file` after a restart.** It cannot survive one without a save-file field, so the non-goals now say so, the book
   says so, and the rule is the bank's. *(Non-goals, The card, Risks.)*
2. **The tempo.** `replace_from_saved` writes the document's `bpm`, so the draft's `into_saved` would have retuned the rack
   to the factory tempo on every recall. The seam is split: `replace_devices_from` for a document with no clock,
   `replace_from_saved` for a song. *(Approaches, The rack's two halves.)*
3. **The duplicate's name.** `distinct_name` on a copy gives `ACID 1 (2) (2)`, not `ACID 1 (3)`; the test list said the
   wrong one, and a stem-stripping rule would have been a second naming rule. *(Duplicate, Tests.)*
4. **The recall was untestable in `main`.** It moves to `rack_recall::recall`, with `main` doing only what needs the audio
   device and MIDI. *(Approaches, Where a recall lives, Tests.)*
5. **Mute, `REVERSE` and transpose across a recall.** The draft said they "do not change"; building every machine anew
   resets them. The spec now says a recall forgets what a restart forgets. *(What a recall does to a machine, Risks.)*
6. **The strip's ids are by index, not by number.** So the drag reuses the existing id with `click_and_drag` and carries the
   dragged device's **number** in the drag state, and no id is renamed. *(Anti-goals, Drag to reorder.)*
7. **`rack_file` belongs on `SongState`, not on `Banks`**, which is dropped when the card closes. *(What exists today, The
   card.)*

## What is left to the builder

- The exact rows of `RackDocument` on disk — `{ version, devices }` is the shape; serde's order is the test's business.
- The wording of the notices, within the shapes above.
- How far the pointer must move before a press becomes a drag, and how the lifted cell and the marker are drawn. The
  round's picture is where the player judges it.
- Where `DUPLICATE` sits on the device card, and whether it and `REMOVE` pair side by side or stack.
- The name of the module the recall lives in; `rack_recall` is the suggestion.
