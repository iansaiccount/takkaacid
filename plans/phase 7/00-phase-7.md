# Phase 7 — the file is a document, and the bar can be shorter

Phase 7 is the phase where the two things phase 6 left half-said are finished.

**The file learns to refuse.** A save file from a *newer* build is currently relabelled: `upgraded` stamps this build's
version over whatever it read, and the next save rewrites a newer document as an older one — silently, and with whatever
fields the newer build had already dropped. Phase 6 made that sharper, not softer: a version-14 file holding a pattern
grid this build has never heard of is read as a `1/16` bar and its sound is gone after one save. The number exists to
prevent that and nothing reads it before it is overwritten.

**The bar can be shorter than its pattern's grid.** Phase 6 made "the bar" a question every consumer asks the pattern.
What it deliberately left is a pattern *shorter* than its bar — the half that would break "the bar is the same bar for
every device", and the reason it is a phase of its own rather than a flag on round 02.

Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-refuse-a-newer-file`](01-refuse-a-newer-file.md) — `persist::load` names a newer document instead of relabelling
  it; `open` gains its second face (a first run, plus a status-line notice); `load_song` and `import_song` follow; the
  refusal never writes. **Nothing on the face changes** beyond one notice, so the book moves in one paragraph.
- [`02-pattern-length-per-device`](02-pattern-length-per-device.md) — a pattern of fewer steps than its bar repeats
  inside the bar, so every device's bar line still lands together. **Not ready to execute**: its design questions are
  listed there for the player to rule on first.

**Gate:** unchanged from phases 1–6 — `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and `./docs/guide/check.sh` whenever the
book moves. A task ends with the gates green, its `## Progress` line current, and a commit. The review is of the branch
at the pull request, by a subagent that wrote none of it (`AGENTS.md`).

**State: round 01 planned, not started; round 02 not designed.**

**What phase 7 does not do**, and where each is recorded: migrating a newer document (it cannot be done — the fields are
not known); refusing an unknown *field* in a document this build can read (a different rule, and the one serde's
`deny_unknown_fields` would give, which would refuse every future addition); a file dialog change (the refusal is a
notice, not a dialog, because a message is a notice in this app); drag-to-reorder devices, duplicating a device, the rack
document, the arrangement, stems (all wishlist, all later).

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| A document whose `version` is **greater** than `SAVE_VERSION` is refused, and the file is never written | A player who upgraded a bundle of songs by hand cannot open them in an older build — which is the point: the alternative is silently discarding what the newer build wrote. |
| The refusal opens **a demo as a first run**, with a notice naming the version — the app always opens | The player loses the session they had open, and says so on the status line. The alternative (refuse to start) is a worse first minute. |
| The refused session **must not autosave** over the file it just refused to read | An autosave one second after launch silently destroys the newer document, which is the loss the round exists to prevent — so this is the ruling the tests must hold. |
| A pattern shorter than its bar **repeats inside the bar** (round 02) | A player wanting a slow four-step loop cannot have one bar per loop; the alternative breaks every device's bar landing on the same bar, which phase 2's swap and join both rely on. |
