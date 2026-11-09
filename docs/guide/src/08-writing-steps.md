# Writing steps

A pattern is **sixteen steps**. Each step is either a rest or a note, and a note carries five things:

| Part | What it is |
|---|---|
| **gate** | whether the step sounds at all |
| **note** | which pitch, in the panel's own spelling (`A1`, `C2`, `E1` …) |
| **octave** | the note's register, moved by the drag and by `shift+up` / `shift+down` |
| **velocity** | how hard it is played. It comes from a MIDI keyboard's own velocity; the mouse and the computer keyboard leave it as it was |
| **accent** | a louder, brighter hit — the emphasis the line leans on |
| **slide** | the note glides into the next step instead of retriggering |

A rest keeps the pitch, accent and slide it had, so turning a step off and on again is not a rewrite.

## Every gesture on the step row

Straight from the help card, which is the list the panel itself keeps:

| Gesture | What it does |
|---|---|
| click | select the step |
| ctrl+click | note on or off |
| shift+click | accent — louder, brighter |
| alt+click | slide — dim means the next step rests, so it cannot glide |
| drag up or down | change the pitch |
| double-click | clear the step, back to a rest |

Two rules ride along with those: a step you click becomes the selected step, and the selection is the one the keyboard
edits.

## Typing a line on the computer keyboard

The home row is a chromatic scale:

```
A W S E D F T G Y H U J K
```

That is C, C#, D, D#, E, F, F#, G, G#, A, A#, B, C — the mapping most DAWs use, so your fingers may already know it.
`Z` and `X` move the keyboard's octave down and up.

A plain letter press **plays** the note: the keyboard is an instrument, and it writes nothing. Hold `shift`, or turn
**`STEP`** on, and the same key becomes an **edit** — the note goes into the selected step and the voice stays quiet. With
`shift` down the selection stays where it is, so you can correct the note on the step you are looking at without the
selection running away.

While you are in the step row, the **arrows** move the selection, `shift+up` and `shift+down` move the selected note by a
step, `enter` turns the step on and off, and `backspace` — while you are editing — empties it to a rest.

**`STEP`** is the mode that makes typing a line quick: with it on, a key **down** writes the note to the selected step and
the key **up** walks the selection on by one. Type a phrase left to right and the sixteen steps fill in behind your
fingers. Press `STEP` again to stop walking.

`CLEAR` empties the whole pattern, and it takes two clicks: the key reads `CLEAR?` for a few seconds in between, and any
edit drops the arm. ([`03-first-sound`](03-first-sound.md) has the same note.)

The next page is about the clock the line runs on.
