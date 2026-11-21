# MIDI

takka-acid listens to one MIDI input, or to several at once. Each plays notes, and can be taught to drive almost every
control on the face.

## Choosing ports

`F2`, then unfold **`MIDI IN  [+]`** and click a port's row: `[x]` is a port the app holds open, `[ ]` is one it does not,
and any number may be on at once. A fresh session starts with the first port the machine reports. The session remembers
the first port it had open across sessions; if that port is missing next time, the card says so —
`MIDI: none (retrying)` — rather than silently picking a different device.

## What each device hears

Which ports are **open** is the settings card's business ([`13-settings`](13-settings.md)). Which of them a *device* hears
is that device's own card's: `F3`, then `PORTS` — `ALL PORTS`, or a tick per port — and `CHANNEL`, which is `CH ANY` or one
channel of sixteen.

`ROLE` decides what a message on that channel does. `PLAY + CTRL` plays it and moves the knobs; `CTRL ONLY` answers with the
knobs and ignores notes, which is what a box of faders on a channel of its own wants: moving a knob must not start a note.

So one open port can feed four devices: give each a channel, or let two share one deliberately. Learn is per device too —
the same controller number can drive a knob on two machines at once.

## Playing notes in

A note from the keyboard plays the voice with its own **velocity**. Two rules decide whether it only plays or also writes:

- **Nothing held:** the note plays over the pattern and changes nothing. This is how you jam against a running line.
- **`STEP` on, or `shift` held:** the note is written into the selected step, with the velocity it arrived with — and a
  hard enough hit marks the step **accented** as well. With `STEP` on, each note you play also walks the selection on, so
  a keyboard player can type a line the way the computer keyboard does.

There is one voice, and the MIDI keyboard and the computer keyboard share it. The last note wins; releasing the computer
keys closes the gate, which also ends a MIDI note that was sounding. **A drum machine takes no notes from a keyboard this
phase** — its card still opens ports and a channel, and a note arriving there changes nothing — because its sixteen voices are
the grid's, not the keyboard's.

## `LEARN`: teaching a control

Eleven controls come factory-mapped ([`20-reference`](20-reference.md) has the list), and any of them — and every knob — can be re-taught:

1. Press **`LEARN`**. Its face changes to **`PICK`** — it is asking which control you mean.
2. Click the knob, switch or readout you want the controller to drive. Its face changes to **`MOVE`**, and every control
   that can be taught wears a border while it waits.
3. Move a fader, knob or button on the controller. That controller number is now that control's.

What the controller does *not* do is jump the control the moment it is taught: **dials take pickup**. A fader parked
somewhere else has to reach the knob before it may turn it, so loading a song — or picking up a controller mid-set — can
never slam the patch. The wave switch is the exception: it is a choice rather than a position, so a fader pointed at the
triangle means the triangle.

`Escape` gives up on a learn in progress and leaves the mapping as it was. The control that is waiting wears a border, so
you can see which one is about to be bound.

## The pattern selector

The pattern selector is a separate case, and the one that behaves most like playing: its knob moves the selection
immediately, the **readout shows where you are going**, and the sound follows at the end of the bar. [`11-the-bank`](11-the-bank.md) explains
why it waits. It also takes pickup, so the fader has to be in the zone of the pattern the selection holds before it may move
the selection — otherwise the first message would jump the bank.

## An external clock

If your controller or interface sends MIDI clock (24 pulses per quarter note), the tempo follows it: the readout shows
the tempo the clock implies and the transport runs to it. While clock is arriving, a tempo you set by hand holds until the
next tick replaces it — the clock is the master.

[`15-keyboard`](15-keyboard.md) is the computer keyboard doing most of the same job.
