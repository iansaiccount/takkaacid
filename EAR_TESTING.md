# Ear testing takka-acid

The test suite pins numbers: RMS, peak, brightness, envelope timing, filter stability, block-size
independence. It cannot tell you whether the thing sounds like acid. That judgement is the **Listen:**
line that appears at the end of most tasks in `plans/`, and it is yours to make. This file is how to make
it without waiting for the panel to be finished.

Two routes: play the app, or render the voice offline to WAVs and A/B them in any player.

## Route 1 — the app

```bash
cd rust
cargo run --release -p acid-app      # or ./target/release/takka-acid
```

A near-black window opens with the `takka-acid` wordmark, a **RUN / STOP** button, and a status line
showing the audio device (`48000 Hz - Default Audio Device`). **RUN** holds a 55 Hz saw through the filter;
**STOP** fades and stops.

What you can judge here today:

- the attack is a pluck, not a flat drone
- the tone darkens as the note falls away (the filter envelope closing)
- STOP is clean — no click, no stuck note — and RUN retriggers cleanly
- the level is quiet but not distorted

What you **cannot** judge here yet: knob sweeps. The seven knobs are Phase 06 and their wiring is Phase 07;
`main.rs` currently has only RUN/STOP, and the voice runs at fixed launch defaults (cutoff 0.35 ≈ 390 Hz,
resonance 0.55, env mod 0.5, decay 0.35, accent 0.8). For a sweep today, use Route 2.

## Route 2 — offline WAVs (the real ear test)

```bash
cd rust
cargo run --release -p acid-core --example earcheck
```

It prints one line per take and writes 16 files to `rust/earcheck/*.wav` (16-bit mono PCM, 48 kHz).
Those files come from the same `Engine::render` path the tests use, so what you hear is exactly what the
assertions measure. The directory is git-ignored — delete it whenever you like.

```text
01-default                  2.25 s  peak 0.18  rms 0.023   launch patch
02-cutoff-0.10              ...    very dark
...
```

Read the printed `peak` / `rms` next to what you hear: if a take sounds louder but measures quieter than
its neighbour, the filter is doing something you have not accounted for — that is worth a note.

## What each take is for

| File | Listen for | A failure sounds like |
|---|---|---|
| `01-default` | the launch patch as a whole: pluck, sweep down, resonant, musical | a flat drone, or a click at the start |
| `02-cutoff-0.10` | very dark, almost a thumb on a bass string | silence, or a thin buzz with no body |
| `03-cutoff-0.35` | the default: squelchy, darkens as it falls | no audible movement across the note |
| `04-cutoff-0.60` | the saw's edge appearing; still not harsh | a fizzy top end that outshines the fundamental |
| `05-cutoff-0.90` | wide open and buzzy, but *clean* | gritty at the top, doubling of pitch (aliasing) |
| `06-resonance-0.20` | soft, no peak; the reference against 07/08 | a whistle that shouldn't be there |
| `07-resonance-0.60` | the 303 "ow": a clear resonant peak riding the sweep | the peak sounding detached from the note |
| `08-resonance-0.95` | loud, whistling at the edge, and — importantly — **no crackle** | crackle, or a runaway tone that never decays |
| `09-envmod-0.0` | a static filter: the sweep is gone, the note is a shape | movement where there should be none |
| `10-envmod-1.0` | the classic "bwaap": full sweep on every note | a sweep so fast it is a click |
| `11-decay-short` | plucky; the tail is gone quickly | a tail that outlasts the file |
| `12-decay-long` | resonance ringing on after the amp has fallen | ringing that never stops, or a dead tail |
| `13-accent-alternating` | accents clearly louder *and* brighter than plain notes | accents only louder, or not distinguishable |
| `14-slide` | 55 Hz gliding up to 110 Hz on one breath, no retrigger | a re-attack, a level jump, or no glide |
| `15-square` | hollow, more harmonics than the saw | a square that sounds identical to the saw |
| `16-everything-max` | max resonance + env mod + decay + accent + cutoff: extreme, loud, still finite | clipping, crackle, or a runaway feedback tone |

## What is true today but is not a bug

- **Everything is quiet** — peaks around 0.15–0.3, RMS around 0.02 (−33 dBFS). Gain staging to a −14 dBFS
  target is Task 46, not yet done. Turn your system volume up rather than concluding the voice is weak.
- **High resonance thins the bass** — an uncompensated ladder loses passband gain as feedback rises
  (`DC gain ≈ 1/(1+k)`), which is why `08-resonance-0.95` measures *quieter* than `06-resonance-0.20`
  even though its peak is stronger. That is the real 303's behaviour too; a bass-boost compensation or a
  nonlinear ladder is in the Phase 12 backlog.
- **One note only** — the sequencer arrives in Phase 05, so every take plays the same 55 Hz root. Pattern
  feel (groove, swing, accents in time) cannot be judged until then.
- **Knobs are fixed in the app** — Route 2 renders the same voice at different settings because it drives
  `Engine` directly; the UI controls arrive in Phase 06/07.

## Recording a verdict

The plan's phase gates ask for a listening note. Write one file per listening session, e.g.
`plans/notes/04-listening.md`:

```markdown
# Phase 04 listening — <date>

Takes heard: 01–16 (`cargo run --release -p acid-core --example earcheck`)

- Default patch: <what it sounds like, in your words>
- Sweep (02–05): <does the cutoff do what you expect? where does it start to fizz?>
- Resonance (06–08): <does the peak sing? any crackle at 0.95?>
- Envelope (09–12): <bwaap or thud? tail length sane?>
- Accent (13): <loud enough? bright enough?>
- Slide (14): <smooth? any click?>
- Square (15): <different enough from the saw?>
- Extremes (16): <anything that breaks up or explodes?>
- Verdict: <ship it / tune X / needs Y>

Numeric context: full suite N tests green, peak/RMS from the run above.
```

Then, if a take sounds wrong, the fix belongs in a **task**, not in a quiet edit: note it here, and either
add a task to `plans/04-filter-vca.md` (if it is this phase's business) or to
`plans/12-deferred-accuracy.md` under Backlog. Every numeric claim the suite makes should survive the fix —
if a change makes the sound better but a test fail, the test is telling you what else you changed.

## Coverage of the plan's Listen: lines

| Phase | Ear check | Status |
|---|---|---|
| 03 oscillator | saw vs square audibly different; tuning sweeps smoothly | covered by `15-square`, `04`–`05` |
| 04 filter/VCA | squelchy resonant bass that darkens as it decays | covered by `01`–`12` |
| 05 sequencer | loop is seamless at the wrap; accents land on the beat | **pending** — no sequencer yet |
| 07 wiring | knob moves heard immediately, no zipper noise | **pending** — no knobs yet |
| 09 master | drive dirties it without crackle; limiter inaudible | **pending** — no drive stage yet |
| 10 MIDI | keyboard plays the voice at the right pitch | **pending** — no MIDI yet |
| 11 release | ten-point release checklist in `plans/11-polish.md` | **pending** |

When a phase adds a Voice/Engine parameter, add a take for it in `takes()` — one entry, a hint, and a
`with_knobs(...)` call — so the ear check keeps pace with the tests.

## Adding or changing a take

`rust/crates/acid-core/examples/earcheck.rs` holds a `takes()` list. Each entry is a name, a one-line
hint, and a `fn() -> Engine` that configures the engine. Non-default gestures (alternating accents, a
slide) are dispatched by name inside `render_take`; add a `match` arm if you need a third kind.

Run it, listen, and if a take is misleading (wrong hint, wrong setting), fix the hint in the same commit —
a mislabelled WAV is worse than no WAV.
