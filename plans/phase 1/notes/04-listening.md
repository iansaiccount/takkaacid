# Phase 04 listening — 2026-11-06

Verdict from the human partner, in their words: **"Sounds great."**

Heard via `cargo run --release -p acid-core --example earcheck`, which renders sixteen takes
(`01-default` … `16-everything-max`) through the same `Engine::render` path the test suite asserts
against — the full list and what each take is for is in [EAR_TESTING.md](../../EAR_TESTING.md).

No per-take notes were given, so this records the overall verdict rather than a pass on each file. The
implementer cannot hear output in this session; every numeric claim in the suite is an offline-render
measurement, and the taste judgement above is the human partner's.

Numeric context at the time of the verdict: 59 tests green across 19 suites,
`cargo clippy --workspace --all-targets -- -D warnings` clean, release build.

Observations to carry forward (none judged a defect, all documented in EAR_TESTING.md):

- Levels are pre-gain-staging: peaks ≈ 0.15–0.3, RMS ≈ 0.02 (−33 dBFS). Task 46 targets −14 dBFS.
- High resonance thins the bass (`DC gain ≈ 1/(1+k)` in an uncompensated ladder) — the 303's behaviour
  too; bass-boost compensation or a nonlinear ladder sits in the Phase 12 backlog.
- Only the 55 Hz root note exists until the sequencer lands in Phase 05, so pattern feel is untested by
  ear so far.

Phase 04 gate: met. Task 19 (launch defaults, readback and the integration test) is the last task in the
phase; the voice is declared done when its test is green.
