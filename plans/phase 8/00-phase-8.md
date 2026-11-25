# Phase 8 — the instrument as a document you can carry

Phase 8 is the phase where **the rack becomes a file**. A song already holds every machine (phase 5's round 01) and a bank is
already a document; what has never had one is the middle amount — the machines with their patches, banks, kits, routes,
names and colours, kept as a preset and recalled into another session. The keys for it have been drawn on the `BANKS` card
and refusing since phase 5, and `main` answers both with a notice naming "the rack round". This is that round.

It is also the phase where the two edits that were waiting on a document finally land, because each of them changes what a
rack *is*: **duplicate a device** and **drag to reorder devices**. Both were deferred in phase 2 for the same reason — the
selection, the strip's cells and the input filters all key on position — and the rack's shape is what settles that.

**The handover that opened this phase was wrong about one thing, and it is worth writing down so nobody rebuilds it.** It said
`open` still builds a rack from one device and the rest are re-created at launch. Phase 5 shipped the opposite: the session
file carries `devices`, `Rack::from_saved` builds every machine, `main::session_for_disk` writes them all, and
`tests/rack.rs`'s `a_saved_rack_of_three_devices_opens_as_three` pins it. So phase 8 is **not** *Save the whole rack* (that
is done and struck from the wishlist); it is *Load and save a rack of its own*.

Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-the-rack-file`](01-the-rack-file.md) — `persist::RackDocument` and `racks/`, the version rule split so a fourth reader
  gets it, `Rack::replace_devices_from` so a document with no tempo cannot retune the rack, `rack_recall::recall` so the
  recall's decisions are testable, the `RACKS` section with its rows and name field, and `main`'s three arms. The book stops
  saying a rack is not saved. **No `SAVE_VERSION` bump**: a rack is a third kind of document, not a third version.
- **02 — duplicate a device.** Named, not yet planned. `Rack::duplicate` built from the source's own `SavedDevice`, a
  `DUPLICATE` key on the device card, the copy selected. Waits on round 01 only because the document decides what a device
  *is* in a file.
- **03 — drag to reorder devices.** Named, not yet planned. `Rack::move_device`, the strip's cell gaining `click_and_drag` on
  its existing id, the dragged device remembered by number, the selection following it. The file needs no change: it is
  already strip order.

**Spec:** [`docs/superpowers/specs/2026-11-25-rack-documents-design.md`](../../docs/superpowers/specs/2026-11-25-rack-documents-design.md)
— agreed with the player on 2026-11-25 and revised once against a builder check, whose seven findings and their answers are
recorded in its last sections. The plans argue from it; where they disagree, the spec wins and the plan is wrong.

**Gate:** unchanged from phases 1–7 — `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` whenever the book moves. A task ends with the gates green, its `## Progress`
line current, and a commit. The review is of the branch at the pull request, by a subagent that wrote none of it
(`AGENTS.md`).

**State: round 01 complete, reviewed and pushed as `phase-8` — PR #15. Rounds 02 and 03 are named in the spec and not yet
planned.**

**What phase 8 does not do**, and where each is recorded: merging a saved rack into the open one (the player chose replace —
spec's Non-goals); a rack file that carries tempo, song name or panel state; remembering a rack file's name across a restart
(a `SAVE_VERSION` bump for a convenience, and a bank's is not remembered either); `IMPORT`/`EXPORT` for racks; an
arrangement; stems; a second rack open at once; reordering by keyboard; a reorder that renumbers.

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| A rack is a document of its own, and a **recall replaces** the rack | A player who wanted to add a saved rack to the one they have cannot. The alternative (append) has no rule for a full rack and makes "the rack" mean the open list rather than the instrument. |
| The rack file carries **no tempo and no song name** | A preset does not bring its own speed; a player who saved a rack at 150 types it after the recall. The alternative is a preset that retunes the song it is loaded into. |
| The tempo's absence is enforced by **splitting `Rack::replace_from_saved`** into `replace_devices_from` plus a tempo write | A second seam through the function both a song load and a rack load pass. A reader who changes one must look at the other, and the tests must keep both halves honest. |
| A recall **forgets what a restart forgets**: mute off, `REVERSE` forwards, transpose in tune | Four machines come back unmuted, which is the loudest thing this round can do. Carrying them by number would be a second identity mechanism. |
| `rack_file` is **session memory**, not a save-file field | After a restart, `SAVE RACK` under a taken name lands beside it (`Foo (2)`) instead of rewriting it. It is the bank's own behaviour, and the book says so. |
| A duplicate's name is `persist::distinct_name`'s, nothing else | A copy of a copy is `ACID 1 (2) (2)`, which reads oddly. The alternative is a stem-stripping rule `rename` and `sanitize` would need too. |
| The strip's cell keeps its **index-keyed id** and gains a drag sense; the drag carries the device's **number** | One interaction with two senses, and a release whose device has gone moves nothing. If a later egui needs the senses apart, the id has to move. |
| No `SAVE_VERSION` bump: a rack is a **kind** of document at the same version, like a bank | A later change to `SavedDevice` moves the session, song and rack formats together, so the version bump covers all three. A separate number would let them drift. |
