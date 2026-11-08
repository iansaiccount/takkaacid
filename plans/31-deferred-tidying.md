# Plan 31 — the tidying plan 30 deferred

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans` (or `subagent-driven-development`). Steps use
> checkbox tracking. Work on `Deepseek-Version`; every task ends in a green commit.

**Goal:** clear the follow-ups plan 30 named but did not do, before phase 2 adds controls: the last hand-written MIDI
pairing, the layer blur between `midi` and `ui`, the 1,100-line `ui/mod.rs`, the two `PanelState` grab-bags, the duplicated
snap-on-release rule, and an `AGENTS.md` that describes a different project.

**Architecture:** no new behaviour. Each task is a move or a derivation, gated by the tests that already exist plus one
new test wherever a derivation could silently produce a different value. `cargo test --workspace`, `cargo clippy
--workspace --all-targets -- -D warnings` and `cargo fmt --check` must be green at every commit.

**Tech Stack:** Rust 2024, egui/eframe 0.37, serde. Repo root `rust/`, crate `rust/crates/acid-app`.

**Spec:** `plans/30-knob-catalogue.md` (this plan clears its "Other tidying" list and the review's deferred minors).

## Global Constraints

- No behaviour change: no knob, controller, saved field or learn slot may move. Existing tests may only change where an
  item removes or renames the API they used.
- The save file must stay byte-identical (`tests/persist.rs::a_file_written_by_this_build_is_the_file_the_old_build_wrote`).
- Every derivation of an order or a length comes from one catalogue; no `_ =>` arm on a catalogue enum.
- Commit messages: `refactor(<area>): <what>`, one task per commit, plan progress updated in the same commit.

## Review Focus

- A moved `pub` item must keep its path or every caller is updated: `ui::knobs`, `ui::waveform_position/from_position`,
  `panel.songs`, `panel.midi_ports` and friends are used from `tests/*`.
- `egui::Id`s are part of the widget tree: splitting `draw_panel` must not change any `Id::unique`/`scope_id` value, or a
  knobs drag in progress loses its widget id.
- Row functions must not reorder calls: the knob row's publish order is what makes a drag heard on the same frame.
- `PanelState` grouping must not change `Default` values or persistence-adjacent defaults.

---

### Task 1: The factory controllers follow the learn map

**Files:** Modify `rust/crates/acid-app/src/midi/cc.rs`; Modify `rust/crates/acid-app/src/ui/knobs.rs`.

**Interfaces:** Produces `pub const fn factory_cc(target: CcTarget) -> u8` (exhaustive, no `_`); `DEFAULT_CC` becomes
`const DEFAULT_CC: [u8; LEARNABLE.len()] = defaults();` where `const fn defaults()` walks `LEARNABLE`.

- [ ] **Step 1: Write the failing test** in `midi/cc.rs`'s test module:

```rust
#[test]
fn a_control_keeps_its_factory_controller_when_the_learn_map_moves() {
    // The pairing is keyed on the control, not on its position: 74 is CUT OFF whoever sits third.
    assert_eq!(factory_cc(CcTarget::Cutoff), 74);
    for (slot, target) in LEARNABLE.iter().enumerate() {
        assert_eq!(DEFAULT_CC[slot], factory_cc(*target), "{target:?} lost its controller");
    }
}
```

- [ ] **Step 2: Run it** — `cargo test -p acid-app --lib cc::` → FAIL, `factory_cc` not found.
- [ ] **Step 3: Implement** `factory_cc` as an exhaustive `const fn` match over all eleven targets, then `defaults()`.
- [ ] **Step 4: Re-run** — PASS, and `the_learn_map_is_still_the_one_every_saved_rig_is_indexed_by` still PASS (the
  numbers are unchanged).
- [ ] **Step 5: Drop `Knob::from_index`** (no caller in `src/`; the loop iterates knobs and the cells are zipped), and its
  two assertions in `knobs::every_knob_is_in_all_once_and_in_index_order`.
- [ ] **Step 6: Commit** — `refactor(midi): key the factory controller on the control, not on its slot`.

### Task 2: The catalogue stops depending on the view

**Files:** Create `rust/crates/acid-app/src/knobs.rs` (moved from `src/ui/knobs.rs`); Modify `src/lib.rs`, `src/ui/mod.rs`,
`src/ui/knob.rs`, `src/ui/learn.rs`, `src/ui/tips.rs`, `src/midi/cc.rs`, `src/persist.rs`, `tests/learn_live.rs`,
`tests/persist.rs`.

**Interfaces:** Produces `acid_app::knobs::{Knob, Kind}`, `acid_app::knobs::{waveform_position, waveform_from_position}`
(re-exported from `ui` so the existing test paths keep working) and `tips::Control::for_knob(knob: Knob) -> Control`.
Consumes nothing new.

- [ ] **Step 1: Move the file** to `src/knobs.rs`, declare `pub mod knobs;` in `lib.rs`, delete the `ui` declaration.
- [ ] **Step 2: Move the wave math** (`waveform_position`, `waveform_from_position`) into `knobs.rs`; re-export both from
  `ui/mod.rs` (`pub use crate::knobs::{waveform_from_position, waveform_position};`) so `acid_app::ui::waveform_position`
  still resolves.
- [ ] **Step 3: Move `Knob::tip`** to `tips::Control::for_knob(knob)`, an exhaustive `match` on the knob, and update its
  three callers plus the tests.
- [ ] **Step 4: Point every import at `crate::knobs`**; `ui::knobs` no longer exists.
- [ ] **Step 5: Run** `cargo test --workspace` → all green; `grep -rn "ui::knobs" src tests` → empty.
- [ ] **Step 6: Commit** — `refactor(knobs): the catalogue lives at the crate root, not under the view`.

### Task 3: `PanelState`'s two grab-bags become structs

**Files:** Modify `src/ui/mod.rs` (struct, `Default`, the drawing code that reads the fields), `src/main.rs` and the tests
that touch them (`tests/*.rs`).

**Interfaces:** Produces

```rust
pub struct SongState { pub card: Option<songs::Songs>, pub request: Option<songs::Request>, pub name: Option<String> }
pub struct PickerState { pub inputs: Option<Vec<String>>, pub input_choice: Option<midi::ports::PortChoice>,
                         pub outputs: Option<Vec<String>>, pub output_choice: Option<String> }
```

as `PanelState::songs: SongState` and `PanelState::picker: PickerState`.

- [ ] **Step 1: Write the failing test** in `tests/songs_live.rs`: opening the card, asking to save, and naming a song are
  one state (`panel.songs.card.is_some()`, `panel.songs.name`), and the picker's four fields are one state — assert a
  fresh `PanelState` has both at their defaults and that opening the card does not disturb the picker.
- [ ] **Step 2: Run it** → FAIL to compile (`panel.songs.card`).
- [ ] **Step 3: Add the two structs** with `#[derive(Default)]` where the defaults are the current values, update
  `Default`, and rewrite the call sites (`panel.songs()` → `&panel.songs.card`, …).
- [ ] **Step 4: Run** `cargo test --workspace` → green, including `songs_live`, `midi_picker_live`, `save_live`.
- [ ] **Step 5: Commit** — `refactor(panel): the songs card and the port picker own their state`.

### Task 4: `ui/mod.rs` splits by row

**Files:** Create `src/ui/row_knobs.rs`, `src/ui/row_perform.rs`, `src/ui/row_transport.rs`; Modify `src/ui/mod.rs`
(keeps `PanelState`, `draw_panel`'s prologue and the top strip / step row / switches, and calls the three row functions).

**Interfaces:** Produces `pub(super) fn draw(state: &mut PanelState, shared: &SharedState, ui: &mut egui::Ui,
l: &layout::Layout, s: f32)` in each row module, returning nothing; the row functions carry their own helpers
(`perform_knob`, `switch_key`, `learn_pick` move with the row that uses them; a helper used by two rows moves to
`ui/controls.rs` rather than being copied).

- [ ] **Step 1: Move the knob row** into `row_knobs.rs` (the `for (knob, cell) in Knob::ALL…` loop and the volume knob if
  it reads better there — it does not: volume belongs to the top strip and stays).
- [ ] **Step 2: Run** `cargo test --workspace` → green; `knob_live`, `learn_live`, `panel_layout` unchanged.
- [ ] **Step 3: Move the performance row** (octave keys, shuffle, density, scale, randomise) into `row_perform.rs`.
- [ ] **Step 4: Run** → green; `perform_live`, `learn_highlight_live` unchanged.
- [ ] **Step 5: Move the transport row** into `row_transport.rs`; run → green, `transport_live` unchanged.
- [ ] **Step 6: Confirm the ids did not move** — `grep -n "Id::unique\|scope_id" src/ui/*.rs` and check every knob's id
  still has the slot, not the row.
- [ ] **Step 7: Commit** — `refactor(ui): a file per panel row`.

### Task 5: One snap-on-release rule

**Files:** Modify `src/ui/panel.rs` (the helper), `src/ui/row_knobs.rs` (the switch arm), `src/ui/row_perform.rs` (the
scale knob).

**Interfaces:** Produces

```rust
/// A control that lands on detents: the position the hand left snaps onto the nearest face. `face_of` turns a position
/// into the face it is near, `position_of` puts the face back on the strip.
pub fn snap_to_detent<F>(position: f32, face_of: impl Fn(f32) -> F, position_of: impl Fn(F) -> f32) -> f32;
```

- [ ] **Step 1: Write the failing test** in `ui/panel.rs`: `snap_to_detent` moves a position onto the nearest of four
  faces, leaves an exact face alone, and clamps out-of-range input (the switch's two ends).
- [ ] **Step 2: Run** → FAIL, no such function.
- [ ] **Step 3: Implement** and use it in both call sites: the switch's release branch and the scale knob's release
  branch. The duplicated comment about frame-by-frame snapping lives on the helper.
- [ ] **Step 4: Run** `cargo test --workspace` → green (`knob_live`'s detent test, `perform_live`'s scale test).
- [ ] **Step 5: Commit** — `refactor(ui): one snap-on-release rule for both detent controls`.

### Task 6: `AGENTS.md` describes this repo

**Files:** Rewrite `AGENTS.md`.

- [ ] **Step 1: Write it** — what the project is (a TB-303-style acid bass synth: Rust engine + egui panel), the cargo
  commands that gate a change, where plans and progress notes live, the real code conventions found in this repo (the
  catalogue pattern, one mapping per control, no `_` arm on a catalogue enum, tests beside behaviour, no font-glyph
  icons — the panel is drawn with egui, so the pygame icon rules plan 30's notes carried are gone).
- [ ] **Step 2: Read it back** and delete any rule that is not true of this tree (grep for each named file).
- [ ] **Step 3: Commit** — `docs: AGENTS.md describes this repository`.
- [ ] **Step 4:** Trim the duplicate assertion in `tests/persist.rs`'s positions loop (the whole-array comparison above it
  already proves it) — same commit, it is one line.

---

## Out of scope

- `PanelState::selected` and the step editor: 51 references, no invariant to protect — the plan's "editor" grouping would
  be churn without a bug it prevents.
- `learn` is already one typed field (`learn::Step`), so it is not a grab-bag.
- Phase 2 controls.

## Progress

- [x] **1. The factory controllers follow the learn map** — `factory_cc(target)` is an exhaustive `const fn` over all
      eleven targets; `DEFAULT_CC` is `factory_pairing()`, a `const fn` walk of `LEARNABLE`. Test
      `cc::a_control_keeps_its_factory_controller_when_the_learn_map_moves` (RED: no such function). The eleven numbers
      are unchanged, so `the_learn_map_is_still_the_one_every_saved_rig_is_indexed_by` still passes.
      Ruling: `Knob::from_index` deleted rather than put to use — nothing in `src/` turns a slot back into a knob (the
      drawing loop iterates the catalogue and `persist` zips it), so a clamped lookup was API with no caller. Cost if
      wrong: a future caller writes three lines of `ALL[i]` instead of calling a helper.
- [x] **2. The catalogue lives at the crate root** — `src/knobs.rs` (was `ui/knobs.rs`); `waveform_position` /
      `waveform_from_position` moved with it and are re-exported from `ui` so `acid_app::ui::waveform_position` still
      resolves; `Knob::tip()` became `tips::Control::for_knob(knob)`, an exhaustive match in the module that owns the tip
      table. `midi::cc` now imports `crate::knobs`, so no MIDI file names a view module.
      Ruling: `Knob::read`/`publish` still take `SharedState` and `learn_target` still returns `midi::cc::CcTarget`, so the
      catalogue is not dependency-free — that mutual reference *is* the single mapping plan 30 built. What changed is that
      nothing under `ui/` owns the catalogue and the MIDI side no longer reaches into the view to build `LEARNABLE`.
- [x] **3. The songs card and the port picker own their state** — `PanelState` loses seven loose fields for two structs:
      `songs: SongState { card, request, name }` and `picker: PickerState { inputs, input_choice, outputs,
      output_choice }`. New test `songs_live::the_card_the_request_and_the_name_are_one_piece_of_state` (RED: no field
      `card`); `songs_live`, `midi_picker_live`, `save_live` all green.
      Ruling: the rest of the struct stays flat. `selected` alone has 51 references and no invariant to protect, so
      grouping the editor would be churn for nothing; `learn` is already one typed field (`learn::Step`). The plan named
      "the learn, songs and picker state"; two of the three were grab-bags, and this is the two.
- [x] **4. `ui/mod.rs` splits by row** — `ui/row_knobs.rs` (the knob loop plus the detent markers),
      `ui/row_perform.rs` (octave, shuffle, density, scale, randomise, and `perform_knob`), `ui/row_transport.rs` (five
      keys and the tempo), and `ui/controls.rs` for the two helpers two rows share (`learn_pick`, `switch_key`).
      `mod.rs` is 1,128 → 810 lines and keeps `PanelState`, the top strip, the step row, the switches, the picker, the
      help and songs cards, and the calls into the rows. All 58 suites green, clippy and fmt clean.
      Every widget id is unchanged (`("knob", slot)`, `("learn-knob", slot)`, the string ids), because `knob_live`'s
      drag and `learn_live`'s click both depend on them.
      Ruling: the three row calls keep their original position in `draw_panel` — the transport row is called after the
      switches, where its code used to be, not next to the other two rows. A later widget wins a hit-test tie in egui, so
      moving a call changes behaviour that no layout test would see.
- [x] **5. One snap-on-release rule** — `crate::knobs::snap_to_detent(position, face_of, position_of)` is the rule both
      detent controls now call: the WAVEFORM switch's release branch and the SCALE knob's. Test
      `knobs::a_detent_position_snaps_onto_its_face_and_an_exact_face_stays_put` (RED: no such function) pins the four
      faces, an exact face, and both ends. The duplicated comment about frame-by-frame snapping lives on the helper.
      Ruling: the helper is in `crate::knobs`, not `ui::panel` as plan 31 sketched — it is about a detent catalogue and
      takes no egui type, and both callers are knobs.
- [x] **6. `AGENTS.md` describes this repo** — rewritten: what the two crates are, the four gates, where plans live and
      how their progress section is kept, and the conventions this tree actually follows (one catalogue per kind of
      control, no `_` arm on a catalogue enum, one mapping per question, panel tests through egui frames, the save file as
      a contract, the two-thread rule, no font glyphs). The pygame icon rules and the Python scratch-file habits it carried
      described a different project and are gone.
      Also trimmed `tests/persist.rs`'s positions loop, which re-asserted the whole-array comparison above it.

## Review

One fresh-context reviewer read the six commits against this plan. Verdict: ready to merge with fixes; no Critical, one
Important, two Minor — all three were documentation or duplication the round itself introduced.

- **Fixed (Important):** `AGENTS.md` listed "tip" among the things `Knob` owns, one commit after `Knob::tip` had been
  deleted. It now says the hover help is `ui::tips::Control::for_knob`, kept with the tip table. Cost of the miss: the
  next agent would have put the tip back on the catalogue and split the mapping again.
- **Fixed (Minor):** the Gates heading said "all four green" over three commands; the scale block computed the same face
  twice (`scale_from_position` for the scale, then inside `snap_to_detent`) — it now snaps first and reads the scale back
  off the snapped position, so there is one source.
- Reviewed and confirmed unchanged by direct comparison: draw order and every `egui::Id`, the waveform's two-master
  branch (both versions write the same face and leave the same position), the scale clamp order, all seven old
  `PanelState` defaults, and the eleven factory controller numbers.
