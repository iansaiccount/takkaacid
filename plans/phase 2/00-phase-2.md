# Phase 2 — a device list, and the first shape of a DAW

> **For agentic workers:** read this overview first, then take the plans in order. Each plan is one round of work with its
> own `## Progress` section; do not start plan 39 before 38's gates are green.

**Design:** `docs/superpowers/specs/2026-11-13-device-list-design.md` — every plan below argues from it. Read the section
named in each plan's **Spec** line before starting that plan, not all of it at once.

**What the phase delivers:** two (or four) acid machines in one window, one transport over all of them, one sound card, each
with its own pattern bank, its own shuffle, its own inputs and its own name — and a panel that displays one at a time with a
strip of the rest.

**Why it is six plans and not one:**

| # | Plan | What it lands | App runs after it? |
|---|---|---|---|
| 38 | [The rack](38-the-rack.md) | `device::Device`, `device::Rack`, the mute flag, the bus, rack-level transport | yes, one device, unchanged |
| 39 | [The mixer](39-the-mixer.md) | `audio::mixer::Mixer`, `Engine::start_at`, join-in-phase | yes, one device, unchanged |
| 40 | [MIDI routing](40-midi-routing.md) | `midi::route`, channels on events, many ports, per-device filters | yes, device 1 hears everything |
| 41 | [The strip](41-the-strip.md) | the strip band, selection, `PanelEdit`, `Ctrl+1..4`, MUTE | yes, four devices, editable one at a time |
| 42 | [The device card](42-the-device-card.md) | the device card, the picker, names, routing UI | yes, fully usable |
| 43 | [Documents and gates](43-documents-and-gates.md) | help, README, the manual listen, the phase gate | yes, documented |

Each lands a working, testable app. The order is forced three times: 39 needs 38's bus, 40 needs 38's bus and 39's
Each lands a working, testable app. The order is forced three times: 39 needs 38's bus, 40 needs 38's `Device` and the bus,
and 42 needs 41's caret and request queue. Nothing else is ordered by dependency — 43 could be folded into 42 if the phase runs
long.

## Invariants that hold in every plan

These are not repeated per plan; they are assumed.

- **The three gates are green at every commit, and every commit is reviewed first.** `cd rust && cargo test --workspace &&
  cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check`, then the review gate above, then the commit. Not
  the other way round.
- **Device 1 behaves exactly as today.** One device, no strip interaction, is the regression every plan re-proves. The
  cheapest expression of this is that the factory rack is indistinguishable from the app before phase 2 in sound, timing and
  saved bytes.
- **`tests/persist.rs` keeps pinning the same bytes.** `SavedState` does not move in phase 2 — no new field, no reordering,
  `SAVE_VERSION` stays 9. A plan that finds it needs a field there has drifted from the design; stop and re-read the spec's
  "What is deliberately not saved".
- **`draw_panel(&mut PanelState, &SharedState, &mut egui::Ui)` keeps its signature.** `shared` is the *selected* device's
  state and the rack is a mirror on `PanelState`. This is what keeps the twenty-four existing `tests/*_live.rs` compilable
  through a change this size; a plan that wants a new argument is choosing to rewrite them.
- **No `_` arm on a catalogue enum** (`DeviceKind`, `Role`, `Channel`, `Ports`, `CcTarget`, `Knob`, `Shortcut`,
  `device::Request`). A new variant must fail to compile where a decision is owed.
- **No font glyphs on the face.** The strip's lamp, its `+` and the caret are painter shapes; every word is ASCII from the
  bundled font.
- **Two threads, two rules.** `BusState` is published by `ArcSwap` and read by the audio callback and the MIDI handlers.
  Neither allocates or locks in steady state; the mixer's reconcile allocates when the device list changes, and that
  exception is named in the code the way `audio/renderer.rs` already names its `Arc` retirement.
- **A panel feature is tested through `draw_panel`** with real egui frames in `tests/*_live.rs`, never through a private
  helper, and a widget's `egui::Id` is part of its behaviour.
- **One catalogue per kind of control.** The strip's cells, the picker's rows and the card's rows each derive their count
  and order from a catalogue or from the rack, never from a hand-written second list.

## The seam, drawn once

```
main (UI thread)                          audio callback
─────────────                             ──────────────
Rack                                      Mixer
 ├ devices: Vec<Device>                    ├ slots: Vec<Slot>  ── reconcile ──┐
 │   ├ state: Arc<SharedState> ────────────┼─ renderer: AudioRenderer        │
 │   ├ bank: PatternBank                   │    └ Engine (own clock, swing)  │
 │   ├ inputs: InputFilter ────────────────┼─ Slot::mute: param::Smooth      │
 │   ├ number, name                        ├ scratch, master: Limiter        │
 ├ selected, next_number                   └ upsert/despawn per published list
 ├ playing: AtomicBool      ── publish ─→ BusState { slots, routes }
 ├ tempo_bpm: Arc<AtomicF32>                    ↑
 └ bus: Arc<ArcSwap<BusState>>                  │ ArcSwap::load per message
                                          MIDI handlers (midir thread)
                                           └ Router { port, follow, gate, tempo }
```


Three readers of `BusState` — the mixer once per block, each port's handler once per message — and one writer, `Rack`.


## The review gate

Every plan in this phase runs the same gate, and every task's last step is it. It is not restated per task — a plan that ends a
task with a bare commit is wrong. Do not skip it because a task looks small; the small ones are where a convention slips.

1. **Gates green.** `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
2. **Review the diff.** Dispatch a fresh reviewer subagent with the task's **Files** and **Interfaces** blocks and the output of
   `git diff --staged`, and nothing else from this plan — the point is to hear whether the task said what it did. Where the
   harness exposes no subagent tool, do the same review yourself: read the staged diff top to bottom and answer the three
   questions as if it were someone else's work.
3. **The three questions.** Does the diff do what the task said and only that? Does it break a rule in `AGENTS.md`? What does it
   leave untested, and is that deliberate?
4. **Action every finding before the commit.** A finding fixed in a later commit has not gated anything. If a finding is wrong,
   say why in the Progress line and move on — that is an answer, not a skip.
5. **Then commit,** and put the review's outcome in the task's `## Progress` line: what it found, and what was done about it.

The whole-branch review at the end of the phase (plan 43, task 5) stays, and it answers a different question: what do the seams
between tasks hide that no single diff showed?

## Progress

**Where the phase is: plans 38 and 39 are complete, plan 40 is 5 of 6.** Everything is committed and the tree is green
(64 test suites, `clippy --all-targets -D warnings` and `fmt --check` clean). A new session resumes by reading the plan named
below — its `## Progress` section carries every commit, what each task's review found, and the rulings — and by `git log`, not
by re-deriving anything.

- [x] **38 — the rack** (`plans/phase 2/38-the-rack.md`): `SharedState::mute`, `Device`, `Rack`, `BusState`, `Rack::rename`,
      the rack's transport and fan-out, and `main` running on a rack of one. Commits `af4843b`..`b8e4f82`, `3591083`.
- [x] **39 — the mixer** (`plans/phase 2/39-the-mixer.md`): `Engine::start_at`, `audio::mixer::Mixer` (slots, reconcile, mute
      ramp, master limiter), `AudioRenderer::seek`, and the `Rig` closure rendering the rack. Commits `ed4a66c`..`784bca2`.
- [ ] **40 — MIDI routing** (`plans/phase 2/40-midi-routing.md`): tasks 1–5 landed (`39e6575`, `34023c5`, `a19908a`, and task
      5's commit); **task 6 (wire the router, delete `main`'s inline handler) is next**. Task 5's review ran fresh (`reviewer`
      subagent, 10 findings, 8 actioned), and its rulings are in that plan's Progress.
- [ ] **41 — the strip** (`plans/phase 2/41-the-strip.md`)
- [ ] **42 — the device card** (`plans/phase 2/42-the-device-card.md`)
- [ ] **43 — documents and gates** (`plans/phase 2/43-documents-and-gates.md`)

**Review mode.** The twelve commits so far ran the review gate *inline* — the executor reading its own staged diff — because
`pi-subagents` was not loaded in that session, so no `subagent` tool could be dispatched. It still caught five real defects
(a wrong doc comment, two transport-write bugs, a device whose engine had no pattern, and a mutation check that did not fire).
Every commit from here should dispatch a fresh reviewer if the tool is available; the plan's Review Focus sections are written
for exactly that brief. Record in each Progress line which mode the gate ran in.
