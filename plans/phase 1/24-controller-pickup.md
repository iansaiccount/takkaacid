# Plan 24 — controller pickup (soft takeover)

A learned fader and the knob it drives are two positions of the same value, and they are almost never in the same place.
Today the first controller message wins: load a song whose cutoff is open, nudge a fader that happens to sit low, and the
filter slams shut. The knob "jumps", and the song that was loaded is no longer the song that plays.

The fix is the rule every hardware synth uses: **the controller has to reach the knob before it may move it**. Turn the
fader towards the knob, keep turning, and the moment the two meet the knob starts to follow. Until then the panel
ignores the fader — visibly, because the knob does not move.

## What it applies to

| Target | Pickup? | Why |
|---|---|---|
| Tuning, Cutoff, Resonance, Env Mod, Decay, Accent, Volume, Tempo | **yes** | continuous: a jump is a jump in sound |
| Waveform | no | four faces: the fader names a wave, and landing on it is the intent |
| Pattern | no | eight zones: the fader names a pattern, same reasoning |
| Swing | no | fixed controller, and a shuffle step is audible but tiny; leave it direct until someone asks |

## The rule

For one control, three numbers matter: what the controller last sent, what the knob is worth, and whether the controller
currently owns it.

- **Not yet engaged.** The controller takes over when its travel *crosses* the knob's value — the last value and this one
  fall on opposite sides of it — or when it is already within a hair of it (2.5 controller steps). Anything else is
  ignored.
- **Engaged.** The knob follows every message.
- **Engaged but the knob moved underneath it** (the hand dragged it, or a song loaded): the controller loses the knob and
  has to cross it again. Without this the takeover survives a song load, which is the bug being fixed.

A step, in order: read the state, decide the takeover, then write the knob and record `last = sent, engaged = decision`.

## Where the state lives

`SharedState::pickup: [Pickup; LEARNABLE.len()]`, one per learnable slot, each holding two atomics — the last controller
value (`u32::MAX` until it sends one, because `0.0` is a real value) and an engaged flag. Two atomics rather than one
packed word: the packing games needed to hide the flag inside an f32 are not worth the two bytes.

The decision itself is a pure function in `midi::cc` (`takeover(last, engaged, sent, current) -> bool`), so the whole
state machine is testable without a controller. The MIDI thread still writes only atomics.

## Tests (failing first)

| Test | What it pins |
|---|---|
| `cc::a_far_controller_does_not_move_the_knob` | first message at 1.0 with the knob at 0.2 leaves it at 0.2 |
| `cc::a_controller_that_reaches_the_knob_takes_over` | sweeping 0.0 → 1.0 past 0.2 moves the knob from the crossing on |
| `cc::a_taken_over_controller_follows_every_later_move` | after the crossing, a smaller move still applies |
| `cc::a_hand_on_the_knob_takes_it_back` | knob moved by hand after a takeover → the next controller move is ignored until it crosses again |
| `cc::the_switches_take_the_value_they_are_pointed_at` | waveform and pattern still apply on the first message |
| `cc::the_tempo_fader_has_to_reach_the_tempo_that_is_playing` | a low fader does not drop a running tempo |
| `pickup_live` (new, live) | with the panel drawing, a CC that is not picked up leaves the drawn knob where it was; after the sweep it follows |

Existing tests that assumed "first message wins" are rewritten to sweep into the value — the behaviour they asserted is
now the bug.

## Global constraints

- The MIDI thread writes atomics only; `tests/no_alloc` stays green.
- No new allocation per message; no locks.
- Every step ends green: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check`.

## Review focus

- The takeover must survive the knob being re-derived from shared state every frame (knobs 1..6 are read back each
  frame; the WAVEFORM and VOLUME faces are not) — the state lives beside the map, not in the panel, so it does.
- A song load moves every knob at once: every engaged controller must lose its knob on the next message. One test loads
  a song mid-test to prove it.
- Snapshots and the audio path must not learn about pickup: it is a MIDI-input policy, not a synthesis one.

## Result

Landed as planned. `SharedState::pickup: [Pickup; 10]` sits beside `cc_map`, and `apply_cc` runs the takeover before it
writes anything. Four tests that asserted "the first message wins" were rewritten to sweep into the value first — the
behaviour they pinned is the bug being fixed, and each rewrite says so in a comment.

What the build added beyond the plan:

- **A mapped-but-ignored message returns `true`.** `apply_cc`'s `bool` meant "is this controller mapped"; a fader that
  has not reached its knob is mapped and understood, just deliberately not obeyed. Returning `false` would have made the
  two cases indistinguishable to any caller that ever looks.
- **The tempo's "current" is a position, not a bpm.** The fader spans `MIN_BPM..=MAX_BPM`, so the knob it has to reach is
  `(bpm - MIN_BPM) / (MAX_BPM - MIN_BPM)`. Without that the comparison would be between a 0..1 controller and a 40..300
  number and every fader would be "far away" forever.
- **`knob_value` is the one place that mapping lives.** One `match` per target, exhaustively over `CcTarget`, so a new
  control cannot quietly join the pickup path with the wrong units.

Mutation checks: forcing the takeover off fails four tests including
`a_far_controller_does_not_move_the_knob`; letting an engaged controller keep the knob after the hand moves it fails
`a_hand_on_the_knob_takes_it_back` and `takeover_wants_a_crossing_or_a_near_miss`.
