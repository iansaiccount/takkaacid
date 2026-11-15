# Copying and pasting a pattern

`Ctrl+C` copies the pattern on the panel. `Ctrl+V` pastes it into the pattern on the panel.

![The status line saying "pasted A1 into A2"](img/clipboard-notice.png)

## What a copy takes

The sixteen steps as they are on screen, edits included, and the name of the slot they came from — so the session says
`copied A1` and nothing else changes.

## What a paste does

It replaces the pattern in the slot on the panel, in the device on the panel, and says what it replaced: `pasted A1 into A2`.

There is **no undo**, and that is on purpose: the pattern a paste replaces is the one you can see, and the notice names both
halves so you know what just went. A paste also makes an armed `CLEAR` stale, so the next click on `CLEAR` asks again rather
than emptying a pattern you have just filled.

The engine picks the new pattern up at its next block, so a paste lands while the line is running — no stop, no restart of the
bar.

## Where the pattern can go

Anywhere the panel can be:

- another slot in the same device — copy `A1`, `shift+right` to `A2`, paste;
- another device — `Ctrl+2` to put that device on the panel, then paste;
- **another window** — the clipboard is text on the operating system's, so a pattern copied in one session of the app can be
  pasted into another, and the two need not be on the same machine.

## What is on the clipboard

Text. A magic first line, then the pattern as JSON, which means a paste knows whether it is looking at one of ours: text from
anywhere else — a letter, a URL, an image — says `the clipboard has no pattern` rather than doing something surprising.

It also means the clipboard is the system's, not the session's: copying a pattern replaces whatever was on it, and nothing
about the clipboard is written to the session file.

## While a card is open

A card's name field keeps the clipboard for itself: with the cursor in it, `Ctrl+C` and `Ctrl+V` are the field's. Everywhere
else on the face, they are the pattern's.
