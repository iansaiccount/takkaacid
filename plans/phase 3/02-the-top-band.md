# Plan 02 — the top band, spread out

> **For agentic workers:** this is **round 3a** of phase 3: a design change to the band the deck already lives in, taken before
> task 4 of `01-the-tape-transport` finishes wiring. `superpowers:executing-plans`, reviewed by a dispatched subagent per commit.
>
> **Review:** a fresh subagent, never the executor. The screenshots are the part a compiler cannot check.

**Goal:** give the deck the width it deserves by moving the session's own cluster to the left and replacing the
`SAVED`/`UNSAVED` word with the song name's own styling.

## What is wrong today

The band was laid out for a wordmark, a saved-or-unsaved word, two help buttons, a long session name and two readout wells. The
deck arrived last and got the gap that was left: six 20-pixel keys in a 56-pixel band, the tempo `−`/`+` jammed against the
pattern well. A tape deck is the thing the hand goes to most on this face and it is the smallest thing on it.

The `UNSAVED` word is also a poor use of the space: it is two words' worth of silkscreen saying something the session name could
say by being drawn differently, and it costs the deck a key and a half.

## The design

### One left cluster, one spread

```
[◉][takka-acid]  SONG New song  [⚙][?]   [◀◀][▶][❚❚][■][▶▶][●]   − [ 120.0 ] +   [PATTERN A1]
└───────── session ─────────┘   └────────────── the deck ──────────────┘  └tempo┘  └ kept ┘
```

- The song name moves into the left cluster, after the wordmark; the gear and `?` follow it. The saved-or-unsaved **word is
  gone**.
- The deck takes the space the word and the name's tail used to hold, and its keys grow with it: from a fixed `0.36 × band
  height` to as wide as the band allows, capped so a key stays a key and not a slab.
- The pattern readout keeps the right wall and the tempo stepper stays beside it, with the number's well taking whatever the
  fixed-width keys leave.

### The session name carries its own state

There is no second word. The name is **always bold** and **highlighted while there are unsaved changes**:

| state | drawing | meaning |
|---|---|---|
| saved | the name in `silkscreen`, on the face | this session is on disk |
| unsaved | the name on a `panel_raised` plate in `led_accent`, the way a lit key looks | there are changes that are not |
| no name | `SONG` alone in `silkscreen_dim` | a session that has never been saved |

*"Bold" on this face means a plate and an accent, not a heavier typeface: the bundled font has one weight, so a bold word would
be a different size and read as a different thing. That is a ruling, not a detail — see the table at the end.*

The name is fitted to its corridor with `text::fit`, so a long name ends in the one ellipsis the font has, and the hover shows
**the whole name and its state**: `SONG A very long name indeed (unsaved)`.

### The dynamic tooltip

Every other tip on this face is static, from `ui::tips`. This one cannot be: it carries a name the player typed. It is drawn
with `Response::on_hover_text` at the call site, and the design says so, because "all help lives in one table" is a rule worth
naming the exception to rather than quietly breaking.

### Ledger: what the new rules cost

| Ruling | Cost if wrong |
|---|---|
| The saved word is replaced by the name's styling | A player who has learned "UNSAVED means at risk" has to learn a plate instead. The help card and page 04 must say it in words, and the tooltip says `(unsaved)` for the pointer. |
| The name's corridor is fixed and the name is fitted | A name longer than the corridor is cut on the face; the tooltip is the only place the whole name lives. That is the same trade the device name already makes. |
| The deck's keys grow to fill the band | Six keys at ~28 px are easier to hit but leave less air between them than the current 2 px; the key size is derived from the band, so a taller band grows the keys as well. |

## The tests that make it safe

Through `draw_panel`, with real frames:

1. `the_name_says_saved_or_unsaved_by_its_own_drawing` — clean: no plate behind the name and `silkscreen` ink; dirty: a
   `panel_raised` plate and `led_accent` ink. (Painted shapes, not a flag.)
2. `a_long_name_is_fitted_and_the_whole_one_is_in_the_tip` — a 60-character name ends in `…` on the face, and the tooltip text
   is the whole name plus `(unsaved)`.
3. `the_deck_takes_the_room_the_word_gave_up` — the six cells are wider than before against a named constant, still inside the
   band, still abutting, and the pattern readout still touches the right wall.
4. `the_top_band_has_no_saved_word` — the painted strings contain no `UNSAVED` and no `SAVED` (the removed word cannot come back
   silently).

Mutation checks: the plate removed from the dirty state, the fit removed from the name, and the deck's width capped back to the
old size — each must fail one of the four.

## Tasks

1. **The band re-derived.** One left cluster (volume, wordmark, name, gear, `?`), the deck spread across what is left, pattern at
   the wall, stepper beside it. Geometry only; no behaviour.
2. **The name.** Plate + accent while unsaved, plain while saved, `SONG` alone when there is no name; fit to the corridor; the
   dynamic tooltip with the whole name and its state.
3. **The deck grows.** Key width derived from the band rather than fixed, capped at a maximum, with the abutting gap kept.
4. **Documents.** Page 04 (the face), 19 (the card's neighbouring picture), 20 (reference) and `check.sh`'s facts; the top-band
   screenshots — `top-strip.png`, `first-sound.png`, `running.png` — retaken; the `UNSAVED` word removed from any prose that
   names it.
5. **The gate.** Three workspace commands, `check.sh`, the four mutations, and the round's review.

## Non-goals

- **A save button in the band.** `SAVE` is a switch in the row below, and stays there.
- **Renaming `PanelState::dirty`.** It is the flag the name's styling reads; the round renames nothing.
- **Moving the volume knob.** It stays first on the left, where the hand finds it.
- **A second line for the name.** One line, fitted, with the tooltip for the rest.
- **The bottom row.** That is task 5 of plan 01, untouched here.

## Progress

- [x] **Task 1 — the band re-derived.** One left cluster (volume, wordmark, session name, settings, help), the deck spread
      across the middle, the tempo stepper, the pattern readout at the wall. Geometry only.
- [x] **Task 2 — the name carries the state.** A raised plate in `led_accent` while there are unsaved changes, plain silk when
      written, `SONG` alone when never named; the name fitted and left-aligned in a fixed corridor; the whole name and its state
      in the tooltip — the one dynamic tip on the face, named as the exception.
- [x] **Task 3 — the deck grows.** Keys from a fixed 20 px to 23 px at the design size, capped so a key stays a key, with the
      gaps taking the slack.
- [ ] **Task 4 — the documents.** The two prose sentences about the name being "in the middle" are corrected. **The screenshots
      are not:** `top-strip.png`, `first-sound.png` and `running.png` still show the old band, and this round is not pushed
      until they are retaken with their `img/README.md` rows.
- [x] **Task 5 — the gate, partly.** 670 tests, clippy `-D warnings`, `fmt --check`, `check.sh` green. The four mutations are
      recorded below as what each test would catch, not yet run.

**Four bugs found by tests rather than by eye:** the deck ran into the tempo stepper (its right edge was measured to the pattern
readout, not the stepper's `−`); the cluster's buttons were measured at one scale and drawn at another (`help_rect` took the
scale, `tape_cells` measured it at 1.0); the cluster's gaps shrank with the panel while the name's corridor was a fraction, so
they collided at 500×340; and six abutting cells computed as `index * stride` need not agree about the edge they share in f32.

**Found by the screenshot, not by a test:** the band read `SONG SONG New song` (the helper already prefixes it — the first fix
exited before writing, so the screenshot was the second report of the same bug), the plate spanned the whole corridor rather than
the name, and the name was centred instead of left-aligned.

**Ruling, with its cost:** the name's corridor is a fixed fraction of the band, so a longer name is fitted with `…` and the
tooltip is the only place it is whole. The alternative — a corridor that grows with the name — moves six keys every time a song
is named, which is worse.

## The player's order, taken

The first build put the name before the buttons, which read as *logo, space, name, space, settings, help, deck* — two gaps
around a name and the deck crammed right. The player asked for **logo, settings, help, name, transport**, with no corner wider
than its contents. Taken:

- the wordmark's corner is now **its own width** (`BRAND_WIDTH`, a multiple of the band's height) instead of 40 % of the band, so
  the icon and the word sit where they end and the next thing follows;
- the order is wordmark → gear → help → name → deck, each hung off the one before it, so no gap can open between them;
- the name's corridor is narrower (10 % of the band) and left-aligned, and the deck takes everything from its right edge to the
  tempo stepper — which is where the width came from.

Measured at the design size: the deck's keys went from 20.2 px to **34.7 px**, and the gaps between them from 2 px to 10.3 px.

## A blocker for the screenshots, found while taking them

The capture recipe in `AGENTS.md` no longer produces a 1960 × 1340 window on this machine: with
`WINIT_X11_SCALE_FACTOR=2` set and a 2600 × 1800 Xvfb screen, the app opens at **1280 × 800** — `xdotool getwindowgeometry`
says so, and so does the grab. Every screenshot of this band is therefore of the wrong size and the wrong scale, and the recipe
needs correcting (the winit/eframe version in use does not honour that variable the way the note assumes) before task 4 can be
finished. The three images are **not** retaken, and this round stays unpushed until they are.

**The blocker, resolved.** The recipe was right and the *display* was wrong: something on this machine pre-starts `Xvfb` at
1280 × 800 on display numbers, and a request for a bigger screen on a taken display fails quietly ("Server is already active")
while the app and the grab both use the small one. `DISPLAY=:90 xdotool getdisplaygeometry` says `2000 1400`, the window comes
out 1960 × 1340, and the three pictures were retaken there. The trap is written into `AGENTS.md`'s capture recipe so the next
round checks the geometry before trusting a grab.

- [x] **Task 4 — the documents.** The three pictures retaken at 1960 × 1340 in the new order (`top-strip.png`,
      `first-sound.png`, `running.png`), their rows in `img/README.md` updated, and the capture trap recorded.
