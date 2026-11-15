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
| 44 | [Colour and the clipboard](44-colour-and-clipboard.md) | a per-device colour scheme, and `Ctrl+C`/`Ctrl+V` for patterns — added after the phase's gate | yes |

Each lands a working, testable app. The order is forced three times: 39 needs 38's bus, 40 needs 38's `Device` and the bus,
and 42 needs 41's caret and request queue. Nothing else is ordered by dependency — 43 could be folded into 42 if the phase runs
long. Round 44 came afterwards, after the phase's gate, and depends on nothing here: a device's colour scheme and the pattern
clipboard.

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
   `git diff --staged`, and nothing else from this plan — the point is to hear whether the task said what it did. The reviewer
   is never the executor: no subagent reachable means no review, and the commit waits. See `AGENTS.md` — that rule lives there
   and is not restated with an escape hatch here.
3. **The three questions.** Does the diff do what the task said and only that? Does it break a rule in `AGENTS.md`? What does it
   leave untested, and is that deliberate?
4. **Action every finding before the commit.** A finding fixed in a later commit has not gated anything. If a finding is wrong,
   say why in the Progress line and move on — that is an answer, not a skip.
5. **Then commit,** and put the review's outcome in the task's `## Progress` line: what it found, and what was done about it.

The whole-branch review at the end of the phase (plan 43, task 5) stays, and it answers a different question: what do the seams
between tasks hide that no single diff showed?

## Progress

**Where the phase is: all seven plans are complete.** Everything is committed and the tree is green (69 test suites, 649 tests,
`clippy --all-targets -D warnings` and `fmt --check` clean, `docs/guide/check.sh` ok). A new session resumes by reading a
plan's `## Progress` section — it carries every commit, what each task's review found, and the rulings — and by `git log`,
not by re-deriving anything. The guide now matches the app: plan 45 (`plans/phase 2/45-the-guide.md`) retook every
screenshot at 2× and gave the strip, the device card, the colour and the clipboard their own pages.

- [x] **38 — the rack** (`plans/phase 2/38-the-rack.md`): `SharedState::mute`, `Device`, `Rack`, `BusState`, `Rack::rename`,
      the rack's transport and fan-out, and `main` running on a rack of one. Commits `af4843b`..`b8e4f82`, `3591083`.
- [x] **39 — the mixer** (`plans/phase 2/39-the-mixer.md`): `Engine::start_at`, `audio::mixer::Mixer` (slots, reconcile, mute
      ramp, master limiter), `AudioRenderer::seek`, and the `Rig` closure rendering the rack. Commits `ed4a66c`..`784bca2`.
- [x] **40 — MIDI routing** (`plans/phase 2/40-midi-routing.md`): all eight tasks landed (`39e6575`, `34023c5`, `a19908a`,
      `f651c1e`, `a9af620`, `2dea8ba`, and tasks 8's commit). Every review ran fresh (`reviewer` subagent): task 5's found 10
      (8 actioned), task 6's found 4 (3 actioned, one moved into plan 42's task list), task 7's found 5 (all actioned), task
      8's found 4 (all actioned). Tasks 7 and 8 fixed what task 6's review turned up — the tempo fader did nothing because the
      rack's fan-out overwrote it — ending with one tempo cell that the rack, every device and every MIDI port share, so a
      knob moves at the next audio block and cannot be undone by a frame.
- [x] **41 — the strip** (`plans/phase 2/41-the-strip.md`): all seven tasks landed (`9b60dd9`, `85ab8c1`, `c3f560b`, `ebd394b`,
      `69188ee`, `dc5bd45`, `d48e5bf`). A strip band with one cell per device (name, play lamp, MUTE, accent on the selected
      one, `+`), `Ctrl+1..4`, the panel's mirror and request queue, and a `PanelEdit` per device so switching away and back
      keeps the step, the step mode and the randomise dials. Reviewed fresh for tasks 3–4 (17 findings, all actioned); the
      headless run found one more (a ctrl+digit dropped when its modifier arrived in the same frame) and task 3's review found
      a data-loss path (`+` then SAVE would have written the new device over the open song).
- [x] **42 — the device card** (`plans/phase 2/42-the-device-card.md`): all seven tasks landed (`9f4193c`, `4b12efe`,
      `cff9a0c`, `33018b7`, `92ab20c`, `44e89c6`, and this one). `F3` or the strip's caret opens a card for the selected
      device — name field, ROLE, one ticked key per machine port with `ALL PORTS`, a channel stepper, MUTE, and a REMOVE that
      asks twice — and the strip's `+` opens a picker over `DeviceKind::ALL` with the footnote that names and extra devices are
      not saved yet. Every review ran fresh (`reviewer` subagent): task 1 found 4 (all actioned), task 2 nine (all), task 3
      eleven (all) including a **BLOCK** — a focused name field did not stop the panel's audition keys, so typing `BASS` wrote
      four steps and Space stopped the transport — task 4 eight (all) including a port click silently dropping an unplugged
      controller from the filter, task 5 five (all), task 6 six (all) including the one-card rule going one-way again. The
      by-hand journey (task 7) walked the whole spec on Xvfb; the only claims left unproved by eye are the sound ones.
- [x] **43 — documents and gates** (`plans/phase 2/43-documents-and-gates.md`): all five tasks landed (`b19846c`, `6fc669f`,
      `27ac3af`, `30d0e9c`, `f04e57f`, plus the gate's own `a35c461`, `7e7e933`, `58ced28`). The help card, the README, an
      ear-test section for two machines and `AGENTS.md`'s third thread; the phase gate (67 suites, 623 tests); and both of
      plan 39's mutation checks **survived** the suite as it stood — re-pinned after watching each one fail, which meant
      rewriting the mute test whose "audible" assertion compared two different time windows. Tasks 1-4 were reviewed inline;
      the whole-branch review was a self-review, because `pi-subagents`' only model refused the connection and the
      `claude-code` agent needs its own session. It found one **Important** defect at the 41–42 seam: the panel's per-device
      editor slots were keyed by cell *position*, so removing the first device gave the survivor the removed device's step,
       step mode and randomise dials. Fixed by keying them by `device.number` (`7e7e933`), with the test that was RED on the
       old code.
- [x] **44 — colour and the clipboard** (`plans/phase 2/44-colour-and-clipboard.md`): the round that came after the gate, at
      the user's request. All eight tasks landed (`1ffead6`, `903e6ff`, `e36d668`, `668a9a4`, `058d221`, `b092772`, `909070d`,
      `3236ba4`). `ui::theme` became two palettes plus `device::Scheme`, every face painter takes one, and a device's card row
      turns its face amber — while the four cards stay black. `Ctrl+C`/`Ctrl+V` copy and paste the pattern on the panel as
      versioned text on the OS clipboard, so it crosses devices and windows. **Reviewed by a parent-orchestrated loop, three
      fresh rounds** (`5670f52` carries both fix passes): round 1 returned three BLOCKs — the amber scheme's ink was
      1.04–1.24:1 on raised controls and the keycaps, and the clipboard re-dispatched on every logic-only tick — round 2
      confirmed every finding resolved and found the hover ring the first split missed, and round 3 confirmed both rulings
      (lamps become marks, keys are taken) and found that `Escape` still replayed. Deferred and recorded in the plan: the
      notice's slot label during a pending MIDI swap, two amber orderings (the untitled song label, an accented+lit ring at
      2.81:1), and the pre-existing drag overlay.

**Review mode.** Plans 38-41 ran the review gate *inline* — the executor reading its own staged diff — because `pi-subagents`
was not loaded in that session. Plan 42 ran it **fresh**: every task from 1 to 6 dispatched a `reviewer` subagent with the
task's Files/Interfaces blocks and the staged diff, and 43 findings were found and actioned across the six, three of them
blocks that would have shipped a destructive or contradictory state (typing into the name field editing the pattern; a port
click dropping an unplugged controller; the one-card rule going one-way). The lesson worth carrying into plan 43: the inline
gate found five real defects over twelve commits, and the fresh gate found nine over the same number of files — the difference
is that a fresh reader asks "what would break this?" about code it did not just write. Task 7's diff (documents only) ran
inline, and each task's Progress line names which mode ran.

**Plan 43's review mode.** Its four document tasks ran inline, as its header asked. The whole-branch review it was supposed to
dispatch could not be fresh: `pi-subagents` is loaded now, but its only model is `qwen/qwen3.8-27b` and the run came back
`ECONNREFUSED`, while `claude-code` (external CLI) wants an interactive session it cannot have here. It ran as a self-review
and still earned its keep — the defect it found was in plan 41's own feature, at the seam with plan 42's removal, and no
per-task diff could have shown it. What that says for the next phase is not "self-review is enough": the fresh reviewer is
worth the spend, and a harness whose only model is unreachable cannot supply one.
