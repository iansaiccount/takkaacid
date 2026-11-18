> **Superseded — kept for the record, not for instructions.** Read `handover-task-5.md` for where the branch stands; the plan Progress section records what shipped.

# Handover — phase 4, task 4 (for a fresh session)

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

**Where the branch stands.** `0520760` is task 3, reviewed and committed; `b74179d` is task 2; `0f7890d` is task 1. Gates
green at `0520760`: `cargo test --workspace` (**721**), `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo fmt --check`, and `./docs/guide/check.sh` from the repo root. The branch is **local only** — nothing is pushed. Tasks
1–3 are done; tasks 4–10 are not started.

## Your task: task 4 of the plan — the face

From the plan, verbatim:

> ### Task 4 — the face
>
> **Files:** modify `rust/crates/acid-app/src/device.rs` (`DeviceKind::Drums` + `Device::on`), `ui/mod.rs` (route by kind),
> `ui/device_card.rs` (the card's rows for a drum device); create `rust/crates/acid-app/src/ui/drum_panel.rs`; add to
> `tests/drum_live.rs` (new).
>
> **Interfaces (consumes Tasks 1–3; produces):** `drum_panel::draw(state, shared, ui, &layout, s, p)`, and `PanelState::drum:
> DrumView { selected: Drum }` (panel state; not saved).
>
> - [ ] `ui/drum_panel.rs` draws: the seven relabelled knobs and the device's `SHUFFLE` dial, the `LP | BP | HP` key, `HIT`, the
>       sixteen-row grid with beat frames, the selected voice's lit plate and row band, a muted voice dim, and the voice's name
>       over the knobs. Everything from `p: &Palette`.
> - [ ] `draw_panel` branches on `device.kind`: a drum device draws `drum_panel` and **not** the knob, performance or step rows.
> - [ ] Live tests in `tests/drum_live.rs`: `the_grid_has_sixteen_labelled_rows_and_sixteen_cells`; `a_click_arms_a_voice_on_a_step`
>       (the `Step`'s mask and `on`); `a_shift_click_accents_it`; `clicking_a_label_selects_that_voice_for_the_knobs`;
>       `a_shift_click_on_a_label_mutes_the_row_and_the_mask`; `the_face_paints_from_the_palette` (the same frame under `Scheme::Black`
>       and a light scheme, asserting the grid's fill colours differ); `the_selected_row_is_plated` (accent fill under the label);
>       `a_click_lands_on_the_step_under_the_pointer_while_the_line_runs` (the frame refreshes the pattern from `shared.pattern()`
>       mid-bar; the write must land where the pointer is, not where the chase moved to — Review Focus 1);
>       `the_grid_stays_hittable_at_both_scale_extremes` (0.65× and 1.6×: every cell's rect is inside the band, wider and taller than
>       8 px, and no label's rect overlaps the cell beside it — Review Focus 4).
> - [ ] Run; expect failures. Implement; run; PASS. Commit: `feat(panel): the drum machine's face — a grid of sixteen voices`.

The `HIT` key auditions the selected voice; the `LP | BP | HP` key replaces the 303's four-face waveform switch; `SHUFFLE` stays
because it is the device's swing and writes the same `shared.swing`. The spec's face section (lines ~140–180) is the authority on
what is drawn and what is gone: on a drum device the step row and the performance row are **gone, not inert**.

## What tasks 1–3 left you

**Task 1** (`acid-core::drums`): `Drum`, sixteen variants (`Kick, Snare, Rim, Clap, ClosedHat, OpenHat, LowTom, MidTom, HighTom,
Crash, Ride, Cowbell, LowConga, HighConga, Clave, Shaker`) with `ALL`, `index()` (bit and grid row), `label()` (`KICK` … `SHK`;
`HighTom` is `HTOM`, not `HIT`), `choke_group()`; and `FilterKind { Low, Band, High }` with `ALL`/`index`/`label`
(`LP`/`BP`/`HP`). Every `match` is written out.

**Task 2** (`acid-core::pattern`): `Step` gained `pub hits: u16, pub accents: u16`, appended after `velocity` with
`#[serde(default)]`, and `hit(drum, accented)`, `clear_hit(drum)`, `is_hit(drum)`, `is_accented(drum)`. The bit is one private
`drum_bit` in `pattern.rs`; there is deliberately **no** `Drum::bit()`. `SAVE_VERSION` is **10**. Two rulings you inherit:
`hit(voice, false)` *clears* the accent bit, and `clear_hit` rests the step only when it actually removed a hit.

**Task 3** (`acid-core::drums` + `acid-app::state`): `Kit { name, voices: [KitVoice; Drum::ALL.len()] }`, `KitVoice`
(`sample: Arc<[f32]>`, `tune`, `attack_ms`, `decay_ms`, `sustain`, `release_ms`, `filter: FilterKind`, `cutoff`, `resonance`,
`mute`) with `Default`, `default_kit()`, `DEFAULT_KIT_NAME = "Init"`, and `Kit::mute_mask() -> u16` / `set_mute_mask(u16)` as
the only pack and unpack. On `SharedState`: `kit: ArcSwap<Kit>`, `drum_mute: AtomicU32` (bit `Drum::index()`, high half unused,
starts `0`), `audition: AtomicI32` (`-1` sentinel), and `kit() -> Arc<Kit>` (`load_full`, like `pattern()`),
`request_audition(Drum)`, `take_audition() -> Option<Drum>`. **`Init` is a word the card will print** — a ruling recorded in the
plan's Progress with its cost, and the player may overrule it for one constant plus one guide sentence.

## Traps this task owns

1. **`StripCell` carries no `DeviceKind`, and `draw_panel` gets no device.** `draw_panel(state, shared, ui)` knows only
   `PanelState` and `SharedState`; `ui/strip.rs:12` (`StripCell`) has `name`, `number`, `scheme`, `running`, `muted`, filled in
   `main.rs`'s frame `logic` (`fn logic` at `main.rs:119`; the strip fill is around `main.rs:513`). Add `kind: DeviceKind` to
   `StripCell`, fill it from `device.kind`, and read the selected device's kind from the cell `draw_panel` already keys
   `edit_slots` by. Use `.get(selected)` — the clamp is `.min(len.saturating_sub(1))`, so an empty rack still leaves index `0`
   out of bounds. Watch `StripCell`'s `#[derive(Clone, PartialEq, Eq, Debug)]` and any test that builds one.
2. **Where a drum knob's value lives — the open question in this task.** The seven knobs' identity is `crate::knobs::Knob`
   (label, learn target, atomic). A drum `ATTACK` is not `Knob::Waveform`'s atomic: it is `KitVoice::attack_ms` of the selected
   voice in the kit. Two candidates: (a) clone-on-write the kit and republish it — `SharedState::set_step`'s `rcu` is the
   precedent, it keeps one truth, and it allocates only on the UI thread (a `Kit` clone is ~1.5 KB, at 60 Hz while dragging);
   (b) borrow the 303's atomics as scratch and commit on selection change — a second source of truth, which is the exact bug
   `AGENTS.md`'s "one mapping per question" exists to prevent. (a) is the recommendation and needs a `SharedState::set_kit`;
   decide, and **record the ruling with what it costs if wrong**. Note `draw_panel` fills `state.knobs[i]` from
   `Knob::ALL[i].read(shared)` unconditionally (`ui/mod.rs:445`) — a drum face must fill those from the selected voice instead.
   The plan's task-4 test list has no knob test, so a reviewer will ask: add one (`turning_attack_edits_the_selected_voice`) or
   say in Progress which later task owns it and why.
3. **The mute a label shift-click writes is `shared.drum_mute`, not the kit's flags.** `KitVoice::mute` is the kit's *starting*
   mask (seeded on load); the live mute the engine reads is the atomic. Write one bit (`1 << Drum::index()`), read-modify-write,
   and do not rewrite the kit — or a saved kit silently keeps the panel's mute. `Kit::mute_mask`/`set_mute_mask` are the pack and
   the unpack; the plan's Progress carries a debt note about a possible `Drum::bit()` if a fourth copy of the shift appears (if it
   does, `pattern::drum_bit` must move onto it in the same commit).
4. **Keyboard shortcuts run per device in `main`, not in the panel.** `main.rs:202` calls
   `dispatch::handle(press, &mut self.panel, &device.state, &mut device.bank)` every frame — so `CLEAR`, `RANDOM`, the octave keys
   and the step shortcuts would still edit a drum device's pattern unless that call is gated on `device.kind`. Pointer routing,
   by contrast, happens where the widget paints, so not drawing a row does remove its clicks: `ui/row_knobs.rs`,
   `ui/row_perform.rs`, `ui/step_key.rs` (and `ui/knob.rs`, `ui/perform.rs`) are the rows a drum face skips.
5. **`DeviceKind` is a catalogue.** `ALL: [DeviceKind; 1]`, `name()`, `enabled()` are written-out matches whose doc comments say
   "One entry today" (`device.rs:30–55`). `Drums` must decide `name()` (the card, the strip and `default_name` all print it:
   `DRUMS 2`) and `enabled()` (shares `MAX_DEVICES`). `Device::on` seeds `factory::starter()` into every new device's bank — a
   drum device's starter is silence (masks zero) unless you decide otherwise; task 9 adds the factory drum banks. Ledger what a
   new drum device opens on.
6. **Geometry from the bands, never design pixels.** `ui/layout.rs` is fractions (`BANDS: [(f32, f32); 6]`) and `PanelLayout`
   has `top, knobs: Row, perform: Row, steps, switches, transport`. The spec's "grid starts at 232, 288 px to the switches row"
   is design arithmetic: derive the grid's band from the bands it replaces (`knobs` + `perform` + `steps`), so 0.65× and 1.6×
   both work — the scale test asserts every cell is inside the band, ≥ 8 px, with no label overlapping the next row's cell.
7. **Ids are behaviour.** `egui::Id` routes a drag: derive each cell's id from `(voice.index(), step)` and the plate's from the
   voice, and do not rename one later. A paint-only test will not catch a rename; it breaks dragging silently.
8. **Palette only.** `theme::palette(state.scheme)` is already in `draw_panel`; `theme::BLACK` is cards, never a face. The
   palette test paints the same frame in `Scheme::Black` and a light scheme and compares the grid's fills, so a literal colour
   fails it.
9. **Review Focus 1 — the mid-bar click.** `draw_panel` does `state.pattern = shared.pattern()` each frame (`ui/mod.rs`). A grid
   click must map the pointer to a step and write through `shared.set_step(index, step)` against the pattern as it is *now*
   (`ui/selection.rs:81/97/175` shows the 303's three writers). A test that runs a bar and clicks mid-bar is in the list; it is
   the one input class the happy path never reaches.
10. **Selected voice: panel-wide or per device?** `PanelState::drum: DrumView { selected }` is panel-wide, while `PanelEdit` /
    `edit_slots` already keep per-device editor state keyed by `device.number` (`ui/mod.rs:303–352`). Keeping it panel-wide is
    what the plan says; note it, so switching to another drum device keeping the last voice is a decision and not an accident.
11. **The card is task 4's only for rows; the kit row is task 6's.** `ui/device_card.rs` gains "the card's rows for a drum
    device" here, and the kit's name + `KIT` key in task 6. Keep `Init` a name the card can print without special-casing.
12. **`SAVE_VERSION` is a queue, and it is worth knowing before task 6.** The spec puts `kit: Option<String>` in the session,
    which is a `SavedState` field and therefore a bump; the plan's non-goals say the rack round takes 11. Whoever lands a field
    first takes the number: if task 6 adds `kit`, task 6 takes 11 and the rack takes 12, and both the plan's non-goal line and
    the wishlist entry have to say so. Task 4 saves nothing — `PanelState::drum` is panel state, not `SavedState`.

## Rules that bite (restated from AGENTS.md because they will)

- **Every commit ends green**: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`, and `./docs/guide/check.sh`. This task changes what a player sees, so the **book is part of this task
  only if a page already describes the drum face — it does not yet** (task 10 writes page 22). Say in the plan's `## Progress`
  why the guide is untouched, rather than leaving it unsaid.
- **Every commit is reviewed by a dispatched subagent before it lands**, never by you: `git diff > .superpowers/sdd/x.diff`,
  dispatch `reviewer` with the task's **Files** and **Interfaces** blocks and the diff, fix what it finds, record it in the
  plan's `## Progress`, then commit. A review whose findings land in a later commit gated nothing.
- **Prove each test bites.** Mutate the implementation, watch the test fail, revert, and write down what you mutated. A script
  that does this in a loop is in `.superpowers/mutate.py`; it asserts a test actually ran, which matters — see the environment
  facts.
- **One catalogue per kind of control; no `_` arm on a catalogue enum; no hand-kept second list.** The drum face's knobs are a
  *view* of `Knob::ALL` and its filter key is `FilterKind::ALL`; a positional `[f32; 7]` of captions beside `Knob::ALL` is the
  bug this repo has paid for twice. A `match` over `Knob`/`Drum`/`FilterKind`/`DeviceKind` is written out in full.
- **No font glyphs** except `…` via `ui::text::fit`/`settings::shorten`. The grid is squares and ASCII captions.
- **A message is a notice, not a dialog.** `PanelState::notice`, and nothing that dies with the card that caused it.
- **Nothing on the audio path allocates or locks**; `tests/no_alloc.rs` is the gate. This task is UI-thread only, but it must
  not put an allocation into a render path on the way.
- **The plan's Progress section is the record.** One entry per task: what shipped, the mutation that proves it, what the review
  found and what you did, and any ruling with what it costs if wrong.

## Environment facts that cost previous sessions time

- **The shell blocks heredocs, `python3 -c`, `bash script.sh` and any unlisted binary.** Write the script with the editor and run
  `python3 path/to/script.py`. `/tmp` is **outside** the editor tools' project root (refused); `.superpowers/` is inside it,
  git-ignored, and is the right place for scratch scripts and diffs.
- **`cargo test --workspace | head` kills the run with SIGPIPE** and you read a partial count. Redirect and sum:
  `cargo test --workspace > /tmp/t.log 2>&1`, then
  `awk '/^test result/ {for(i=1;i<=NF;i++) if($i=="passed;") p+=$(i-1); for(i=1;i<=NF;i++) if($i=="failed;") f+=$(i-1)} END {print p, f}' /tmp/t.log`.
- **`cargo test -- --exact` needs the full test path** (`drums::tests::name`, `state::tests::name`), not a bare name: a filter
  that matches nothing exits **0** with "0 passed", which reads exactly like a pass. Assert a test ran.
- **In a `cmd1 && cmd2` chain a failed `cmd1` silently skips the rest**, and a `cd` inside the chain leaves later commands in the
  wrong directory. Run gates separately or join with `;`.
- **`cargo fmt` collapses blank lines but never adds one.** An `insert` can leave a `use` glued to the next item and two items
  with no blank line between them, and `cargo fmt --check` will still say nothing is wrong. Read the diff.
- **Editing anchors are fragile around doc comments and module boundaries.** An `insert` after the last line of a `mod tests`
  puts the new test *outside* the module — it compiles, runs, and passes with the wrong path. Check `cargo test -- --list`.
- **Screenshots** (task 10, not yours): something pre-starts `Xvfb` at 1280×800 on display numbers and a request for a bigger
  screen on a taken display fails quietly. `DISPLAY=:N xdotool getdisplaygeometry` must say `2000 1400`; the window must measure
  1960×1340. The full recipe is in `AGENTS.md`.
- `TAKKA_ACID_DATA_DIR` points the app at a scratch data directory; a first run there is the factory session.
- **Live tests drive the real panel**: `tests/*_live.rs` build `RawInput` frames and call `draw_panel` — see
  `tests/step_mode_live.rs` for the `screen()`/`run_on`/`click`/`press` harness to copy. Their `screen()` is 980 × 560, not the
  670-tall design size; the scale-extreme test should size its own screen.

## The end of this session

When task 4 is committed: add its `## Progress` entry (what shipped, the mutations you ran, the review's findings, every
ruling with its cost), **rewrite this file as `plans/phase 4/handover-task-5.md`** for the next task — task 5 is *the WAV
decoder*: `acid-app/src/wav.rs`, `decode(bytes, target_rate) -> Result<Arc<[f32]>, WavError>`, PCM 8/16/24/32-bit and 32-bit
float, mono or stereo folded down, linearly resampled, fixtures built as byte arrays in the test — and tell the player. Do not
push; the player decides when the branch goes up.
