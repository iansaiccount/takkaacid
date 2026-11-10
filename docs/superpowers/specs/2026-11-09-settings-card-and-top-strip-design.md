# Settings card, and a quieter top strip

**Status:** awaiting approval. **Date:** 2026-11-09. **Branch:** `Deepseek-Version`.

## Intent

The top strip has accumulated switches and a live text line: the wordmark is crowded by `AUTO OFF` and `TIPS ON`, and two
device lines — MIDI at the top, audio at the bottom — both *display* a device and *act* as the picker for it. The player
wants one place for the settings they actually change, opened by a gear with a shortcut, and the strip to breathe, with
the song's name where the eye lands: the centre.

The `?` that opens help is also broken: when the song label was added, its painter got clipped to the status band, and that
clip landed on the help dot too. Putting the `?` back is part of this work.

## Goals

- One settings surface with four things in it: tooltips, autosave, MIDI in, sound out — with both device lists **inline**.
- A gear icon beside the `?`, and **F2** to open and close settings.
- Top strip: `AUTO`, `TIPS` and the MIDI status text gone; the song's name centred.
- Bottom: the audio status text gone.
- The `?` reads as a question mark again.

## Non-goals

- No change to the SONGS card, the save model or the song format.
- No new device backends: the same lists and the same choices, reached from a different surface.
- Settings are not a new file. Tooltips and autosave keep travelling in `SavedState`; the card adds no persisted state.

## Constraints

- **No font glyphs** (AGENTS.md): the gear is egui shapes, not `⚙`.
- The panel stays a pure painter with a request queue; the streams and their probes stay in `main`.
- Every removal must keep the picker behaviour: a choice writes `picker.input_choice` / `picker.output_choice`, and `main`
  acts on it next frame.
- The top band is 56 design px tall and 948 wide, and must hold: volume knob, wordmark with `?` and gear and the SAVED
  lamp, the centred song name, and the pattern and tempo displays.

## Approaches

**A. A settings card over the panel (picked).** The same shape as the help and songs cards: a slab that covers the panel,
dismissed by its own `CLOSE`, by Escape, or by a click outside it. The gear is a second small dot beside the `?`, so the
two round buttons read as one cluster: *help*, *settings*.

**B. A second window.** `egui` viewports could float settings as its own OS window. Rejected: the app is one surface by
design — a card over the panel, like help and songs, keeps that true, and a second window needs its own layout, focus and
dismissal rules for no gain.

**C. Keep the status lines, add only a small device popup.** Cheapest, and rejected: it leaves `AUTO`/`TIPS` crowded against
the logo and leaves the song's name with nowhere central to live. The request was de-cluttering, and this does not
de-clutter.

## Design

### The top strip after

```
 ◯ vol    ACID  (?)  (⚙)  ●          SONG Acid 1          B1·P3   ▮▮▮▮  137
```

- `panel::gear_rect(window)`: an 18 design-px dot, right of the `?` dot, on the same centre line — the pair sits inside the
  wordmark's corner, left of the SAVED lamp. Drawn as a ring, eight teeth as short radial strokes, and a hub, all in
  `theme::silkscreen_stroke`, so it reads as a gear at 120 px and at 200 px.
- The `?` dot keeps `help_dot`'s geometry and its plain `ui.painter()` — no clip rect. Song label and status text keep their
  own clipping; the help dot never had one.
- `panel::song_label_rect` moves to the strip's centre: from the wordmark's right edge to the pattern display's left edge,
  text centred. The name keeps the existing 18-character truncation and the existing dim-vs-bright rule (bright once the
  session is a file), so `SONG untitled` still says what it means.
- Removed: `ui::tips_rect`, `ui::autosave_rect`, the `AUTO` and `TIPS` keys, the top MIDI status text and its
  click-to-pick behaviour.

### The bottom

`main::ui` no longer paints an audio line, and the output picker that hung off it is gone. Nothing else lives down there;
the panel's own transport row is the last thing on the face.

### The settings card (`ui/settings.rs`)

```
SETTINGS                                            SOUND OUT: Speakers   ●
  TOOLTIPS      ON      AUTOSAVE     OFF
  MIDI IN
    Arturia KeyStep 37                            (current)
    IAC Driver Bus 1
    None
    RETRY   <- only while the supervisor has given up
  SOUND OUT
    Speakers
    HDMI / DisplayPort 2
```

- Toggle rows reuse `transport::transport_button`, lit when on, so they look like the panel's other switches.
- Device rows are the existing picker rows: the current one lit, a click asks for it (`Request::MidiIn(port)` /
  `Request::SoundOut(name)`), and `main` writes `picker.input_choice` / `picker.output_choice` the way it always has.
- The lists are filled when the card opens: `main` owns the port list and the supervisor, so `settings_open` rising is the
  refresh trigger, exactly as the pickers were opened before.
- The card carries the MIDI supervisor's state where the top line used to: the current port, `none (retrying)`, and
  `RETRY` while the supervisor has given up.
- Dismissal: `CLOSE`, Escape, a click outside. Opening settings closes help and the songs card; opening either of those
  closes settings. One card at a time.

### Keys

`F2` toggles settings. Escape closes whatever is open, in this order: settings, help, songs card, learn. `?` keeps opening
help. The help card's key list and the README's key table gain `F2`.

## What is left to the builder

- The gear's tooth count and stroke weight that still reads at the smallest scale (0.65).
- Whether the settings card needs a scroll region on a machine with many ports — it should, once there are more rows than
  the card height.
- The tests: a settings card opened by F2 and by the gear; each toggle and each device row asking for what it should; the
  song label centred and clear of the wordmark, the lamp and the displays; the `?` outside any clip; and the removals
  (the AUTO/TIPS key tests and the status-line picker tests go with the code).
