# A rack of devices

One window can hold **four machines**, and the strip above the panel is the whole rack.

![Two devices, the second on FESTIVAL CITRUS](img/two-devices.png)

## The strip

![The strip: one cell per device, and the add key](img/strip.png)

A cell per device, left to right, then `+`. Each cell carries:

- **its name** — `ACID 1`, `ACID 2`, or whatever you typed on the device's card;
- **a lamp**, lit while that device's sequencer is running;
- **`M`**, that device's mute;
- **a border**, on the one cell the panel is showing, and a caret before its name — the way into that device's card.

Click a cell to put that device on the panel, or press `Ctrl+1` to `Ctrl+4`. The panel shows one device at a time; the strip is
how you know which of them you are editing before you turn anything.

**Clicking a cell is not a piano key.** It changes what you are looking at, and nothing else: the device you left keeps
playing, at its own tempo, with its own pattern.

## Adding one

`+` at the right end of the strip asks what kind of device to add — a picker opens with the kinds there are, `ACID` and
`DRUMS` — and a click puts the new machine last in the strip and on the panel.
[`22-the-drum-machine`](22-the-drum-machine.md) is what the second kind does; the two share the strip, the transport and the
bar, and nothing else about them is the same.

Click the row and the device is there, last in the strip and on the panel. At four devices the row greys out; the key still
answers, so a click says the rack is full rather than doing nothing.

## What a device is

Each device is a whole machine, not a subset of one:

- **its own bar of steps** — sixteen, twelve or thirty-two, at the pattern's own grid — and its own **sixty-four patterns** in
  eight banks of eight;
- its own **seven knobs** and volume;
- its own **shuffle**, its own octave transpose, its own `RANDOM` density and scale;
- its own **MIDI inputs** — which of the open ports it hears, on which channel, and whether it plays notes or only answers
  controllers (that is its card's business, [`19-a-devices-card`](19-a-devices-card.md));
- its own **name**, and its own **colour**.

What they share is the clock and the sound card. `PLAY`, in the deck, starts them all, the tempo is one tempo, and a device
added while the line is running **waits for the bar**: it makes no sound until the bar the rack is in finishes, then begins on
the downbeat with every other device, so it never comes in half a bar out of step. While it waits, its cell in the strip is
already lit — it is playing the rack's transport, and the bar is what it is waiting for — and its step LEDs sit at the first
step until they start moving together with everyone else's. Two devices playing one pattern
therefore sound like one machine with two voices; two devices with different patterns sound like two machines playing
together.

## Mute, not stop

`M` on a cell silences that device without stopping it: the fade is 10 ms — a hundredth of a second, too short to hear as a
fade and long enough not to click — and the sequencer keeps running underneath. Unmute a bar later and it is where it would
have been.

## The theme

A device's face can be **any of twenty-two themes** — the instrument's own black, and twenty-one built from the palettes on
the music colour palette page the themes were taken from — and the card's `COLOUR` row opens the list
([`19-a-devices-card`](19-a-devices-card.md)). Put one machine on `FESTIVAL CITRUS` and the next on `BLACK` and the strip
tells them apart before you click: the cell above each device wears that device's colours, while the panel wears the selected
one's.

## What a restart keeps, and what it does not

The session file holds **every machine in the rack**: each one's patterns, patch, kit, swing, learned controllers, ports,
channel, name and colour, plus the first MIDI port the app had open. Four devices saved are four devices after a restart, in
the same order and with the same names.

Two things each device forgets for itself — its `REVERSE` direction and its own transpose. Both are performance, not patch:
they are deliberately outside the file, so a restart brings every device back running forwards and in tune, and one machine
playing a semitone up never drags the others with it.

A kit is sounds as well as a name. A kit that was edited — a copy of a factory kit, or one imported — has a *folder* written
beside the session when it is saved, so the samples come back with the name. A folder that has been moved or deleted is said
on the status line when the machine tries to load it, and the machine keeps the silence rather than a substitute.

## A rack of your own

The session file keeps the rack you are working on. A **rack file** keeps the machines themselves, as a preset you can recall
into any session: `BANKS`'s `RACKS` section saves one, lists the ones you have, loads one and deletes one
([`12-songs-and-demos`](12-songs-and-demos.md#the-four-lists) is that section of the card).

A load replaces every machine in the session and leaves the song's name and the tempo alone — a rack built at 150 is heard at
whatever the song is playing when you recall it, which is what keeps four machines on one clock. What a restart forgets, a
recall forgets too: the machines come back unmuted, running forwards and in tune. And the file's name is remembered for the
session and not across a restart, so a save under a name already on disk lands beside it rather than over it.
