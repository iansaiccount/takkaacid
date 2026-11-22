# Banks, racks and songs — phase 5's documents

Design agreed with the player on 2026-11-22, from `plans/future/wishlist.md`'s Phase 5 entry, *Save the whole rack*, *Load and
save a rack of its own*, and *A new drum machine arrives on the 808, on purpose*.

## Intent

A player can keep three different amounts of their work, and today they can only keep one of them. They can save a **song** —
which is really the first device — while a bank and a rack have no document of their own. This phase makes the three amounts
real, in the order that makes each one safe:

1. **The song holds the whole rack.** Four machines, their patches, their banks, their kits, their names, their colours and
   their ports. Without this a saved song is not a song, and exporting one exports a quarter of the rack.
2. **A bank is a file.** Sixty-four patterns, `A`..`H`, nameable, loadable into any device of either kind, and deletable.
3. **A rack is a place on the face.** The buttons for saving and loading a rack as its own preset are drawn and refuse, because
   the preset is a later round and the face should not have to be found again for it.

The player's words, describing what they wanted: "load and save pattern banks for each machine type, load and save racks —
combination of machines and their knob settings and banks, and load and save whole songs like any normal DAW." The fourth thing
a DAW's song carries — an arrangement, what plays when — is deliberately **not** this phase; it is in the wishlist, and it is
the reason the song document is designed to grow a field rather than to be rebuilt.

## What exists today

- **One document, one device.** `persist::SavedState` (`SAVE_VERSION` **11**) holds `bank`, `knobs`, `bpm`, `swing`,
  `midi_input`, `song`, `cc_map`, `tooltips`, `density`, `scale`, `autosave`, `kit`. It is written to `<data>/state.json` on
  every save and read back by `persist::open` on launch.
- **Songs are that document under a name.** `<data>/songs/<name>.json`, with `save_song`, `load_song`, `delete_song`,
  `list_songs`, `export_song`, `import_song`, `save_target`, `free_song_name`. Import never overwrites.
- **Nothing else is a document.** A bank lives inside a song; the clipboard's pattern text (`clipboard.rs`) is the only pattern
  out of the app and has its own `VERSION` line and magic.
- **A rack is four devices and cannot be written down.** `Rack::from_saved` builds exactly one `Device` from a `SavedState`;
  `main::save` and `main::session_for_disk` snapshot `rack.saved_device()` — the first device — and `main::load_song_now`
  applies a loaded song into `rack.device_mut()`, the **selected** device. So a load writes over whichever machine is selected
  and a save forgets the rest. `Device::name` and `Device::scheme` are memory-only; `InputFilter` (ports, channel, role) too.
- **The picker says so.** The `ADD DEVICE` card carries a footnote that names and extra devices are not saved yet, and page 20's
  files table repeats it.
- **The card is called `BANKS` and lists songs.** `ui::banks` paints `DEMO BANKS` and `MY BANKS`, and every row of `MY BANKS` is
  a *song file*. The panel's switch that opens the card is captioned `BANKS`. There is no bank file anywhere in the app, so the
  word is currently a lie in both directions.
- **The default kit is the 808 by accident.** `kits::fill_if_silent` gives a silent device
  `factory_kit(kits::default_name(), rate)`, and `default_name()` is `FACTORY[0]`.

## Goals

- A song saved and loaded with up to `MAX_DEVICES` (4) devices keeps every machine: kind, name, colour, ports, channel, role,
  patch, bank and kit.
- A song written by this build still opens, with device 1 intact, in a build that knows version 11.
- A bank document that is named, saved, listed, loaded into the selected device of either kind, and deleted — through the card,
  with the same two-click guard the demo rows already have.
- Rack save/load keys on the card that are drawn, refuse, and say which round will make them work.
- The card's words tell the truth: songs are songs, banks are banks.
- The 808 is the stated default kit, not the first row of a table.

## Non-goals

- **An arrangement.** No timeline, no sections, no song mode. The wishlist entry says what it would cost; the song document is
  designed so that adding an `arrangement` field later is a bump, not a rewrite.
- **A rack document that loads.** The keys are drawn and refuse; `save_rack`/`load_rack` do not exist this phase.
- **A single-pattern file.** The wishlist keeps it. The clipboard already moves a pattern, and a third document shape in one
  round is how a save format grows a field nobody can explain.
- **Refusing a file from a newer build.** The wishlist's *A file from a newer build is refused, not relabelled* is still open,
  and a v12 file opened by a v11 build is exactly its case. It is not fixed here; see **Risks**.
- **Merge or recall semantics for a rack**, **bank reordering**, **bank deletion beyond the file**, **autosave for banks**.

## Anti-goals

- A second save path beside `save_target`. A bank save decides in-place vs new file the same way, in one function.
- A `devices` list that is written but not read, so a file claims four machines and a session opens one.
- Renaming an `egui::Id` anywhere in the card: the drag-and-click tests route by id and the ids are behaviour.
- Any lock, allocation or `Arc` drop added to the audio path. Nothing in this phase touches the callback.
- A bank file that can silently replace a bank the player has not saved: the arm makes the loss visible.

## Approaches considered

### How the song grows a rack

**(a) A list of devices, flat fields kept as device 1's mirror.** `SavedState` gains `devices: Vec<SavedDevice>`; the existing
flat `bank`/`knobs`/`kit` stay and mean device 1, so a v11 build reading a v12 file loads device 1 correctly, and a v12 build
reading a v11 file gets one device because `devices` is empty and `sanitize` rebuilds it from the mirror. *Picked.* It is the
wishlist's own decided shape, and the only one where an old build and a new file do not produce silence.

**(b) A list only, dropping the flat fields.** Cleaner file, but a v11 build cannot read a v12 song at all (missing fields are
serde errors), and every existing v11 song needs a migration on read. Costs the one property the save-file rule cares about:
a document outlives the build that wrote it.

**(c) A nested rack document inside the song** — `{ song: {...}, rack: { devices: [...] } }`. This is the shape a separate rack
file wants, and it is exactly why it should not be here yet: it commits the song's bytes to a nesting that the rack round would
then have to reuse or diverge from, and the player has already sent that round to the future.

### Where a bank file's patterns land

**(a) The selected device's bank is replaced — all 64 patterns.** `DEMO BANKS` already does exactly this (`main`'s
`LoadDemo` arm), including the two-click arm while the session is dirty and `pattern_swap::adopt`. *Picked.* One behaviour, one
sentence in the book, and the guard already exists.

**(b) Into the next free bank, or a bank chosen in the dialog.** Invents a notion of a bank slot the app does not have: a device
holds one `PatternBank` of eight named banks, all of them always present, so "free" is not a state a bank can be in.

**(c) Merged pattern by pattern.** No player asked for it, and there is no face that could show what merged.

### What a bank file says about its machine

**(a) A `kind` caption, and a mismatch is a warning.** A step holds both machines' fields (`hits`/`accents` beside the 303's
note fields), so a drum bank is not *wrong* on a 303 — it is silent, which is worse than wrong because nothing says why.
*Picked.* The file travels with the fact, the loader says it in a notice, and loading still happens.

**(b) No kind at all.** Smallest file; a player who loads a drum bank on the acid face gets silence and no explanation, and the
card cannot file the rows under a heading.

**(c) A kind, and a mismatch is refused.** Honest about a document that cannot play, but it makes the app decide that the player
may not move patterns between faces — which is a rule nobody agreed to.

## The design

### The song document — `SAVE_VERSION` 12

```rust
pub const SAVE_VERSION: u32 = 12;

pub struct SavedState {
    // unchanged, in place, and now device 1's own mirror
    pub version: u32,
    pub bank: PatternBank,
    pub knobs: Knobs,
    pub bpm: f32,
    pub swing: f32,
    pub midi_input: Option<String>,
    pub song: Option<String>,
    pub cc_map: Vec<u8>,
    pub tooltips: bool,
    pub density: f32,
    pub scale: u8,
    pub autosave: bool,
    pub kit: Option<String>,
    // appended, so a v11 file's prefix still reads
    #[serde(default)]
    pub devices: Vec<SavedDevice>,
}

pub struct SavedDevice {
    pub number: usize,
    pub name: String,       // the strip's caption
    pub kind: String,       // `DeviceKind::name()`: "ACID" | "DRUMS"
    pub scheme: String,     // `Scheme::name()`: "BLACK" | "VINYL NOIR" | ...
    pub inputs: SavedInputs,
    pub bank: PatternBank,
    pub knobs: Knobs,
    pub kit: Option<String>,
    pub swing: f32,        // per device: the panel's SHUFFLE is each machine's own
    pub cc_map: Vec<u8>,   // per device: a learned rig belongs to the machine it teaches
}

pub struct SavedInputs {
    pub ports: Ports,       // serde-derive on the route type
    pub channel: Channel,
    pub role: Role,
}
```

- **The flat fields stay, and stay authoritative for device 1.** `upgraded` leaves them alone; `sanitize` rebuilds `devices`
  from them when the list is empty (a v11 file), and rewrites them from `devices[0]` when it is not, so the two can never
  disagree in a file this build writes.
- **Catalogue entries are stored by caption.** `DeviceKind` and `Scheme` gain `from_name(&str) -> Option<Self>` derived by
  scanning their own `ALL` (`Scheme::ALL`, `DeviceKind::ALL`) — no second list, no serde derive spelling the variants a second
  time. `sanitize` repairs an unknown caption to the default (`ACID`, `BLACK`).
- **Structural route data uses serde.** `Ports`, `Channel` and `Role` (`midi::route`) derive `Serialize, Deserialize`: they are
  data, not faces, and a second plain-JSON shape for them would be a second mapping for nothing.
- **`saved_to_shared` and `apply_saved` split.** `saved_device_to_shared(&SavedDevice, bpm) -> Arc<SharedState>` is the
  per-device half (knobs, cc map, swing, pattern); `apply_saved` keeps the rack half (tempo). `shared_to_saved` becomes
  `saved_device_from(&Device, &SavedState) -> SavedDevice` so `main` composes a rack without a second snapshot rule.
- **`Rack::from_saved` builds every device**, numbers preserved (`next_number = max(number) + 1`), and the rack's tempo from
  `bpm`. `Rack::replace_from_saved(&SavedState)` is what a song load calls: it rebuilds the device list, keeps the bus and the
  transport, and republishes `BusState`.
- **`main::load_song_now` loads a rack, not a device.** The selected-device apply goes away with the `device_mut()` call;
  the unsaved-work `save()` before a load stays, and so does the two-click guard in the card.
- **Kits load per device after the rack is built**: for each device with `kind.has_kit()`, `load_kit_now(index, name, rate)`.
  A device's kit that will not decode is a notice and keeps `Init`, exactly as today.
- **`sanitize` learns the list:** empty → one device built from the mirror, and **its kind is inferred**, because a v11 file
  records none (`state.rs` says so in as many words): `DRUMS` when the mirror's `kit` is `Some(_)` — the only thing that ever
  writes a kit name is a machine that has one — and `ACID` otherwise. A drum device that never left the `Init` kit cannot be
  recovered and opens as an acid; the file does not say, and a guess about anything else would be worse. More than
  `MAX_DEVICES` → truncated (the first four survive, because they are the ones the strip can show); `number` duplicates →
  renumbered upward; names made distinct the way the card already does it; blank name →
  `DeviceKind::default_name(kind, number)`; each device's `swing` clamped; each device's `cc_map` repaired by the same rules the
  flat one gets (too long → the factory pairing, every entry clamped to 127).

### The bank document

```rust
// <data>/banks/<name>.json
pub struct BankDocument {
    pub version: u32,   // SAVE_VERSION, the one number for every saved document
    pub kind: String,   // the caption the bank was written from
    pub bank: PatternBank,
}
```

`persist` grows `save_bank`, `load_bank`, `list_banks`, `delete_bank`, `bank_path` and `bank_name` — the same shape as the
song functions, in the same module, holding the same rules. **No `export_bank` and no `import_bank`:** a bank lives in
`<data>/banks/`, and that file is its own export — page 12 already tells a player that a file dropped into `songs/` will load,
and `banks/` is the same sentence. The cost of that ruling is that a player handing a bank to a friend has to find the data
folder; the alternative is two more keys on a card that is already four sections tall.

- **One validator for every document name.** `persist::song_name` becomes `persist::document_name` — the same rules, a name
  that is trimmed, short and not a path — and `save_song`, `delete_song`, `save_target` and the card all call it. A bank cannot
  be named something a song may not be, and there is no second list of forbidden characters anywhere.
- `save_bank` writes through `write_atomic`, like every other document.
- `load_bank` sanitises (`bank.normalise()`), and an unknown `kind` caption is kept as written rather than repaired: the file
  still loads, and the mismatch notice is what the player sees.
- **Where a bank save lands is `save_target`'s question, and the answer is the session's memory.** `SongState` gains
  `bank_file: Option<String>` beside `name` and `file` — the bank file the selected device's bank came from — set when a bank is
  saved or loaded, cleared by `NEW`, by a song load and by a change of selection (the mirror is about one device's bank and must
  not outlive that). The card holds only what is typed, `Banks::bank_name: String`, like `Banks::name` today. With them,
  `save_target(panel.songs.bank_file.as_deref(), typed)` writes in place when the name is unchanged and a new file otherwise —
  the same rule the song's field follows, so the two cannot drift.

`main` owns the two decisions the card reports: save the selected device's bank under the typed name, or load a listed one into
it. A bank save is not a dialog: it writes into the library, which is the folder `PATTERN BANKS` reads.

### The face — the `BANKS` card, four sections

The card keeps its caption and gains two sections; one existing heading is renamed so the words are true.

| section | rows | what a click does |
|---|---|---|
| `DEMO BANKS` | `demo::songs()`' names | unchanged: on an acid face the song, on a drum face that demo's groove |
| `PATTERN BANKS` | the player's `banks/*.json`, each with `SURE` delete | **new:** loads that bank over the selected device's bank, armed twice while the session is dirty, then a notice |
| `SONGS` (renamed from `MY BANKS`) | the player's `songs/*.json`, each with `SURE` delete | unchanged |
| `RACKS` | two keys, `SAVE RACK` / `LOAD RACK` | **new:** refuse with a notice naming the later round; the keys are drawn like `IMPORT` is (always enabled) so a click is answered |

- The footer keeps `NEW`, `IMPORT`, `EXPORT`, `CLOSE`, and they stay the **song's** doors. A bank is saved from its own
  heading: a short name field and one `SAVE BANK` key beside it. A footer key whose meaning depends on a heading is a key nobody
  can describe in one sentence, and a bank has no file dialog to open.
- New `Request` variants: `SaveBank(name)`, `LoadBank(name)`, `DeleteBank(name)`, `SaveRack`, `LoadRack`. Every one is
  `main`'s to answer and every one writes nothing on its own. `main`'s drain matches `Request` in full, so a variant with no arm
  does not compile — which is why this list is the round's checklist rather than a sketch.
- Every new key gets a stable `egui::Id` (`bank-save`, `bank-load-{index}`, `bank-delete-{index}`, `rack-save`, `rack-load`),
  so wiring the rack round cannot rename one.
- The list budget is the existing `list_limit`: four sections and a footer in 670 design pixels is tight at the smallest
  window. If a section does not fit, its heading is not drawn either — the card lists what fits rather than promising rows it
  cannot show, and that rule already exists for the two lists it has.

### The picker's footnote goes

`ui/device_card.rs`'s `NAMES AND EXTRA DEVICES ARE NOT SAVED YET` line is deleted in the round that saves them, and page 19's
sentence with it.

### The 808 default, stated

`kits.rs` gains `pub const DEFAULT_FACTORY_KIT: &str = "808"`, `default_name()` returns it, and a test asserts it is in
`FACTORY`. No behaviour changes; the table's order stops being a decision.

### Two seams the whole-rack round has to touch

- **Ports survive as names, and a missing one is said once.** `Ports::Named` holds the system's own port names, so a song from
  another machine names ports this one has not got: the device is deaf, exactly as the file says. Nothing is rewritten —
  dropping the names would lose the routing the day the port comes back — and the load path leaves **one** notice naming the
  ports that are not open, in the status line's one-line voice, the way a kit's per-voice problems are already summarised.
- **After a song loads, the panel shows the rack's first device.** The file records no selection, and "stay where I was" means
  nothing once the rack it pointed into has been replaced. `pending_pattern`, the open device card and the strip all reset
  against slot 0. Cost if wrong: a player who loaded a song to work on device 3 clicks once.

## What each round does

### 5.1 — the whole rack in the song (`plans/phase 5/01-the-whole-rack.md`)

1. `SavedDevice` (with `swing` and `cc_map`), `SavedInputs`, serde on the route types, `DeviceKind::from_name`,
   `Scheme::from_name`, `SAVE_VERSION` 12.
2. `persist`: `saved_device_to_shared`, `saved_device_from`, `sanitize`'s device list and the inferred kind, `upgraded` at 12.
3. `Rack::from_saved` for N devices, `Rack::replace_from_saved`, `next_number`, `BusState` republish, selection reset to 0.
4. `main`: `save`/`session_for_disk` capture every device; `load_song_now` replaces the rack; per-device kit loads; the
   missing-port notice.
5. The picker's footnote, deleted.
6. The 808 pin.
7. `panel.songs.file` is seeded from `SavedState.song` on launch. A restart forgets which file the session came from, so the
   first `SAVE` after one writes `Name (2)` beside the song it has just opened; this round rewrites that path, so it is the
   round that stops it, and a live test holds it.
8. The book: page 18, 19, 20, and the `BANKS` card's `MY BANKS` paragraph that promises one device.

### 5.2 — banks are files, and a rack has a place (`plans/phase 5/02-bank-files.md`)

1. `BankDocument`, the five `persist` functions, `bank_name` shared with `song_name`, `Banks.file`.
2. `ui::banks`: `PATTERN BANKS` with its name field, `SAVE BANK` and `SURE` delete; `SONGS`; `RACKS`; the new ids and requests.
3. `main`: save, load and delete a bank for and from the selected device, the kind-mismatch notice, and the two refusal notices
   for the rack keys.
4. The book: page 12 rewritten for three documents, page 20's files table, `check.sh` pins, the card's pictures retaken.

## Tests

- `tests/persist.rs`: version pinned at **12** with the byte pin beside it; a three-device document round-trips every field;
  `sanitize` repairs an empty list, a five-device list, duplicate numbers, duplicate and blank names, an unknown `kind` and
  `scheme` caption; a v11 document (no `devices`) loads as one device with device 1's bank and knobs, and re-saving writes both
  shapes consistently; `upgraded` at 11 leaves the mirror alone.
- `tests/songs_live.rs`: a two-device song saved through the card and loaded back gives two devices with their names, colours,
  ports, patches, swings and learned maps; loading it over a dirty session still takes the second click; a song load replaces the
  rack rather than writing into the selected device — the test selects the second device first, which is the bug this round fixes;
  a song whose device names a port this machine has not got loads deaf and says so once; a restart followed by `SAVE` keeps the
  same file instead of writing `Name (2)`.
- `tests/banks_live.rs` (new): save → the file exists and the row appears; saving again under the same name rewrites **that**
  file rather than making a copy; load → all 64 patterns land in the selected device and the dirty flag is set; a dirty session
  arms the load twice; a `DRUMS` bank on an `ACID` device loads and says so; delete takes two clicks and removes the file;
  `SAVE RACK` and `LOAD RACK` write nothing and leave a notice.
- `tests/kits.rs`: `default_name()` is `"808"` and is in `FACTORY`.
- `tests/persist.rs` (bank document): round trip, `bank_name` refusing a path, a v11-shaped document's `kind` repair.
- Mutation checks: the five that matter are `sanitize`'s truncation, the mirror rewrite, the inferred `DRUMS` kind, the
  per-device kit load, and the `kind` mismatch notice. Each is a mutation script under `.superpowers/`, and a `SKIP` is never a
  pass.

## Risks

- **A v11 build reading a v12 song.** It reads the flat mirror, so device 1 survives and the other three are silently dropped on
  its next save. This is the wishlist's *A file from a newer build is refused* case and it stays open; the version bump is a
  promise, not a guard, exactly as `tests/persist.rs` says today. Cost: a player who keeps two builds installed loses the extra
  devices of a song by opening it in the older one.
- **The card's density.** Four sections, a footer and a text field in one card. If `PATTERN BANKS` does not fit above the fold
  at 980 × 670 the section is not drawn, and the round's own screenshot is what shows it — look at the grab before it goes in the
  book, as phase 4's task 10 did.
- **`MY BANKS` → `SONGS`.** A player who has learned the old word loses it. It is the price of the word being true once bank
  files exist; the wishlist's rename carry (phase 3) is what left the two meanings in one card.
- **Loading a bank replaces work.** The arm is the guard, and the notice says what was replaced. If the player would rather a
  bank load never replaces an unsaved bank, that is a face decision to make in the round.
- **A song from another machine is partly deaf.** Ports are system names, so a shared song opens with devices that hear nothing
  until those ports exist. One notice says so; nothing is rewritten, which is the point. A bank's `kind` can also name a machine
  this build does not have — the file still loads and the mismatch notice is all the app can honestly say.

## What is left to the builder

- The exact rows of `SavedInputs` on disk (serde's tagged shape versus three plain fields) — either is fine, and the test pins
  the one chosen.
- Whether `Rack::replace_from_saved` rebuilds devices in place or swaps the vector; the audio path is rebuilt from `BusState`
  either way, and `tests/no_alloc.rs` is the gate that says when it went wrong.
- The card's section order. `PATTERN BANKS` above `SONGS` is the pick, because a bank is the thing a player reaches for while
  making a tune and a song is what they do when they are done; `DEMO BANKS` first is unchanged.
