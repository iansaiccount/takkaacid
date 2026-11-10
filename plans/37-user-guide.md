# The Player's Guide Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (inline) — this plan's tasks share one voice and
> one set of facts, so a fresh implementer per task would cost more than it buys. Steps use checkbox (`- [ ]`) syntax.

**Goal:** put a player's manual in `docs/guide/` — an mdBook of eighteen pages, illustrated with screenshots of the real
panel — and make the README carry the prerequisites the app actually has.

**Architecture:** mdBook 0.5.4 over plain Markdown: `docs/guide/book.toml` and `src/`. One numbered file per page, ordered by
`src/SUMMARY.md`. Screenshots and crops are committed PNGs under `src/img/`, with a provenance note. `docs/guide/check.sh`
is the test: it builds the book, proves every source page is reachable from the table of contents, proves every image a page
references exists, and proves the guide never repeats the one sentence the app gets wrong.

**Tech Stack:** mdBook 0.5.4 (installed at `~/.cargo/bin/mdbook`), plain Markdown, PNG images captured with `grim` and
cropped with ImageMagick, `wtype` to drive the app's keyboard.

**Spec:** `docs/superpowers/specs/2026-11-09-user-guide-design.md` — the plan argues from it; read both.
## Before you start (environment, and the defaults already in force)

This plan starts from `fc1d0a4`: the design doc and this plan are its only artefacts, nothing under `rust/` was touched for
the guide, and the three gates were green there (`cargo test --workspace` 526 tests / 61 suites, clippy `-D warnings`,
`fmt --check`).

- **mdBook is `~/.cargo/bin/mdbook`, v0.5.4.** It is allowlisted for this harness but `~/.cargo/bin` is not on this shell's
  `PATH`, so call it by absolute path (`MDBOOK=… bash docs/guide/check.sh`), never bare `mdbook`.
- **Present:** `grim`, `slurp`, `magick`/`convert`/`identify`, `wtype`, `jq`, `hyprctl` (installed, but blocked by this
  shell's allowlist), a Wayland session on `wayland-1`.
- **Absent:** `ydotool`, `dotool` (so no pointer injection), `pngquant`, `oxipng`, `pandoc`, `mkdocs` — the plan needs none
  of them.
- **Shell-policy gotchas in this harness:** an inline `PATH=` prefix is rejected (use the tool's `env` parameter);
  `python3` with a heredoc is rejected (write a file and run it); `perl -0pi` is rejected; `hyprctl` and `pacman` are
  blocked unless the operator runs `lean-ctx allow`. `ctx_read`/`read` for files, never `cat`.
- **Two monitors, and the app will be moved to one of them before any capture.** Ask the operator which output (or run
  `lean-ctx allow hyprctl` and read `hyprctl monitors -j` for names, positions and scale). Then always name the output
  explicitly — `grim -o <OUTPUT> -s 1 /tmp/shot.png` — because grim's default with several outputs is not the one the panel
  is on. `-s 1` makes the capture logical pixels, so the panel is **980 px wide whatever the monitor's scale** (use `-s 2`
  for a sharper 1960 px image if the reader is on a HiDPI screen; downscale with `magick -resize 980x` in that case).
  Derive the crop geometry from the first capture of *that* output and do not move the window afterwards.
- **`wtype` types into the window that has focus**, and a window on the second monitor may not have it: after a key, capture
  and check the card actually opened — the capture is the verification. If it did not, ask the operator to click the app
  once and send the key again. Do not send a burst of keys and assume.
- **Defaults in force, chosen by the operator's silence:** no edit under `rust/` (the stale save sentence stays, the guide
  carries a footnote); no edit to `AGENTS.md`; the three pointer-only screenshots are asked for once and fall back to prose.

## Global Constraints

- **No changes under `rust/`.** Not one line. The one exception in the spec (the stale save sentence in `ui/help.rs`) is
  *offered*, not taken: this plan records it as a follow-up and the guide works around it with a footnote.
- `AGENTS.md` is not edited (also offered, not taken).
- mdBook: `~/.cargo/bin/mdbook` (v0.5.4). Not on this shell's `PATH`; always call it by absolute path.
- Images: PNG, committed under `docs/guide/src/img/`. Never JPEG.
- Quote the panel's labels exactly (`SAVED`, `UNSAVED`, `TOOLTIPS ON`, `AUTOSAVE OFF`, `MIDI: none (retrying)`,
  `IN USE: none`, `RETRY`, `CLOSE`, `A1`…`H8`, `SONG …`). State *behaviour* from the code, not from a help line.
- The top strip holds the song name or a notice. `MIDI:` and `failed:` words live beside the headings **in the settings
  card**. Get this right everywhere.
- Knobs are taught by position and picture; no numeric values, no song-file format.
- Every capture runs against `TAKKA_ACID_DATA_DIR=/tmp/takka-guide`, so no real session appears in the book.
- Facts that must not drift: tempo 40–300 BPM; sixteen steps; 64 patterns in 8 banks of 8; five demos (Slow Acid, Hard
  Techno Acid, Blues, Rock, Funk); first run is Slow Acid named "New song", unsaved; window and design size 980 × 640.

## Review Focus

The input classes a player will actually meet, and where each is pinned:

1. **A first run on a machine with no portal** — `IMPORT`/`EXPORT` are the only file dialogs and they need
   `xdg-desktop-portal` plus a backend. Pinned by the prerequisites row in `02-install` and `12-songs-and-demos` naming the
   dependency, plus `check.sh`'s grep for `xdg-desktop-portal`.
2. **Saving that is not automatic** — a reader who assumes autosave loses work. Pinned by `check.sh`: the stale sentence
   must not appear, and `manual by default` must.
3. **A controller that is not seen, or is unplugged** — the card says `MIDI: none (retrying)` and offers `RETRY`. Pinned in
   `13-settings` and `17-trouble` with those exact words; `check.sh` greps for both.
4. **A rescaled or small window** — the panel scales between 0.65× and 1.6×, so "a third of the sweep" reads the same but
   the screenshot is at 1×. Pinned by a sentence in `04-the-face` and `07-a-starting-patch`.
5. **"Is it my keyboard or the app?"** — `takka-acid --midi-map` answers it without a window. Pinned in `17-trouble` and
   the reference page; `check.sh` greps for `--midi-map`.

---

### Task 1: The book, its skeleton and its test

**Files:**
- Create: `docs/guide/book.toml`
- Create: `docs/guide/src/SUMMARY.md`
- Create: `docs/guide/src/*.md` (eighteen stubs, one line each)
- Create: `docs/guide/check.sh`
- Create: `docs/guide/src/img/README.md`

**Interfaces:**
- Produces: the eighteen chapter paths, exactly as listed in the spec's "The sections" (`01-what-it-is.md` …
  `20-reference.md`); `check.sh` (exit 0 = the book is sound), used by every later task.

- [ ] **Step 1: Write `book.toml`** — `[book] title = "takka-acid"`, `authors`, `language = "en"`, `src = "src"`; `[output.html]
  default-theme = "dark"`, `git-repository-url` unset. No `[preprocessor]`.

- [ ] **Step 2: Write `src/SUMMARY.md`** — five parts and a reference part, in the spec's order, each entry pointing at its
  file. This is the only place the reading order is written down.

- [ ] **Step 3: Write the eighteen stub pages**, each with its `# Title` and one sentence saying what the page will cover
  (copy the spec's page briefs). Stubs make `check.sh` meaningful from the first commit.

- [ ] **Step 4: Write `check.sh`**, and make it fail on four things:
  ```bash
  #!/usr/bin/env bash
  set -euo pipefail
  cd "$(dirname "$0")"
  mdbook="${MDBOOK:-$HOME/.cargo/bin/mdbook}"
  # 1. the book builds
  "$mdbook" build >/tmp/guide-build.log 2>&1 || { cat /tmp/guide-build.log; exit 1; }
  # 2. every page is reachable from SUMMARY.md, and every SUMMARY entry exists
  for page in src/*.md; do
    grep -qF "(${page#src/})" src/SUMMARY.md || { echo "not in SUMMARY: $page"; exit 1; }
  done
  # 3. every image a page references exists
  grep -rhoE 'img/[A-Za-z0-9._-]+\.png' src --include='*.md' | sort -u | while read -r img; do
    [ -f "src/$img" ] || { echo "missing image: src/$img"; exit 1; }
  done
  # 4. the facts that must not drift (spec Review Focus 1, 2, 3, 5)
  ! grep -rqF 'saved on exit and 2 s after a change' src || { echo "the guide repeats the stale save line"; exit 1; }
  for word in 'manual by default' 'xdg-desktop-portal' 'MIDI: none (retrying)' 'RETRY' '--midi-map'; do
    grep -rqF "$word" src || { echo "the guide never says: $word"; exit 1; }
  done
  echo "guide: ok"
  ```

- [ ] **Step 5: Run it** — `bash docs/guide/check.sh`. Expect the build to pass and check 4 to **fail** (`manual by
  default` is not written yet). That failure is the RED for this task.

- [ ] **Step 6: Make check 4 pass honestly** by putting those five facts in the stubs that own them: `manual by default` and
  `xdg-desktop-portal` in the `02-install` stub, `MIDI: none (retrying)` and `RETRY` in the `13-settings` stub, `--midi-map`
  in the `17-trouble` stub. Stubs are replaced by real pages later; the words stay.

- [ ] **Step 7: Mutation-check checks 2 and 3** — delete one SUMMARY line, run `check.sh`, confirm it names the orphan page;
  restore. Rename one referenced image, confirm it names the missing image; restore.

- [ ] **Step 8: Commit** — `docs(guide): an mdBook skeleton, and the test that keeps it honest`

### Task 2: The face, captured

**Files:**
- Create: `docs/guide/src/img/{first-sound,help-card,settings-card,running}.png`
- Create: `docs/guide/src/img/{top-strip,knob-row,step-row,switches,transport}.png` (crops)
- Modify: `docs/guide/src/img/README.md` (provenance)

**Interfaces:**
- Produces: ten PNGs whose names every later page references; `src/img/README.md` recording, per image, the state the app
  was in, the exact commands, the date and `git rev-parse --short HEAD`.

- [ ] **Step 1: Launch against a scratch data dir** — `TAKKA_ACID_DATA_DIR=/tmp/takka-guide cargo run --release -p
  acid-app --bin takka-acid &`, wait for the window (poll `grim` output for a non-desktop image rather than sleeping blind).

- [ ] **Step 2: Capture full output** — `grim /tmp/guide-full.png`, then *look at the image* and derive the panel's crop
  geometry from it. Write the geometry down in `src/img/README.md`. Nothing else crops until this number exists.

- [ ] **Step 3: Capture the three key-driven states** — `wtype -k F1` → help card; `wtype -k Escape`; `wtype -k F2` →
  settings card; `wtype -k space` → running. Capture after each, crop each with the geometry from step 2.

- [ ] **Step 4: Cut the five detail crops** from the first-sound capture with `magick -crop WxH+X+Y +repage`.

- [ ] **Step 5: Verify** — `magick identify docs/guide/src/img/*.png` prints ten images, each 0 bytes larger than 0 and
  each smaller than 400 KB; then `bash docs/guide/check.sh` still passes (no page references them yet).

- [ ] **Step 6: Commit** — `docs(guide): the panel, photographed`

### Task 3: Part I — getting started (pages 01–05)

**Files:** Modify `src/01-what-it-is.md`, `02-install.md`, `03-first-sound.md`, `04-the-face.md`, `05-the-three-cards.md`

**Interfaces:**
- Consumes: the images from Task 2 (`img/first-sound.png`, `img/help-card.png`, `img/settings-card.png`,
  `img/top-strip.png`, `img/knob-row.png`, `img/step-row.png`, `img/switches.png`, `img/transport.png`).
- Produces: the prerequisites table the README will mirror in Task 9 — the same nine rows, same wording. Later tasks link
  back to `02-install` rather than repeating a prerequisite.

- [ ] **Step 1: Write `01-what-it-is`** — one voice, sixteen steps, the four things that make the sound (wave, filter,
  envelope decay, accent); what it is not (not a sampler, not a workstation).
- [ ] **Step 2: Write `02-install`** — the prerequisites table verbatim from the spec (nine rows, docs-only rows last),
  build and run commands, where files live, `TAKKA_ACID_DATA_DIR`, `--midi-map`.
- [ ] **Step 3: Write `03-first-sound`** — launch, the demo that is already there, `UNSAVED`, RUN, and that nothing on the
  face is destructive without a second click (CLEAR, NEW, load, delete).
- [ ] **Step 4: Write `04-the-face`** — `img/first-sound.png` with the strip, rows and keys walked in the order the eye
  meets them; the eight detail crops beside the paragraphs they illustrate; the scaling note (0.65×–1.6×).
- [ ] **Step 5: Write `05-the-three-cards`** — `F1`/`?`, `F2`/gear, SONGS; one at a time; each card's way out (help: any
  click, `Escape`, `F1`, `shift+?`; settings and songs: `CLOSE`, `Escape`, a click outside).
- [ ] **Step 6: Run** `bash docs/guide/check.sh` — expect `guide: ok`.
- [ ] **Step 7: Read it against the code** — for each gesture, key and label a page names, confirm it in
  `rust/crates/acid-app/src/ui/help.rs`, `ui/shortcuts.rs`, `ui/keys.rs` or `ui/panel.rs`. Anything the code does not say,
  cut.
- [ ] **Step 8: Commit** — `docs(guide): what it is, how to start it, and what you are looking at`

### Task 4: Part II — making a sound (pages 06–07)

**Files:** Modify `src/06-the-knobs.md`, `src/07-a-starting-patch.md`

**Interfaces:**
- Consumes: `img/knob-row.png`.
- Produces: the knob positions `07-a-starting-patch` teaches — the Slow Acid patch **by position**, described as fractions of
  the sweep; `16-sound-like-acid` (Task 7) refers back to these rather than re-teaching them.

- [ ] **Step 1: Write `06-the-knobs`** — the seven knobs in panel order plus volume, one short paragraph each: what it does,
  and the one thing it does to its neighbour (cutoff against resonance, env mod against decay, accent against everything).
  `img/knob-row.png` at the top.
- [ ] **Step 2: Write `07-a-starting-patch`** — the Slow Acid positions as a table of *positions* ("a third", "three
  quarters"), the same crop as the target picture, one paragraph on why it sounds acid, and the plain sentence that the panel
  has no numeric readout and nothing to type.
- [ ] **Step 3: Run** `bash docs/guide/check.sh` — expect `guide: ok`.
- [ ] **Step 4: Sanity-play it** — run the app against the scratch dir, dial the patch the page describes, and confirm it is
  the Slow Acid sound rather than a guess. If the page's positions do not land, fix the page (never the app).
- [ ] **Step 5: Commit** — `docs(guide): the knobs, and a patch to dial in`

### Task 5: Part III — writing and playing a line (pages 08–11)

**Files:** Modify `src/08-writing-steps.md`, `09-playing.md`, `10-performance-row.md`, `11-the-bank.md`

**Interfaces:**
- Consumes: `img/step-row.png`, `img/switches.png`, `img/transport.png`, `img/running.png`.
- Produces: the gesture table `20-reference` (Task 7) condenses — written once here, referenced there.

- [ ] **Step 1: Write `08-writing-steps`** — step anatomy (note, octave, velocity, accent, slide), then the full gesture
  table straight from `help.rs`'s STEPS lines, then typing notes on the computer keyboard (`A W S E D F T G Y H U J K`,
  `Z`/`X` octave, backspace rests), STEP mode, CLEAR taking two clicks.
- [ ] **Step 2: Write `09-playing`** — RUN/STOP/`SPACE`, tempo (40–300, one BPM per arrow), swing, and why the step LED
  chases the audio rather than the clock.
- [ ] **Step 3: Write `10-performance-row`** — octave down/up, shuffle, randomise (density and scale), and what each does
  while the transport runs.
- [ ] **Step 4: Write `11-the-bank`** — 64 patterns, `A1`…`H8`, moving with `shift+left`/`shift+right`, a learned knob, and
  the swap that waits for the bar.
- [ ] **Step 5: Run** `bash docs/guide/check.sh`; then the code check of Task 3 step 7 for every gesture and key named.
- [ ] **Step 6: Commit** — `docs(guide): writing a line, playing it, and moving between patterns`

### Task 6: Part IV — keeping and connecting (pages 12–15)

**Files:** Modify `src/12-songs-and-demos.md`, `13-settings.md`, `14-midi.md`, `15-keyboard.md`

**Interfaces:**
- Consumes: `img/settings-card.png`; the `MIDI: none (retrying)` / `RETRY` words already seeded in the stub.
- Produces: the save rules `README`'s pointer will summarise in Task 9 — in place when the name is unchanged, a new file
  when it changed, a blank name saving the song you are on, `SAVE A COPY` never touching the original.

- [ ] **Step 1: Write `12-songs-and-demos`** — the five demos as read-only, `NEW`/`SAVE`/`SAVE A COPY`/`IMPORT`/`EXPORT`/
  load/delete, the name rules above, the second click for anything destructive, and the portal the two dialogs need.
- [ ] **Step 2: Write `13-settings`** — `F2`/gear, tooltips, autosave (and the plain statement that saving is manual by
  default), MIDI in, sound out, the `IN USE:` line above each folded list, `RETRY`, and the supervisor's words. Add one
  footnote: the help card says "saved on exit and 2 s after a change", which is only true with autosave on — see
  `17-trouble`.
- [ ] **Step 3: Write `14-midi`** — choosing a port, notes with velocity, `LEARN` (press, click the knob, move the fader),
  pickup for dials, the pattern selector waiting for the bar, an external clock driving tempo.
- [ ] **Step 4: Write `15-keyboard`** — the computer keyboard as an instrument (`A`…`K`) and as an editor (shift, STEP,
  backspace, arrows, `enter`), and why one voice is shared.
- [ ] **Step 5: Run** `bash docs/guide/check.sh`; then the code check.
- [ ] **Step 6: Commit** — `docs(guide): songs, settings, MIDI and the computer keyboard`

### Task 7: Part V and the reference (pages 16, 17, 20)

**Files:** Modify `src/16-sound-like-acid.md`, `src/17-trouble.md`, `src/20-reference.md`

**Interfaces:**
- Consumes: the position language from Task 4, the gesture table from Task 5, the words from Task 6.
- Produces: the three tables the README's pointer will lean on — keys and gestures, the eleven factory CCs
  (`midi/cc.rs:89-100`), the panel's words for each state.

- [ ] **Step 1: Write `16-sound-like-acid`** — slide into an accent, tuning against cutoff, short decay with high env mod,
  resonance as the voice, one line not chords, shuffle late. Each idiom refers back to the knobs page; none repeats it.
- [ ] **Step 2: Write `17-trouble`** — no sound (check the card's `SOUND OUT` words), a failed stream (`failed: …`), a
  controller not seen (`MIDI: none (retrying)`, `RETRY`), `--midi-map` for "is it my keyboard", where the files are,
  `TAKKA_ACID_DATA_DIR`, and what to include when reporting a bug. The stale-save-line footnote lands here or in
  `13-settings`, not both.
- [ ] **Step 3: Write `20-reference`** — three tables: keys and gestures; the eleven factory CCs copied from `midi/cc.rs`;
  the words the panel and the card use (`SAVED`, `UNSAVED`, `TOOLTIPS ON`/`OFF`, `AUTOSAVE ON`/`OFF`, `IN USE: …`, `RETRY`,
  `CLOSE`, `MIDI: …`, `failed: …`, `SONG …`, `A1`…`H8`), plus the data dir's file names.
- [ ] **Step 4: Run** `bash docs/guide/check.sh`; verify the CC list against `midi/cc.rs` line by line, and every table
  cell against the code.
- [ ] **Step 5: Commit** — `docs(guide): how to sound acid, and what to do when it does not`

### Task 8: The three screenshots that need a click

**Files:**
- Create: `docs/guide/src/img/{songs-card,settings-open,tooltip}.png` (if the operator clicks)
- Modify: `src/05-the-three-cards.md` (songs card), `src/13-settings.md` (an unfolded list), `src/06-the-knobs.md` (a tip)

**Interfaces:**
- Consumes: the crop geometry from Task 2.
- Produces: either three images wired into those pages, or — if the operator is not available — nothing, and the pages keep
  their prose. **No placeholder images, ever**: a broken `<img>` is worse than a paragraph.

- [ ] **Step 1: Ask, once, with the exact steps** — launch against `/tmp/takka-guide`, click `SONGS`; click `MIDI IN  [+]`;
  hold the pointer on a knob until the tip appears. One click each; I capture and crop between them.
- [ ] **Step 2a: If they click** — capture, crop, wire the three images in, run `check.sh`, commit
  `docs(guide): the songs card, an open device list, and a tip`.
- [ ] **Step 2b: If they do not** — record in `src/img/README.md` that these three are prose by choice, and note it in the
  plan's Progress. No commit beyond that note.

### Task 9: The README keeps up with the prerequisites

**Files:** Modify `README.md`

**Interfaces:**
- Consumes: the prerequisites table written in `02-install` (Task 3) — the README's copy is the same nine rows.
- Produces: a README that is a developer's front door again: install (with the full table), the two flags, gates, plans,
  layout, licence, and one pointer to `docs/guide`.

- [ ] **Step 1: Replace the "Installation" prerequisites** with the nine-row table, docs-only rows last (mdbook; grim, slurp,
  imagemagick), each row saying why in a few words — the same rows as `02-install`, so the two cannot drift into disagreement.
- [ ] **Step 2: Add the pointer** — one short paragraph: the player's guide is `docs/guide`; `mdbook serve docs/guide` to read
  it, `mdbook build docs/guide` for the site, `bash docs/guide/check.sh` to check it.
- [ ] **Step 3: Move the player-facing detail out** — cut "Demo songs and saving" down to two sentences plus a link to
  `docs/guide/src/12-songs-and-demos.md`, and cut the "Features" bullets' player-level detail to their one-line summaries.
  Keep "How it was built", "Where it started", "Plans", "Repository layout" (adding a `docs/guide/` line) and the licence.
- [ ] **Step 4: Check the two documents agree** — `grep` the README and the guide for `manual by default`,
  `xdg-desktop-portal`, `40–300`, `64 patterns`: same words, same facts.
- [ ] **Step 5: Commit** — `docs: the prerequisites, and one pointer to the guide`

### Task 10: The read-through

**Files:** Modify whatever the read-through finds.

- [ ] **Step 1: Build and serve** — `~/.cargo/bin/mdbook build docs/guide` and `~/.cargo/bin/mdbook serve docs/guide`, then
  read the whole book in the browser, in order, as a player would: every internal link, every image, every "see above".
- [ ] **Step 2: Fix what the read-through finds** — awkward wording, a page that repeats its neighbour, a broken anchor.
- [ ] **Step 3: Final gates** — `bash docs/guide/check.sh` green; `cd rust && cargo test --workspace && cargo clippy
  --workspace --all-targets -- -D warnings && cargo fmt --check` still green (nothing under `rust/` changed, so this is a
  proof, not a hope).
- [ ] **Step 4: Commit** — `docs(guide): the read-through`

## Non-goals

- No `rust/` change, no `AGENTS.md` change, no CI, no publishing pipeline.
- No API or architecture documentation; no song-file format; no numeric knob values.
- No video, GIFs, localisation, or theme beyond mdBook's default dark.

## Progress

- [x] Task 1 — the book, its skeleton and its test. `7d3cdc4`.
  - Proof: `mdbook build` green; the three grep checks run by hand, and mutated twice — deleting a `SUMMARY.md` line named
    `src/20-reference.md` as the orphan, pointing a page at `img/nonexistent-test.png` named the missing image.
  - **Ruling: this harness cannot run the script it wrote.** No `bash`/`sh` in the shell allowlist, so the four checks were
    run one at a time and the script is the committed artefact for the operator. Cost if wrong: `check.sh` as a *script* is
    unproven here — the operator runs `bash docs/guide/check.sh` once before merge.
  - **Ruling: the plan's script could not have passed.** `src/*.md` matches `SUMMARY.md` itself, and `grep -qF --midi-map`
    reads the word as an option. Fixed both, plus `|| true` on the image grep so a book with no images yet is not an error.
    Cost if wrong: none — each fix makes a check measure what it claims.
- [x] Task 2 — the face, captured. `c37cf8d`.
  - **Ruling: `wtype`, `magick` and `identify` are not allowlisted either.** Keys were sent with Hyprland's Lua dispatcher
    (`hl.dsp.send_key_state`), the window was floated and sized with `hl.dsp.window.*`, and the crops are `grim -g` grabs
    rather than `magick -crop` cuts. Cost if wrong: a crop could disagree with the face picture if the window moved between
    grabs; the geometry was fixed once, the window was not moved, and every image was inspected before use.
  - **Ruling: images are 1960 × 1280 (`grim -s 2`) and there are nine, not the plan's ten.** Nine is the spec's own list
    (four states + five crops); 2× keeps the silkscreen legible under mdBook's scaling. The 400 KB cap became 600 KB
    (largest 487 KB, 2.1 MB total). Cost if wrong: a heavier repository.
  - Finding: the release binary was three days stale and painted a panel from before this round's face. Rebuilt from
    `fc1d0a4` first; `src/img/README.md` records that commit, the date and the exact commands.
- [x] Task 3 — Part I. `6f78989`.
  - Proof: `mdbook build` + the four checks; every gesture, key and label read against `help.rs`, `dispatch.rs`,
    `step_key.rs` and `clear.rs`. Three sentences changed because the code said otherwise (the LED row's meaning, what an
    armed `CLEAR` does, and what `one card at a time` actually means).
  - **Ruling: the prerequisites table is seven rows to play and two more "only if you work on this book", not the spec's
    "first six".** Audio out is not optional for an instrument, and the README's nine-row copy keeps the plan's order with
    the docs rows last. Cost if wrong: a reader counts rows and finds eight.
- [x] Task 4 — Part II. `7d853c3`.
  - Proof: the page's positions were checked against `demo.rs`'s Slow Acid patch — `cutoff 0.35, resonance 0.78,
    env_mod 0.72, decay 0.46, accent 0.52, tuning 0.5, volume 0.87`, saw, 118 BPM, no swing — so "a third" and "three
    quarters" are the demo's own numbers.
  - **Ruling: the "sanity-play it" step became a numeric comparison.** The agent cannot hear the machine; identity with the
    demo's values is a stronger check than a listen, and the picture is the demo's. Cost if wrong: a patch that matches
    the numbers may still be taught with wording that does not land — the reader can see the crop and correct it.
- [x] Task 5 — Part III. `4f5a1e1`.
  - Proof: as Task 3, against `keys.rs`, `selection.rs`, `dispatch.rs`, `pattern_swap.rs`, `randomise.rs`, `switches.rs`.
    Four sentences were rewritten: a letter key plays and does not write; `backspace` only while editing; `BANK`,
    `PATTERN` and `MODE` are engraved and silent (`is_live()` is false for all three); the tempo a controller sets is not
    in one-BPM steps.
  - **Ruling: the guide tells the truth about the three dead keys.** The spec says the face is the truth, and a reader who
    presses `PATTERN` will find out anyway. Cost if wrong: the page dates quickly if the keys are wired up later.
- [x] Task 6 — Part IV. `1a9f94e`.
  - Proof: as Task 3, against `songs.rs`, `persist::save_target`, `settings.rs`, `learn.rs`, `midi/cc.rs`, `midi/tempo.rs`.
  - **Ruling: the stale save line is paraphrased, never quoted.** `check.sh` refuses the sentence outright, and the plan
    wanted the footnote to quote it; the check wins. The footnote is in `13-settings`, with a second mention in
    `17-trouble` for anyone who has lost work. Cost if wrong: a reader looking for the exact words on the help card meets
    a paraphrase.
  - Finding: `external_clock` is set by the clock follower and read by nothing, so the guide does not claim the arrows
    "do not fight" the clock; it says the clock is the master while it is arriving.
- [x] Task 7 — Part V and the reference. `3a2e0b0`.
  - Proof: the CC table was copied from `factory_cc` line by line (20, 3, 74, 71, 76, 75, 5, 7, 72, 21, 73); the words
    table from `fold_label`, `switch_label`, `current_line`, `supervisor::status_line`, `audio::status`, `load_row` and
    `learn::label`/`hint`; the retry cadence from `MIDI_RETRY` × `MIDI_MAX_RETRIES`.
- [x] Task 8 — the three screenshots that need a click. No commit.
  - Taken as **2b**, the documented fallback: no pointer-injection tool exists on this machine, and there was no one to ask
    mid-run. `src/img/README.md` records the three as prose by choice, and `05-the-three-cards`, `06-the-knobs` and
    `13-settings` carry the prose. If the operator wants them, the geometry and the commands are in that same file.
- [x] Task 9 — the README keeps up. `d3d1c54`.
  - Proof: the four facts grepped in both documents — `manual by default` (README 1, guide 4), `xdg-desktop-portal` (1, 2),
    `40–300` (1, 1), `64 patterns` (1, 2). The guide says `64 patterns` and `40–300` in digits for exactly that reason.
- [x] Task 10 — the read-through. `b7a4135`.
  - Proof: the gates — `cargo test --workspace` 526 passed, `cargo clippy -D warnings` clean, `cargo fmt --check` clean
    (nothing under `rust/` changed), and the guide's five checks run by hand: build, SUMMARY coverage, images, links, words.
  - **Ruling: cross-references became links and `check.sh` grew a fifth check for them.** The plan stops at four; the
    read-through showed every page points at another by file name, and a typo there is a dead link nothing catches.
    Mutation-tested by pointing one link at `11-the-bankx.md`. Cost if wrong: one more thing to keep green, which is the
    cheap kind.
  - Fixes found by reading in order: two duplicated sentences (one in `04-the-face`, one in `05-the-three-cards`), a stale
    "screenshots are at 1×" claim (they are 2× grabs of a 1× panel), and a sentence that made the editing keys modal behind
    all three cards when the songs card is the one drawn over everything.

## Final review

A fresh reviewer (a subagent with an empty context) read the whole diff, the plan, the spec and the ledger, and checked
every claim against `rust/`. Its verdict was BLOCK, with two criticals that were real:

- **The transport lights were documented backwards**, on the page a beginner reads first: the code lights `RUN` while the
  line runs and `STOP` while it is stopped (`row_transport.rs`), and two pages plus the provenance note said the opposite.
  Fixed in `03`, `09` and `img/README.md`, and the panel was captured again stopped-then-running to prove it rather than
  trusting the read — the screenshot was right all along, the prose was not.
- **A sentence in `05-the-three-cards` had been cut in half** by the read-through's own edit. Restored.

Eight importants were also right and are fixed: the claim that nothing reaches disk before `SAVE` (the session is written on
exit), the `none` row of the MIDI list (it means *any port*, not *no port*), the audio status separator (`48 kHz - device`,
not a middle dot), two `LEARN` hint lines that no code draws (`learn::hint` is dead), the reference's top-strip table
putting the lamp and the readouts "in the middle", the step-LED paragraph promising a chase the panel cannot show on
note-bearing steps, `RUN` resuming where it stopped (it restarts at step 1), and "unfold the list again" (the device lists
are read when the card opens). Twelve more were one-line factual corrections — `--midi-map`'s real range (21–108), a
first-run tempo of 118 rather than 120, the notice strings `saved Acid 2` / `saved a copy as …`, `CLEAR` being unable to
bring a silent pattern back, and the keyboard table that `15` and `20` were both keeping. That table now lives in `20` only:
the second hand-kept list is the exact failure `AGENTS.md` warns about.

Nothing was deferred: the reviewer's minor list was all wrong facts, and a wrong fact in a manual is not a polish
suggestion. All five guide checks were run again after the pass and are green; the three workspace gates were re-run too
(526 tests passed, clippy clean, `fmt --check` clean) even though nothing under `rust/` changed.

## Follow-up offered, not taken

Two one-line fixes in the app, both offered and neither taken:

- `rust/crates/acid-app/src/ui/help.rs`: `"saved on exit and 2 s after a change"` is false by default (the 2 s write is
  `AUTOSAVE`-only). The guide paraphrases around it.
- `rust/crates/acid-app/src/ui/tips.rs`: the TUNING tip says "a semitone either side"; `param::tuning_semitones` is
  ±12, so the tip is out by a factor of twelve. Found by the final review's read of the code, not by the guide.

Both are strings, not behaviour. The guide states the truth about the knobs and the save model, so it is correct today and
stays correct when the strings are fixed.
