# Reference

Three tables and the files: every gesture, the factory controller map, and the words the panel prints.

## Keys and gestures

**On a step key**

| Gesture | What it does |
|---|---|
| click | select the step |
| ctrl+click | note on or off |
| shift+click | accent |
| alt+click | slide |
| drag up or down | change the pitch |
| double-click | clear the step to a rest |

**On a knob**

| Gesture | What it does |
|---|---|
| drag up or down | the whole sweep in one short pull |
| shift+drag | a tenth of the movement |
| wheel | nudge |
| double-click | back to the centre of the sweep |

**On the computer keyboard**

| Key | What it does |
|---|---|
| `A W S E D F T G Y H U J K` | play a chromatic scale; with `shift` or `STEP`, write the note to the selected step |
| `h` | on a drum device, strike the selected voice — `HIT`'s key |
| `l` / `b` / `p` | on a drum device, the low, band or high side of the selected voice's filter |
| `Z` / `X` | octave down / up, two either way |
| `space` | play / pause, same as `PLAY` / `PAUSE` |
| `,` | step the playhead back a bar (drawn, not wired yet) |
| `.` | step the playhead on a bar (drawn, not wired yet) |
| `R` | record (drawn, not wired yet) |
| `left` / `right` | move the selection one step, on either face |
| `shift+left` / `shift+right` | previous / next pattern |
| `up` / `down` | note up / down a step on an acid device; on a drum device, the previous / next voice, repeating while held |
| `shift+up` / `shift+down` | tempo, one BPM (and they repeat while held) |
| `enter` | note on or off; on a drum device, write the selected voice into the step (`shift` accents it) |
| `backspace` | empty the step to a rest, while editing |
| `F1`, `shift+?`, or the `?` | the help card |
| `F2`, or the gear | the settings card |
| `F3`, or the caret on the selected cell | the device card |
| `Ctrl+1` … `Ctrl+4` | put that device on the panel |
| `Ctrl+C` / `Ctrl+V` | copy the pattern on the panel / paste one into it |
| `escape` | close the topmost card — the picker, the device card, the kit browser, settings, help, the bank browser — then give up on a `LEARN` |

**The panel's own keys:** `REVERSE`, `TRANS -`, the transpose's readout, `TRANS +` and `TBC` twice, then `LEARN`, in the
bottom row; `OCT -` `OCT +` `CLEAR` `RANDOM` in the performance row's left cluster, with `SHUFFLE`, density and scale at its
right; `SAVE` `BANKS` `STEP` `COPY` `PASTE` in the switch row (`BANK` and `PATTERN` are engraved and do nothing yet).
**The strip above the panel:** a click on a cell selects that device, its `M` mutes it, and `+` at the right end adds one.

## The factory controller map

Eleven controls ship taught to a controller. Any of them can be re-taught with `LEARN` ([`14-midi`](14-midi.md)).

| Control | CC |
|---|---|
| Waveform (the SAW switch) | 20 |
| Tuning | 3 |
| Cut off | 74 |
| Resonance | 71 |
| Env mod | 76 |
| Decay | 75 |
| Accent | 5 |
| Volume | 7 |
| Tempo | 72 |
| Pattern | 21 |
| Swing (`SHUFFLE`) | 73 |

The waveform has no pickup — a fader pointed at a face means that face. Everything else, the pattern selector included,
has to be reached before it takes over.

## The words the panel prints

**The strip** above the top strip holds a cell per device: its name, a lamp while it runs, its `M` mute, and `+` at the
right end.

**The top strip** holds the volume knob and the wordmark, the settings gear and the help dot, the song name — amber while unsaved,
plain silk once written, the whole name and its state in its tooltip — the tape deck, the tempo stepper, and the pattern
and tempo wells at the right.

| Word | Where | Meaning |
|---|---|---|
| the song name's colour | after the `?` | amber: unsaved; plain silk: on disk |
| `SONG <name>` | after the `?` | the session's name; `SONG` alone when it has none |
| a notice | after the `?` | something just happened — `saved Acid 2`, `saved a copy as Acid 2`, `could not import: …` — for a few seconds, then the name returns |
| `A1` … `H8` | the pattern well | the pattern playing |
| a number | the tempo well | the tempo in BPM |

**Armed keys** say what they are waiting for:

| Word | Where |
|---|---|
| `CLEAR?` | the CLEAR key, for a few seconds after the first click |
| `SURE?` | `NEW`, while the session is unsaved |
| `<name> - click again to replace` | a song row, while loading over unsaved work |
| `SURE` | the `x` on a song row, after one click |
| `PICK` | the `LEARN` key, waiting for you to click a control |
| `MOVE` | the `LEARN` key, waiting for the controller |
| `CONFIRM?` | `REMOVE` on a device card, after the first click |

**The device card.** `NAME (ENTER)`, `ROLE` (`PLAY + CTRL` / `CTRL ONLY`), `PORTS` (`ALL PORTS`, then a ticked row per port),
`CHANNEL` (`CH ANY` or `1`…`16`), `COLOUR` (the theme's name, and the key that opens the list of twenty-two), `KIT` (a drum
machine's, and the name of the kit it plays), `MUTE OFF` / `MUTE ON`, `REMOVE`, `CLOSE`.

**The picker.** `ADD DEVICE`, a row per kind (`ACID`, `DRUMS`), and `CLOSE`.

**The drum machine's face.** `DRUMS` is the device; its seven dials are the selected voice's — `ATTACK`, `DECAY`, `SUSTAIN`,
`RELEASE`, `CUT OFF`, `RESONANCE`, `TUNE` — with the three-way key `LP | BP | HP` beside them, `HIT` to audition that voice,
and `SHUFFLE` set apart from the seven because it is the device's swing. That voice's name is printed above the dials. The
grid's sixteen rows are `KICK`, `SNR`, `RIM`, `CLAP`, `CHH`, `OHH`, `LOT`, `MID`, `HTOM`, `CRS`, `RDE`, `CWB`, `LCO`, `HCO`,
`CLV`, `SHK`, one per voice top to bottom, and one cell per step: a click strikes, `SHIFT`-click accents, a click on the
label selects the voice and `SHIFT`-click on the label mutes it. The letter that presses a control is underlined on it — `H`
on `HIT`, `L`, `B` and `P` under the filter's sides — and each of those nine controls explains itself on hover. `OCT`, `CLEAR`,
`RANDOM` and the density and scale dials are not on that face, `STEP` lights and writes nothing there, and the machine takes no
notes this round.

**The clipboard's notices**, in the same corridor as the name: `copied A1`, `pasted A1 into A2`, `the clipboard has no pattern`.

**The settings card.**

| Word | Meaning |
|---|---|
| `TOOLTIPS ON` / `TOOLTIPS OFF` | hover help |
| `AUTOSAVE ON` / `AUTOSAVE OFF` | whether changes are written without `SAVE` |
| `MIDI IN  [+]` / `[-]` | the input list, folded / unfolded |
| `SOUND OUT  [+]` / `[-]` | the output list, folded / unfolded |
| `IN USE: <name>` | the device in use, above its list |
| `IN USE: none` | no MIDI input is open |
| `MIDI: <name>` | an input port is open |
| `MIDI: none (retrying)` | no port yet, still trying |
| `MIDI: none (gave up)` | no port, and it has stopped — `RETRY` appears beside it |
| `RETRY` | try the MIDI input again |
| `48 kHz - <device>` | the stream is healthy: rate and device name |
| `failed: <reason>` | the stream is not running, and why |
| `CLOSE` | put the card away |

**The kit browser:** `KITS`, `FACTORY`, `MY KITS`, `IMPORT`, `CLOSE`, each of the three kits that ship marked `- READ ONLY`,
and the row being asked about on `<name> - click again to replace`.

**The bank browser:** `BANKS`, `DEMO BANKS`, `PATTERN BANKS`, `SONGS`, `RACKS`, `SAVE`, `SAVE A COPY`, `SAVE BANK`, `SAVE RACK`,
`LOAD RACK`, `NEW`, `IMPORT`, `EXPORT`, `CLOSE`, and the `x` / `SURE` on each of your own rows, of a song and of a bank.

**The help card** has four sections — `STEPS`, `KEYBOARD`, `MIDI`, `NOTES`.

## The files

| Path | What it is |
|---|---|
| `$TAKKA_ACID_DATA_DIR` | the whole data directory, when that variable is set |
| `$XDG_DATA_HOME/takka-acid` | otherwise, this |
| `~/.local/share/takka-acid` | or this, when `XDG_DATA_HOME` is unset too |
| `state.json` | the session: **every device** — its kind, name, colour, ports, channel, patterns, patch, kit, swing and learned controllers — plus the rack's tempo, the panel's switches and the MIDI choice. Device 1 is also written in the old flat fields, so an older build still opens it |
| `songs/<name>.json` | one file per named song; the name is the file name |
| `banks/<name>.json` | one file per saved pattern bank: its sixty-four patterns and the machine it was written from. That kind is a warning, not a check — a bank from another machine loads and plays nothing, and the status line says so |
| `kits/<name>/` | a kit: its `kit.json` and one `<VOICE>.wav` per sampled voice. The three that ship are compiled into the app and have no folder here; a kit is written when a session that plays an edited one is saved |

Song and bank names are trimmed, limited to 48 characters, and may not contain `/ \ : * ? " < > |` or start with a dot.

## The one flag

`takka-acid --midi-map` prints the note map — one row per note number, 21 to 108, with the keyboard's name for it, the
note the app writes and shows, and the frequency it plays — then exits. No window, no audio device.
[`17-trouble`](17-trouble.md) says when to reach for it.
