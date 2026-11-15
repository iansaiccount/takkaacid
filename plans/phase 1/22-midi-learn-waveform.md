# Plan 22 — retire RESET, teach the waveform knob

Two changes to the MIDI-learn corner of the panel.

1. **RESET goes.** Its only visible state was "lit while learning", so `get rid of the reset button when learning` is the
   whole button: a bright key beside LEARN that silently rewrote the entire map was a foot-gun, and the flow it sat in
   (press LEARN, pick a knob, move a fader) never needed it.
2. **The waveform joins the learnable knobs.** It was excluded because a continuous controller and a four-position
   switch do not obviously agree. They do if the controller value is snapped to the nearest detent — the same rule the
   knob itself follows on release, so a learned fader and a drag land on the same four faces.

## Task 1 — remove RESET

- `ui::switches`: drop `Switch::Reset` from the enum, `ALL`, `index`, `label`. Seven switches remain, and the cell
  layout follows `index`, so nothing is renumbered by hand.
- `ui::mod`: drop the RESET block and `tips::Control::Reset` (and its `TIPS` entry, so `ALL` is 17).
- `midi::cc`: drop `reset_defaults` and the test that drove it — it had no caller left, and dead escape hatches rot.
- `ui::help`: the NOTES line names RESET; rewrite it to describe LEARN alone.
- `tests/learn_live`: drop the RESET click test; add one asserting the switches row holds seven keys and none of them
  reads RESET, so the removal is checked from the outside rather than by absence of code.

## Task 2 — the waveform as a learnable knob

- `acid_core::oscillator::Waveform` gains `position(self) -> f32` (0.0, 1/3, 2/3, 1.0, from `index`) and
  `from_position(f32) -> Waveform` (nearest detent, clamped). One table, used by the panel, by the wheel, and by MIDI.
  `ui::waveform_position` / `ui::waveform_from_position` become delegations so the existing tests keep their names.
- `CcTarget::Waveform`; `LEARNABLE` grows to eight entries with `Waveform` **first**, because the panel reads left to
  right and the waveform knob is the leftmost.
- Factory controller: **CC 20**, an undefined controller no common box sends by accident, so a fresh session never has a
  fader fighting the mouse. `DEFAULT_CC` is `[20, 3, 74, 71, 76, 75, 5, 7]`.
- `apply_cc` gains `Some(CcTarget::Waveform) => state.set_waveform(Waveform::from_position(knob))`. The MIDI thread
  stays arithmetic and atomics only: `from_position` allocates nothing, so `tests/no_alloc` stays green.
- `learn::control_for_knob(0)` returns `Some(CcTarget::Waveform)`; the "no controller drives it" comment goes.
- **The knob face has to follow a learned controller.** Today `knobs[0]` is read once (`knobs_synced`) because
  re-deriving it every frame pinned the face at a detent and a drag could never cross a midpoint. The fix is to sync
  after the widget, and only when the widget is not being dragged: a hand on the knob wins, a learned fader moves it.
  With that, `knobs_synced` has no job left and goes.

### Saved files

`SharedState::cc_map` becomes `[AtomicI32; 8]` and the persisted `cc_map` becomes a `Vec<u8>`, because a fixed
`[u8; 7]` field cannot grow: serde would refuse a 7-element array and `load_or_default` would throw the whole session
away as "unreadable". A `Vec` plus an upgrade step is honest — the file keeps whatever length it was written with, and
the upgrade pads it in place.

- `SAVE_VERSION` 5.
- `upgraded()`: a `cc_map` of seven entries is a map from before the waveform knob; prepend the waveform's factory
  controller, leaving the other seven exactly where they were (Tuning still on CC 3, Volume still on CC 7).
- `sanitize()`: clamp every entry to `0..=127`, and reset the whole map to the factory pairing if it is longer than
  `LEARNABLE` (a file from a build with more knobs than this one).

## Tests (failing first)

| Test | What it pins |
|---|---|
| `cc::waveform_cc_snaps_to_the_nearest_detent` | CC 0 / 42 / 85 / 127 → Saw / Square / Sine / Triangle |
| `cc::a_learned_waveform_controller_is_found_and_moves_the_switch` | `set_cc(Waveform, 64)`, `cc_target(64)` → `Waveform`, `apply_cc` moves it |
| `cc::one_controller_drives_one_control` (existing, 8 slots) | learning 74 for Waveform clears Cutoff's slot |
| `learn::the_waveform_knob_can_be_taught` | `control_for_knob(0)` is `Waveform`; 1..=6 unchanged; 7 is `None` |
| `persist::a_file_from_before_the_waveform_knob_keeps_its_map` | 7-entry map → 8, Tuning 3 / Volume 7 unmoved, Waveform 20 |
| `persist::the_learned_midi_mapping_round_trips` (existing, 8 entries) | new length round-trips |
| `knob_live::the_waveform_face_follows_a_learned_controller` | `shared.set_waveform(Triangle)` with the panel idle → knob 0 draws at the Triangle detent |
| `knob_live` (existing drag/wheel tests) | a hand on the knob still wins; snapping stays on release |
| `learn_live::the_switches_row_has_no_reset` | seven switches, no RESET label |

## Global constraints

- Nothing in the audio callback allocates or locks; `tests/no_alloc.rs` stays green.
- ASCII legends only; no control face may be a font glyph.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.
- Mutation-check the two new guards: the not-dragged condition on the face sync, and the 7→8 map upgrade.

## Review focus

- A learned waveform controller and a drag must agree on the same four positions — if they disagree, one of the two is
  snapping at the wrong moment.
- The persisted-map upgrade must not move an existing mapping: a player's fader that taught Cutoff must still be Cutoff.
- MIDI still touches atomics only.

## Result

Both tasks landed. `SAVE_VERSION` is 5, `LEARNABLE` is eight entries with the waveform first, and the switches row is seven
keys.

What the build taught us, beyond the plan:

- **A stale bug in the save file.** `sanitize` clipped `knobs.waveform` to `1`, a leftover from the two-state switch:
  a session saved on SINE or TRIANGLE re-opened on SQUARE. It now clips to the catalogue's last index, and
  `every_wave_survives_a_round_trip` walks all four through a real save and load. This is the kind of thing the
  waveform work was going to need sooner or later; it was silent before because nobody had saved a non-square wave.
- **The persisted map had to become a `Vec`.** A fixed `[u8; 7]` cannot grow: serde refuses a seven-element array for an
  eight-element field, `load_or_default` treats that as an unreadable file, and the player loses the patterns too. So the
  field is a `Vec`, `upgraded()` prepends the switch's factory controller, and `sanitize()` resets the whole map if a
  file ever arrives from a build with *more* learnable knobs. `apply_saved` also fills slots a short map does not name,
  so loading an old *song* cannot leave the switch listening to whatever the last session taught it.
- **`knobs_synced` is gone.** It existed because re-deriving the knob's position from the switch every frame pinned it at
  a detent mid-drag. The switch now has two masters, and the rule is explicit where the knob is drawn: while
  `response.dragged()` the hand wins and the panel publishes; otherwise the face is snapped and, if the hand is not the
  one that moved it, the shared value wins — so a learned fader moves the face and the panel does not fight it. The drag
  test from plan 20 (`dragging_the_waveform_knob_moves_one_wave_per_detent`) is what caught two earlier drafts of this:
  one lost the release snap, the next let a second drag overshoot to TRIANGLE.
- **Exact ties round up.** `0.5` sits precisely between SQUARE and SINE; it always goes up, so a fader paused at 64 and
  the same fader nudged to 65 agree. A test pins it, because the alternative — a floor — would give TRIANGLE a single
  controller step of its own at 127.

Mutation checks: removing the map upgrade fails `a_map_from_before_the_waveform_knob_keeps_every_fader_it_had`;
flooring `from_position` fails the detent test; and the earlier drafts of the face sync failed the drag test.
