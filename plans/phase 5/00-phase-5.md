# Phase 5 — banks, racks and songs

Phase 5 is the phase where the player's work has a document. A **song** becomes the whole rack instead of the first device, a
**pattern bank** becomes a file of its own, and a **rack** gets a place on the face that will hold its own document later.
Rounds live here as `NN-topic.md`, one per round, in the shape the earlier phases use.

- [`01-the-whole-rack`](01-the-whole-rack.md) — the deferred round the wishlist has been holding since phase 2: `SAVE_VERSION`
  **12**, `SavedState::devices`, a song that saves and loads every machine, and the picker's footnote gone.
- [`02-bank-files`](02-bank-files.md) — a bank document (`banks/<name>.json`) saved, listed, loaded and deleted from the
  `BANKS` card, the card's lists renamed so its words are true, and the two `RACKS` keys drawn and refusing.

**Spec:** [`docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md`](../../docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md)
— approved in conversation on 2026-11-22. The plans argue from it; where they disagree, the spec wins and the plan is wrong.

**Gate:** unchanged from phases 1–4 — the three workspace commands, `docs/guide/check.sh` when the book moves, and a subagent
review of the branch at the pull request (`AGENTS.md`). Commits are made with the gates green and the Progress lines current;
the review is of the branch, not of each commit.

**State: opened 2026-11-22; both rounds committed on `phase-5`, nothing pushed.** `cargo test --workspace` **937**, clippy
`-D warnings`, `fmt --check` and `./docs/guide/check.sh` were green at the head of round `02`. Round `01` took `SAVE_VERSION`
**12** and round `02` shares that number because a bank is a second kind of saved document, not a second save version. What is
left for the player: the pull-request review of both rounds together, and the push.

**What phase 5 does not do**, and where each is recorded: an **arrangement** (what plays when) is a later round with its own
`SAVE_VERSION` bump — `plans/future/wishlist.md`, *The arrangement*; a **rack document** that loads is later — *Load and save a
rack of its own*, and round `02` draws its buttons and nothing behind them; a **single-pattern file** stays in the wishlist; and
**refusing a file from a newer build** is still the wishlist's *A file from a newer build is refused, not relabelled*.

**Rulings this phase rests on, and what they cost if wrong:**

| Ruling | Cost if wrong |
|---|---|
| The song grows `devices` and keeps the flat fields as device 1's mirror | The file carries each machine twice, and `sanitize` has one rule to keep them agreeing. Dropping the mirror instead makes every v11 song a migration and makes an older build read nothing. |
| A catalogue entry is stored by its caption, and the caption↔variant mapping is derived from the catalogue | `DeviceKind` and `Scheme` grow `from_name`; a renamed caption renames what is in every file. Storing an index instead makes a table reorder change what a file means. |
| A v11 file's device kind is inferred from `kit: Some(_)` | A drum device that never left the `Init` kit opens as an acid. The file says nothing, so no better inference exists. |
| A bank save writes into `banks/` and that file is its own export | Handing a bank to a friend means finding the data folder. The alternative is two more keys on a card already four sections tall. |
| A loaded bank replaces the selected device's bank, armed twice while dirty | Work is replaced by a click, exactly as `DEMO BANKS` already does. The alternative invents a bank-slot notion the app does not have. |
| After a song loads the panel shows slot 0 | A player who loaded a song to work on device 3 clicks once. The file records no selection, and "stay where I was" means nothing once the rack is replaced. |
| Ports stay as written; a missing one is a single notice | A shared song opens with deaf devices until the ports exist — honest, and reversible when the port returns. |
