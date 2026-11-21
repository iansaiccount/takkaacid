# Handover — after phase 4, into the rack round (for a fresh session)

Paste this whole file as the first message. It is written to be self-contained: the session it is for has no memory of the one
that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth and drum machine in Rust, at
`/home/ian/Projects/takkaacid`, on branch **`phase-4`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the
project's standing instruction and outranks anything here that contradicts it. Then read `plans/phase 4/00-phase-4.md` (the
phase that just finished), and the plan it ran to, `plans/phase 4/01-the-drum-machine.md`, whose `## Progress` is the record
of every ruling and every review round.

**Where the branch stands.** Phase 4 is **complete**: all ten tasks are committed on `phase-4`, and the work is **local
only** — nothing is pushed, and the player decides when it goes up. Gates at the head of the branch:
`cargo test --workspace` (**870**), `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` — all green. `SAVE_VERSION` is **11** (`SavedState::kit`); the rack round takes **12**. The
whole-branch review of the phase is the commit that carries this file; its findings are quoted at the bottom.

> **Superseded for the head of the branch.** This file was written when phase 4's ten tasks were the tip. Seven rounds have
> landed since (`02`–`09`), so the gate line above is history: `plans/phase 4/00-phase-4.md`'s **State** paragraph is the
> current one, and its round list is the current history. The rules it states still hold; the branch review it calls done is
> now due again at the pull request (`AGENTS.md`).

## What phase 4 shipped, in one page

- **A second device kind.** `DeviceKind::{Acid, Drums}`; a `DRUMS` device has its own state, bank, ports, channel, colour and
  name, in the same rack, on the same clock, bar, `REVERSE`, transpose and swing as the 303. Its machine is
  `audio::renderer::Machine`, chosen once at open from `SharedState::kind` (the one answer to "what kind is this device" —
  `Device` has no `kind` field).
- **The pattern serves both machines.** `Step` carries `hits: u16` and `accents: u16` beside the 303's fields, written only
  through `Step::hit(Drum, accented)` / `clear_hit(Drum)`, so a step is "on" because one of the two machines says so.
- **A drum machine** (`acid_core::drums::Drums`): sixteen sample voices, per-voice ADSR, a state-variable `LP | BP | HP`
  filter, tuning, a mute mask and a choke group for the hats, all on the 303's `Sequencer` through the new `Performer`
  trait.
- **Kits.** A kit is sixteen voices; factory kits (`808`, `909`, `Acoustic`) are compiled in and read-only, a kit is a folder
  `<data>/kits/<name>/kit.json` + one WAV per voice, the player's own import from WAVs, and `SavedState::kit` names the one
  the saved device plays.
- **The face** (`ui/drum_panel.rs`): the seven 303 slots repurposed as the selected voice's dials, `LP | BP | HP`, `HIT`,
  the device's `SHUFFLE`, and a sixteen-row grid whose rows are `Drum::ALL` — the editor, in place of the step and
  performance rows.
- **Five factory drum banks**, one per demo variation, carried by `Demo::drums` beside the songs; a `DEMO BANKS` row on a
  drum machine loads that demo's eight grooves at its tempo and shuffle, and leaves the kit and the session's name alone.
- **The book**: `docs/guide/src/22-the-drum-machine.md` and the pages that link it, plus the two pictures, and `check.sh`'s
  pins (`KICK`, `LP | BP | HP`, `MY KITS`, `drum bank`, `drum machine`, `DRUMS`).

## The round after this one, as the project has it recorded

The next round is the **rack document** — the whole session saved rather than the first device. The record for it is the
wishlist's first entry under *Decided, waiting for its round* (`plans/future/wishlist.md`), and the spec's **Non-goals** say
the same: `SAVE_VERSION` **12**, `SavedState` gaining a `devices` list of `SavedDevice { name, kind, inputs, bank, knobs }`,
`sanitize` repairing the list the way it repairs a bank, and the picker's `NAMES AND EXTRA DEVICES ARE NOT SAVED YET`
footnote finally going away. (The wishlist's shape is the record's, not a design: what a device's entry must also carry —
its kit's name, its colour and scheme, both of which live only in memory today — is the first thing the round's plan has to
decide.) **There is no plan for it yet**: the project's rule is that a round gets a numbered plan in `plans/phase N/` before
code is touched, and that plan is written from a design doc agreed with the player (`docs/superpowers/specs/`). So the first
task of the next session is not code.

## What is outstanding, smallest first

Each of these is also in `plans/future/wishlist.md`, which is where an idea goes before it earns a round.

1. **Per-voice hover help on the drum face.** The seven dials, the `LP | BP | HP` key and `HIT` print their captions and no
   tooltip; `SHUFFLE` and the shared keys keep the 303's tips. `ui::tips` is a 40-variant catalogue with a positional `TIPS`
   table, so this is nine new player-facing sentences plus the mapping from a drum dial to a `Control` — a face change, so
   it wants the player's eye on the words. Page 22 says plainly that there is none, which is true today.
2. **A rate change undoes the player's mutes.** Re-decoding a kit at a new sample rate goes through `load_kit_now`, which is
   the *load* path (`adopt_kit`) and reseeds `drum_mute` from the kit's own flags. The fix is either a parameter on
   `load_kit_now` ("load" vs "re-decode") or the rate-change arm publishing through `set_kit`; it lives in `main`, which
   has no tests, so paying it means moving the rate-change arm into a function a test can call.
3. **The drum machine's gain cannot take the idle snap.** `Engine::set_volume` jumps its smoother when the transport is
   stopped and no voice is sounding; the drum machine has no "nothing is sounding" answer, so a `HIT` pressed in the same
   block as a VOLUME move fades in over its 10 ms. The wishlist says exactly what `Drums` would have to answer.
4. **A newer save file is not refused.** A document whose `version` is above `SAVE_VERSION` loads as best it can instead of
   saying so. It is the one path every file goes through, so it belongs with the round that rewrites that path — the rack
   document.

## Questions the next session should put to the player, not decide

- **Does the branch go up?** Nothing is pushed. The phase is the whole round on one local branch; the player decides when
  it becomes a PR (the repository's history so far merges a phase at a time).
- **The drum machine's demo banks are one gesture away from a song.** Today a `DEMO BANKS` row loads the acid song on a 303
  and the drum bank on a drum machine, by name. The spec's "later, a song loads the acid bank and the drum bank together" is
  the rack round's, and whether it is one click or two is a face question.
- **Per-voice hover help** (item 1): nine sentences on the face are the player's to hear before they are written.
- **The grooves and the kits have not been heard.** `EAR_TESTING.md` is how the project judges sound by ear; the five drum
  grooves and the three kits' per-voice defaults were written from the head, both records say so, and no test can replace a
  listen. If a groove is wrong it is a table; if a kit's balance is wrong it is constants in `kits.rs`.

## Environment facts that cost previous sessions time (still true)

- **The shell blocks** heredocs, `python3 -c`, `bash script.sh`, `xargs`, `identify` and any unlisted binary. Write a script
  with the editor and run `python3 path/to/script.py`; `.superpowers/` is the git-ignored scratch directory, and `/tmp` is
  fine for a shell redirect (`cargo test > /tmp/t.log`) but not as an editor path.
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect and sum with the `awk` one-liner in `AGENTS.md`'s
  neighbourhood; the count at the end of phase 4 is **870**.
- **Never `git checkout <file>` to undo a scratch mutation** — it reverts to HEAD and takes the session's uncommitted work
  with it (it happened in task 9 and the file had to be rebuilt by hand).
- **Screenshots:** check `xdotool getdisplaygeometry` says at least `2000 1400` before launching, then
  `xdotool getwindowgeometry` must say `1960x1340`; something on this machine pre-starts `Xvfb` at 1280×800 and a bigger
  request on a taken display fails quietly. `xdotool key` needs `xdotool windowfocus <id>` first under a bare Xvfb with no
  window manager. Park the pointer away from the panel before a grab.
- **A picture is a test.** Task 10's two pictures exposed a real defect (the kit browser printed `MY KITS` over its first
  row) that the suite had never asked about; look at every grab before it goes in the book.
- **Mutation scripts are the proof, and a `SKIP` is never a pass** (`.superpowers/mutate_task*.py`). The last one taught
  this: a guide mutation that walks `src/*.md` misses `src/img/README.md`, and a pin that lives in two places keeps passing.

## The whole-branch review of phase 4

**Verdict: BLOCK on one Critical and three Importants, all fixed in the commit that carries this file.** The seams between
the ten tasks held up under the reviewer's look — one answer to "what kind is this device", one mute-mask pack, one clock,
one save field, every catalogue match written out, the audio path still allocation-free by the gate that exists — and the
book described the panel that ships apart from the sentences below.

- **Critical — the page the new page hangs from printed its paragraph twice.** `04-the-face`'s "The second face" began
  twice because the first draft's continuation was deleted and its opening sentence was left behind. `check.sh` proves
  presence and absence, not duplication. Fixed; nothing else in the guide duplicated.
- **Important — page 22 said the drum filter was the 303's.** It is a state-variable filter where the 303's is a ladder; what
  the two share is the cutoff/resonance *mapping*, which is what makes `LP | BP | HP` the same idea heard three ways. Page
  corrected.
- **Important — `LEARN` on a drum face could copy a kit.** The key is drawn on every face and there is no `CcTarget` for a
  drum voice, so a click picked nothing — but a *drag* turned the voice and, on a factory kit, made the "read-only" copy
  because the hand meant to teach. The dials and the filter key now stand still while a learn is choosing, a test holds it,
  and the wishlist carries the two ways to finish the job (per-voice CC, or hiding `LEARN` there — a face change, so the
  player's call). Page 22 says what `LEARN` can do on that face today.
- **Important — the factory kits' sample tables were positional and nothing bound a row to a voice.** Swapping the 808's
  kick and snare rows left every existing check green. `every_factory_voice_plays_the_file_named_after_it` now decodes the
  repository's `<VOICE>.wav` and compares it to the compiled-in voice, and the swap is a recorded mutation.
- **Minors, all actioned:** page 22's kit folder is written when the session is saved, not when a dial moves; `20-reference`
  and `ui/dispatch` count five cards inside the panel and name the kit browser in the escape order; the kit is in the three
  enumerations of what is on disk (`20`'s `state.json` line and files table, `18`'s restart paragraph, `12`'s one-song
  section); `1u32 << drum.index()` is `u32::from(drum.bit())`; and `Kit::set_mute_mask` is deleted — no production caller,
  and a doc that claimed a live unpack nobody performs. The retired `Arc<Kit>` freed on the audio thread is now a wishlist
  line, because it ships knowingly and the record did not say so.

**Where it leaves the branch:** `cargo test --workspace` **870**, clippy, fmt and `./docs/guide/check.sh` green, and
`.superpowers/mutate_task10.py` **9 of 9** caught with nothing skipped. Nothing is pushed. The player decides when it goes
up.
