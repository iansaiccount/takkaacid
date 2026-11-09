# A user guide for takka-acid — design

**Status:** awaiting approval. **Date:** 2026-11-09. **Branch:** `Deepseek-Version`.

## Intent

The instrument is finished enough to play and has no player's manual. The README is the developer's front door — install,
build, gates, how the thing was made, the plans — with a few paragraphs about playing bolted on the end. A person who
downloads `takka-acid` to make acid lines should be able to read one document, in the order they will actually do things:
start it, make a sound, write a line, keep it.

Chosen with the operator: **mdBook**, and screenshots of the running panel, captured with `grim`.

## Goals

- One player-facing guide, in the order a player learns: getting started, making a sound, writing a pattern, playing it,
  the performance row, the bank, songs and demos, settings, MIDI, sound design, troubleshooting, reference.
- Screenshots of the real panel — this is an instrument that looks like a machine, and the guide's job is to teach a face.
- Every prerequisite the software actually needs, in one place, including the ones the README misses today.
- The README keeps what belongs to a developer (build, gates, plans, layout) and links to the guide instead of duplicating it.

## Non-goals

- No API documentation, no rustdoc, no design or architecture prose: `AGENTS.md`, `plans/` and `docs/superpowers/specs/`
  already hold that, and the guide must not become their second copy.
- No product code changes. Not one line of `rust/`. Screenshots are taken of the app as it ships.
- No publishing pipeline: no GitHub Pages workflow, no CI for the book. `mdbook build` in `docs/guide` is the build.
- No video, no animated GIFs, no localisation.

## Anti-goals (this would be a failure even if it works)

- The guide and the README disagreeing about how to install or save. Two documents, one fact.
- Screenshots that no longer match the panel, with nothing to say which build they came from.
- A guide that documents the *implementation* ("the `ArcSwap` holds the live pattern") instead of the instrument.
- Instruction that only makes sense on the operator's machine (a specific MIDI controller, a specific device name).

## Constraints

- **No font glyphs on the face** (`AGENTS.md`): screenshots inherit this — the `?`, the gear and the `…` are what the app
  paints, so the guide must not claim to show icons it does not.
- **The face is the truth.** Where the panel says `SAVED`/`UNSAVED`, `A1`…`H8`, `TOOLTIPS ON`, `MIDI: none (retrying)`,
- **The face is the truth.** Where the panel says `SAVED`/`UNSAVED`, `A1`…`H8`, `TOOLTIPS ON`, `MIDI: none (retrying)`,
  the guide quotes those words exactly; the reference section is a table of them. Two placements to keep straight: the top
  strip carries the song's name or a notice and nothing else; the `MIDI:` and audio words sit beside the `MIDI IN` and
  `SOUND OUT` headings *in the settings card*. Only the settings card and the songs card dismiss on "a click outside" —
  the help card is painted shapes under a shield that covers the window, so any click closes it, as do `Escape`, `F1` and
  `shift+?`.
- **Images are committed**, under `docs/guide/src/img/`, as PNG: near-black faces with thin amber silkscreen smear badly
  under JPEG.
- **mdbook is installed** at `~/.cargo/bin/mdbook` (0.4.x). It is not on this harness's shell allowlist and `~/.cargo/bin`
  is not on its `PATH`, so the book can be written here but only built/served by the operator (`mdbook serve docs/guide`)
  or after `lean-ctx allow mdbook`.
- **Reproducible screenshots, without touching real data:** the app honours `TAKKA_ACID_DATA_DIR`, so every capture runs
  against a scratch directory and the operator's session, songs and settings are never in a screenshot or a seed file.

## Approaches

**A. mdBook (picked).** Markdown in `docs/guide/src/`, `book.toml` beside it, `mdbook serve` for live review and
`mdbook build` for a static site that can be opened from disk or published later. Search, sidebar, prev/next, and the
browser's own print-to-PDF for free. Rust-native: the same toolchain builds the app and the book, and Markdown stays the
only format. Cost: one dependency, and the operator has to run `mdbook build` (or allowlist it).

**B. A Markdown tree with no tool** (`docs/guide/*.md` + an index). Zero install, reads on the forge, diffs clean. Rejected:
no search, no navigation, nothing printable as one document, and a dozen pages end up either one enormous file or a pile
of orphan links.

**C. MkDocs Material.** The best-looking of the three and the best search. Rejected: a Python virtualenv and an
`mkdocs.yml` in a repository whose every other tool is Rust or the agent's own, for a twelve-page book.

## Design

### Where it lives

```
docs/guide/
  book.toml                 # title, src = "src", no theme overrides beyond a logo-less default
  src/SUMMARY.md            # the table of contents, four parts + reference
  src/00-*.md …             # one file per section, numbered so the filesystem reads in order
  src/img/*.png             # screenshots and crops
  src/img/README.md         # how each image was taken: the command, the state, the date, the commit
```

`book.toml` sets `title = "takka-acid"`, `src = "src"`, and leaves the theme alone except for a dark-ish accent; the
author's own preference wins later.

### The sections

Part I — **Getting started**
1. `01-what-it-is.md` — one voice, sixteen steps, what it is not; the four things that make the sound.
2. `02-install.md` — the prerequisites table (below), build, run, where files live, `--midi-map`.
3. `03-first-sound.md` — first launch, the demo, the `SAVED`/`UNSAVED` lamp, RUN, and the promise that nothing you press is
   destructive.
4. `04-the-face.md` — a labelled tour: top strip (volume, wordmark, lamp, gear, `?`, song name, pattern and tempo readouts),
   knob row, performance row, step row, switches, transport. One screenshot, plus crops.
5. `05-the-three-cards.md` — help (`F1`/`?`), settings (`F2`/gear), songs; one card at a time; how each dismisses (CLOSE,
   Escape, a click outside).

Part II — **Making a sound**
6. `06-the-knobs.md` — waveform, tuning, cutoff, resonance, env mod, decay, accent, volume: what each does, what it does
   *to the next one*, and the order the panel draws them in.
7. `07-a-starting-patch.md` — the Slow Acid patch *dialled in* by position, with a crop of the knob row as the target
   picture. The panel prints no knob values and accepts no numbers — it is an instrument, not a plugin — so the page
   describes positions ("a third of the sweep") and shows the picture, and says plainly that there is nothing to type.

Part III — **Writing and playing a line**
8. `08-writing-steps.md` — steps, note, octave, velocity, accent, slide; the full gesture table (click, ctrl+click,
   shift+click, alt+click, drag, double-click); typing notes on the computer keyboard; STEP mode; CLEAR taking two clicks.
9. `09-playing.md` — RUN/STOP/`SPACE`, tempo, swing, why the step LED chases the audio.
10. `10-performance-row.md` — octave down/up, shuffle, randomise with density and scale.
11. `11-the-bank.md` — 64 patterns, `A1`…`H8`, moving between them from the keys, a learned knob, and the swap that waits
    for the bar.

Part IV — **Keeping and connecting**
12. `12-songs-and-demos.md` — the five demos, `NEW`/`SAVE`/`SAVE A COPY`/`IMPORT`/`EXPORT`/load/delete, and exactly what a
    save does to a name (in place vs a new file), and that a blank name saves the song you are on.
13. `13-settings.md` — tooltips, autosave, MIDI in, sound out, `RETRY`, the device in use named above each list, and what
13. `13-settings.md` — tooltips, autosave, MIDI in, sound out, `RETRY`, the device in use named above each list, and the
    supervisor's words beside each heading (`MIDI: none (retrying)`, `failed: …`) — those live on the card, not the strip.
14. `14-midi.md` — choosing a port, playing notes in, `LEARN`, pickup, the pattern selector waiting for the bar, an
    external clock.
15. `15-keyboard.md` — the computer keyboard as an instrument and as an editor.

Part V — **Getting good**
16. `16-sound-like-acid.md` — the idioms: slide into an accent, tuning against cutoff, short decay with high env mod,
    resonance as the voice, one line not chords, shuffle late not early.
17. `17-trouble.md` — no sound, a stream that failed, a controller that is not seen, `--midi-map`, the status line's
17. `17-trouble.md` — no sound, a stream that failed, a controller that is not seen, `--midi-map`, the words the card uses
    for each state, where the files are, `TAKKA_ACID_DATA_DIR`, and how to ask a good question about a bug.

**Reference**
18. `20-reference.md` — keys and gestures, the MIDI CC map (eleven factory CCs), the words the panel and the card use for
    each state, the files and their names.

Each page: 1–3 screens of prose, a table or a screenshot where a table would be worse, and no more than one idea per page.

### The prerequisites table (the README is missing some of these)

Verified against the crates and this machine:

| Needed | Why | README today |
|---|---|---|
| Rust ≥ 1.85 (edition 2024; developed on 1.99.1) | the workspace | yes |
| `build-essential`, `pkg-config` | linking | yes |
| `libasound2-dev` (ALSA headers) | `cpal` and `midir` speak ALSA directly | yes |
| A Wayland or X11 session | egui needs a surface to draw on | **no** |
| A working OpenGL 3.3+ / GLES driver (eframe's glow backend) | the panel is GPU-drawn | **no** |
| `xdg-desktop-portal` **and a backend** (`-hyprland`, `-gtk`, …) plus a D-Bus session | `rfd` is built with `xdg-portal`: `IMPORT`/`EXPORT` are the only file dialogs and they need the portal | **no** |
| PipeWire or ALSA for audio out | `cpal`'s default host | implicitly |
| `mdbook` (0.4.x) | building this guide only | **no** |
| `grim`, `slurp`, `imagemagick` | regenerating the guide's screenshots only | **no** |

The guide carries the first six as "what you need to play"; the README carries all of them, with the last two marked as
"only if you are working on the docs".

### Screenshots

Every capture comes from a scratch session, so nothing of the operator's is in the book:

```bash
TAKKA_ACID_DATA_DIR=/tmp/takka-guide cargo run --release -p acid-app --bin takka-acid
grim /tmp/shot.png                       # full output
magick /tmp/shot.png -crop WxH+X+Y +repage docs/guide/src/img/03-first-sound.png
```

The window is 980 × 640 (the design size) and its position depends on the compositor, so the crop geometry is derived
once from the first full-output capture, inspected and recorded in `src/img/README.md` — not guessed.

**What I can capture without a pointer**, because `wtype` can type keys at the focused window:

| Image | How |
|---|---|
| `first-sound.png` — the face, demo loaded, `UNSAVED` lamp | launch, wait, capture |
| `help-card.png` | `wtype -k F1` |
| `settings-card.png` (both sections folded) | `wtype -k F2` |
| `running.png` — playing, step LED chasing | `wtype -k space` |
| crops: `knob-row.png`, `step-row.png`, `switches.png`, `transport.png`, `top-strip.png` | `magick` crops of the face capture |

**What needs a pointer** (this machine has `grim` and `slurp` but no pointer-injection tool; `ydotool`/`dotool` are not
installed): the SONGS card, a settings section unfolded showing the device rows, and a hover tooltip. Three options, and
this is the one question the design leaves open:

- **a. The operator clicks and I crop** — I give the exact window states and the crop commands; three screenshots, no new
  software, and the shots are of the same build.
- **b. Install a pointer tool** (`dotool`, needs the `input` group; or `ydotool` with a uinput rule) and I capture all of
  them.
- **c. Leave those three to prose** and ship the key-driven shots plus crops. The SONGS card is the one that suffers: it
  is the card with the most words on it.

I would take **a**, and c as the fallback if the operator does not want to stop work to click three times.


### One place the face is wrong today

The help card says `saved on exit and 2 s after a change`. Only the exit write is unconditional; the two-second write
happens when `AUTOSAVE` is on (`main.rs` `logic`, the `self.panel.autosave &&` branch; README: "Saving is manual by
default"). The guide cannot quote that line and also tell the truth, so the proposed fix is the string itself: change it to
`saved on exit; autosave writes as you go` — a one-line edit in `ui/help.rs`, no behaviour change. That is the one
exception to "no product code changes", and it is asked for, not taken.

If the operator would rather not touch `rust/`, the guide states the behaviour correctly and the help card keeps its wrong
line; the reference page's "words for each state" table then carries a footnote saying which sentence the app gets wrong.
That is the fallback, not the plan.
### README changes

- Keep: the one-paragraph "what it is", installation, the two flags, "Working on it" (gates), plans, layout, licence.
- Move into the guide, and leave a pointer in the README: the whole "Demo songs and saving" section and the "Features"
  bullets' player-facing detail.
- Add to `Installation`: the prerequisites table (above), with the docs-only rows last, and a line saying the player's
  guide is in `docs/guide` (`mdbook serve docs/guide` to read it, `mdbook build` to make the site).
- Add to the layout block: `docs/guide/` with one line saying what it is.
- **One line in `AGENTS.md`**, asked for not taken: "a change to what the face says updates `docs/guide`". The guide quotes
  `ui/help.rs` and `ui/tips.rs`, which is a hand-kept second copy of two catalogues — the exact failure `AGENTS.md` warns
  about — and the non-goals rule out a Rust-side drift check, so a convention line is the cheapest honest mitigation.
### What is left to the builder

- The exact wording; how much of the gesture table goes on `08-writing-steps` versus the reference page.
- Whether `03-first-sound` uses the Slow Acid demo or a blank pattern to teach RUN.
- The crop geometry, after looking at the first capture.
- Whether `book.toml` gets a theme tweak beyond the default dark theme.
- How the reference page's gesture table is layered on top of `08-writing-steps` without saying the same thing twice.

## Rulings

- **The guide is the player document; the README stops duplicating it.** Otherwise every save-model change has two homes
  and one of them rots. Cost if wrong: a reader who only ever opens the README has to click once more.
- **One page per idea, numbered files.** Cost if wrong: renumbering when a section is inserted later; `SUMMARY.md` is the
  order that matters, so a renumber is a rename plus two edits.
- **Screenshots are committed PNGs with a provenance note**, not generated by the build. Cost if wrong: they age; the note
  in `src/img/README.md` says which commit and date they came from, which is what makes them replaceable.
- **No product code changes for the guide's sake.** If a screenshot needs a state the app cannot reach with the keyboard,
  the answer is a click, not a debug flag. Cost if wrong: three screenshots need a human.
- **The guide quotes the panel's labels; it states behaviour from the code.** A label is what a reader has to recognise on
  the face, so it is copied exactly; a sentence about what the app *does* is checked against `main.rs`/`ui/` and written
  truthfully even where a help line is stale. Cost if wrong: one place (the save line, above) where the guide and the face
  disagree until the string is fixed.
- **No numbers for the knobs.** The panel shows positions, not values, and accepts no typed number, so the guide teaches
  positions and pictures; `SavedState` is a contract and stays out of a player's manual. Cost if wrong: a reader who wanted
  `cutoff: 0.35` has to dial it by eye — which is what the instrument asks of them anyway.
