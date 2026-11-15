# Plan 30 — one knob catalogue, before phase 2

## What is wrong today

The seven tone knobs are identified by a bare `usize` in **six** places that must be kept in step by hand:

| Where | What it encodes |
|---|---|
| `PanelState::knobs: [f32; 7]` | slot → position |
| `ui/mod.rs:357-379` | slot → which `shared.*` atomic to publish |
| `ui/mod.rs:313` (label) + `waveform_detents` | slot 0 is the waveform switch, specially cased |
| `tips::Control::knob(index)` | slot → tooltip |
| `learn::control_for_knob(index)` | slot → MIDI-learn target |
| `persist::Knobs` (positional struct) + `midi::cc::LEARNABLE` | slot → saved field, slot → controller |

Every one of them is a `match` on `0..=6` with a `_` fallback, so a knob inserted or reordered in one place and not the
others produces **silence or a wrong knob**, never a compile error. The same shape already bit us twice this month: the
`cc_map`/`pickup` array length (an index panic) and the duplicated `CC_MAP` list.

## The design

One catalogue, `ui::knobs`, is the only place a tone knob's identity lives:

```rust
pub enum Knob { Waveform, Tuning, Cutoff, Resonance, EnvMod, Decay, Accent }

impl Knob {
    pub const ALL: [Knob; 7];            // drawing order, left to right
    pub fn index(self) -> usize;
    pub fn from_index(index: usize) -> Knob;      // clamps
    pub fn label(self) -> &'static str;           // silkscreen caption (WAVEFORM's comes from the wave catalogue)
    pub fn tip(self) -> tips::Control;
    pub fn learn_target(self) -> midi::cc::CcTarget;
    pub fn read(self, shared: &SharedState) -> f32;          // what the knob shows
    pub fn publish(self, shared: &SharedState, position: f32); // what the knob writes
    pub fn kind(self) -> KnobKind;                // Dial { snap_on_release } | Switch(Waveform)
}
```

Then: the knob loop iterates `Knob::ALL`, so `state.knobs[knob.index()]`, `knob.publish(...)`, `knob.tip()` and
`knob.learn_target()` replace all five `match`es. `tip::Control` loses `knob(index)`; `learn::control_for_knob` takes a
`Knob` and becomes a lookup. `persist::Knobs` keeps its feel for old files but gains `from_positions(&[f32; 7])` /
`positions()` built from `Knob::ALL`, so the saved file's order is the catalogue's order by construction.

The waveform special case moves **into** the catalogue rather than disappearing: it is a switch, so
`publish`/`read` route through `waveform_from_position`/`waveform_position`, and `snap_on_release` is what the loop asks
instead of `if i == 0`.

## The tests that make it safe

| Test | Pins |
|---|---|
| `knobs::every_knob_is_in_all_once_and_in_index_order` | `index()` matches `ALL` position |
| `knobs::each_knob_has_an_english_label_and_a_tip` | no `_` fallback can swallow a new knob |
| `knobs::publishing_a_knob_moves_exactly_that_atomic` | publish/read round trip per knob (7 cases) |
| `knobs::the_saved_order_is_the_panel_order` | `positions()` ∘ `from_positions()` is the identity, and `persist::Knobs` agrees |
| `knobs::the_learn_target_of_every_knob_is_distinct` | one controller slot per knob |
| `panel_layout` (existing) | seven cells still fill the row |

Mutation check: reorder `Knob::ALL` without touching anything else and the round-trip test must fail for at least one
knob — i.e. the catalogue, not the call sites, controls order.

## Task list

1. Add `ui::knobs` with the catalogue and its unit tests; nothing else uses it yet. **Green commit.**
2. Point the drawing loop at `Knob::ALL` (labels, tips, publish, the waveform switch via `kind()`); delete the three
   `match`es in `mod.rs`. Existing `knob_live`/`tip` tests must stay green untouched. **Green commit.**
3. Point `learn` and `tips` at `Knob` and delete their index mappings; `learn_live`/`tips_live` stay green. **Green commit.**
4. Derive `persist::Knobs` order from `Knob::ALL`; `persist` tests keep the on-disk field names, so add a test that a
   file written by this build is byte-identical to one written by the old code. **Green commit.**
5. Sweep the leftovers: `midi::cc::LEARNABLE` declared as `Knob`-derived for the first seven slots plus the three
   non-knob targets (volume, tempo, pattern, swing), and any other `match` on a control index.

## Other tidying worth doing in the same phase (found while looking)

- **`ui/mod.rs` is 1,100 lines** and holds the knob row, the performance row, the transport row, the status line, the
  learn flow and the volume knob. Splitting by row (`ui/row_knobs.rs`, `row_perform.rs`, `row_transport.rs`) gives phase 2
  somewhere obvious to put new controls.
- **`PanelState` is a 40-field grab bag**; the learn, songs and picker state could each be a sub-struct with its own
  invariants (`songs: Option<SongCard>`, `learn: LearnFlow`, …), which is also what makes `Default` and the tests readable.
- **The "second master" pattern** (hand vs controller) now exists for the waveform and for pickup; phase 2 will add more
  (density, scale if they become learnable). One `Pickup`-shaped helper reused by both would stop the third copy.
- **`AGENTS.md` is another project's file** and should be rewritten to describe this repo before phase 2 starts.

## Non-goals

- No behaviour change: this is a rename-and-centralise round. Every existing test must pass untouched, and the new tests
  exist to prove the catalogue — not to replace the behavioural ones.
- Not a rewrite of the panel's drawing; `knob::knob` stays as it is.

## Progress

- [x] **1. `ui::knobs`** — `Knob`, `Kind`, `ALL`, `index`/`from_index`/`kind`/`label`/`tip`/`learn_target`/`read`/`publish`, with
      five unit tests (slot order, caption+tip, publish/read round trip, the waveform is the only switch, distinct learn
      targets). `29a6b7b`.
- [x] **2. The drawing loop** — `mod.rs` walks `Knob::ALL` zipped with the row's cells: the label, the tooltip, the
      publish and the learn target all come from the knob, and the two masters of a switch are the `Kind::Switch` arm of
      one `match knob.kind()`. The three `match`es on the slot are gone (`knob_live`, `panel_layout`, `tips_live` green
      untouched).
      Ruling: `theme::KNOB_LABELS` deleted, not kept beside `Knob::label` — it was the seventh copy of the captions and
      nothing else read it; `knob::the_knob_is_not_a_font_glyph` now walks the catalogue instead. Cost if wrong: none,
      the catalogue's captions are the same seven strings.
      Ruling: `Knob::label` takes `&SharedState` (the plan's signature) because the waveform captions itself from the
      wave catalogue; the other six ignore it, which is cheaper than a second query path.
- [x] **3. Learn and tips** — `tips::Control::knob(index)` and `learn::control_for_knob(index)` are gone; `Knob::tip()`
      and `Knob::learn_target()` are the only mappings. `learn_live`'s index test became a click-through over the row, so
      it now covers the wiring and not just the lookup.
      Ruling: `control_for_knob` deleted rather than "made a `Knob` lookup" as the plan says — `mod.rs` already calls
      `knob.learn_target()`, so a wrapper would be a second mapping over the same catalogue. Cost if wrong: a caller
      outside this crate loses a helper it never had (the crate is an app, not a library).
      Ruling: `tips`'s knob tests walk `Knob::ALL` and compare against `Control::ALL`'s first seven, so the tip table's
      knob prefix and the drawing order are pinned to each other.
- [x] **4. The file's order** — `persist::Knobs::positions()` / `from_positions(&[f32; 7])` are the only way in and out of
      the seven tone knobs, and `apply_saved` / `shared_to_saved` go through them (`Knob::publish` / `Knob::read`), so
      the hand-written field list is gone from both directions. Three tests: the positions round trip in catalogue order,
      a struct built through the catalogue is byte-identical to the old hand-written one, and save→load moves the same
      atomics.
      Ruling: `positions`/`from_positions` each hold one exhaustive `match` on `Knob` — the file's *field names* are the
      wire format (serde), so only the order can be derived, not the fields. No `_` arm: a new knob variant fails to
      compile there. Cost if wrong: none, the format is unchanged and pinned by the byte test.
      Ruling: `shared_to_saved` reads the tone knobs with `Knob::read(shared)` rather than from the `Parameters` snapshot
      it already takes — a `Knob` → `Parameters` mapping would be a second field list for the same seven values. Cost if
      wrong: a save can catch a knob a few instructions after the bpm it was captured beside; nothing reads a save as a
      sample-accurate frame.
      Mutation check: swapping `Tuning` and `Cutoff` in `Knob::ALL` fails `every_knob_is_in_all_once_and_in_index_order`
      and `every_knob_slot_names_its_own_control` (the round trip itself is order-agnostic by construction, which is the
      property the catalogue buys). Reverted.
- [x] **5. The sweep** — `midi::cc::LEARNABLE` is now `learnable()`, a `const fn` that walks `Knob::ALL` for slots
      0..7 and then `OUTSIDE_THE_ROW` (volume, tempo, pattern, swing); `Knob::learn_target` is a `const fn` so the const
      table can be built at compile time. `DEFAULT_CC`, `SharedState::cc_map` and `SharedState::pickup` take their length
      from `LEARNABLE`, so the length that panicked once cannot drift. New test
      `cc::the_learn_slots_are_the_knobs_in_panel_order_then_the_rest`.
      Ruling: `apply_cc`'s `match target` stays as it is — it is an exhaustive match on targets, not on a slot index, and
      routing the seven knobs through `Knob::publish` there would need an all-seven dead arm to stay exhaustive. The
      waveform snapping rule is single-sourced in `waveform_from_position`; only its call site is repeated. Cost if
      wrong: a second place to touch when the control path learns something new about a knob.
      Ruling: the guard test passed before the derivation (the values were already right) — the change is what makes them
      stay right: `LEARNABLE` now follows `Knob::ALL`, and a catalogue of a different length fails to compile.

## Review

One fresh-context reviewer read the five commits against this plan (`67ebbbe` is the fix pass). Verdict: ready to merge
with fixes; no Critical, two Important, nine Minor.

Fixed (Critical/Important, one pass):

- **The sweep missed the seventh slot list** — `ui/mod.rs` synced the six dials from their atomics line by line before
  the loop, duplicating `Knob::read`. Now one walk over `Knob::ALL` that skips the switch. Cost of the miss: reordering a
  knob *with* `index()` updated would have left one knob's face driving another knob's sound, green everywhere.
- **The learn-slot test was tautological** — `LEARNABLE` is built from `learn_target`, so comparing the two proved
  nothing; permuting two arms passed. It now pins the slot names and the factory CCs as literals (the contract a saved
  `cc_map` is indexed by), and `knobs::each_knob_is_named_by_its_own_caption_tip_and_learn_target` pins each lookup to its
  knob's own name. Mutation-checked: swapping the `Resonance`/`EnvMod` learn arms fails it.
- Also taken from the Minors because they are compile-time for free: `kind()`'s `_ => Dial` arm is gone,
  `layout::KNOB_COUNT` is `Knob::ALL.len()` (so the drawing loop's `zip` cannot silently drop a knob), the switch caption
  reads `shared.waveform().name()` instead of round-tripping a position, `from_positions` documents that `volume` is not
  one of the seven, and `midi::cc`'s header says eleven CCs.

Deferred minors (recorded, not acted on):

- `DEFAULT_CC` is still a positional literal; its pairing is now pinned by a test rather than derived from a
  `factory_cc(target)` table.
- `Knob::from_index` has no caller in `src/` (tests use it). Kept: it is the catalogue's own clamped lookup.
- `midi::cc` imports `ui::knobs` (compile-time only, `learn_target` is a `const fn`). A top-level `crate::knobs` would
  break the layer blur; not worth a move in this round.
- `tests/persist.rs`'s positions loop re-asserts what the whole-array comparison already proved; it names the knob that
  moved, which is why it stays.

Not in this plan's scope, still stand from "Other tidying": splitting `ui/mod.rs` by row, `PanelState` sub-structs, one
`Pickup`-shaped helper for the two-master controls, and rewriting `AGENTS.md`, which still describes another project.
