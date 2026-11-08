# Plan 32 — learn polish, the pattern controller, and two missing tips

## What is wrong today

1. **The teach highlight disappears too early.** `panel::learn_highlight` is drawn only while `learn::Step::Choosing`,
   so the moment the player clicks a control the border goes out — during the very window (`Step::Listening(target)`) when
   they need to know what they are about to bind.
2. **The pattern can not be taught from where it is shown.** The learn target for `CcTarget::Pattern` is the PATTERN
   *engraving* in the switches row, not the pattern readout in the top strip. The readout is the thing the player is
   looking at when they want to teach "change the pattern".
3. **A controller jumps the pattern mid-bar.** The MIDI thread writes an absolute request and `main` applies it in the same
   frame, so a fader moved through the zones restarts the bar on every message, and there is no pickup rule: a fader parked
   on another pattern slams the selection the moment it twitches.
4. **No hover help on the two readouts.** `tips::Control` has no entry for the tempo or the pattern display, and the help
   card therefore documents neither.

## The design

**Highlight.** `learn::highlight(step, control) -> bool` is the one rule: true while choosing, and true for the control a
listening learn is waiting for. Every site that paints a teachable control asks it instead of comparing to `Choosing`.

**Pattern readout.** The pattern display becomes the pattern learn target — click it while choosing (as with the BPM
readout beside it), and it wears the pick colour while choosing and while listening.

**The pattern controller, in three parts.**
- *Pickup*, like every other control: `takes_pickup(Pattern)` becomes true, and `knob_value(Pattern)` answers with the
  middle of the zone the playing pattern occupies (`(number + 0.5) / 8`). A fader parked elsewhere is not an instruction.
- *A request, not a jump*: `apply_cc` keeps writing the absolute index into `state.pattern_request`, unchanged.
- *Swap on the pattern's last step*: `main` holds the request as `pending_pattern` and applies it when the transport
  reports step 0, or a wrap, or is stopped — `ui::pattern_swap::swap_now(playing, position, last)` is that rule, a pure
  function with a test. Meanwhile `panel.pattern_id` shows the *selection*, so cycling with the knob is visible, and the
  audible swap lands on the bar.

**Tips.** `tips::Control::Tempo` and `Control::Pattern` join the table (appended, so no existing index moves) and both
readouts call `tips::hover` every frame.

## The tests that make it safe

| Test | Pins |
|---|---|
| `learn::a_listening_target_keeps_its_border` | `highlight(Choosing, _)` true; `highlight(Listening(Cutoff), Cutoff)` true; a neighbour's false; `Idle` false |
| `learn_live::the_tempo_and_pattern_readouts_can_be_taught` | clicking each readout while choosing binds that control (the PATTERN key no longer does) |
| `tips::every_readout_has_its_own_help` | the two new controls have distinct titles and a description, in ASCII |
| `tips_live` (existing, extended) | hovering the pattern and tempo displays shows their help |
| `cc::a_pattern_fader_has_to_reach_the_playing_pattern` | a far fader requests nothing; sweeping onto the zone requests that pattern |
| `pattern_swap::the_swap_waits_for_the_bar` | stopped → now; step 0 → now; a wrap → now; mid-bar → not yet |

## Task list

1. `learn::highlight` + use it at every teachable site (knobs row, volume, shuffle, the two readouts). **Green commit.**
2. `tips::Control::{Tempo, Pattern}` and their readouts' hover. **Green commit.**
3. The pattern learn target moves to the readout (click + highlight); the PATTERN engraving stops answering. **Green commit.**
4. Pattern pickup (`takes_pickup`, `knob_value`). **Green commit.**
5. Deferred swap: `ui::pattern_swap::swap_now`, `main` holds the request and shows the selection; the display follows the
   held index. **Green commit.**

## Non-goals

- No change to how a pattern is chosen by the keyboard (shift+arrows stay immediate — the player asked for them).
- No new save field: a pending swap is a live performance gesture, not session state.
- Bigger MIDI clock quantisation work (this is the panel's own bar boundary).

## Progress

- [x] **1 + 3. Highlight, and the pattern readout as the learn target** — `learn::highlight(step, control)` is the one
      rule: every teachable control wears the border while choosing, and the picked one keeps it while listening. All
      four sites use it (knobs row, volume, shuffle, both readouts). The pattern learn target moved from the PATTERN
      engraving to the pattern display, so `learn_live::the_tempo_and_pattern_readouts_can_be_taught` (renamed) and
      `learn_highlight_live::the_picked_control_keeps_its_border_while_listening` (was `…goes_away…`, expected 0 strokes)
      were rewritten and now pin the new behaviour.
      Ruling: tasks 1 and 3 land in one commit — the readout move and the highlight share the same two call sites in
      `ui/mod.rs`, and splitting them would leave a commit whose readout teaches a control it does not outline.
