# Plan 33 — the demo songs, by the numbers and by ear

Rendered with `cargo run --release -p acid-app --example earcheck-demos`, which writes one take per demo to
`rust/earcheck/demos/` (16-bit mono PCM, 48 kHz, eight bars — one per pattern of the demo's bank). The numbers below are
from that run; the files are the ear test. **Nobody has listened to these yet** — that part is yours.

| Demo | File | Length | Peak | RMS | Brightness |
|---|---|---|---|---|---|
| Slow Acid | `slow-acid.wav` | 16.30 s | 0.513 | 0.0704 | 0.071 |
| Hard Techno Acid | `hard-techno-acid.wav` | 13.53 s | 0.485 | 0.0696 | 0.142 |
| Blues | `blues.wav` | 20.91 s | 0.175 | 0.0699 | 0.020 |
| Rock | `rock.wav` | 15.27 s | 0.170 | 0.0691 | 0.025 |
| Funk | `funk.wav` | 18.47 s | 0.597 | 0.0703 | 0.047 |

Every pattern in every bank sounds: the quietest per-pattern RMS is 0.0399 (`funk` A8, the sparse-reading variation) and
none is silent.

## Two things the render corrected

- **The level set was uneven by ~11 dB.** Blues and Rock measured RMS 0.20–0.23 against the acid demos' 0.05–0.06, because
  a high-resonance ladder loses passband gain (`DC gain ≈ 1/(1+k)`, the same effect `EAR_TESTING.md` documents for
  `08-resonance-0.95`). The arch patches now carry a trimmed master volume (acid 0.87/0.95, funk 0.88, blues 0.47, rock
  0.44), which brings all five inside 0.2 dB of each other. **Ruling:** the volume knob is part of a demo's patch, so a
  demo loads at the level it was mixed at. Cost if wrong: a demo arrives quieter than a factory session, and the player
  turns the knob up; nothing else is affected.
- **Peaks tell a second story worth a listen.** Blues and Rock peak around 0.17 because their lines are steady and
  unaccented; the acid and funk lines peak at 0.5–0.6 on their accents. If that reads as too polite on the square patches,
  the fix is `accent` (0.30/0.40) rather than the line.

## What to listen for, per demo

- **Slow Acid** — the classic reference. The slides on the off-beats (steps 4, 8, 12, 16) should be audible as a glide, not
  a new note; the accent on 1 and 9 should be brighter, not merely louder.
- **Hard Techno Acid** — a 16th chug. What matters is that the octave jump on 1, 5, 9, 13 reads as a jump and not as a
  glitch, and that the repeated E never turns into a drone.
- **Blues** — the shuffle comes from `swing: 0.35`, not from the notes. If it sounds square rather than swung, the swing
  value is the knob to turn (try 0.45). The walk (A A C C♯ D D♯ E D) should sound like a bassist walking, not like a scale.
- **Rock** — straight eighths, root and fifth. It should sound stubborn, not noodly; the slide on the last step should
  carry into the next bar's downbeat.
- **Funk** — the ghosts (velocity 60) should be audible as ghosted notes, not as a quieter repeat of the main line. If the
  line reads as busy mush at 104 BPM, the density is the thing to change (drop the ghost on step 13 first).

## Verdict

_(write here after listening — what works, what to change, and whether a demo needs its line, its patch or its volume
touched. The three levers are the note table in `demo.rs`, the patch numbers beside it, and the master volume that sets the
level of the set.)_
