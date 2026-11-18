> **Superseded — kept for the record, not for instructions.** Read `handover-task-6.md` for where the branch stands; the plan Progress section records what shipped.

# Handover — phase 4, task 5 (for a fresh session)

Paste this whole file as the first message. It is written to be self-contained: the session it is for has no memory of the one
that wrote it.

---

You are picking up **takka-acid**, a TB-303-style acid bass synth in Rust, at `/home/ian/Projects/takkaacid`, on branch
**`phase-4`**. Read `/home/ian/Projects/takkaacid/AGENTS.md` first — it is the project's standing instruction and outranks
anything here that contradicts it.

**The round.** Phase 4 is the **drum machine**: sixteen sample voices against the 303's sixteen steps, sharing the bar, banks,
transport, transpose, reverse and themes. The approved design is
`docs/superpowers/specs/2026-11-18-drum-machine-design.md` (the spec — it wins any argument with the plan). The plan, ten
tasks, is `plans/phase 4/01-the-drum-machine.md`; its `## Progress` is the record and its Global Constraints and Review Focus
are binding. The phase overview is `plans/phase 4/00-phase-4.md`.

**Where the branch stands.** `718b8d2` is task 4, reviewed and committed; `de0f8e2` is the committed handovers; `0520760` is
task 3, `b74179d` task 2, `0f7890d` task 1. Gates green at `718b8d2`: `cargo test --workspace` (**745**),
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, and `./docs/guide/check.sh` from the repo root.
The branch is **local only** — nothing is pushed. Tasks 1–4 are done; tasks 5–10 are not started. `SAVE_VERSION` is still
**10** (task 4 saved nothing), which matters for task 6 — see trap 8.

## Your task: task 5 of the plan — the WAV decoder

From the plan, verbatim:

> ### Task 5 — the WAV decoder
>
> **Files:** create `rust/crates/acid-app/src/wav.rs`; register it in `lib.rs`; test in the same file.
>
> **Interfaces (produces):** `pub fn decode(bytes: &[u8], target_rate: f32) -> Result<Arc<[f32]>, WavError>` — mono, at
> `target_rate`, from PCM 8/16/24/32-bit integer or 32-bit float, mono or stereo folded down, linearly resampled;
> `pub enum WavError { NotRiff, NotWave, Unsupported { what: &'static str }, TooLong }`.
>
> - [ ] Tests: `a_sixteen_bit_mono_file_decodes_to_the_same_samples`, `a_stereo_file_is_folded_to_mono`,
>       `a_forty_four_one_file_is_resampled_to_forty_eight` (length scales, peak preserved within a tolerance),
>       `a_float_file_decodes`, `eight_and_twenty_four_bit_decode`, `adpcm_is_refused_by_name` (the error names the format),
>       `a_file_that_is_not_riff_is_refused`. Build the fixtures as byte arrays in the test, not as files.
> - [ ] Run; expect failures. Implement the parser (RIFF/WAVE chunks, `fmt `, `data`; ignore unknown chunks).
> - [ ] Run; PASS. Commit: `feat(wav): a hand-rolled decoder for a kit's samples`.

**The task's place in the round.** Task 5 is pure and display-free: bytes in, mono samples out at the engine's rate. Task 6
turns it into `<data>/kits/<name>/kit.json` + one WAV per voice and fills `KitVoice::sample: Arc<[f32]>`; task 7 renders the
samples. A decoder that panics on a malformed file, or names the wrong format in its error, becomes a kit that crashes or
lies in task 6 — so the error type is part of this task's interface, not a detail.

## What tasks 1–4 left you

**Task 1** (`acid-core::drums`): `Drum` (sixteen variants, `ALL`, `index()`, `label()`, `choke_group()`) and `FilterKind`
(`LP | BP | HP`). Every `match` is written out.

**Task 2** (`acid-core::pattern`): `Step` carries `hits: u16` and `accents: u16`; `hit/clear_hit/is_hit/is_accented`;
`SAVE_VERSION` is 10.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice`
(`sample: Arc<[f32]>`, `tune`, `attack_ms`, `decay_ms`, `sustain`, `release_ms`, `filter`, `cutoff`, `resonance`, `mute`),
`default_kit()`, `DEFAULT_KIT_NAME = "Init"`, `Kit::mute_mask`/`set_mute_mask`; `SharedState` has `kit: ArcSwap<Kit>`,
`drum_mute`, `audition`, `kit()`, `set_kit(Arc<Kit>)`, `request_audition`, `take_audition`.

**Task 4** (`acid-app`): `DeviceKind::Drums` (`DRUMS`, same rack cap) and `Device::on` (a drum device opens on the 303's
starter with zero masks — an empty grid, and it still *sounds* the 303 until task 8's `Machine`); `StripCell.kind`;
`PanelState::showing_kind()`; `PanelState::drum: DrumView { selected: Drum }`; `ui/drum_panel.rs` (the face, `Dial`, `DrumKey`,
geometry helpers) with `SharedState::set_kit` as the clone-on-write publish; `ui/dispatch.rs` gates the keyboard's step editor
for a drum device. `ui/device_card.rs` is untouched: its rows are kind-agnostic and the kit row is task 6's. The guide's picker
pages (`05`, `18`, `20`), its screenshot and `check.sh` now name `DRUMS`; **the drum face has no page yet — task 10 writes page
22.**

## Traps this task owns

1. **No new dependency.** The plan's Tech Stack says it: the decoder is hand-rolled — `std` only, no `hound`. The workspace
   has no `wav` crate and adding one is a plan change, not a task decision.
2. **8-bit PCM is unsigned**, 16/24/32-bit are signed little-endian. An 8-bit sample is `(byte as f32 - 128.0) / 128.0`, and a
   16-bit one is `i16::from_le_bytes(..) as f32 / 32768.0`. 24-bit must **sign-extend** (three bytes → `i32`, left-shift into
   the top byte, arithmetic-shift back) or every negative sample becomes a huge positive one. 32-bit integer and 32-bit float
   share a width and not a meaning: the `fmt` tag decides.
3. **`fmt ` is the format.** `wFormatTag` 1 is PCM, 3 is IEEE float; everything else is `Unsupported { what }` naming the
   format a player would recognise — `"ADPCM"`, `"A-law"`, `"mu-law"`. `adpcm_is_refused_by_name` asserts the name, not just
   the variant. `0xFFFE` (WAVE_FORMAT_EXTENSIBLE) is the one to decide deliberately: it usually wraps plain PCM in a
   sub-format field, and refusing it by name is honest for this round; if you read it, ledger that the sub-format is PCM-only
   and what happens to a float extensible file. The `fmt` chunk's `cbSize` extension bytes are ignored, not misread.
4. **Chunks are word-aligned.** A chunk is `id(4) + size(4 LE) + body`, and an **odd** size is followed by one pad byte the
   size does not count. Walk `RIFF → WAVE`, then the chunks in order (a `LIST` or `INFO` before `data` is normal), skip what
   you do not know, and never trust `data`'s declared size past `bytes.len()` — a truncated file is an error, not a panic and
   not a short read that pads with garbage. Every slice access goes through a checked helper; `from_le_bytes` on an exact-width
   slice (`<&[u8; 2]>::try_from(..)`), never `get_unchecked`.
5. **Stereo folds to mono by averaging** (`(l + r) * 0.5`), not by summing: summing a correlated stereo pair clips the kit
   before task 7's limiter ever sees it. Mono stays mono; `channels` other than 1 or 2 is `Unsupported { what: "3 channels" }`
   (a `&'static str`, so pick fixed words, not `format!`).
6. **Resampling is linear and the output length is computed, not grown.** `out_len = round(input_len * target_rate / rate)`;
   the source position is a float advanced by `rate / target_rate`, and the last output sample must not read past the end.
   `a_forty_four_one_file_is_resampled_to_forty_eight` asserts the length scales and the peak survives within a tolerance —
   a nearest-neighbour "resampler" would pass a length check and fail the peak/tone one only if the tolerance is tight.
   Decide `target_rate <= 0` (a refuse, not a divide by zero) and ledger it.
7. **`TooLong` needs a number.** The enum promises it; pick the cap (samples, not bytes) and say why in the doc comment — a
   hostile 4 GB WAV must be refused before the `Vec` is allocated. If you decide the cap belongs to the caller instead, that is
   a plan change and a ruling with its cost.
8. **The tests build their fixtures as byte arrays.** No fixture files, no `include_bytes!` of a `.wav` in this task: a test
   helper that writes the `RIFF`/`fmt `/`data` header and the sample bytes by hand is the fixture, and it is also how the
   odd-chunk and truncated-file cases get made. `a_file_that_is_not_riff_is_refused` can be eleven bytes of text.
9. **The error is the interface task 6 reports through.** Give `WavError` `Debug`, `PartialEq` (the tests compare it), and
   whichever of `Display`/`std::error::Error` you need to turn it into the notice that names the voice and the file (Review
   Focus 2 in the plan). Do not invent a second error type in task 6 for the same bytes.
10. **The module is registered in `lib.rs`** (`pub mod wav;`), or the tests cannot reach it from `tests/` and task 6's
    `persist` cannot call it. The unit tests live in `wav.rs`'s `mod tests`, so a mutation run uses
    `cargo test -p acid-app --lib wav::tests::<name> -- --exact` — the full path, or it matches nothing and "passes".
11. **Nothing here touches the save file or the audio path.** No `SavedState` field, no `SAVE_VERSION` bump (still 10), and no
    allocation on the audio path: `decode` is called on the UI/main thread when a kit loads. `tests/no_alloc.rs` must stay green
    because nothing on the callback changes.
12. **The guide is untouched, deliberately, and task 5 must say so in Progress.** A decoder behind a kit that task 6 does not
    load yet changes nothing a player can reach; no page in `docs/guide` becomes wrong. Task 6 is the one that moves the book
    (the kit row, the browser, the factory kits).

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh` from the repo root.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you. Save the diff to a file under
  `.superpowers/review/`, dispatch the `reviewer` agent with this task's **Files** and **Interfaces** blocks (not the whole
  plan), tell it the traps above, and act on what it finds before committing. A review whose findings land in a later commit
  gated nothing. The task's `## Progress` line records what the review found and what was done.
- **Prove each test bites.** Mutate the implementation, watch the test fail, revert, and write down what you mutated.
  `.superpowers/mutate_task4.py` is the pattern to copy: it refuses a filter that matches nothing and asserts `1 passed` or
  `1 failed` before trusting a run.
- **No `_` arm on a catalogue enum; no hand-kept second list; no font glyphs; a message is a notice.** Task 5 adds no UI, but
  a kit name and a notice are task 6's and will want those rules.
- **The plan's Progress is the record.** One entry per task: what shipped, the mutations, the review's findings and what was
  done, every ruling with what it costs if wrong.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write the script with the editor and
  run `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside it,
  git-ignored, and is the right place for scratch scripts and diffs. `identify` is not allowlisted (use `file` for a PNG's
  dimensions); `Xvfb`, `xdotool` and `import` are.
- **`cargo test --workspace | head` kills the run with SIGPIPE** and you read a partial count. Redirect and sum:
  `cargo test --workspace > /tmp/t.log 2>&1`, then
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **`cargo test -- --exact` needs the full test path** (`wav::tests::name`, `drums::tests::name`); a filter that matches
  nothing exits **0** with "0 passed", which reads exactly like a pass. Assert a test ran.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest**, and a `cd` inside the chain leaves later commands in
  the wrong directory. Run gates separately or join with `;`.
- **`cargo fmt` collapses blank lines but never adds one.** An `insert` can leave a `use` glued to the next item or two items
  with no blank line between them, and `cargo fmt --check` will still say nothing is wrong. Read the diff.
- **Editing anchors are fragile around doc comments and module boundaries.** An `insert` after the last line of a `mod tests`
  puts the new test *outside* the module — it compiles, runs, and passes with the wrong path. Check `cargo test -- --list`.
- **Live tests** (`tests/*_live.rs`) drive the real panel through `RawInput` frames. Task 5 needs none; its tests are unit tests
  in `wav.rs`.
- **Screenshots** (task 10, not yours): something pre-starts `Xvfb` at 1280×800 on display numbers and a request for a bigger
  screen on a taken display fails quietly. `DISPLAY=:N xdotool getdisplaygeometry` must say `2000 1400`; the window must
  measure 1960×1340. Task 4 retook `picker.png` this way on a fresh `:88`. The full recipe is in `AGENTS.md`.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; a first run there is the factory session.

## The end of this session

When task 5 is committed: add its `## Progress` entry (what shipped, the mutations you ran, the review's findings, every
ruling with its cost), **rewrite this file as `plans/phase 4/handover-task-6.md`** for the next task — task 6 is *kit documents
and the factory kits*: `persist::KitDocument`, `save_kit`/`load_kit`/`import_kit`/`kit_names`, `kits.rs`'s compiled-in factory
three, `ui/kits.rs` (the browser), the card's kit row, CC0 samples in `rust/crates/acid-app/kits/` with `kits/CREDITS.md`,
tests in `tests/kits.rs` and the live kit tests — and mark this one superseded, as task 4's marks task 3's. Two things to carry
into that handover: **the `Init` word is what the card will print** (settled in task 3; the player may overrule it for one
constant, one pinned assertion and one guide sentence), and **`SAVE_VERSION` is a queue** — the spec puts `kit: Option<String>`
in the session, so if task 6 lands that field it takes **11** and the rack round takes **12**, and the plan's non-goal line and
the wishlist entry must say so. Do not push; the player decides when the branch goes up.
