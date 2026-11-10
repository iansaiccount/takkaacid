# When it does not work

Most of the panel's own diagnosis is already on the face: it prints what it found, in the strip and in the settings card.
This page is the order to check things in.

## There is no sound at all

1. **Is the line running?** `RUN` lit, `STOP` dark. `SPACE` toggles it.
2. **Does the pattern have notes?** A pattern of rests is silent and looks almost the same as a pattern — the keycaps
   print `--` and their LEDs are dark. Press `RANDOM` and see whether a line comes back; `CLEAR` cannot help: it empties a pattern, it does not fill one.
3. **Is the filter shut?** Cutoff at the bottom with a sine wave is a very quiet machine. Turn `CUT OFF` up.
4. **Is the volume down?** The knob in the top-left corner, not one of the row.
5. **What does the settings card say?** `F2`, and look at the words to the right of `SOUND OUT`. Healthy, they read
   something like `48 kHz - Default Audio Device`. If they begin `failed:` the audio stream is not running and nothing
   else matters until it is.
6. **Are the notes inside the instrument's range?** `OCT -` or `OCT +` pressed a few times too often walks a line out of
   what the voice can play, and those steps go quiet.

## The stream failed

`failed: …` beside `SOUND OUT` means the audio device is gone or busy — another app took exclusive hold of it, or a USB
interface was unplugged. Unfold `SOUND OUT  [+]` and pick another device; the app reopens the stream on the one you
choose. If only one device is listed and it says `failed:`, close whatever else is holding the device, then close the card
and open it again with `F2` — the lists are read when the card opens — and pick the device.

## The controller is not seen

Look at `MIDI IN` in the settings card (`F2`):

- **`MIDI: none (retrying)`** — the app is looking: it tries every two seconds, and gives up after about half a minute,
  at which point the words change. Check the cable, the controller's own MIDI mode, and that another program is not
  holding the port.
- **`MIDI: none (gave up)`** — it has stopped trying. Press **`RETRY`**, which appears beside the section while this is
  showing.
- **`MIDI: <name>`** — a port is open; if notes still do nothing, the trouble is upstream of the app (go on to
  `--midi-map`).

Then unfold `MIDI IN  [+]` and check the list: an interface with several ports may have more than one, and the app is
holding the one you picked last time. The list is read when the card opens, so close and reopen it (`Escape`, then `F2`)
after plugging anything in.

## "Is it my keyboard or the app?"

```bash
takka-acid --midi-map
```

This prints one row for every note in an 88-key range (21 to 108): the number, the keyboard's name for it, the note the app
writes, the note it shows and the frequency it plays — then it exits. No window, no audio device, no controller needed. Press
a key on the controller and compare the note you expected with the row for the number that arrives:

- **The app's row matches what you pressed** — the app is fine; the problem is in the sound (back to the top of this page)
  or in the pattern.
- **The row does not match** — the keyboard is transposed, on a different octave, or sending a note number the mapping
  does not reach. That is a controller setting, not an app setting.

[`14-midi`](14-midi.md) explains what each note does once it arrives.

## Your work is not there

The session lives in a data directory:

- `$TAKKA_ACID_DATA_DIR` if it is set,
- otherwise `$XDG_DATA_HOME/takka-acid`,
- otherwise `~/.local/share/takka-acid`.

Inside it, `state.json` is the session and `songs/` holds one file per named song. Both are read at startup: a run with a
different `TAKKA_ACID_DATA_DIR` sees none of the work from a run with the default one.

**And the warning worth repeating:** saving is manual by default. The help card's notes say the session is saved on exit
and again two seconds after a change; only the exit save always happens — the two-second one needs `AUTOSAVE` on, and it
is off unless you turned it on. If you lost work, that is the most likely reason, and [`13-settings`](13-settings.md) is where the switch
lives.

## Reporting a bug

Include, in this order:

1. **What you did**, in the smallest steps that show it.
2. **What happened**, and what you expected instead. A screenshot of the panel is worth a paragraph.
3. **The words on the face** — the notice in the strip, and the `MIDI:` / audio words in the settings card. They are the
   app's own diagnosis; a bug report that quotes them saves a round trip.
4. **Your machine**: the OS and session (Wayland or X11), the audio server (PipeWire or ALSA), and the audio and MIDI
   devices, by name.
5. **How you built it** — the commit you are on, or the release binary.
6. **What `takka-acid --midi-map` prints**, when the trouble is MIDI.

[`20-reference`](20-reference.md) has every word the panel uses, so you can be sure of the exact spelling when you quote it.
