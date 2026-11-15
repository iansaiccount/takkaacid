# Plan 29 — the whole-branch review, and what came back

A reviewer read the whole branch (`8bc1f19..de28af8`, 130 commits, ~31.8k lines) in three passes. Verdict: **with
fixes** — architecture sound (atomics-only audio hand-off, a correct SPSC ring, MIDI on its own thread, a versioned and
idempotent save migration), one critical defect and two important ones. This plan is the fix round.

## Fixed

**Critical — the step clock was never restarted.** `Sequencer::start` reset the position but not the clock, and stopping
only stops rendering, so the clock stayed frozen mid-step. Press RUN after STOP and the downbeat lasted whatever
fraction of a step was left — 15 ms instead of 115 ms — and with swing the bar could start on the short half of the pair.
`Clock::restart()` now puts the clock on step 0 with a whole step in hand, and `start` calls it.
Tests: `a_restart_hands_the_downbeat_a_whole_step_however_late_it_was_stopped`, `a_restart_at_full_swing_starts_on_the_long_step`.

**Important — a short bank panicked on load.** `PatternBank::get` clamped against the catalogue but not against the
*stored* length, so a truncated `state.json` aborted the app at startup; `sanitize` could not repair it because it never
looked at the length, and `load_song` did not sanitise at all. Added `PatternBank::normalise()` (pad/truncate to 64 and
clamp the selection to a pattern that exists), a panic-proof `get` with an empty-pattern fallback, `normalise` from
`sanitize`, and `sanitize` from `load_song`.
Tests: `a_truncated_bank_is_repaired_rather_than_panicking`, `a_bank_with_no_patterns_at_all_reads_silence_instead_of_panicking`.

**Important — MIDI velocity was parsed and then thrown away.** The audition played at full level while the step it wrote
carried the struck velocity, so a soft key sounded loud and then played back 12 dB quieter. `SharedState::midi_velocity`
carries it, `Engine::note_at(hz, accent, velocity)` plays it, and the renderer passes it through; `note_on` keeps its
full-velocity meaning for the on-screen keyboard, which has no velocity to offer.
Test: `a_soft_midi_note_auditions_at_the_velocity_it_will_be_written_with`.

## Also fixed (minor)

- `midi_gate` is stored with `Release` and read with `Acquire`; `Relaxed` on both sides let the audio thread start the
  *previous* note's pitch for one block.
- `tests/panel_layout.rs` learned about the performance row — six bands now, not five, and its cells are checked
  against the neighbours it can actually reach.
- `screw_positions` scales its inset with the panel, like the frame it decorates.
- A song now restores the TIPS switch, which it had been writing into the file and never reading back.
- `ACCENT_GAIN`'s doc said +6 dB over a value of 1.6 (≈ +4.1 dB); the doc now matches the number and the code comment.
- `filter.rs` claimed the coefficient is "never rebuilt per sample"; the voice rebuilds it every sample because the filter
  envelope is a per-sample signal. The doc now says so.
- `tests/no_alloc.rs` built its fixture `Arc` inside the measured window, so its `<= 2` allowance hid a real allocation.
  The fixture is moved out and the window now allows at most one, named below.

## Named rather than fixed

- **The audio thread can drop the last `Arc<Pattern>`** when it retires a pattern (`renderer.rs`). It normally frees
  nothing — the UI's bank holds the same pattern — but in principle it calls the allocator. The clean fix is to stop
  holding an `Arc` on the audio side and copy the sixteen steps, which changes `Engine::set_pattern` and
  `Sequencer::set_pattern` signatures. Documented in `renderer.rs`; queued as its own piece of work.
- **`AGENTS.md` at the repo root is a stale copy of another project's notes** (pygame/lean-ctx). Its two live rules —
  no glyph faces, ASCII legends — are honoured, but it is not a description of this repository. Rewriting it is a
  documentation task of its own.

## Verification

`cargo test -p acid-core` (32 suites, exit 0), `cargo test -p acid-app` (exit 0), `cargo clippy --workspace --all-targets
-- -D warnings` (exit 0), `cargo fmt --check` (clean).
