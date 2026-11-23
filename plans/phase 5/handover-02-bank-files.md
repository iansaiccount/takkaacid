# Handover — phase 5, round 02 (bank files)

Paste this whole file as the first message of the next session. It is written to be self-contained: that session has no memory
of the one that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth and drum machine in Rust, at
`/home/ian/Projects/takkaacid`, on branch **`phase-5`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the
project's standing instruction and outranks anything here that contradicts it. Then read, in this order:

1. `plans/phase 5/00-phase-5.md` — what the phase is, its rulings, and what it does not do.
2. `plans/phase 5/01-the-whole-rack.md` — the round that just finished. Its `## Progress` is the record of every task, test,
   mutation run and ruling. **Do not re-do any of it.**
3. `docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md` — the design both plans argue from. Where a plan and the
   spec disagree, the spec wins and the plan is wrong.
4. `plans/phase 5/02-bank-files.md` — the round you are here to run.
5. `plans/future/wishlist.md` — *Load and save a rack of its own* and *A new drum machine arrives on the 808* are the entries
   this phase wrote; the rest is context.

## Where the branch stands

**Round `01` is complete**: all six tasks committed on `phase-5`, local only — nothing is pushed, and the player decides when it
goes up. Gates at its head: `cargo test --workspace` **920**, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` — all green. Commits, oldest first:

- `2202f9d` the design, the plans and the wishlist entries
- `5401f3f` `SavedState::devices`, `SAVE_VERSION` **12**, the device-1 mirror
- `2860f01` `rack_to_saved` / `saved_device_from` / `saved_device_to_shared`
- `694a3d9` `Rack::from_saved` for N devices, `replace_from_saved`
- `0ccc708` `main`'s one session builder, the load path, the missing-port notice, the 808 pin, the book and `img/picker.png`
- `79d359f` this handover's first version
- `34bf1f4` **round 02, task 1**: `BankDocument` in `banks/<name>.json`, the bank `persist` functions, `song_name` →
  `document_name`

**What rounds 01 and 02/task 1 changed for you:** `persist::SavedState` now carries `devices: Vec<SavedDevice>`,
`SavedInputs`, per-device `swing` and `cc_map`, and the flat fields are device 1's mirror. `persist::rack_to_saved(&Rack)` is the
one builder `main` saves with; `Rack::replace_from_saved` is what a load calls. `persist::document_name` (renamed from
`song_name`) is the one file-name validator, and `persist::BankDocument`, `bank_path`, `list_banks`, `save_bank`, `load_bank`,
`delete_bank` and `free_bank_name` exist and are tested — nothing in the face uses them yet.


## What round 02 is

A pattern bank becomes a **document**: `banks/<name>.json`, saved, listed, loaded into the selected device of either kind, and
deleted — through the `BANKS` card, whose lists are reworded so they are true, with the two `RACKS` keys drawn and refusing.
`plans/phase 5/02-bank-files.md` has the four tasks, their tests and their mutations. **Task 1 is done and committed** (see its
`## Progress`); tasks 2, 3 and 4 are yours. The design's *The bank document* and *The face* sections are the shape.


**Rulings already made, so you do not re-open them:**

| Ruling | Cost if wrong |
|---|---|
| A bank is `{ version, kind, bank }` in `<data>/banks/<name>.json`, sharing `SAVE_VERSION` with the song | One number for every saved document; the clipboard's own magic is a private format, not a second version |
| `kind` is the device's caption, and a mismatch is a **warning**, never a refusal | A drum bank on an acid face loads and plays nothing; the notice says so |
| A load replaces the **selected device's whole bank**, armed twice while the session is dirty — exactly what `DEMO BANKS` already does | Work is replaced by a click; the alternative invents a bank-slot notion the app does not have |
| Where a save lands is `save_target(panel.songs.bank_file, typed)` — `SongState` gains `bank_file: Option<String>` | In place when the name is unchanged, a new file otherwise, the same rule the song field follows |
| No `export_bank`, no `import_bank` | The file in `banks/` is its own export; handing a bank to a friend means finding the data folder |
| `MY BANKS` is renamed `SONGS`; `PATTERN BANKS` and `RACKS` are new sections | A player who learned the old word loses it; the word was a lie once bank files exist |
| The card's requests: `SaveBank(String)`, `LoadBank(String)`, `DeleteBank(String)`, `SaveRack`, `LoadRack` | `main`'s drain matches `Request` in full, so a missing arm does not compile |
| `RACKS`' two keys refuse with a notice naming the later round; they open no dialog and write nothing | The place on the face is found and labelled once, and wiring later cannot rename the ids |

## Environment facts that cost previous sessions time (still true)

- **`main` is a binary crate with no test harness.** A decision that needs a test goes into `persist` (or another lib module) and
  `main` calls it — that is how `wanted_kits` and `unopened_ports` came to exist in round 01. Do not plan a live test that would
  have to call `TakkaApp`.
- **The shell blocks** heredocs, `python3 -c`, `bash script.sh`, `xargs`, `identify`, and any unlisted binary (`graphify` was
  allowlisted with `lean-ctx allow graphify`). Write a script with the editor and run `python3 path/to/script.py`.
  `.superpowers/` is the git-ignored scratch directory.
- **Two lines with identical text share one hashline anchor.** A range ending on a bare `}` can land on the wrong one; it did in
  round 01 and the block had to be repaired by hand. End ranges on a distinctive line.
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect to a file and sum the `test result` lines.
- **Never `git checkout <file>` to undo a scratch mutation** — it reverts to HEAD and takes uncommitted work with it.
- **Mutation scripts are the proof, and a `SKIP` is never a pass** (`.superpowers/mutate_phase5_*.py`). Two misses and one skip
  were found and fixed across rounds 01–02; all are recorded in the plans' Progress. A skip usually means the target text appears
  twice — narrow the mutation until it applies exactly once.

- **Screenshots:** check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400` before launching (something on this
  machine pre-starts `Xvfb` at 1280×800), then `xdotool getwindowgeometry` must say `1960x1340`. Fresh `TAKKA_ACID_DATA_DIR`,
  pointer outside the window or a tooltip lands in the book, and a crop is a second grab. Round 02 retakes the three
  `banks-card-*` pictures; `docs/guide/src/img/README.md` carries the rows.
- **The card's geometry is hand-computed in `tests/songs_live.rs`** (`card()`, `field_y()`, `row_height()`…). Round 02 changes the
  card's sections, so those helpers move with it — a click test that misses its row silently asserts nothing.

## How to work

One task at a time, in plan order. For each: write the failing test, run it, implement, run it, run the mutation script, keep the
plan's `## Progress` line current, and commit with the gates green. The three workspace commands plus `./docs/guide/check.sh`
whenever the book moves. Update `docs/guide` in the same push as the behaviour, including the page the words are on and the
`check.sh` pins — functionality is documented before it is pushed.

**The review is at the pull request, not per commit**, and it is a subagent that did not write the code (`AGENTS.md`). It reads
the branch diff against the plans and reports; you fix what it finds. Round `02` plus round `01` are reviewed together when the
player asks for the PR.

**Questions the player has already answered, so do not ask again:** banks card (not the tape deck) for the document keys; bank
files live in the library and their `kind` mismatch warns; a loaded bank replaces the selected device's bank; the rack keys are
stubs this phase; `SAVE` after a restart must keep its file. **One thing still open for them:** whether to take the PR review
before round `02` or after it — ask once, at the start.
