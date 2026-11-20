# Plan 02 — the drums are silent, and the kit cannot be replaced

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, each commit reviewed by a
> dispatched subagent before it lands (this repo's rule, which is stricter than the skill's default).

**Goal:** a drum machine makes a sound when it is added, plays the pattern drawn on it, says which kit it is playing, and lets
a player hear any single voice with one click.

**Spec:** [`docs/superpowers/specs/2026-11-18-drum-machine-design.md`](../../docs/superpowers/specs/2026-11-18-drum-machine-design.md)
(the kit and the face argue from it; the bugs do not — a silent machine is a defect, not a design).

## What is wrong today, with the evidence

Reported from a real session: *"I can't hear the drums at all. A kit starts with the `Init` kit … I can't replace the `Init`
kit either. Playing a pattern doesn't play any sounds."* Reproduced on a headless display at 1960 × 1340 with a fresh
`TAKKA_ACID_DATA_DIR` (`Xvfb :88`, the app at 48 kHz): `+` → `DRUMS`, `BANKS` → `Hard Techno Acid` fills the grid, `SPACE` runs
the bar, and the machine is silent; `F3` → `KIT` → click `808` leaves the header reading `NOW: Init`, with no notice visible.

1. **A drum device added from the picker never gets a kit.** `SharedState::with_tempo_for` builds `default_kit()` — the
   `Init` kit, sixteen voices with **empty** samples (`acid-core/src/drums.rs`, `state.rs:239-241`) — and the only automatic
   load is `main`'s startup arm, which loads **device 1**'s kit (`main.rs:502-509`), plus the rate-change arm. A drum machine
   added later is device 2, and nothing ever loads a kit into it. It is silent for ever, and its pattern is silent with it.
2. **No kit can be loaded by hand either, and nothing says why.** `main`'s status copy is a **no-op**: it reads the state's
   `AudioStatus`, and when the rig's line differs it writes the *same* status back (`main.rs:176-195`); the rig keeps only a
   string (`audio/mod.rs:330-333`). So `SharedState::status` stays `Starting`, `main::sample_rate()` reads that field and
   answers `None` for ever (`main.rs:644-652`), `kit_rate` is never set, and `apply_kits_request` refuses every load with *"the
   audio device is not running yet, so no kit can be loaded"* (`main.rs:766-773`) — while the stream has been running since the
   first frame.
3. **The kit browser covers the status line**, so even that refusal is invisible: the notice lands on the top strip, under a
   card that fills the panel. A player sees a click do nothing.
4. **The face does not say which kit it is playing** (the name is on the device card's `KIT` row, which the player has to
   open), and **one click on a voice's row does not audition it** — a plain click only selects the voice for the dials, and
   `HIT` plays the *selected* one. That is why the `Init` kit's silence reads as "the drum machine is broken" rather than
   "this kit has no samples".

## The design

- **The rig is the authority for the stream's rate.** `Rig::sample_rate() -> Option<f32>` reads its own handle; `main` asks it
  for the rate it decodes at. The `SharedState::status` field stops being a pretend-copy and becomes honest: every device's
  state is told the rig's `AudioStatus` when it changes (one loop, one clone), or the no-op block goes and the field is
  documented as the rig's mirror. Either way, **no kit decision depends on it.**
- **A kit is filled in, not waited for.** A drum device whose kit has no samples at all has never had one; `main` gives it the
  default factory kit (`kits::default_name()`, `808`) the frame the rate is known and again after any device is added. A
  device whose kit *has* samples is never touched, so a player's kit, a factory kit they picked, and a kit that failed to load
  (which leaves the old one in place) are all left alone.
- **The face says the kit's name**: `KIT 808` at the right-hand end of the caption band above the dials, in silk, from
  `shared.kit().name` — so the face and the card cannot disagree.
- **A voice's label is its preview**: a plain click on a row's label selects the voice **and auditions it** (the same
  `request_audition` `HIT` uses), so one click hears what the row is. `SHIFT`-click still mutes, `HIT` still plays the
  selected voice, and the transport does not have to be running for either.
- **The browser's refusals are visible on the card**: `apply_kits_request`'s "not running yet" line also goes to
  `PanelState::kits.message`, which the card paints — "a message is a notice, not a dialog", and a notice that a card covers is
  not a notice.

## Tests that make it safe

In `rust/crates/acid-app/tests/drum_live.rs`, through `draw_panel` as the face is:

- `a_drum_device_with_no_kit_is_given_the_default_one` — a rack, a `DRUMS` device, `kits::fill_silent_drum_devices` at 48 kHz:
  the device's kit is named `808`, every voice has samples, and a device that already had a kit is untouched.
- `the_face_names_the_kit_it_plays` — the drum face paints the kit's name (a text shape carrying `808`, and `Init` after a
  fresh state), so the picture and the face agree.
- `clicking_a_voices_label_auditions_it` — a plain click on `KICK`'s label asks the state for a `KICK` audition
  (`take_audition()`), and a `SHIFT`-click still mutes instead of auditioning.

In `rust/crates/acid-app/src/kits.rs` the predicate the fill uses is **`needs_a_kit(kit)` — the `Init` kit's own name and no samples**,
and it is pinned in `tests/kits.rs` by `a_drum_device_with_no_kit_is_given_the_default_one` and
`a_players_kit_that_decoded_to_nothing_is_not_filled` (a kit with samples does not need one; a *player's* kit with none does not
either). The status mirror is pinned in `audio/status.rs` (`the_mirror_takes_the_device_from_the_stream_and_the_reason_from_the_line`
and the `is_failed` test beside it), so the two pure pieces of this round have unit tests where they live.

What has no test is the wiring: `logic`'s copy loop and the fill call, `apply_kits_request`, and the `kit_notice` seam — the
`TakkaApp` they hang off needs a live `Rig` and a window. The **manual reproduction is their proof**: the same click sequence as
above, ending with `NOW: 808`, `KIT 808` on the face, and the grid audible. Its screenshots go in the Progress, not the book.
(`main.rs` does have a small test module — three tests about ports and demo patterns — so "the binary has no tests" is not the
reason; the reason is that these call sites need a device and a window.)
## Tasks

1. `Rig::sample_rate()` and `Rig::audio_status()` (through `audio::status::mirror`), `main::sample_rate()` reads the rig, and the
   status block tells every device what the rig says. Prove the rate reaches the kit loader by the manual run: `NOW: 808` after a
   click.
2. `kits::fill_silent_drum_devices(&[Arc<SharedState>], rate) -> usize` + `kits::needs_a_kit(&Kit) -> bool`, called from
   `main` when the rate is known and again every frame — a device added from the picker is an event the rate arm never sees. The
   shipped shape is `kits::fill_if_silent(&SharedState, rate) -> bool`, asked per device with no `Vec` per frame, and the test is
   `a_drum_device_with_no_kit_is_given_the_default_one` in `tests/kits.rs` (the plan first named a slice-taking function and a
   `drum_live.rs` test; corrected here).
3. `drum_panel`: the kit's name in the caption band; the label's click auditions. Live tests
   (`the_face_names_the_kit_it_plays`, `clicking_a_voices_label_auditions_it`).
4. `main`: every kit message goes through one `kit_notice` seam, which puts it on the browser's own line while the browser is up
   as well as on the status line — the browser fills the panel, and a notice under a card is a notice nobody reads.
5. The book: page 22 gains "the face names its kit" and "a click on a voice's label hears it"; page 19 unchanged; `check.sh`
   pins unchanged (the kit's name is data, not a fixed word).
6. The wishlist: `LEARN` stays on the face and the feature is the wishlist's (already written there); the one-gesture demo
   pairing is the rack round's (already written there, and the player has said it waits).
## Non-goals

Per-voice MIDI CC and MIDI pads; a kit picker on the face; a save-file change (`SAVE_VERSION` stays 11); a second kit format.

## Progress

- [x] **Task 1 — the rate comes from the rig.** `Rig::sample_rate() -> Option<f32>` reads its own handle; `main::sample_rate()`
      asks the rig, and `logic`'s status block now tells **every** device's state what the rig says — it used to read the
      state's status and write the same value back, which is the bug that made the rate `None` for ever. **Ruling:** the rig is
      the authority for "what rate is a kit decoded at", and the state's `AudioStatus` is its mirror; if wrong, one accessor and
      one loop. The copy loop in `logic` has no test (it needs a live `Rig` and a window), so its proof is the manual run below; the
      mapping it feeds is `audio::status::mirror`, which does.
- [x] **Task 2 — a silent drum machine is given the default kit.** `kits::needs_a_kit(&Kit)` answers "nobody has loaded one"
      (every sample empty), and `kits::fill_if_silent(&SharedState, rate)` gives such a device the default factory kit (`808`).
      `main` asks once a frame inside the rate arm, so a device added from the picker, a saved session whose kit failed to load,
      and the first frame after the stream opens all end up audible; a device that already has samples is never touched.
      Test: `a_drum_device_with_no_kit_is_given_the_default_one` (tests/kits.rs) — a fresh drum device is filled, asking again
      changes nothing, a kit the player picked keeps its name, and the 303 has no kit to fill. **Ruling:** the fill runs every
      frame rather than only on the rate arm, because "a drum device appeared" is an event the rate arm never sees; the cost is
      two cheap checks a frame and a decode only when a device has no samples. If wrong, one call site.
- [x] **Task 3 — the face says its kit, and a voice's label is its preview.** `KIT <name>` is printed at the right-hand end of
      the caption line above the dials, from `shared.kit().name` (so the face and the card's `KIT` row cannot disagree), and a
      plain click on a row's label now *auditions* the voice as well as selecting it — the `SHIFT`-click that mutes still does
      not make a sound. Tests: `the_face_names_the_kit_it_plays` (the painted text is `KIT Init` on a fresh state and `KIT 808`
      after a factory load) and `clicking_a_voices_label_auditions_it` (the click leaves `Some(Drum::Snare)` in the audition
      slot, and the mute leaves `None`).
- [x] **Task 4 — a refusal the card covers is a refusal nobody reads.** `apply_kits_request`'s two "the audio device is not
      running yet" lines also land in `PanelState::kits.message`, which the browser paints at its foot. Not tested (main).
- [x] **Task 5 — the book.** Page 22: the label both selects and hears the voice (and a `SHIFT`-click does not), the face
      prints `KIT <name>`, and the new paragraph that says a machine arrives on `808` while `KIT Init` means "no samples".
- [x] **Task 6 — the wishlist.** `LEARN` stays on the face and the entry says why and what the feature would need; the
      one-gesture demo pairing stays the rack round's. Both were already written; nothing changed but the wording checked
      against the player's answer.

**The manual reproduction, before and after.** Before: fresh data directory, `+` → `DRUMS`, `BANKS` → `Hard Techno Acid`, `SPACE`
running, silent; `F3` → `KIT` → click `808` left the header on `NOW: Init`. After the fix, the same sequence: the face reads
`KIT 808`, the header says `NOW: 808 - READ ONLY`, and the machine is audible. (Screenshots in `.superpowers/`, not the book: they
are evidence for this record, and the book's pictures are the ones `img/README.md` records.)

**Mutations: 5 of 5 caught, 0 skipped** (`.superpowers/mutate_silent_drums.py`) — the fill made unconditional (a kit the player picked
is overwritten); the `Init` name dropped from the predicate (a player's kit that decoded to nothing is taken away); the kit caption
removed from the face; the label's audition removed; and the status mirror putting the display line where the device name belongs.

**Review: BLOCK on two Importants, both fixed here, and every Minor actioned.** (1) **The fill could take a kit away.** Its
predicate was "all sixteen samples are empty", and `persist::load_kit` is *designed* to load a folder whose WAVs are missing —
with a problem per voice and an empty sample for each. So a player who loads or imports such a kit would have watched the face
say `KIT 808` one frame later, against page 22's promise that "a kit whose samples will not read still loads" and page 19's
"putting the right WAV in the folder … is the whole repair". `needs_a_kit` now means **the `Init` kit** — the reserved name and
no samples — and `a_players_kit_that_decoded_to_nothing_is_not_filled` pins the case. (2) **The status mirror carried the wrong
fields.** It put the rig's *display line* into `AudioStatus::Running.device` (`"48 kHz - PipeWire"`), which ticked no row on the
settings card because no row is called that, and into `Failed.reason`, which `status_text` then prefixed again. The mapping is
now `audio::status::mirror(Option<(&str, u32)>, line)` — the stream's own device and rate, or the failure without its `failed: `
prefix — with `Rig::audio_status()` asking it and `the_mirror_takes_the_device_from_the_stream_and_the_reason_from_the_line`
holding both. The mirror is also *testable* now, which the first cut's "no test can reach it" was wrong about: `main` has a
`#[cfg(test)] mod tests`, and the pure part simply did not live in one. Of the Minors: page 22 said the kit caption was "the
same silk" as the voice's name when it is smaller and dimmer (it says dim silk now); the record now says **the fix revives a
path nobody had exercised** — `write_kit` early-returns when `kit_rate` is `None`, and `kit_rate` was never set, so no kit edit
had ever written a folder from the app and "saved with the session" had always been a promise the app did not keep. It is
reachable now, and the manual reproduction below exercises it (edit a dial on `808`, save, look for
`<data>/kits/808 (2)/kit.json`); "which factory kit is this name" is one `kits::factory_kit(name, rate)`, asked by both the
panel's load path and the fill; and the Tasks section above is reconciled with what shipped.
**The browser's messages are one seam**: `main::kit_notice` writes the status line and, while the browser is up, its own line —
so a failed load, a voice that would not decode, a copy's name and an import's hole are all visible on the card the player is
looking at, instead of being painted underneath it. (Notices from other subsystems keep the one notice the panel has; this is the
kit's seam, not a card-specific message list.)

**The revived `write_kit` path, exercised by hand, and what it taught.** With the rate fixed, the copy rule finally reaches the
disk: on a fresh session, `+` → `DRUMS` — on `808`, the fill's doing — then `F3` → `KIT` → `909` (the browser reads `NOW: 909`),
`CLOSE`, and turn `ATTACK`: the face says `KIT 909 (2)`, because the copy takes the *loaded* kit's name. Then `BANKS` → `SAVE`, and
`<data>/kits/909 (2)/` holds `kit.json` and the sixteen WAVs, with `state.json` beside it.
The document says `"kit": null`, which is right by this phase's rule and worth writing down: the save file names **the saved
device's** kit — device 1, the acid machine, whose kit is `Init` and is therefore no name at all — so a kit edited on the second
machine is written as a folder and named by nobody until the rack round. That is plan 01's recorded limitation (task 6's entry,
`plans/phase 4/01-the-drum-machine.md`), now verified rather than assumed.
