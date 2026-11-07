# Step selection and note editing — design

2026-11-07. Short design: a small change to one area of the panel.

## Intent

Two gaps make step editing awkward today. The note a step holds is invisible — the keycap shows only its
number — and changing it needs a vertical drag with no readout, so you find the note by ear. And nothing on
the panel says which step the keyboard is about to act on.

So: an explicit selected step with a visible border, the note printed on every keycap, and keyboard editing
of the selected step's note and position.

## What the panel shows

Each keycap keeps its shape, accent bar and slide bar. Inside the white key, the **note** takes the middle
where the step number was, and the **step number** moves to the bottom-right corner, small:

```
 ┌──────────────┐
 │              │
 │     D#0      │   note, centred
 │            3 │   step number, small, bottom corner
 └──────────────┘
```

A rest shows `--` in the note position, so the row reads as sixteen notes, not nine notes and seven blanks.

The **selected** step gets a 2 px amber outline around its whole cell — outside the white key, so it stays
visible when the step is lit and when it is a rest.

## Gestures

Selection is explicit, so the click stops editing (decided: option B).

| Gesture | Before | After |
|---|---|---|
| click | toggle step | **select** the step |
| ctrl+click | — | toggle the step on or off |
| shift+click | accent | accent (unchanged) |
| alt+click | slide | slide (unchanged) |
| double-click | clear | clear (unchanged) |
| drag vertically | change pitch | change pitch **and** select the step it started on |

A drag has to select: it is the only gesture that can target a step without a click, and leaving the
selection behind would make the border lie about what the keyboard edits.

## Keyboard

| Keys | Action |
|---|---|
| shift+left / shift+right | move the selection one step, clamped at 16 and 1 |
| shift+up / shift+down | raise or lower the selected step's note by a semitone |
| enter | toggle the selected step on or off |

A semitone per press, chromatic, clamped to the panel's existing `-12..=24` absolute range — the same range
and the same helper the vertical drag uses (`edit::nudge_pitch`). Nudging a rest turns it into a note at the
nudged pitch, matching how accent and slide already force a step on.

Shift+arrows are free: unshifted arrows keep tempo (up/down) and pattern (left/right).

## Selection lifetime

The selection is one index in `0..16` held in `PanelState`. It is not saved, and it resets to the first step
when the pattern changes, because it names a position in the pattern on screen rather than anything about the
music.

## Non-goals

- No audition on select. Sounding a note from a selection would have to share the one gate with the computer
  keyboard and MIDI, and that is a separate decision.
- No multi-step selection, no copy/paste of steps.
- The playing-position LED is unchanged and independent of the selection; both may light the same key.
