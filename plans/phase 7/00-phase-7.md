# Phase 7 — the file is a document, and the bar keeps what it stops playing

Phase 7 is the phase where the two things phase 6 left half-said are finished.

**The file learns to refuse.** A save file from a *newer* build is currently relabelled: `upgraded` stamps this build's
version over whatever it read, and the next save rewrites a newer document as an older one — silently, and with whatever
fields the newer build had already dropped. Phase 6 made that sharper, not softer: a version-14 file holding a pattern
grid this build has never heard of is read as a `1/16` bar and its sound is gone after one save. The number exists to
prevent that and nothing reads it before it is overwritten.

**A grid change stops playing; it does not destroy.** Phase 6 made a narrower grid rest the steps outside the new bar and
count them on the status line ("2 CLEARED"). The player has ruled the other way: the notes outside the new bar are
**kept and silent**, and widening again plays them — including after a save. That reverses a phase-6 ruling, and it is
phase 7's because the file has to carry what is not playing.

Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-refuse-a-newer-file`](01-refuse-a-newer-file.md) — `persist::load` names a newer document instead of relabelling
  it; `open` gains its second face (a first run, plus a status-line notice); `load_song` and `import_song` follow; the
  refusal never writes. **Nothing on the face changes** beyond one notice, so the book moves in one paragraph.
- [`02-a-grid-change-keeps-the-notes`](02-a-grid-change-keeps-the-notes.md) — `with_grid` keeps the steps it cannot
  play; the invariant reads "not played" instead of "a rest"; the file grows a `held` list that is written only when it
  has something in it, and `SAVE_VERSION` goes to **14**.

**Gate:** unchanged from phases 1–6 — `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and `./docs/guide/check.sh` whenever the
book moves. A task ends with the gates green, its `## Progress` line current, and a commit. The review is of the branch
at the pull request, by a subagent that wrote none of it (`AGENTS.md`).

**State: both rounds planned, neither started.**

**What phase 7 does not do**, and where each is recorded: migrating a newer document (it cannot be done — the fields are
not known); refusing an unknown *field* in a document this build can read (a different rule, and the one serde's
`deny_unknown_fields` would give, which would refuse every future addition); a file dialog change (the refusal is a
notice, not a dialog, because a message is a notice in this app); a `length` field, a per-device length, or a pattern
that repeats inside its bar (the player's model has none of these — see round 02's non-goals); drag-to-reorder devices,
duplicating a device, the rack document, the arrangement, stems (all wishlist, all later).

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| A document whose `version` is **greater** than `SAVE_VERSION` is refused, and the file is never written | A player who hand-carried a newer song into an older build cannot open it — which is the point: the alternative is silently discarding what the newer build wrote. |
| The refusal opens **a demo as a first run**, with a notice naming the version — the app always opens | The player loses the session they had open, and is told on the status line. The alternative (refuse to start) is a worse first minute. |
| The refused session **must not autosave** over the file it just refused to read | One autosave tick silently destroys the newer document, which is the loss the round exists to prevent. |
| A narrower grid **holds** the steps it stops playing, in memory and in the file | A pattern costs a `held` list to carry notes nobody is hearing; the alternative is the destruction phase 6 shipped, which the player has rejected. |
| `SAVE_VERSION` **14** for the `held` field | A version-13 build reading a version-14 file ignores `held` and plays the bare bar — narrowed, as it left it. |
| 12 is the triplet grid's bar, and a 16-step pattern narrowed to it plays 12 and holds 4 | None: it is what the grid means. |
