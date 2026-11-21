# Plan 04 — `HIT` is the only sound a selection makes

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`, one commit per task, and a subagent review of the
> branch at the pull request (`AGENTS.md`).

**Goal:** selecting a voice on the drum face — by clicking its label or by walking the grid with `up` / `down` —
makes no sound. `HIT` plays the selected voice; nothing else does.

**Spec:** none. This is the player's correction to plan 03's ruling, in their words: *"Stop the drum hits sounding when I
click the name of the sound on left hand side. The hit button is good enough. That was my mistake."* Plan 03's Progress keeps
the then-ruling and this round supersedes it; a plan is a record, so neither is rewritten.

## What is wrong today

Two places sound a voice as a side effect of moving the selection, and both were deliberate:

- `ui::drum_panel`'s label click: `state.drum.selected = drum; shared.request_audition(drum);` — added in plan 02's task 3
  ("a voice's label is its preview") because a click used to do nothing audible and the `Init` kit's silence read as a broken
  machine. Plan 02's Progress also records that the fix that made a machine audible removed the reason: a kit now arrives.
- `ui::dispatch`'s `DeviceKind::Drums` arm: the walk requests an audition of every voice it lands on, added in plan 03 to match
  the label's preview.

The player has now used both and does not want them: a selection is not a hit. It also made the walk unusable as navigation —
holding `down` fired a hit per row.

## The design

- **The label selects and mutes; it does not sound.** A plain click sets `panel.drum.selected` (and so the dials and the
  plate); a `SHIFT`-click still toggles the mute. No `request_audition`.
- **The walk selects; it does not sound.** `up` / `down` still clamp, still refuse behind a card, still leave the
  acid face's note nudge alone, and still repeat while held — silently.
- **`HIT` is the one sound**, with the transport stopped or not, exactly as it is today.

The book says it: page 22's label paragraph loses "and hears it" and the sentence that sells a row by ear, and gains the one
thing a player needs in its place — `HIT` is how a voice is heard. Page 22's keyboard paragraph loses "each voice they land on
is heard, so a kit can be walked by ear". Page 15 and page 20 never claimed the walk sounds, so they do not move; `check.sh`'s
`walk it up and down the grid` pin still holds.

**Ruling, and what it costs if wrong:** the selection never sounds, on either route. If the player wants a silent walk but a
clicking preview back (or the reverse), each is one `request_audition` line and a test; they are deliberately not a setting,
because a face should not need a switch to say whether a click is a hit.

## Tests that make it safe

In `rust/crates/acid-app/tests/drum_live.rs`, through `draw_panel` for the click and through `dispatch::presses` /
`dispatch::handle` for the keys, the way `main` delivers them:

- `clicking_a_voices_label_selects_without_sounding_it` (renames and inverts plan 02's test) — a plain click on `SNR`'s label
  leaves `panel.drum.selected == Drum::Snare` and `shared.take_audition() == None`; a `SHIFT`-click still mutes and still does
  not select.
- `plain_up_and_down_walk_the_selected_voice` — keeps every assertion it has except the auditions: after `down` and
  after the held repeat, `shared.take_audition()` is `None` while the selection has moved.

Mutations: put an audition back in the label click, and in the walk — each must fail a named test — beside plan 03's four
(no clamp, no card refusal, swapped directions) re-run to prove the walk's tests still bite.

## Tasks

### Task 1 — the selection is silent

**Files:**
- Modify: `rust/crates/acid-app/src/ui/drum_panel.rs` (the label click)
- Modify: `rust/crates/acid-app/src/ui/dispatch.rs` (the walk)
- Test: `rust/crates/acid-app/tests/drum_live.rs`
- Modify: `docs/guide/src/22-the-drum-machine.md`, `plans/phase 4/00-phase-4.md`

**Interfaces:**
- Consumes: nothing new.
- Produces: nothing new. `SharedState::request_audition` keeps its one caller, `HIT`.

1. Rewrite the two tests to expect silence; run them — expected FAIL (the audition slot holds the selected voice).
2. Delete the two `request_audition` calls and the comments that justify them. Run the tests — expected PASS.
3. Mutation check, recorded in Progress: label click auditions, walk auditions, no clamp, no card refusal, swapped directions.
4. Page 22's two paragraphs; list this round in `00-phase-4.md`; run `./docs/guide/check.sh`.
5. Full gates, review, commit.

## Non-goals

A keyboard key for `HIT` (the player has asked whether one exists; that is its own decision and, if wanted, its own task); the
label's selection behaviour, the mute, the walk's clamp, its repeat and its card rule, all of which stay; a settings switch for
preview sound; a save-file change; a screenshot (the face paints exactly what it painted).

## Progress

- [x] **Task 1 — the selection is silent.** The two tests were rewritten first and failed with the audition still in place
      (`clicking_a_voices_label_selects_without_sounding_it` and the walk's three `None` assertions), then the two
      `request_audition` calls went: the label click keeps `state.drum.selected = drum` and the `SHIFT` mute, and the walk keeps
      its clamp, its repeat and its card refusal. `HIT` is now the one non-test caller of `SharedState::request_audition` (the
      tests in `tests/mixer.rs` and `tests/no_alloc.rs` call it too). **Mutations: 5 of 5 caught, 0 skipped**
      (`.superpowers/mutate_hit_only.py`) — a label click that auditions again, a walk that auditions again, no clamp, no card
      refusal, swapped directions — each named by the test that caught it. Gates: `cargo test --workspace` **880**, clippy
      `-D warnings`, `fmt --check`, `./docs/guide/check.sh` ok.
      **The book moved:** page 22's label paragraph now says neither a selection nor a mute makes a sound, and names `HIT` as how
      a voice is heard; its keyboard paragraph says the walk selects and does not sound. Page 20 already said only "label
      selects the voice", page 15 never claimed the walk sounds, and `check.sh` gained a negative guard —
      `! grep -rqE 'hears it|walked by ear|identify by ear'` — so the withdrawn sentences cannot come back. `00-phase-4.md`
      lists this round, and plan 03's Task 2 entry now carries a pointer that this round supersedes its audition. No screenshot:
      the face paints nothing new.
      **Ruling:** the selection never sounds, on either route, and it is not a setting — a face should not need a switch to say
      whether a click is a hit. If the player wants the click's preview back and not the walk's (or the reverse), each is one
      `request_audition` line and one test.
      **Review: PASS on one Important and four Minors, every one actioned.** The Important: the walk test's comment still said
      the held key "hears each one" two lines above the assertion that it sounds nothing — the last comment the plan's step 2
      ordered deleted; it now says "walks row after row, silently". Of the Minors: `check.sh` gained the guard above rather than
      leaving the rule to the next editor; plan 03's entry names its supersession; page 22's mute is silent by the same sentence
      rather than by implication; and this entry says "non-test caller" rather than overstating the census.
