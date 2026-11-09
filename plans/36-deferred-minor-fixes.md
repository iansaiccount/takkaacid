# The Minor List, Worked Through — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: `superpowers:executing-plans`. One green commit per task, on
> `Deepseek-Version`.

**Goal:** clear the Minor list plan 35 recorded and deliberately left standing — dead code, stale words, the label's
centre, long notices, the `README` sentence, `Request::Close`'s unreachable arm in `main`, the dim that only settings
painted, and device rows that run under the scrollbar.

**Architecture:** no new surface. Every fix either deletes something the removals left behind, moves one rule into the
module that owns it (`ui::text` for fitting a line, `ui::modal` for the scrim and the dismissal shield,
`settings::Action` for what a request means), or points a stale sentence at the thing that exists now.

**Spec:** `docs/superpowers/specs/2026-11-09-settings-card-and-top-strip-design.md` (round 35's product spec; this round
changes no product behaviour it promised).

## Global Constraints

- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`.
- No colour literal outside `ui/theme.rs`: the scrim becomes `theme::SHADE`.
- One card at a time, one modal idiom: help, songs and settings all dim the face and dismiss the same way after this.
- `SavedState` untouched, no `SAVE_VERSION` bump.
- The song label's rect stays inside the free interval between the wordmark and the pattern display — it may never
  touch the wordmark, the saves lamp, the gear, the `?`, the pattern display or the tempo display.

## Review Focus

- **A notice longer than the label rect:** the beginning of the message must survive; nothing may be clipped at both ends.
- **A notice exactly at the width:** no ellipsis, no clip.
- **A machine with twelve ports, the MIDI section open:** the far right of the panel is the scrollbar, not a row.
- **Hovering the settings card's two switches with help on and off.**
- **Closing the settings card:** `main` is never asked and holds no arm for it.
- **A very small window:** the symmetric label window must collapse rather than invert, and must not panic.

---

### Task 1: One scrim, one shield — the three cards dim and dismiss alike

**What is wrong:** `draw_settings` paints `Color32::from_black_alpha(90)` over the whole window and registers a
dismissal shield; help and songs register a shield and paint no scrim, so the three cards look like two different
idioms, and the literal lives outside `theme.rs`.

**Design:** `ui/modal.rs` — `pub fn shield(ui: &mut Ui, window: Rect, id: &str) -> Response`: paint `theme::SHADE` over
`window`, then register the click interaction with `ui.scope_id().with(id)`. Each card calls it *before* it paints its
own controls, because a later widget wins a hit-test tie (that is the bug plan 35 fixed once already).

**Files:**
- Create: `rust/crates/acid-app/src/ui/modal.rs`
- Modify: `rust/crates/acid-app/src/ui/theme.rs` (`SHADE`), `ui/mod.rs` (help, songs, settings all call `modal::shield`)
- Test: `rust/crates/acid-app/tests/modal_live.rs`

**Tests:**
- `every_card_dims_the_face_and_dismisses_on_a_click_outside` — for each of the three cards: open it, paint a frame,
  assert a `Shape::Rect` with `fill == theme::SHADE` covering the window; then click the window corner and assert the
  card closed.
- `help_still_closes_on_a_click_outside_while_settings_remains_one_at_a_time` — opening settings closes help, opening
  help closes settings (already true; pinned here from the card's own side).

- [ ] **Step 1: Write the failing tests** in `tests/modal_live.rs`.
- [ ] **Step 2: Run** `cargo test -p acid-app --test modal_live` — expect FAIL (module missing).
- [ ] **Step 3: Implement** `theme::SHADE`, `modal::shield`, and wire the three cards.
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `refactor(ui): one scrim and one shield for every card`.

### Task 2: A line of text fits the rect it is given

**What is wrong:** the song label and the notice share `song_label_rect`, and the rect is narrower than a long notice,
so `with_clip_rect` cuts the message at *both* ends — the player reads neither its beginning nor its end.

**Design:** `ui/text.rs`:
```rust
pub fn fit_with(text: &str, width: f32, width_of: impl Fn(&str) -> f32) -> String  // pure: cut + "…"
pub fn fit(painter: &Painter, text: &str, font: FontId, width: f32) -> String      // measured by the painter
```
`fit` measures with `Painter::layout_no_wrap`, so the cut lands where the text really stops. The panel keeps its clip
rect as belt and braces, and stops guessing: no second character cap.

**Files:** create `ui/text.rs`; modify `ui/mod.rs` (the label line) and `ui/panel.rs`/`ui/mod.rs`'s `song_label`.

**Tests:** unit tests in `ui/text.rs` with a fake measure (10 px a character): a short line is untouched; a long one
ends in `…` and measures inside the width; a width too small for even the ellipsis gives `""`; and a live test
`a_long_notice_keeps_its_beginning` (notice of 200 chars → painted text starts with the notice's first words, ends with
`…`, and is no wider than the label rect, measured with the same font).

- [ ] **Step 1: Write the failing tests**, including the live one in `tests/notice_live.rs`.
- [ ] **Step 2: Run** — expect FAIL (`ui::text` missing; the live test sees a clipped string).
- [ ] **Step 3: Implement** `ui/text.rs` and use it for the label line.
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `fix(panel): a long notice keeps its beginning instead of losing both ends`.

### Task 3: The name is centred on the strip, not on the gap

**What is wrong:** `song_label_rect` spans the whole free interval, so the text centres at the interval's midpoint —
**62 design px right of the strip's centre** at 980 wide, because the left cluster (volume, wordmark, lamp, gear, `?`)
is far wider than the right (pattern, tempo). The test hides it behind a 12 % tolerance.

**Design:** keep the interval as a bound, centre the rect: half-width = `min(centre - interval.left, interval.right -
centre)`, so the rect is symmetric about the strip's centre and still clear of every neighbour. If the centre falls
outside the interval (a window so small that the two sides meet), fall back to the interval itself. Text stays
`CENTER_CENTER` in the rect, so a fitted line clips symmetrically if it ever has to.

**Files:** modify `ui/panel.rs` (`song_label_rect`), `tests/notice_live.rs`.

**Tests:** rewrite `the_song_label_is_centred_in_the_strip` to assert the centre is within **1 px** of the strip's
centre and the rect is clear of the wordmark, the lamp, the gear, the `?`, the pattern display and the tempo display;
add `the_song_label_window_stays_symmetric` (distance to each neighbour equal within 1 px) and
`the_song_label_window_survives_a_tiny_window` (no inverted or empty rect at a 320 × 200 window).

- [ ] **Step 1: Write the failing test.**
- [ ] **Step 2: Run** `cargo test -p acid-app --test notice_live` — expect FAIL (62 px off).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `fix(panel): the song's name sits on the middle of the strip`.

### Task 4: The two tips entries are the card's switches, and they talk

**What is wrong:** `tips::Control::{Tooltips, Autosave}` describe controls whose keys are gone, so the table carries
two entries nothing can show — and the settings card's two switches, which *are* those controls, explain nothing.
`tips::hover` takes `&PanelState`, so a surface painted with two flags cannot use it.

**Design:** `tips::text_for(wants_help: bool, control)` and `tips::hover(response, wants_help, control)` — one mapping
(`Control::tip()`), and `tips` no longer knows what a `PanelState` is. Every call site passes `state.tooltips`. The
card calls the same function for its two switches, so the entries are live again and the card explains itself.

**Files:** modify `ui/tips.rs`, `ui/controls.rs`, `ui/row_knobs.rs`, `ui/row_perform.rs`, `ui/mod.rs`, `ui/settings.rs`.

**Tests:** `tips::text_for_answers_only_when_help_is_wanted`; live: `the_settings_switches_explain_themselves` (hover
the TOOLTIPS switch with help on → its tip is painted; with help off → nothing), same technique as the readouts test.

- [ ] **Step 1: Write the failing tests.**
- [ ] **Step 2: Run** — expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `feat(settings): the two switches explain themselves, where the old keys did`.

### Task 5: `main` holds no arm that cannot run

**What is wrong:** the card applies `CLOSE` and both folds itself, so `Request::{Close, ToggleMidi, ToggleSound}` can
never reach `main`, yet `main` has arms for them. A reader cannot tell the living from the dead.

**Design:** split the vocabulary in `ui/settings.rs`:
```rust
pub enum Request { Tooltips, Autosave, MidiIn(Option<String>), SoundOut(String), Retry }  // main's work
pub enum Own { ToggleMidi, ToggleSound, Close }                                            // the card's own state
pub enum Click { Ask(Request), Own(Own) }                                                  // what a click asked for
pub enum Action { ToggleTooltips, ToggleAutosave, Retry, Choose(Pick) }                    // what main does
impl Request { pub fn action(&self) -> Action }
```
`paint -> Option<Click>`; `draw_settings` applies `Own`; `main` matches `Action` — every arm reachable, one total
mapping from request to effect, and it is testable without the binary.

**Files:** modify `ui/settings.rs`, `ui/mod.rs` (`draw_settings`), `main.rs` (`apply_settings_request`),
`tests/settings_live.rs`.

**Tests:** `a_click_on_close_closes_the_card_and_asks_nothing_of_main`; rewrite `a_device_row_asks_for_the_choice_main_applies`
as `a_device_row_asks_for_the_action_main_takes` over `Request::action()`; the fold and switch tests stay as they are.

- [ ] **Step 1: Write the failing tests.**
- [ ] **Step 2: Run** — expect FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `refactor(settings): the card keeps what it does, main keeps what it does`.

### Task 6: Rows stop at the scrollbar

**What is wrong:** `scrollable` builds each row at `area.width()`, the panel's full width, while the scroll area lays
its bar over the right edge — so every row runs under the bar.

**Design:** size each row from `ui.available_width()` inside the scroll area, clamped to the panel, so the rows end
where the content area ends.

**Files:** modify `ui/settings.rs` (`scrollable`); test `tests/settings_live.rs`.

**Tests:** `a_scrolling_list_leaves_its_scrollbar_alone` — twelve ports, MIDI section open: a click at
`midi_panel.right() - 2` asks for no device, while a click in the middle of the first row still does.

- [ ] **Step 1: Write the failing test.**
- [ ] **Step 2: Run** — expect FAIL (the row takes the click).
- [ ] **Step 3: Implement.**
- [ ] **Step 4: Run** `cargo test --workspace`.
- [ ] **Step 5: Commit** — `fix(settings): the device rows end before the scrollbar`.

### Task 7: The dead code and the stale words go

**What is wrong:** `ui::picker` has no caller (`picker::closes_on_click_outside` is the shield's job now);
`panel::midi_status_rect` described the status line that no longer exists; and five sentences still describe the old
face: `supervisor.rs` "press REPROBE", `main.rs` "press the amber dot", `help.rs` "click the MIDI line" and "TIPS and
AUTO live in settings", `ui/mod.rs`'s doc comments about REPROBE and the TIPS/AUTO keys, plus `README.md`'s "between
the gear and the help `?`" for a lamp that is left of both.

**Design:** delete the two dead items; point each sentence at what exists (the settings card's `RETRY` row, its
`TOOLTIPS` / `AUTOSAVE` switches, `F2` for the device lists); say where the lamp really is.

**Files:** delete `ui/picker.rs`; modify `ui/mod.rs`, `ui/panel.rs`, `ui/help.rs`, `ui/tips.rs`, `midi/supervisor.rs`,
`main.rs`, `README.md`, `tests/supervisor.rs` (if it pins the words).

- [ ] **Step 1: Delete the dead items**, run `cargo test --workspace` and `cargo clippy` to prove nothing referenced them.
- [ ] **Step 2: Fix the sentences**, update any test that pins them.
- [ ] **Step 3: Run** the three gates.
- [ ] **Step 4: Commit** — `docs: the words catch up with the face, and two dead items go`.

## Non-goals

- No new behaviour beyond the minors listed; no change to the SONGS card's contents, the save model, the transport or
  the song format.
- No re-layout of the top strip or of the settings card.

## Progress

- [x] Task 1 — one scrim, one shield. `theme::SHADE`, `ui::modal::shield` (paints the dim, registers the dismissal click,
      and documents the register-it-first rule), and all three cards call it. `tests/modal_live.rs` (3): every card dims
      the window and closes on a click outside it, and nothing dims when no card is up. RED seen: the help card painted no
      scrim. 517 tests over 60 suites.
- [x] Task 2 — a line of text fits. `ui::text::{fit_with, fit}`: measured by the painter, cut at the end, an ellipsis that
      counts against the width, and nothing at all rather than a lone ellipsis when there is no room. The label line uses
      it, and `song_label`'s private 18-character cap is gone — one truncation rule now, not two.
      Tests: `tests/text_fit.rs` (5, pure arithmetic) and `notice_live` (8; RED seen: the notice was painted whole and the
      clip rect ate both ends). 524 tests over 61 suites.
- [x] Task 3 — the name centred on the strip. `song_label_rect` is now symmetric about `top.center().x`, half as wide as the
      nearer neighbour allows, and falls back to the gap itself when a tiny window makes the two meet. RED seen: the name
      sat at 545 against a strip centre of 490. Tests: `the_song_label_is_centred_in_the_strip` (1 px, and clear of the
      lamp, gear and `?` as well), `the_song_label_window_uses_all_the_room_the_tighter_side_allows`,
      `the_song_label_window_survives_a_tiny_window`.
      Ruling: the symmetry test I first wrote compared the gaps to the two neighbours, which is a different property and
      failed against a correct rect (0 left, 110 right — the left cluster is tighter). Rewritten to state the real rule:
      centred on the strip, reaching the nearer bound, never past either. — Cost if wrong: nothing pins "equal gaps",
      because equal gaps and a strip-centred window cannot both hold.
- [x] Task 4 — the two tips entries talk again. `tips::text_for`/`tips::hover` take `wants_help: bool` instead of
      `&PanelState`, so the settings card — painted with two flags, not with the panel — gives its two switches the help the
      deleted TIPS/AUTO keys had. The TIPS tip now titles itself `TOOLTIPS`, matching the caption it describes.
      Tests: `tips::the_hints_stay_silent_when_the_player_turned_them_off` (rewritten for the new argument),
      `settings_live::the_settings_switches_explain_themselves` (RED seen: nothing was painted for the switch).
      **Found while doing it:** an egui tooltip waits for the pointer to stop moving, and a `PointerMoved` every frame at
      one position keeps it away — so `tips_live::the_two_readouts_answer_the_pointer` had been passing on the panel's own
      `TEMPO -` / `TEMPO +` captions and never showed a tooltip at all. Both it and the switch-silences-the-help test now
      settle the pointer and assert on the tip's *body*.
      Ruling: the hover helper is duplicated in `settings_live` and `tips_live` rather than shared through a `tests/common`
      module. — Two independent test binaries, twelve lines each, and the comment explaining the still-pointer trap travels
      with both. — Cost if wrong: one more copy if a third file needs to hover something.
- [x] Task 5 — no unreachable arm in `main`. `settings::{Request, Own, Click, Action}`: `Request` names only what `main`
      can do, `Own` is the card's own three (two folds and CLOSE), `paint` returns a `Click`, `draw_settings` applies the
      `Own` half, and `main` matches `Request::action()` — six cases, all reachable. `Request::pick` became `action`,
      which is total rather than `Option`.
      Tests: `a_device_row_asks_for_the_action_main_takes` (compile-RED, then the whole mapping),
      `the_card_keeps_its_own_folds_and_its_close` (CLOSE closes and asks `main` for nothing). 528 tests over 61 suites.
- [x] Task 6 — rows stop at the scrollbar. `scrollable` asks for a bar that *allocates* its own column
      (`spacing.scroll.floating = false`) and sizes each row from the style's `allocated_width()` when the list is longer
      than `VISIBLE_ROWS`, so rows end where the content ends on the very first frame (egui animates a floating bar in).
      Test: `a_scrolling_list_gives_the_scrollbar_its_gutter` — paints one frame and measures the widest row face: full
      width for two ports, 13.8 px short of the panel for twelve. RED seen: 582.37 of 582.12.
      Rulings: (a) the assertion is on painted rects, not clicks — a click was already clipped to the scroll viewport, so
      a click test passed while every face ran under the bar; (b) the bar's width is read from the *child's* style, because
      `ui`'s still says the bar floats and a floating bar allocates nothing. — Cost if wrong: a non-floating bar is drawn
      in egui's stock colours at the gutter, which is a look the panel does not otherwise use.
- [x] Task 7 — dead code and stale words. Deleted `ui::picker` (its subject was the shield, which `ui::modal` now owns) and
      `panel::midi_status_rect`; `cargo check --all-targets` proved nothing referenced either. Words caught up:
      `supervisor.rs` gives up as `MIDI: none (gave up)` and `main`'s log line names RETRY and F2 instead of an amber dot;
      `help.rs` points at `F2` for the input port and at the switches by their real names; `mod.rs`'s field docs talk about
      the settings card's RETRY row and switches; `tips.rs`'s `Autosave` doc and its catalogue test; `README` puts the lamp
      left of the gear and the `?`. The 64-character help-line budget caught the first attempt at the help wording, which
      is the constraint doing its job. 527 tests over 61 suites.
