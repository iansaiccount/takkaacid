# Handover — phase 5 closed, and what is still open

Paste this whole file as the first message of the next session. It is written to be self-contained: that session has no memory of
the one that wrote it.

You are picking up **takka-acid**, a TB-303-style acid bass synth and drum machine in Rust, at `/home/ian/Projects/takkaacid`, on
branch **`phase-5`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the project's standing instruction and outranks
anything here that contradicts it. Then read, in this order:

1. `plans/phase 5/00-phase-5.md` — what the phase is, its rulings, and what it does not do.
2. `plans/phase 5/01-the-whole-rack.md` — round 01. Its `## Progress` is the record of every task, test, mutation run and ruling.
3. `plans/phase 5/02-bank-files.md` — round 02, including *two* branch reviews and what was done about every finding.
4. `docs/superpowers/specs/2026-11-22-banks-racks-and-songs-design.md` — the design both rounds argue from.
5. `plans/future/wishlist.md` — *Load and save a rack of its own*, *The arrangement*, *A file from a newer build is refused,
   not relabelled* and *A single-pattern file* are the entries this phase wrote or left standing.

**Do not re-do anything in either `## Progress`.** Trust those sections and `git log` over any recollection.

## Where the branch stands

Both rounds are complete on `phase-5`, pushed, and **PR #12 is open against `main`**
(<https://github.com/iansaiccount/takkaacid/pull/12>), base `1ad1918` (the merge of phase 4). Gates at its head:
`cargo test --workspace` **938**, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and
`./docs/guide/check.sh` — all green. Nothing is merged, and merging is the player's call.

The branch has been reviewed twice by subagents that wrote none of it, and **every finding was fixed on the branch**: the first
review read rounds 01 + 02/task 1 at the player's request mid-round, the second read the whole branch before the PR. Both are
recorded in the plans' `## Progress`, each finding with what was done and what it cost.

What the phase delivered:

- **A song is the whole rack.** `SAVE_VERSION` **12**, `SavedState::devices`, device 1 mirrored in the old flat fields so a v11
  build still opens the file, per-device kits, the missing-port notice, the picker's footnote gone, the 808 stated as the default.
- **A bank is a file.** `banks/<name>.json`, `{ version, kind, bank }`, saved and loaded through the `BANKS` card, its `kind`
  mismatch a warning and never a refusal.
- **The card's words are true.** `DEMO BANKS`, `PATTERN BANKS` (rows, `x`/`SURE`, a name field and `SAVE BANK`), `SONGS` (was
  `MY BANKS`), and `RACKS` with `SAVE RACK` / `LOAD RACK` drawn and refusing with a notice that names the later round.

## What still needs addressing

Nothing here is a bug in what merged unless it says so. It is the list a next session should read before planning.

**1. The PR is waiting on the human.** Merging is theirs. If they ask for a change, it lands on `phase-5` and the PR follows.
If they ask for another review, dispatch a subagent that wrote none of it, with the plans' **Files** and **Interfaces** blocks and
the diff against `1ad1918` — never a self-review (`AGENTS.md`).

**2. Untested glue, by name.** Every one is a one-line wiring, and each is recorded where it was written rather than hidden:

- `main`'s `save_bank_now`/`load_bank_now` calls and the delete arm's bookkeeping (`main.rs`, the card's request drain). The
  decisions they carry live in `persist` and are tested there; the *calls* are not.
- The `bank_file` clears in `main`: the drum-face demo arm, and `ConfirmRemove`. The acid demo, `NEW`, a song load and
  `Request::Select` clear it too, all on the same rule.
- The two `RACKS` notices' text ("the rack round comes after this one").
- `ui::mod`'s `open_banks` filling `card.bank_names` from `list_banks` and `card.bank_name` from `bank_file`. The retaken
  `img/banks-card.png` is its evidence: the row it shows is a bank file, not a hard-coded name.
- Round 01's promised `a_restart_then_save_keeps_the_same_file` does not exist; the decision moved into
  `persist::session_song_file` and is tested there.

**Why they are untested, precisely:** `main` is a binary crate, so *integration* tests (`tests/*.rs`) cannot reach it. It does
carry `#[cfg(test)] mod tests` for its own free functions (`Midi`, `still_wanted`), so a free function there **can** be unit
tested; what cannot is a `TakkaApp` method, which needs the app, a rig and an audio device. When a decision needs a test, put it
in `persist` (or another lib module) and have `main` call it — that is how `wanted_kits`, `unopened_ports`, `session_song_file`
and `bank_kind_notice` came to exist.

**3. One known wrinkle, worth a small round if it ever bites.** Changing device with the keyboard (`Ctrl+2`) **while the `BANKS`
card is open** clears `SongState::bank_file` but leaves `card.bank_name` showing the old bank's name. `SAVE BANK` then treats the
visible name as a *new* name and writes `Easy (2)` beside the file instead of over it. Nothing is destroyed — `save_target` is
doing exactly its job — but the picture and the outcome disagree with the player's intent. The fix is to clear `card.bank_name`
wherever `bank_file` is cleared; it was left out because no test drives a device change under an open card and the review found
no lost work.

**4. `tests/songs_live.rs`'s `click()` has no hover frame.** egui only reports a click on a widget it saw hovered, so a click sent
as press-then-release alone is silently dropped whenever the row was not under the pointer in the previous frame. `banks_live`'s
helper sends a `PointerMoved` frame first (`banks_live.rs`'s `click`); `songs_live`'s works today only because its clicks follow
one another over the same rows. Align it in the round that next touches that file — a flaky helper that passes is the worst kind.

**5. The screenshot recipe has a hazard this phase found.** The card's **text fields cannot be typed into under Xvfb**:
`xdotool type` and `xdotool key --window <id>` both print `XGetInputFocus returned the focused window of 1` and deliver nothing,
though a click does focus the field. A picture that needs a typed name has to arrive another way — this round wrote the file into
`banks/` with a throwaway test and relaunched. `docs/guide/src/img/README.md` records that run in full, as it does for every
picture.

**6. A save-path rule to keep.** `SavedState.song` is the name a session was last **shown** under; the file it is on is
`SongState::file`, and `persist::save_song` stamps the name it wrote to. A demo or a `NEW` song has a name and no file, which is
why the launch seeds the file through `persist::session_song_file` and not from the name. Anyone touching the save path must keep
that split, or the first `SAVE` after a restart writes over a song the player had under the same name.

**7. What phase 6 inherits.** The wishlist's decided next things: a **rack document that loads** (the keys are already drawn and
their ids fixed — `rack-save`, `rack-load`), the **arrangement** (what plays when; it is its own `SAVE_VERSION` bump, and the song
document was built to grow a field rather than be rebuilt), a **single-pattern file**, and **refusing a file from a newer build**
(a v12 file opened by a v11 build keeps device 1 and drops the rest on that build's next save — the spec's recorded risk).
Open `plans/phase 6/00-` and write the round before touching code, in the shape plans 01 and 02 use.

## How to work (unchanged)

One task at a time, in plan order. For each: write the failing test, run it, implement, run it, run the mutation script, keep the
plan's `## Progress` line current, commit with the gates green. A `SKIP` in a mutation run is never a pass — narrow the mutation
until it applies exactly once. The three workspace commands plus `./docs/guide/check.sh` whenever the book moves; the book travels
in the same push as the behaviour it describes.

**The review is at the pull request, not per commit**, by a subagent that did not write the code. It reports; the executor fixes
what it found and records it. Stop and ask the player only when a finding needs a decision that is theirs.

## Environment facts that cost previous sessions time (still true)

- **`main` is a binary crate**: integration tests cannot reach it; its own `#[cfg(test)] mod tests` covers only free functions.
  Decisions that need a test go into `persist` (or another lib module) and `main` calls them.
- **The shell blocks** heredocs into `python3` (write the script to a file and run `python3 path/to/script.py`), `bash script.sh`,
  `xargs`, `identify`, `setsid`, and any unlisted binary. `.superpowers/` is the git-ignored scratch directory; the mutation
  scripts and review packages live there and are named in the plans.
- **Two lines with identical text share one hashline anchor.** End a range on a distinctive line, not on a bare `}`.
- **`cargo test --workspace | head` kills the run with SIGPIPE.** Redirect to a file and sum the `test result` lines.
- **Never `git checkout <file>` to undo a scratch mutation** — it reverts to HEAD and takes uncommitted work with it.
- **`pkill -f "target/debug/takka-acid"` matches the shell command that runs it** and kills your own shell (exit 143). Use a
  pattern that the command line itself does not contain.
- **Screenshots:** check `DISPLAY=:N xdotool getdisplaygeometry` says at least `2000 1400` *before* launching (something on this
  machine pre-starts `Xvfb` at 1280×800), then `xdotool getwindowgeometry` must say `1960x1340`. A fresh `TAKKA_ACID_DATA_DIR`,
  the pointer parked outside the window, and a crop is a second grab, never a cut-out. See item 5 above for the text-field hazard.
- **The card's geometry is hand-computed in the live tests** (`card()`, `field_y()`, `row_centre()`, `songs_top()`…). The `BANKS`
  card's sections moved in round 02 and the helpers moved with them; a click test that misses its row asserts nothing.
