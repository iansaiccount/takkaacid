# Plan 45 — The player's guide catches up

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline). This round is documents and pictures,
> no code. Each task ends green: `bash docs/guide/check.sh`, which is the guide's only test.
>
> **Review:** the phase's gate (`00-phase-2.md`), inline, at the end of each task — with the guide there is no diff a
> compiler has read, so the review is the reader's: does every sentence match the panel, and is every picture the state it
> claims?

**Goal:** the guide on disk describes the app on disk. It was written before phase 2 and says so in one sentence at the top;
this round removes the need for that sentence, pictures the states it could not, and gives the four new features the pages
they never had.

**Architecture:** prose in the book's existing voice, plus a fresh set of 2× screenshots taken from the real binary against
a scratch data directory — the same recipe that produced the old ones, with two differences this machine allows: an offscreen
X display instead of a live one, and pointer injection, so the states that were "prose because the capture machine has no
pointer" get pictures.

**Spec:** this plan. The authority for every fact in the prose is the code and the app's own words — `ui/help.rs`,
`ui/device_card.rs`, `ui/strip.rs`, `ui/shortcuts.rs`, `ui/settings.rs`, `persist.rs` — not another document.

## What is wrong today

- **Every page is a phase out of date.** The panel grew a 30 px strip band above the top strip: `04-the-face` says "six
  bands" and "designed at 980 × 640", and every screenshot in `src/img` was grabbed from commit `fc1d0a4`, before the strip
  existed. The pictures are `980 × 640`-era; the panel is `980 × 670`.
- **The cards page is wrong in kind.** There are four cards inside the panel plus the picker over them, not three: `05`
  says "three cards open over the panel", names help/settings/songs, and gives the Escape order without the device card or
  the picker. All of it is now false in the one place a player looks first when stuck.
- **Four features have no page.** The strip (a cell per device, `+`, click and `Ctrl+1..4`, `MUTE`), the device card (`F3`:
  name, role, ports, channel, `COLOUR`, `MUTE`, `REMOVE`), the colour scheme (per device, face inverts, cards stay black,
  not saved), and the clipboard (`Ctrl+C`/`Ctrl+V`, what a paste costs). The help card names them in one line each; the guide
  says nothing.
- **The pages that do exist assume one machine.** `14-midi` talks about "the" input ports and has no place for per-device
  ports, channel or role; `20-reference`'s key table has no `Ctrl+1..4`, no `F3`, no copy/paste; `15-keyboard`'s "why one
  voice is not a limitation" argues from a window that can now hold four.
- **The screenshots could not show a card that needs a click.** `img/README.md` records the limit: "there is no pointer tool
  on the capture machine", so the songs card, an unfolded settings list and a knob tooltip are prose. That limit is gone.

## The design

**Pictures.** One session, offscreen, against `TAKKA_ACID_DATA_DIR=/tmp/takka-guide-45`, 2× scale, pointer and keys injected
(`xdotool`), `import -window` for the grab. Every existing image is retaken (the panel moved) and these are added:

| Image | State |
|---|---|
| `first-sound.png` | a first run: one device, the strip with its single cell, the Slow Acid demo, stopped (retaken) |
| `running.png` | the transport running (retaken) |
| `help-card.png` | the help card, which now names the devices, `Ctrl+C/V` and the save limit (retaken) |
| `settings-card.png` | the settings card (retaken) |
| `songs-card.png` | the songs card — the first picture of it |
| `device-card.png` | `F3` on the first device: name, role, ports, channel, colour, mute, remove |
| `picker.png` | the strip's `+`: `ACID`, the footnote that names what is not saved, `CLOSE` |
| `strip.png` | a crop of the strip with two devices, the second amber — the strip as a legend |
| `two-devices.png` | the panel showing an amber device, with the black one beside it in the strip |
| `clipboard-notice.png` | a crop of the status line saying `pasted A1 into A2` |
| the five crops | `top-strip`, `knob-row`, `step-row`, `switches`, `transport`, at the new geometry |

`img/README.md` records the session, the commit, the geometry of each grab and the new recipe, because that file is what
makes the pictures replaceable.

**Pages** (all in the book's voice: second person, short sentences, ASCII, no marketing):

- `01-what-it-is` — the "pictures are older than the panel" note goes; the opening that says "one voice" becomes one voice
  *per device*, with the strip named in a sentence and a pointer to the new page.
- `04-the-face` — seven bands; the strip first; `980 × 670`; the switches' paragraph stays true (BANK/PATTERN/MODE are still
  engraved and silent — checked in `switches::is_live`).
- **New `05-a-rack-of-devices`** — the strip: a cell per device, click or `Ctrl+1..4`, `MUTE`, `+` and the picker, four
  devices, what a colour scheme is for, and that names, extra devices and colours are not saved.
- **New `06-a-devices-card`** — `F3`: the rows, `COLOUR`, `REMOVE` asking twice, `ROLE`/`PORTS`/`CHANNEL` in a sentence each
  with the routing page carrying the detail.
- **New `07-copy-and-paste`** — `Ctrl+C`/`Ctrl+V`, what lands where, what a paste replaces, that the clipboard is text so it
  crosses windows, and what a restart costs.
- `05-the-three-cards` becomes **the four cards**: help, settings, songs, the device card, the picker over them, and the
  Escape order from `ui::dispatch::escape`; it keeps its filename (links point at it) and gains the songs-card picture.
- `14-midi` — ports, channel and role belong to a device: which ports are *open* is the settings card's; which of them a
  device hears is its card's. One paragraph, not a rewrite.
- `15-keyboard`, `20-reference` — the new keys in the table and the prose; the save-limit sentence in the words the picker
  uses (`NAMES AND EXTRA DEVICES ARE NOT SAVED YET`).
- `12-songs-and-demos`, `13-settings` — one sentence each where the save limit or the MIDI-in list now means something
  different.

**The test.** `check.sh`'s fourth check gains the facts this round must not lose: the strip's gestures (`Ctrl+1`), the device
card (`F3`), the colour row (`COLOUR`), the clipboard (`Ctrl+C`), and the limit (`NOT SAVED YET`). The `01` page's stale
disclosure is *not* a check — its absence is proved by the new pictures.

## Tasks

Each ends with `bash docs/guide/check.sh` and a review of the prose against the app.

- [ ] **Task 1 — the pictures.** Take the session above, retake the nine existing images and add the six new ones; update
  `img/README.md` with the commit, the date, the geometry and the recipe (offscreen 2×, `xdotool` for the pointer).
- [ ] **Task 2 — the frame.** `01-what-it-is` and `04-the-face` (seven bands, the strip first, `980 × 670`), with the new
  images wired in.
- [ ] **Task 3 — the three new pages** (`05-a-rack-of-devices`, `06-a-devices-card`, `07-copy-and-paste`), and `SUMMARY`
  renumbered around them.
- [ ] **Task 4 — the pages that already exist:** the four-cards page, `14-midi`, `15-keyboard`, `20-reference`,
  `12-songs-and-demos`, `13-settings`.
- [ ] **Task 5 — the test and the record:** `check.sh`'s new facts, `img/README.md`'s replacement recipe, this plan's
  `## Progress`, and `00-phase-2.md`'s note that the guide round is done.

## Non-goals

- **A new book theme or layout.** The pages are markdown in the existing shape; the book's look is not this round's.
- **A picture of every control.** The guide has one picture per idea and prose for the rest; a screenshot of each knob is a
  maintenance cost with no reader.
- **The Wayland-native look.** The grabs are the same 980 × 670 panel from the same binary, rendered offscreen; the book is
  not claiming to show anyone's desktop.
- **Rewriting the pages phase 2 did not touch** (`06-the-knobs`, `16-sound-like-acid`, `17-trouble` beyond the strip's
  mention).

## Progress

**The round is done.** `bash docs/guide/check.sh` → `guide: ok` (book builds, every page reachable from `SUMMARY`, every
image and every cross-reference resolves, and the new facts are present). The workspace's three gates are unchanged from
`5670f52` — this round touched no Rust — and were re-run for the record: 69 suites, 649 tests, 0 failed; clippy and fmt clean.

- [x] **Task 1 — the pictures.** Fifteen images, all retaken or new, at 1960 × 1340 from the design-size window at 2×.
  `img/README.md` rewritten: the offscreen recipe, the geometry, the per-image state, and the reason the recipe changed
  (pointer injection is available, so the cards that were prose are pictures — `songs-card`, `device-card`, `picker`, the
  colour strip, and the clipboard notice are new).
- [x] **Task 2 — the frame.** `01-what-it-is` lost the "the pictures are older than the panel" note (the pictures are not),
  and its "one voice" became one voice *per device* with the rack named and linked. `04-the-face` is seven bands, the strip
  first, `980 × 670`.
- [x] **Task 3 — the three new pages**, in `SUMMARY` where they belong in the reading order: `18-a-rack-of-devices` (the
  strip, adding, muting, what a device is, the colour, the restart cost), `19-a-devices-card` (every row, the colour, the
  two-click removal, the one-card rule) and `21-copy-and-paste` (what a copy takes, what a paste replaces, where it can go,
  what is on the clipboard).
- [x] **Task 4 — the pages that already exist.** `05` is **the four cards**, with the picker described and the Escape order
  taken from `ui::dispatch::escape` rather than from memory; `12-songs-and-demos` gained "one song, one rack";
  `13-settings` says which ports are *open* and points at the device card for what a device hears; `14-midi` gained "what
  each device hears"; `15-keyboard` lost its one-window assumptions and gained the new keys; `20-reference` gained the key
  rows, the strip's words, the device card's, the picker's and the clipboard's, and the file table now says `state.json`
  holds the first device only.
- [x] **Task 5 — the test and the record.** `check.sh`'s fact list gained `Ctrl+1`, `F3`, `COLOUR`, `Ctrl+C`,
  `NOT SAVED YET` and `four machines`; this section; the phase overview's guide line.

**What the round found, and the rulings.**

- **A false alarm, and a lesson worth the detour.** Trying to photograph the songs card, the card never appeared, and the
  `SONGS` key never lit. Two code hunches followed (a shield swallowing the click; a missing frame-start guard), the first
  was implemented, and the test written for it *passed with the fix removed* — which is the evidence that the fix treated
  nothing. Instrumenting the app showed the widget under the pointer was `save-session`: the click was landing in `SAVE`.
  The key rects are `561–691` for `SAVE` and `697–828` for `SONGS`; read off a screenshot, both look like "the sixth key".
  **Ruling:** the speculative guard, the `draw_settings` parameter and the test were reverted, and `git diff` on those three
  files is empty. The lesson is written into `img/README.md`: take rects from the layout, never from a picture.
- **A tenth of a second is not 10 ms.** The first draft of `18-a-rack-of-devices` repeated the plan's old wording; the
  provider is `MUTE_MS = 10.0`, so the page says 10 ms and calls it a hundredth. The same error was corrected in the README
  and `EAR_TESTING.md` during plan 43, and this is the third place it tried to land.
- **"In step" was an overclaim.** Two devices come in on the same *step*, but the seek does not align them inside it (plan
  43's measurement: up to a step of offset), so the page says "two machines playing together" and leaves it there.
- **Page numbers are a hint; `SUMMARY` is the order.** The new pages are `18`, `19` and `21` rather than a renumbering of
  eighteen files: a reader never sees the numbering, every cross-reference (which `check.sh` guards) would move with it, and
  the diff would be a rename storm for no reading benefit. `05` keeps its filename and changes its title, which is the same
  trade at one file instead of eighteen.
- **The capture is offscreen, and says so.** `img/README.md` records `Xvfb` + `WINIT_X11_SCALE_FACTOR=2` + `xdotool`
  instead of `grim` on a live output: same binary, same 980 × 670 face, rendered at 2×, and reproducible without taking over
  someone's desktop.
- [x] **The amber palette softened after this round** (`two-devices`, `strip`, `clipboard-notice` retaken; `19-a-devices-card`'s
  colour paragraph corrected, because the keys are now the *brighter* gold and its old wording said the key faces were dark).
  Plan 44's record carries the change and why it needed a new palette role.
