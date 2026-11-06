# Phase 10 — MIDI in, clock sync and CC control

> Part of [00-main-plan.md](00-main-plan.md). Tasks **47–51**. Depends on: Phase 09.
> **Delivers:** play it from a keyboard, follow a DAW's clock, and move the seven knobs from a controller —
> and behave perfectly when no MIDI interface is attached, which is the common case.

**Rule for this phase:** nothing about MIDI may block, panic or take the audio thread with it. Messages
arrive on a MIDI thread; they write atomics only.

---

### Task 47: Port choice and connection

**Files:**
- Create: `rust/crates/acid-app/src/midi/mod.rs`, `rust/crates/acid-app/src/midi/ports.rs`
- Modify: `rust/crates/acid-app/src/lib.rs` (`pub mod midi;`), `Cargo.toml`
  (`cargo add -p acid-app midir`)
- Test: `rust/crates/acid-app/src/midi/ports.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `midir`.
- Produces:
  - `pub fn pick_port(names: &[String], wanted: Option<&str>) -> Option<usize>` — an exact match first,
    then a case-insensitive substring match, then `Some(0)` if `wanted` is `None` and ports exist.
  - `pub fn list_ports() -> anyhow::Result<Vec<String>>` — `Err` with the message
    `"no MIDI input ports found"` when the list is empty.
  - `pub struct MidiIn { connection: midir::MidiInputConnection<()> }` with
    `pub fn open(port_name: Option<&str>, handler: impl FnMut(u64, &[u8]) + Send + 'static)
    -> anyhow::Result<MidiIn>` and `pub fn port_name(&self) -> &str`.
- Later tasks rely on: `pick_port`, `MidiIn::open` (Task 48 wiring, Task 51 retry).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn an_exact_name_wins_over_a_substring() {
    let names = vec!["Arturia KeyStep".to_string(), "Launchkey MIDI".to_string()];
    assert_eq!(pick_port(&names, Some("Launchkey MIDI")), Some(1));
    assert_eq!(pick_port(&names, Some("keystep")), Some(0));
    assert_eq!(pick_port(&names, Some("ARTURIA")), Some(0));
}

#[test]
fn a_missing_name_and_no_ports_are_handled() {
    let names = vec!["Port A".to_string()];
    assert_eq!(pick_port(&names, Some("nope")), None);
    assert_eq!(pick_port(&[], None), None);
    assert_eq!(pick_port(&names, None), Some(0));
}

#[test]
fn no_ports_is_a_clear_error() {
    // list_ports() with an empty registry: the message must name the problem, not just fail.
    assert_eq!(empty_ports_error(), "no MIDI input ports found");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app midi`
Expected: FAIL — module `midi::ports` not found.

- [ ] **Step 3: Implement**

`pick_port` normalises with `to_lowercase()`. `list_ports` maps `midir::MidiInput::ports()` through
`port_name()`, filtering `None`. `empty_ports_error()` is a small helper returning the constant so the
test does not depend on a device being present.

- [ ] **Step 4: Run to confirm passing**

Run: `cd rust && cargo test -p acid-app midi`
Expected: PASS — 3 passed.
Run the app with and without a MIDI keyboard attached: with none, startup logs a single info line and
carries on; with one, the port name is printed.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): port enumeration and selection"
```

---

### Task 48: Message parsing and note handling

**Files:**
- Create: `rust/crates/acid-app/src/midi/message.rs`
- Modify: `rust/crates/acid-app/src/midi/mod.rs` (`pub mod message;`), `src/audio/renderer.rs`
  (honour `midi_hz` / `midi_gate`), `src/state.rs` (`midi_hz: AtomicF32`, `midi_gate: AtomicBool`,
  `midi_accent: AtomicBool`)
- Test: `rust/crates/acid-app/src/midi/message.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `acid_core::pitch::midi_to_hz`.
- Produces:
  - `pub enum MidiEvent { NoteOn { note: u8, velocity: u8 }, NoteOff { note: u8 },
    ControlChange { cc: u8, value: u8 }, Clock, Start, Stop, Continue }`
  - `pub fn parse(bytes: &[u8]) -> Option<MidiEvent>` — accepts 2- and 3-byte messages on channels 0–15,
    ignores anything else (including system-exclusive and real-time bytes other than clock/start/stop).
  - `pub const ACCENT_VELOCITY: u8 = 100;`
  - `pub fn is_accent(velocity: u8) -> bool` — `velocity >= ACCENT_VELOCITY`.
  - `pub fn note_hz(note: u8) -> f32` — `midi_to_hz(note as f32 - 12.0)` so a keyboard's middle C plays an
    octave below the panel's home register.
  - `pub fn handle(event: MidiEvent, state: &SharedState) -> bool` — `true` if it changed something.
    `NoteOn` with velocity 0 is a `NoteOff`.
- Later tasks rely on: `parse`, `handle` (Task 51).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn note_on_and_off_parse_on_any_channel() {
    assert_eq!(parse(&[0x90, 60, 100]), Some(MidiEvent::NoteOn { note: 60, velocity: 100 }));
    assert_eq!(parse(&[0x9F, 36, 1]), Some(MidiEvent::NoteOn { note: 36, velocity: 1 }));
    assert_eq!(parse(&[0x80, 60, 0]), Some(MidiEvent::NoteOff { note: 60 }));
    assert_eq!(parse(&[0x8A, 61, 64]), Some(MidiEvent::NoteOff { note: 61 }));
}

#[test]
fn control_clock_and_transport_parse() {
    assert_eq!(parse(&[0xB0, 74, 127]), Some(MidiEvent::ControlChange { cc: 74, value: 127 }));
    assert_eq!(parse(&[0xF8]), Some(MidiEvent::Clock));
    assert_eq!(parse(&[0xFA]), Some(MidiEvent::Start));
    assert_eq!(parse(&[0xFC]), Some(MidiEvent::Stop));
    assert_eq!(parse(&[0xFE]), None, "active sensing is not interesting");
}

#[test]
fn garbage_never_panics() {
    for a in 0u8..=255 {
        for b in 0u8..=255 {
            let _ = parse(&[a]);
            let _ = parse(&[a, b]);
            let _ = parse(&[a, b, 64]);
            let _ = parse(&[a, 0x90, b, 64]);
        }
    }
}

#[test]
fn velocity_accent_threshold_and_note_mapping() {
    assert!(!is_accent(99));
    assert!(is_accent(100));
    assert!(is_accent(127));
    assert!((note_hz(69) - 220.0).abs() < 0.5, "A4 should play as A3");
}

#[test]
fn a_note_event_reaches_the_shared_state() {
    let s = SharedState::new();
    assert!(handle(MidiEvent::NoteOn { note: 45, velocity: 120 }, &s));
    assert!(s.midi_gate.load(std::sync::atomic::Ordering::Relaxed));
    assert!(s.midi_accent.load(std::sync::atomic::Ordering::Relaxed));
    assert!((s.midi_hz.get() - note_hz(45)).abs() < 0.01);
    assert!(handle(MidiEvent::NoteOff { note: 45 }, &s));
    assert!(!s.midi_gate.load(std::sync::atomic::Ordering::Relaxed));
}

#[test]
fn note_on_with_zero_velocity_is_a_note_off() {
    let s = SharedState::new();
    handle(MidiEvent::NoteOn { note: 50, velocity: 100 }, &s);
    assert!(handle(MidiEvent::NoteOn { note: 50, velocity: 0 }, &s));
    assert!(!s.midi_gate.load(std::sync::atomic::Ordering::Relaxed));
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app midi::message`
Expected: FAIL — module `midi::message` not found.

- [ ] **Step 3: Implement**

Status byte high nibble: `0x9` note on, `0x8` note off, `0xB` CC; `0xF8` clock, `0xFA` start, `0xFB`
continue, `0xFC` stop. Reject other `0xF` bytes and any message whose length does not match its status.
In `AudioRenderer::render_into`, after `apply`, check `midi_gate`: on rising edge call
`engine.note_on(midi_hz, midi_accent)`; on falling edge `engine.note_off()`. A MIDI note takes priority
over the sequencer until it is released (document this in the help overlay, Phase 11 Task 54).

- [ ] **Step 4: Run to confirm passing, then play**

Run: `cd rust && cargo test -p acid-app midi`
Expected: PASS — 6 passed.
**Play:** connect a keyboard, press and hold notes — they sound at the right pitch, hard hits accent, and
releasing returns to the pattern.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): note, clock and cc message handling"
```

---

### Task 49: Following an external clock

**Files:**
- Create: `rust/crates/acid-app/src/midi/tempo.rs`
- Modify: `rust/crates/acid-app/src/midi/mod.rs` (`pub mod tempo;`), `src/state.rs`
  (`external_clock: AtomicBool`)
- Test: `rust/crates/acid-app/src/midi/tempo.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `acid_core::timing::{clamp_bpm, DEFAULT_BPM}`.
- Produces:
  - `pub const PPQN: u32 = 24;`
  - `pub fn bpm_from_interval(interval_ms: f32) -> Option<f32>` — `60_000 / (interval_ms · PPQN)`,
    returning `None` when the interval is outside `2 ms..=200 ms` (i.e. outside 12–1250 BPM at 24 ppqn).
  - `pub struct TempoFollower { last: Option<std::time::Instant>, ema_bpm: Option<f32>, ticks: u32 }`
    with `pub fn on_clock(&mut self, now: std::time::Instant) -> Option<f32>` — returns `Some(bpm)` once
    at least `8` intervals agree within 5 %, smoothed with an EMA (`alpha = 0.2`); `pub fn bpm(&self) ->
    Option<f32>`; `pub fn reset(&mut self)`.
  - `pub fn follow_clock(state: &SharedState, bpm: f32)` — writes `tempo_bpm` and sets
    `external_clock = true`.
- Later tasks rely on: nothing; the behaviour is what matters.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_24_ppqn_stream_at_20_8333_ms_is_120_bpm() {
    let bpm = bpm_from_interval(60_000.0 / (120.0 * 24.0)).unwrap();
    assert!((bpm - 120.0).abs() < 0.01);
}

#[test]
fn absurd_intervals_are_ignored() {
    assert_eq!(bpm_from_interval(0.5), None);
    assert_eq!(bpm_from_interval(5_000.0), None);
}

#[test]
fn the_follower_settles_on_a_steady_clock() {
    let mut f = TempoFollower::new();
    let step = Duration::from_secs_f64(60.0 / (128.0 * 24.0));
    let start = Instant::now();
    let mut reported = None;
    for i in 0..48 {
        reported = f.on_clock(start + step * i).or(reported);
    }
    let bpm = reported.expect("a steady clock must be followed");
    assert!((bpm - 128.0).abs() < 1.5, "followed {bpm}");
}

#[test]
fn a_single_stray_tick_does_not_change_tempo() {
    let mut f = TempoFollower::new();
    let step = Duration::from_secs_f64(60.0 / (130.0 * 24.0));
    let start = Instant::now();
    for i in 0..48 {
        f.on_clock(start + step * i);
    }
    let settled = f.bpm().unwrap();
    let stray = start + step * 49 + Duration::from_millis(15);
    let after = f.on_clock(stray);
    let after_bpm = after.unwrap_or(settled);
    assert!((after_bpm - settled).abs() < 5.0, "{settled} -> {after_bpm}");
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app midi::tempo`
Expected: FAIL — module `midi::tempo` not found.

- [ ] **Step 3: Implement**

Only feed the EMA when consecutive `bpm_from_interval` values agree within 5 %; otherwise decrement a
confidence counter and keep the previous value. `reset()` clears `last`, `ema_bpm` and `ticks`.

- [ ] **Step 4: Run to confirm passing, then sync**

Run: `cd rust && cargo test -p acid-app midi::tempo`
Expected: PASS — 4 passed.
**Play:** send clock from a DAW (or `jack`/`alsa` MIDI clock) at 124 BPM — within a bar the app's tempo
display reads 124 and the loop stays in time. Stop the clock and the app keeps its last tempo.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): follow an external 24-ppqn clock"
```

---

### Task 50: CC control of the seven knobs

**Files:**
- Create: `rust/crates/acid-app/src/midi/cc.rs`
- Modify: `rust/crates/acid-app/src/midi/mod.rs` (`pub mod cc;`)
- Test: `rust/crates/acid-app/src/midi/cc.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `SharedState` knob atomics.
- Produces:
  - `pub enum CcTarget { Tuning, Cutoff, Resonance, EnvMod, Decay, Accent, Volume, Tempo, Swing }`
  - `pub const CC_MAP: [(u8, CcTarget); 9]` — `3 → Tuning`, `74 → Cutoff`, `71 → Resonance`,
    `76 → EnvMod`, `75 → Decay`, `5 → Accent`, `7 → Volume`, `72 → Tempo` (mapped to
    `MIN_BPM..=MAX_BPM`), `73 → Swing`.
  - `pub fn cc_target(cc: u8) -> Option<CcTarget>`
  - `pub fn cc_value(knob_value: u8) -> f32` — `value as f32 / 127.0`.
  - `pub fn apply_cc(state: &SharedState, cc: u8, value: u8) -> bool` — `false` when unmapped.
- Later tasks rely on: nothing; `CC_MAP` is also printed by the help overlay (Phase 11 Task 54).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn every_mapped_cc_moves_its_own_knob() {
    let s = SharedState::new();
    assert!(apply_cc(&s, 74, 127));
    assert!((s.cutoff.get() - 1.0).abs() < 1e-6);
    assert!(apply_cc(&s, 71, 0));
    assert_eq!(s.resonance.get(), 0.0);
    assert!(apply_cc(&s, 75, 64));
    assert!((s.decay.get() - 64.0 / 127.0).abs() < 1e-6);
    assert!(apply_cc(&s, 7, 32));
    assert!((s.volume.get() - 32.0 / 127.0).abs() < 1e-6);
}

#[test]
fn tempo_cc_spans_the_whole_range() {
    let s = SharedState::new();
    apply_cc(&s, 72, 0);
    assert_eq!(s.tempo_bpm.get(), acid_core::timing::MIN_BPM);
    apply_cc(&s, 72, 127);
    assert_eq!(s.tempo_bpm.get(), acid_core::timing::MAX_BPM);
    apply_cc(&s, 72, 64);
    let mid = s.tempo_bpm.get();
    assert!(mid > 150.0 && mid < 190.0, "midpoint was {mid}");
}

#[test]
fn unmapped_cc_are_ignored_quietly() {
    let s = SharedState::new();
    assert!(!apply_cc(&s, 1, 100));
    assert_eq!(cc_target(1), None);
    assert_eq!(cc_target(74), Some(CcTarget::Cutoff));
    assert_eq!(CC_MAP.len(), 9);
}

#[test]
fn swing_cc_is_clamped_to_the_shuffle_range() {
    let s = SharedState::new();
    apply_cc(&s, 73, 127);
    assert!((s.swing.get() - 1.0).abs() < 1e-6);
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app midi::cc`
Expected: FAIL — module `midi::cc` not found.

- [ ] **Step 3: Implement**

`apply_cc` matches `cc_target`, then writes the appropriate atomic — tempo and swing use
`timing::clamp_bpm(MIN_BPM + knob · (MAX_BPM - MIN_BPM))` and `knob.clamp(0.0, 1.0)`. Log the mapping once
at startup at info level.

- [ ] **Step 4: Run to confirm passing, then wiggle**

Run: `cd rust && cargo test -p acid-app midi::cc`
Expected: PASS — 4 passed.
**Play:** map a controller's knobs to CC 74/71/75 and sweep them while the loop runs — the panel knobs
follow the hardware, with no fighting when the mouse is used afterwards.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): cc control of knobs, tempo and swing"
```

---

### Task 51: No device, unplugged device, and reconnection

**Files:**
- Create: `rust/crates/acid-app/src/midi/supervisor.rs`
- Modify: `rust/crates/acid-app/src/midi/mod.rs`, `src/main.rs`, `src/ui/mod.rs`
  (a one-line status in the top strip)
- Test: `rust/crates/acid-app/src/midi/supervisor.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Consumes: `MidiIn`, `Action`-style retry logic (same shape as `audio::supervisor`, Phase 02 Task 8).
- Produces:
  - `pub const MIDI_RETRY: std::time::Duration = std::time::Duration::from_secs(2);`
  - `pub const MIDI_MAX_RETRIES: u32 = 15;`
  - `pub enum MidiAction { Open, Wait, GiveUp }`
  - `pub struct MidiSupervisor` with `pub fn new() -> Self`,
    `pub fn on_error(&mut self, now: std::time::Instant) -> MidiAction`,
    `pub fn on_success(&mut self)`, `pub fn should_probe(&self, now: std::time::Instant) -> bool`.
  - `pub fn status_line(connected: Option<&str>, retries: u32) -> String` — `"MIDI: none (retrying)"`,
    `"MIDI: Launchkey MIDI"`, `"MIDI: none (press REPROBE)"` after giving up; always ASCII.
- Later tasks rely on: `status_line` (Phase 11 Task 55).

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn probing_waits_two_seconds_between_attempts() {
    let mut s = MidiSupervisor::new();
    let t0 = Instant::now();
    assert_eq!(s.on_error(t0), MidiAction::Wait);
    assert!(!s.should_probe(t0 + MIDI_RETRY / 2));
    assert!(s.should_probe(t0 + MIDI_RETRY));
}

#[test]
fn it_gives_up_after_fifteen_attempts_and_can_be_reset() {
    let mut s = MidiSupervisor::new();
    let t0 = Instant::now();
    let mut action = MidiAction::Wait;
    for i in 0..MIDI_MAX_RETRIES {
        action = s.on_error(t0 + MIDI_RETRY * (i + 1));
    }
    assert_eq!(action, MidiAction::GiveUp);
    s.on_success();
    assert_eq!(s.on_error(t0 + MIDI_RETRY * 40), MidiAction::Wait);
}

#[test]
fn status_lines_are_plain_ascii_and_name_the_state() {
    assert_eq!(status_line(Some("Launchkey MIDI"), 0), "MIDI: Launchkey MIDI");
    assert!(status_line(None, 1).contains("retrying"));
    let given_up = status_line(None, MIDI_MAX_RETRIES);
    assert!(given_up.contains("none"));
    for line in [status_line(None, 0), given_up] {
        assert!(line.is_ascii(), "{line}");
    }
}
```

- [ ] **Step 2: Run to confirm failure**

Run: `cd rust && cargo test -p acid-app midi::supervisor`
Expected: FAIL — module `midi::supervisor` not found.

- [ ] **Step 3: Implement**

Mirror `audio::supervisor`'s shape. `main.rs` probes MIDI from `update` when `should_probe` is true,
never blocking: enumeration happens on a `std::thread::spawn` and the result arrives on a channel. Draw
`status_line` in `SILKSCREEN_DIM` at the right of the top strip. Add a REPROBE click target (a small
painted dot) when the supervisor has given up.

- [ ] **Step 4: Run to confirm passing, then unplug**

Run: `cd rust && cargo test -p acid-app midi::supervisor`
Expected: PASS — 3 passed.
**Play:** start the app with no interface (status says retrying, everything else works), plug a keyboard
in — within 2 s the status names it and notes play. Unplug it mid-note — no stuck note, status returns to
retrying.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/acid-app
git commit -m "feat(midi): non-blocking probe, retry and reconnection"
```

---

**Phase 10 done when:** a keyboard plays the voice, a DAW clock sets the tempo, hardware knobs move the
panel, and the app is completely unaffected by the absence of MIDI.
