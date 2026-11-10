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
| `Z` / `X` | octave down / up, two either way |
| `space` | start / stop, same as `RUN` / `STOP` |
| `left` / `right` | move the selection one step |
| `shift+left` / `shift+right` | previous / next pattern |
| `shift+up` / `shift+down` | note up / down a step |
| `up` / `down` | tempo, one BPM (and they repeat while held) |
| `enter` | note on or off |
| `backspace` | empty the step to a rest, while editing |
| `F1`, `shift+?`, or the `?` | the help card |
| `F2`, or the gear | the settings card |
| `escape` | close a card; then give up on a `LEARN` |

**The panel's own keys:** `RUN` `STOP` `TEMPO -` `TEMPO +` `LEARN` on the transport row; `CLEAR` `SAVE` `SONGS` `STEP` in
the switch row (`BANK` `PATTERN` `MODE` are engraved and do nothing yet); `OCT -` `OCT +` `RANDOM` on the performance row.

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

**The top strip** holds four things: the lamp inside the wordmark's corner, the song name in the middle, and the pattern
and tempo wells at the right.

| Word | Where | Meaning |
|---|---|---|
| `SAVED` / `UNSAVED` | by the wordmark | whether what is on screen is on disk |
| `SONG <name>` | the middle | the session's name; `SONG untitled` when it has none |
| a notice | the middle | something just happened — `saved Acid 2`, `saved a copy as Acid 2`, `could not import: …` — for a few seconds, then the name returns |
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

**The songs card:** `SONGS`, `DEMO SONGS`, `MY SONGS`, `SAVE`, `SAVE A COPY`, `NEW`, `IMPORT`, `EXPORT`, `CLOSE`, and the
`x` / `SURE` on each of your own rows.

**The help card** has four sections — `STEPS`, `KEYBOARD`, `MIDI`, `NOTES`.

## The files

| Path | What it is |
|---|---|
| `$TAKKA_ACID_DATA_DIR` | the whole data directory, when that variable is set |
| `$XDG_DATA_HOME/takka-acid` | otherwise, this |
| `~/.local/share/takka-acid` | or this, when `XDG_DATA_HOME` is unset too |
| `state.json` | the session: bank, patch, tempo, swing, switches, MIDI choice |
| `songs/<name>.json` | one file per named song; the name is the file name |

Song names are trimmed, limited to 48 characters, and may not contain `/ \ : * ? " < > |` or start with a dot.

## The one flag

`takka-acid --midi-map` prints the note map — one row per note number, 21 to 108, with the keyboard's name for it, the
note the app writes and shows, and the frequency it plays — then exits. No window, no audio device.
[`17-trouble`](17-trouble.md) says when to reach for it.
