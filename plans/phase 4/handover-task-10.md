# Handover — phase 4, task 10 (for a fresh session)

> **Superseded — kept for the record, not for instructions.**
>
> Picked up and carried through in the commit after it: task 10 is done, its Progress entry is in
> `plans/phase 4/01-the-drum-machine.md`, and the live handover is `plans/phase 4/handover-phase-5.md`.

Paste this whole file as the first message. It is written to be self-contained: the session it is for has no memory of the one
that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth in Rust, at `/home/ian/Projects/takkaacid`, on branch
**`phase-4`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the project's standing instruction and outranks
anything here that contradicts it.

**The round.** Phase 4 is the **drum machine**: sixteen sample voices against the 303's sixteen steps, sharing the bar, banks,
transport, transpose, reverse and themes. The approved design is
`docs/superpowers/specs/2026-11-18-drum-machine-design.md` (the spec — it wins any argument with the plan). The plan, ten
tasks, is `plans/phase 4/01-the-drum-machine.md`; its `## Progress` is the record and its Global Constraints and Review Focus
are binding. The phase overview is `plans/phase 4/00-phase-4.md`.

**Where the branch stands.** Tasks 1–9 are done and committed; **task 10 is the last one**. `SAVE_VERSION` is **11**
(`SavedState::kit`); the rack round takes **12**. The branch is **local only** — nothing is pushed. Every task was reviewed by
a dispatched subagent before it landed, and the record of what each review found is in the plan's Progress: read the entries
for tasks 4, 6 and 8 at least, because they are the ones that moved the book.

## Your task: task 10 of the plan — the book

From the plan, verbatim (its **Files** line was extended during task 9 — see below):

> ### Task 10 — the book
>
> **Files:** create `docs/guide/src/22-the-drum-machine.md`; modify `src/SUMMARY.md`, `04-the-face.md`, `20-reference.md`,
> `19-a-devices-card.md`, `14-midi.md` and `15-keyboard.md` (both say a device's one voice is played by the keyboard, which is
> no longer true of a drum device — task 8 added the one sentence each; re-read them with the drum machine in mind),
> `01-what-it-is.md` (the intro's framing: task 8 made a drum machine audible, so "what it is not" needs the page that says what
> it now is), `docs/guide/check.sh` (facts), `12-songs-and-demos.md` (task 9 added the drum-bank paragraph, which wants the
> cross-reference to page 22 from where a player arrives at `DEMO BANKS`), and `src/img/README.md` + the screenshots.
> - [ ] Page 22: what the drum machine is, the grid, the lit plate, the knobs' relabelled meaning, `SHUFFLE`, the kit browser and
>       kits, the factory banks. Facts pinned in `check.sh`: `KICK`, `LP | BP | HP`, `MY KITS`, `Drum machine`.
> - [ ] Screenshots at 1960 × 1340 (the AGENTS.md recipe, checking the display size first): the drum face, the kit list.
> - [ ] `./docs/guide/check.sh`; PASS. Commit: `docs(guide): the drum machine, its card and its kits`.

**This is a documentation task with a test.** `./docs/guide/check.sh` is that test and is part of every gate run from now on:
it builds the book, proves every page is reachable from `SUMMARY.md`, every image a page names exists, every page a page links
to exists, and every pinned fact is still said. A page that contradicts the panel is a bug, and a stale screenshot is a
contradiction the eye believes.

## What the drum machine does now, so the pages say it

Written down here so you do not have to read four thousand lines of `## Progress` to find out what to describe. Everything
below is built and tested; if a page disagrees with it, the page is wrong.

- **A second device kind.** `DeviceKind::ALL` is `[Acid, Drums]`, `DRUMS` is the caption, and the strip's `+` opens the
  picker that offers `ACID` and `DRUMS`. A drum device gets its own state, bank, ports, channel, colour and name (`DRUMS 2`).
- **The face** (`ui/drum_panel.rs`): the seven 303 slots relabelled left to right `ATTACK DECAY SUSTAIN RELEASE CUT OFF
  RESONANCE TUNE`, the waveform switch becomes the three-way filter key `LP | BP | HP`, a `HIT` key auditions the selected
  voice (it sounds with the transport stopped), and the device's `SHUFFLE` dial sits between them. Under them: a sixteen-row
  grid, one row per voice, captioned from the catalogue (`KICK`, `SNR`, `RIM`, `CLAP`, `CHH`, `OHH`, `LOT`, `MID`, `HTOM`,
  `CRS`, `RDE`, `CWB`, `LCO`, `HCO`, `CLV`, `SHK`), a cell per step, every fourth column framed. A click arms a voice on a
  step; `SHIFT`-click accents it; clicking a row's label selects that voice for the dials; `SHIFT`-clicking the label mutes
  the row. The selected row's label is a lit plate in the accent, its row carries a faint band, and its name is printed above
  the dials. A muted row's label prints dim and its cells lose their accent. Every surface paints from the device's `COLOUR`.
- **What is gone on a drum face**: the step row and the performance row (`OCT`, `CLEAR`, `RANDOM`, density, scale). The
  switches row keeps every key (`BANK`, `PATTERN`, `COPY`, `PASTE`, `SAVE`, `BANKS`, `STEP`). `STEP` still lights and walks its
  lit state on a drum face, but the editor it opens writes nothing there — the keyboard's step editor is the 303's, gated by
  kind.
- **No notes, this phase.** A drum device ignores the computer keyboard, the step-mode note queue and MIDI notes; a key or a
  note on a drum device is silent, by decision (`14-midi` and `15-keyboard` each already carry one sentence saying so, from
  task 8). MIDI pads and per-voice CC are the spec's non-goals.
- **The card** (`19-a-devices-card`): a drum device's device card gains a `KIT` row with the kit's name; the key opens the
  **kit browser**. The other rows (ROLE, PORTS, CHANNEL, COLOUR, MUTE, REMOVE) are kind-agnostic and unchanged.
- **The kit browser** (`ui/kits.rs`): `FACTORY` then `MY KITS`, the three that ship (`808`, `909`, `Acoustic`) marked
  `- READ ONLY`, `IMPORT` and `CLOSE` along the bottom. Selecting a factory kit loads it; the first dial turn on a factory
  kit makes a copy under a free name and says so on the strip (`<original> is read-only: this is <copy> now, saved with the
  session`), while the browser's header reads `NOW: <name> - READ ONLY`. Loading over an unsaved edit takes
  two clicks, the row printing `<name> - click again to replace`.
- **Kits are folders**: `<data>/kits/<name>/kit.json` plus one `<VOICE>.wav` per sampled voice. The player's own kit can be
  imported from a WAV set; a broken voice costs one voice and is named, not the kit. A kit is `SAVE_VERSION` 11's
  `SavedState::kit` — the *name*; the samples live in the folder.
- **`DEMO BANKS` on a drum machine.** The `BANKS` card lists the same five demo names it always has (`Slow Acid`, `Hard
  Techno Acid`, `Blues`, `Rock`, `Funk`); on a drum device a click loads that demo's eight-pattern **drum bank** into the
  device, at the demo's tempo and shuffle, with the machine's kit and the session's name left alone. On a 303 a click still
  loads the whole demo song. Page 12 already says this (task 9) and links `18-a-rack-of-devices`; **page 22 is what it should
  link for the drum side**.
- **The three kits' names and per-voice settings are listening work nobody has heard**, and so are the five drum grooves —
  both records say so (`kits.rs`'s module doc and the plan's task 6 and task 9 entries). Page 22 must not claim the factory
  kits are transcriptions of an 808 or a 909: `kits/CREDITS.md` and the plan's ruling 3 say the files come from **two** CC0
  collections — Sonic Pi's sample set, each file a Freesound one-shot, and VCSL — and that the kit names are presets, not
  transcriptions. Say what a kit is; do not lend it an authenticity the files do not all support.

## What tasks 1–9 left you (signatures you will write about)

```rust
// acid-core::drums
pub enum Drum { Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom, MidTom, HighTom, Crash, Ride, Cowbell,
                LowConga, HighConga, Clave, Shaker }
impl Drum { pub const ALL: [Drum; 16]; pub fn index(self) -> usize; pub fn bit(self) -> u16;
            pub fn label(self) -> &'static str; pub fn choke_group(self) -> u8; }
pub enum FilterKind { Low, Band, High }   // labels "LP" | "BP" | "HP"
pub struct Kit { pub name: String, pub voices: [KitVoice; Drum::ALL.len()] }
pub struct KitVoice { pub sample: Arc<[f32]>, pub tune, attack_ms, decay_ms, sustain, release_ms,
                      pub filter: FilterKind, pub cutoff, pub resonance, pub mute }
pub fn default_kit() -> Kit;  pub const DEFAULT_KIT_NAME: &str = "Init";
pub struct Drums;   // new, set_kit, set_mute_mask, audition, render, set_pattern, set_bpm, set_swing, position,
                    // bar_line, start, start_at, start_later, stop, pause, resume, set_reverse, set_transpose
// acid-core::sequencer
pub trait Performer { fn step(&mut self, step: &Step); fn release(&mut self);
                      fn render(&mut self, out: &mut [f32]); fn arm(&mut self); }
// acid-app::state
SharedState { pub kind: DeviceKind, kit: ArcSwap<Kit>, drum_mute: AtomicU32, audition: AtomicI32, … }
  kit(), set_kit(Arc<Kit>)   // the EDIT path — seeds no mutes
  adopt_kit(Kit)             // the LOAD path — seeds the mute mask from the kit
  request_audition(Drum), take_audition() -> Option<Drum>
// acid-app::device
DeviceKind { Acid, Drums }  // ALL, name(), has_kit(), enabled(count)
Device::kind() -> DeviceKind      // reads `self.state.kind`; Device has NO kind field
Device::load_bank(PatternBank)    // a bank is patterns only; opens on its first
Device::publish_pattern()         // private; one spelling of "the state plays the bank's current pattern"
// acid-app::demo
pub struct Demo { pub name: &'static str, pub state: SavedState, pub drums: [Pattern; 8] }
pub fn songs() -> Vec<Demo>;  pub fn drum_banks() -> Vec<(&'static str, PatternBank)>
pub fn load_drum_bank(&mut Rack, index) -> Option<&'static str>   // bank + tempo + shuffle, nothing else
// acid-app::kits
factory_kits(rate) -> Vec<Kit>;  factory_names() -> Vec<&'static str>;  is_factory(name);  is_reserved(name);
default_name() -> &'static str;  factory_sample_bytes() -> usize
// acid-app::persist
kits_dir(); save_kit(&Kit, &Path, f32); load_kit(&Path, &str, f32) -> Result<LoadedKit, KitError>;
import_kit(&Path, &str, &Path, f32) -> Result<ImportedKit, KitError>; kit_names(&Path); free_kit_name(…)
```

Nothing in the save file moves in task 10. `SAVE_VERSION` stays **11**.

## Traps this task owns

1. **A page that contradicts the panel is a bug, and a stale screenshot is a contradiction the eye believes.** Retake every
   picture whose panel changed — at minimum the face itself (the knob row is the same picture on a 303 and different on a drum
   device) and the device card's, if a card picture shows a row set the drum machine changed. `docs/guide/src/img/README.md`
   records per image **the state and the command that took it**; a new picture gets a row, a retaken one gets its row updated,
   and both files travel with the code that changed them.
2. **The screenshot recipe is load-bearing, and the display check is not a formality.** On this machine something pre-starts
   `Xvfb` on display numbers at **1280 × 800**, and asking for a bigger screen on a taken display fails quietly ("Server is
   already active") — a whole round's screenshots were 1280 × 800 for that reason. So: pick a display, run
   `DISPLAY=:N xdotool getdisplaygeometry`, and launch only if it says `2000 1400` or more; then
   `xdotool search --name takka` and `xdotool getwindowgeometry` must say **1960x1340**. If it does not, the picture is the
   wrong size and no amount of cropping will fix it. `TAKKA_ACID_DATA_DIR` must be a **fresh** directory so the picture shows a
   first run and nothing of the photographer's own; `xdotool mousemove X Y` / `click 1` take **doubled** coordinates from
   logical ones; park the pointer away from the panel before a grab or a hover tooltip lands in the book; and a crop is a
   second grab with `-crop`, never a cut-out of a finished image, with its rect taken from the layout (`ui/layout.rs`,
   `ui/strip.rs`, `ui/device_card.rs`, `ui/drum_panel.rs`), not from reading a screenshot.
3. **Getting to the drum face in a screenshot.** A first run opens on the Slow Acid song with one `ACID` device. The strip's
   `+` opens the picker (`img/picker.png` already shows both rows); the `DRUMS` row adds the machine, and the face draws
   itself with an **empty grid** until a bank is loaded — for a picture of the grid with something in it, open `BANKS` and
   click a demo row (or press a few cells). The `KIT` key on the device card (`F3`, or the strip's caret) opens the kit
   browser for the kit-list picture.
4. **No font glyphs.** Everything on the face is an egui shape or ASCII, and the one exception is `…` (U+2026) via
   `ui::text::fit`/`settings::shorten`. Do not write `_`-joined words into a caption, and do not put `(`/`)` where the face
   prints square brackets or pipes.
5. **Prose names controls as the face spells them, in backticks** — `KICK`, `LP | BP | HP`, `HIT`, `SHUFFLE`, `KIT`, `MY KITS`.
   A claim worth pinning goes into `check.sh`'s list, which fails when the guide stops saying it; that list is why a removed
   feature cannot leave a sentence behind. The negative guards (`! grep`) are for the claims a rewrite could put *back*.
6. **Page 22 needs a cross-reference from where a reader arrives**, and one from it back out: from `12-songs-and-demos`
   (`DEMO BANKS` on a drum machine), from `04-the-face` (the second face), and from `19-a-devices-card` (the `KIT` row and
   the browser). `check.sh` check 3b fails on a link to a page that does not exist, so the page and the `SUMMARY.md` line
   land together.
7. **Do not promise controls the face does not have.** The seven dials, the `LP | BP | HP` key and `HIT` carry **no hover
   tips** (task 4's ruling 5: the tip table is the 303's, and drum variants were left for this page — adding them is a code
   change with its own test, not a page's sentence, so either add them properly or write no tip promise); `SHUFFLE` and the
   shared keys (`BANKS`, `SAVE`, `STEP`, the transport) keep the 303's tips. There is no drum step row, no `CLEAR`/`RANDOM` on
   a drum face, no MIDI pads and no per-voice CC; and `STEP` lights and toggles while writing nothing on a drum device.
8. **The `## Progress` entry is part of the task**, not an afterthought: what shipped, the mutations, what the review found and
   what was done about it, every ruling with what it costs if wrong, and — this task being the book itself — the list of pages
   it moved and the pictures it retook.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cd rust && cargo test --workspace` (expect **864** before this task, more if it adds tests),
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and `./docs/guide/check.sh` from the repo root.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Save the diff under
  `.superpowers/review/` (a new file needs `git add -N` before `git diff` sees it), hand the reviewer the task's **Files**
  block and the diff, and act on what it finds in the same commit — regenerating the diff after every fix. The reviewer
  reports; it does not edit. If the harness offers no subagent tool, stop before the commit and say so.
- **Prove each test bites.** A documentation task's "tests" are `check.sh`'s checks: a pinned fact, a page link, an image
  reference. Prove each new pin bites by removing the sentence it guards and watching `check.sh` fail. The mutation scripts
  for the round are `.superpowers/mutate_task*.py`; a *shell* check is not in them, so say in the Progress which pin was
  proven and how.
- **The plan's Progress is the record**, one entry per task, and it is how the next session resumes: `git log` and that
  section, not memory.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write a script with the editor and
  run `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside
  it, git-ignored, and is the right place for scratch scripts, diffs and commit messages
  (`git commit -F .superpowers/commitmsg.txt`). A shell *command* may still write to `/tmp` (`cargo test > /tmp/t.log`).
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect and sum:
  `cargo test --workspace > /tmp/t.log 2>&1`, then
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
  The count at task 9 is **864 passed, 0 failed**.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest, and a `cd` inside the chain leaves later commands in
  the wrong directory** — run gates as separate commands.
- **Never `git checkout <file>` to undo a scratch mutation**: it reverts to HEAD and throws away the task's uncommitted work
  in that file (it happened in task 9; the file had to be rebuilt by hand). Restore the file from a saved copy, which the
  mutation script already does in its `finally`.
- **`cargo fmt` collapses blank lines but never adds one**, and a long line it reflows can break a mutation script's `old`
  string. Read the diff after formatting, and re-run any script after it.
- **Editing anchors are fragile around doc comments, module boundaries and script blocks.** Prefer anchors served by `read`
  immediately before the edit, and check the diff rather than trusting the intent.
- **`tests/no_alloc.rs` is the audio-path gate**; nothing in task 10 touches it, but a page must not claim the drum machine
  allocates in the callback.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; `cargo run -p acid-app --bin takka-acid --midi-map` prints
  the note map without opening a device or a window.
- The kit browser's rows are capped above the footer keys; the list has no scrolling yet, so a screenshot with more kits than
  fit shows the first few. A fresh data directory shows the three factory kits and no `MY KITS` — if the picture needs
  `MY KITS`, make a copy by turning a dial on a factory kit (which creates one, under a free name) before the grab.

## Facts to carry into the plan's Progress (from tasks 6, 8 and 9)

- **`adopt_kit` and `set_kit` are the load and edit paths**, and a kit reload through the load path reseeds the player's
  mutes. That is a wishlist entry, not this round's.
- **`take_audition` is drained once per block, by the renderer, and nothing else may.**
- **`main`'s `kit_rate`/`write_kit()` order is load-bearing**: the write happens at the old rate, before the new one is
  adopted, or every WAV is stamped with a rate its samples were not made at.
- **`SharedState::kind` is the one answer to "what kind is this device" and `Device` has no field for it**, so the face and
  the sound cannot disagree.
- **`SAVE_VERSION` is 11 and the rack round takes 12.**
- **The book is task 10's** — and tasks 8 and 9 moved it already, for things a player hears or reads differently:
  `01-what-it-is` (the "not a drum machine" clause), `14-midi` and `15-keyboard` (a drum device takes no notes), the
  picker's prose and screenshot in `05`/`18`, `DRUMS` pinned in `check.sh`, and `12-songs-and-demos` (the drum-bank
  paragraph and the `drum bank` pin). Re-read those pages rather than assuming they are clean.

## The end of this session

When task 10 is committed: add its `## Progress` entry (what shipped, which pins and images, what the review found and what
was done, every ruling with its cost), and rewrite this file as `plans/phase 4/handover-task-11.md` **only if a task 11
exists** — task 10 is the plan's last task, so the honest alternative is the phase's own close: mark
`plans/phase 4/00-phase-4.md` as finished, note what the round shipped, and prepend the one-line
`> **Superseded — kept for the record, not for instructions.**` note to this file, as every handover before it did. A
whole-branch review of `phase-4` against `main` is the phase's own gate before the player decides to push; nothing is pushed
by an executor.

Do not push; the player decides when the branch goes up.
