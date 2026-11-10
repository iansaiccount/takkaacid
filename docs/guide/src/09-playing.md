# Playing it

![The transport](img/transport.png)

**`RUN`** starts the line and **`STOP`** stops it; the space bar is both. The transport says what is happening rather than
what happened last: `RUN` is lit while the line runs, `STOP` is lit while it is stopped.

![The panel while the transport runs](img/running.png)

## Tempo

The tempo readout sits in the top strip, in BPM, from **40–300**. Two things move it a BPM at a time:

- `TEMPO -` and `TEMPO +`, one press each way.
- `up` and `down` on the computer keyboard.

There is a third way in, and it is not a nudge: a learned controller fader throws the tempo wherever you leave it
([`14-midi`](14-midi.md)).

Holding `up` or `down` sweeps the tempo; those two are the only keys in the panel that repeat while held.


## Swing

Swing makes the first of each pair of steps long and the second short by the same amount, so a pair always lasts exactly
two steps and the bar length never changes however far you turn it. At zero the sixteenths are even; turned up, the second
of each pair arrives late and the line leans. That lean is the shuffle.

The knob is **`SHUFFLE`**, in the performance row, and it is [`10-performance-row`](10-performance-row.md)'s subject.

## Why the step LED follows the sound

The panel keeps no clock of its own. Every frame it asks the **audio engine** where it is in the pattern and lights the LED
over that step, so the light is where the sound is rather than where a timer in the panel thinks it should be.

Two things follow. The light and the sound cannot drift apart when the machine is busy: the panel may skip a frame, and the
light is then a frame behind, but it never runs ahead of what you hear. And because a step that holds a note keeps its LED
lit whether or not the transport is on it, the *moving* light is easiest to follow across the rests — which is exactly
where you want to know where the bar is.

A stopped line lights no position: the row shows which steps carry notes and nothing else.

## Starting and stopping

`STOP` leaves the pattern where it is, but `RUN` does not resume from there — it starts again from step 1, with the clock
on the downbeat, so the first note you hear after a start is always the first note of the bar. Nothing is quantised: your
presses land when you press them.

The pattern itself is untouched by any of this — but the touch of a transport key does mark the session unsaved, because
whether the line was running is part of the session that gets saved.

[`10-performance-row`](10-performance-row.md) is what to reach for while the line is playing.
