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
  middle of the zone the *selection* occupies (`(number + 0.5) / 8`) — `SharedState::pattern_selected`, which is the
  pending swap while one is waiting. A fader parked elsewhere is not an instruction, and one mid-sweep keeps turning.
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
| `tips::the_two_readouts_have_their_own_help` | the two new controls have distinct titles, a description in ASCII, and the shortcuts they really have |
| `learn_live::the_tempo_and_pattern_readouts_can_be_taught` | clicking each readout while choosing binds that control, and the PATTERN engraving no longer answers |
| `cc::a_pattern_fader_has_to_reach_the_pattern_before_it_moves_it` | a far fader requests nothing; sweeping onto the zone requests that pattern |
| `cc::a_sweep_keeps_turning_while_the_swap_waits_for_the_bar` | the fader keeps turning while the sound is held, because the pickup measures the selection |
| `pattern_swap::the_swap_waits_for_the_bar` | stopped → now; the last step → now (so the next trigger is the new bar); a wrap → now; mid-bar → not yet |

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
- [x] **2. Tips for the two readouts** — `tips::Control::Tempo` and `Control::Pattern` appended to the table (indices 23
      and 24, so nothing existing moves), with the arrows as the tempo's key and none for the pattern; both readouts get a
      hover sense and `tips::hover` beside their paint. Test `tips::the_two_readouts_have_their_own_help`.
- [x] **4. Pattern pickup** — `takes_pickup(Pattern)` is true and `knob_value(Pattern)` answers with the middle of the zone
      the playing pattern occupies (`pattern_zone`), so a fader parked on pattern 8 no longer jumps the selection.
      Two more changes fell out of making that honest: `pattern_number`/`pattern_zone`/`pattern_number_of` are one mapping
      each (the request and the pickup read the same zones), and what a controller *writes* for a stepper is its zone
      middle (`written`), or staying inside the zone after the swap would count as the hand stealing the knob.
      Tests: `cc::a_pattern_fader_has_to_reach_the_pattern_before_it_moves_it` (RED: the far fader requested pattern 7);
      the three older pattern tests now model the app (`main` applies the request, which is what moves the knob) and the
      stepper test no longer claims the pattern is one.
- [x] **5. The swap waits for the bar** — `ui::pattern_swap::swap_now(playing, position, last_position)` is the rule
      (stopped, step 0, or a step that went backwards), unit-tested. `main` now holds the request as `pending_pattern`,
      shows it in the readout (`panel.pattern_id`) so cycling is visible, applies it through `shortcuts::select_pattern`
      when the bar allows, and drops it on a song load or a new song. `last_position` is kept so a bar crossed between
      two frames is still noticed.
      Ruling: the readout shows the *selection*, not the pattern that is still sounding for the rest of the bar. That is
      what makes cycling with the knob usable ("see where you are going"); the cost is that for up to one bar the face
      names a pattern the engine has not switched to yet. Cost if wrong: a player who edits notes in that window edits
      the sounding pattern while the display names the next one.
      Ruling: a pending swap is live performance state only — no new save field. Cost if wrong: quitting mid-cycle
      forgets the swap, which a player would expect anyway.

## Review

One fresh-context review of the three commits. Verdict: block — one Critical, four Important, seven Minor. The Critical
and all four Important are fixed here; the minors that were cheap went with them.

- **Critical, fixed:** the pattern pickup went deaf after the first message. The swap is deferred, so the *sounding*
  pattern no longer follows the fader, and `takeover`'s "the knob still holds what I wrote" rule compared the fader's last
  write against a value that had not moved. `SharedState::pattern_selected` now holds the selection (the pending swap),
  `knob_value(Pattern)` measures against it, and `main` sets it when it takes a request and clears it on adopt. Test first:
  `cc::a_sweep_keeps_turning_while_the_swap_waits_for_the_bar` failed on the third zone before the fix.
- **Important, fixed:** the swap landed one step *after* the bar, because the panel only sees a step after the audio
  thread has triggered it — the new pattern's first step was never heard. `swap_now` is now due on the last step (and on a
  wrap, and when stopped), so the trigger that follows is the new bar's. The test's mid-bar case moved off step 15.
- **Important, fixed:** the choosing hint still said "click a knob, PATTERN or the BPM", pointing at an engraving that no
  longer answers. It says "a knob or a readout" now, and `learn_live` pins that the engraving leaves the panel in PICK.
- **Important, fixed:** a keyboard pattern move did not drop a held swap, so the bar undid the player's deliberate move.
  `main` compares the bank around the dispatch loop and clears the pending swap. The "adopt a pattern" block is now
  `pattern_swap::adopt`, called by the keyboard, the controller swap and both song paths — so it also disarms CLEAR and
  resets the step selection, which the controller path used to miss.
- **Important, fixed:** the wiring tests. The engraving test exists, and the hover test does too — after a first attempt gave
  up on it: the tooltip's galley *is* in `eframe::FullOutput.shapes`, so `tips_live::the_two_readouts_answer_the_pointer`
  hovers each readout across four frames (egui's tooltip delay needs a still pointer and a moving clock) and reads the
  painted text back.
- **Minors taken:** the `_ =>` arm in `apply_cc`'s `written` match is exhaustive; a request naming the pattern already
  playing is dropped (a resting fader used to re-publish the pattern and mark the session unsaved every bar); the PATTERN
  tip no longer claims it has no key (shift+arrows); `pattern_number_of` asks `PatternBank::id_of`; stale comments and the
  plan's test names corrected.

## Leftovers, closed

The round opened with a few things deliberately left, and the review's deferred minors. All are now done:

- **The live hover test exists.** `tips_live::the_two_readouts_answer_the_pointer` walks `FullOutput.shapes` for text
  galleys, hovers each readout across four frames (a still pointer, a moving clock) and finds the tip's title in the
  painted text. The earlier note that a tooltip's text could not be read back was wrong.
- **One predicate behind the pickup's two questions.** `has_a_position(target)` is what `takes_pickup` and
  `knob_value`'s shape both mean; `cc::a_control_that_takes_pickup_is_exactly_one_with_a_position` walks `LEARNABLE` and
  pins them together.
- **`row_transport::draw` dropped its unused `s`**, and the duplicated doc line above `PanelState::reprobe_requested` went
  with it.
- **`README.md`** — what the instrument is, its features, how to build and run it, where it keeps its files, how the plans
  are shaped (and why they are published: they are the record of how an agent built the product), and the original
  `Prompt.txt` brief that started it.
