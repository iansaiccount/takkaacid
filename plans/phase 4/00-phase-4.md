# Phase 4 — the drum machine

Phase 4 is a second machine in the rack: **sixteen sample voices against the 303's sixteen steps**, with their own kits and the
same bar, banks, transport and themes. Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-the-drum-machine`](01-the-drum-machine.md) — the whole phase in one round: the face first, then the pattern's masks, the
  machine, the kits and the factory banks.
- [`handover-task-10`](handover-task-10.md) — superseded. The briefing the last task's session worked from; kept for the record.
- [`handover-phase-5`](handover-phase-5.md) — **the live handover.** Where the phase finished, what it shipped, what is
  outstanding, and the questions the next round starts from.
- [`02-the-drums-are-silent`](02-the-drums-are-silent.md) — the defect round after the phase: a drum device added from the picker
  had no kit, a kit refused to load because the rig's rate was never learned, and the face did not say which kit it played.
- [`03-the-kick-and-the-voice-keys`](03-the-kick-and-the-voice-keys.md) — the `808` kit's kick was a 40 Hz sine, inaudible on a
  small speaker, so it is replaced by an audible CC0 kick and pinned by a test; `shift+up` / `shift+down` walk the drum face's
  selected voice up and down the grid.
- [`04-hit-is-the-only-sound`](04-hit-is-the-only-sound.md) — the player's correction to plan 03: a click on a voice's label and a
  walk of the selection make no sound; `HIT` is how a voice is heard.
- [`05-the-hit-key`](05-the-hit-key.md) — the keyboard's `HIT`: on a drum face `h` strikes the selected voice, and the press filter
  now asks which face is showing, because the same `h` is one of the 303's notes.
- [`06-the-drum-step-editor`](06-the-drum-step-editor.md) — **planned, next.** The drum grid gains the 303's step editor: plain
  `left` / `right` move a lit column, `enter` writes the selected voice into that step (silently; `shift+enter` accents), and
  the selection does not advance on a write.
- [`07-the-drum-keys-and-tooltips`](07-the-drum-keys-and-tooltips.md) — **planned, after 06.** The filter takes `L` / `B` / `P`,
  each key underlines the letter that presses it (`H` on `HIT`), every drum control gets a tooltip, and the help card learns
  the drum face's keys.

**Spec:** [`docs/superpowers/specs/2026-11-18-drum-machine-design.md`](../../docs/superpowers/specs/2026-11-18-drum-machine-design.md)
— approved in conversation on 2026-11-18. The plan argues from it; where they disagree, the spec wins and this plan is wrong.

**Gate:** unchanged from phases 1–3 — the three workspace commands, `docs/guide/check.sh` when the book moves, and a fresh
subagent review before every commit (`AGENTS.md`).

**State: complete, 2026-11-19; four rounds since, two planned (06, 07).** All ten tasks are committed on `phase-4` (local
only — nothing is pushed). The branch is the whole round: `cargo test --workspace` **870**, `cargo clippy --workspace
--all-targets -- -D warnings`, `cargo fmt --check` and `./docs/guide/check.sh` all green at its head, and each task's own
mutation run is recorded in the plan's `## Progress`. `SAVE_VERSION` is **11**; the rack round takes **12**. The whole-branch
review's findings are in `handover-phase-5`.

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| The drum pattern is the 303's `Pattern` with two `u16` masks added | Every pattern grows for the 303 too, and `SAVE_VERSION` moves. The alternative — a second pattern type — makes the bank browser, the clipboard and the save file branch. |
| One machine per device; the machine is an enum in `AudioRenderer` | A third machine is an arm in five `match`es rather than a file. The wishlist's trait object converts later if a machine ever wants its own crate. |
| A kit is a folder; factory kits are compiled in and read-only | About 2 MB in the binary, and editing a factory kit makes a copy. A bundled kit cannot be fixed in place. |
| The drum face draws from `theme::Palette` only | More work in every drawing call than `theme::BLACK`; a hard-coded colour breaks a scheme and is caught by a test. |
| Factory drum banks ship in code beside the acid factories | The demo variations' drum patterns are data in the binary; changing one means a build, not a file. |
