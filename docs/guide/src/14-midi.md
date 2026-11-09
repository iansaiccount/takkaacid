# MIDI

takka-acid has one MIDI input. It plays notes, and it can be taught to drive almost every control on the face.

## Choosing a port

`F2`, then unfold **`MIDI IN  [+]`** and click the port. The list ends with `none`, which closes the port without opening
another. The choice is remembered across sessions; if the port is missing next time, the card says so — `MIDI: none
(retrying)` — rather than silently picking a different device.

## Playing notes in

A note from the keyboard plays the voice with its own **velocity**. Two rules decide whether it only plays or also writes:

- **Nothing held:** the note plays over the pattern and changes nothing. This is how you jam against a running line.
- **`STEP` on, or `shift` held:** the note is written into the selected step, with the velocity it arrived with — and a
  hard enough hit marks the step **accented** as well. With `STEP` on, each note you play also walks the selection on, so
  a keyboard player can type a line the way the computer keyboard does.

There is one voice, and the MIDI keyboard and the computer keyboard share it. The last note wins; releasing the computer
keys closes the gate, which also ends a MIDI note that was sounding.

## `LEARN`: teaching a control

Eleven controls come factory-mapped (`20-reference` has the list), and any of them — and every knob — can be re-taught:

1. Press **`LEARN`**. Its face changes to **`PICK`**, and the hint line says `LEARN: click a knob or a readout`.
2. Click the knob, switch or readout you want the controller to drive. The face changes to **`MOVE`** and the hint says
   `LEARN: move a MIDI control`.
3. Move a fader, knob or button on the controller. That controller number is now that control's.

What the controller does *not* do is jump the control the moment it is taught: **dials take pickup**. A fader parked
somewhere else has to reach the knob before it may turn it, so loading a song — or picking up a controller mid-set — can
never slam the patch. The wave switch is the exception: it is a choice rather than a position, so a fader pointed at the
triangle means the triangle.

`Escape` gives up on a learn in progress and leaves the mapping as it was. A control that is listening shows a border, so
you can see which one is about to be bound.

## The pattern selector

The pattern selector is a separate case, and the one that behaves most like playing: its knob moves the selection
immediately, the **readout shows where you are going**, and the sound follows at the end of the bar. `11-the-bank` explains
why it waits. It also takes pickup, so the fader has to be in the zone of the pattern the selection holds before it may move
the selection — otherwise the first message would jump the bank.

## An external clock

If your controller or interface sends MIDI clock (24 pulses per quarter note), the tempo follows it: the readout shows
the tempo the clock implies and the transport runs to it. While clock is arriving, a tempo you set by hand holds until the
next tick replaces it — the clock is the master.

`15-keyboard` is the computer keyboard doing most of the same job.
