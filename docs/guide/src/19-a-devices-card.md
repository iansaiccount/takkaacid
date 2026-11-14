# A device's card

`F3`, or the caret on the selected cell, opens the card for the device on the panel.

![The device card for ACID 1](img/device-card.png)

Everything on it is about that one device, and every row acts on it immediately:

- **`NAME (ENTER)`** — what the strip and the notices call it. Type and press `enter`. A name another device already holds
  lands as `BASS (2)`, so two cells can never read the same.
- **`ROLE`** — `PLAY + CTRL` or `CTRL ONLY`. Controller-only is for a box of knobs on a channel of its own: it will not start
  a note just because a message arrived ([`14-midi`](14-midi.md)).
- **`PORTS`** — `ALL PORTS`, then one row per port the machine reports. A ticked row is a port this device hears; unticked,
  and it ignores that port even though the app has it open. Which ports are *open at all* is the settings card's.
- **`CHANNEL`** — `CH ANY`, or one channel of sixteen.
- **`COLOUR`** — `BLACK` or `YELLOW`. A click flips it.
- **`MUTE OFF` / `MUTE ON`** — the same mute as the cell's `M`, said in full.
- **`REMOVE`** — takes two clicks, like `CLEAR`. The second one is the answer.

**Out:** `CLOSE`, `Escape`, or a click outside the card.

## The colour

![The same two devices, the second one amber](img/two-devices.png)

`YELLOW` turns that device's face amber — legends near-black, knobs and key faces dark — and the amber stays with the device,
not with the window. Put one machine on amber and the next on black and the strip becomes a legend: each cell wears its own
device's colours, while the panel wears the one you are on.

The four cards stay black whatever the face does. A card is furniture laid over the instrument, not part of it.

The colour is **not saved**. A restart brings every device back black.

## Removing a device

`REMOVE` asks twice: the first click turns the key into `CONFIRM?`, and any other key or click disarms it. Then the device
goes, its cell with it, and the panel moves to a neighbour. The last device cannot go — one machine is the minimum a rack can
hold.

Nothing on disk changes when a device is removed: the session file still holds the first device only, so removing that one
changes what the next save will write, and removing any other does not
([`12-songs-and-demos`](12-songs-and-demos.md)).

## One card at a time

The picker (`+` on the strip) is the card painted over everything, the device card is under it, and `F1`/`F2`/`F3` each put the
others away — [`05-the-three-cards`](05-the-three-cards.md) has the whole rule, including what `Escape` gives up first.
