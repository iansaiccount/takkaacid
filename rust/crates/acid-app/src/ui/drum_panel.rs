//! The drum machine's face: the selected voice's seven dials, the three-way filter key, `SHUFFLE`, `HIT`, and the grid.
//!
//! The grid is the editor — a row per voice, a cell per step — and it takes the band the 303 spends on its step row, its
//! performance row and the lower half of its knob row. Everything here is derived from [`layout::PanelLayout`]'s bands, never
//! from design pixels, so the face is the same face at 0.65x and at 1.6x.
//!
//! The seven dials are the 303's seven slots, relabelled for the selected voice; their values live in the kit (one truth,
//! clone-on-write through [`crate::state::SharedState::set_kit`]) and never in the 303's atomics.

use std::sync::atomic::Ordering;

use acid_core::drums::{Drum, FilterKind, KitVoice};
use acid_core::pattern::{STEPS_PER_PATTERN, Step};
use eframe::egui::{self, Align2, Pos2, Rect};

use crate::knobs::Knob;
use crate::state::SharedState;

use super::{PanelState, kits, knob, layout, theme, tips, transport};

/// The slice of the 303's knob band the dials take: the rest goes to the grid, and the dials give up height rather than the
/// grid giving up a row. 0.55 of the band puts the grid's first row where the spec's mockup starts it.
const DIAL_BAND: f32 = 0.55;

/// The voice's name, over the dials, as a fraction of the dial band.
const CAPTION: f32 = 0.16;

/// The label gutter, as a fraction of the grid's width: four characters and the air around them.
const LABEL: f32 = 0.09;

/// How much of a row's pitch the band keeps, so sixteen rows read as rows and not as one slab.
const ROW_INSET: f32 = 0.10;

/// The dials' ranges. A `KitVoice` holds the engine's units — milliseconds and semitones — and a knob is a position, so these
/// are the one place the two meet. Task 6's listening pass sets the defaults; this sets what a hand can reach.
const ATTACK_RANGE_MS: f32 = 200.0;
const DECAY_RANGE_MS: f32 = 2_000.0;
const RELEASE_RANGE_MS: f32 = 1_000.0;
const TUNE_RANGE_SEMITONES: f32 = 12.0;

/// Which voice the dials are editing. Panel state, not saved: the whole kit is in memory only this phase.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DrumView {
    pub selected: Drum,
}

impl Default for DrumView {
    fn default() -> Self {
        Self {
            selected: Drum::Kick,
        }
    }
}

/// One key the drum face owns, and what it does.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FaceKey {
    /// Walk the dials up (`-1`) or down (`1`) the grid, in [`Drum::ALL`]'s order — the order the rows are drawn in. The
    /// caller clamps at the ends with [`super::selection::move_selection`], the rule the 303's step border already keeps.
    MoveVoice(i8),
    /// Move the step selection back (`-1`) or on (`1`), the 303's plain arrows. The caller clamps the same way.
    MoveStep(i8),
    /// Write the selected voice into the selected step, `enter` and `shift+enter`: the grid's own click gesture, and silent.
    Write { accented: bool },
    /// Strike the selected voice without writing it: `HIT`'s key, so a voice can be heard without the pointer.
    Hit,
}

/// What a key does on the drum face, if anything.
///
/// One mapping for the face's keys: plain `left`/`right` move the step, `shift` and `up`/`down` walk the voice, `enter` writes
/// the voice into the step (`shift` accents it), `h` strikes it, and everything else belongs to another reader — the transport,
/// the patterns, or (on the 303's face) the notes. The same `h`, `enter` and arrows are the 303's, which is why the press filter
/// has to be told which face is showing before it takes a press.
pub fn face_key(key: egui::Key, shift: bool) -> Option<FaceKey> {
    match (key, shift) {
        (egui::Key::ArrowUp, true) => Some(FaceKey::MoveVoice(-1)),
        (egui::Key::ArrowDown, true) => Some(FaceKey::MoveVoice(1)),
        (egui::Key::ArrowLeft, false) => Some(FaceKey::MoveStep(-1)),
        (egui::Key::ArrowRight, false) => Some(FaceKey::MoveStep(1)),
        (egui::Key::Enter, _) => Some(FaceKey::Write { accented: shift }),
        (egui::Key::H, false) => Some(FaceKey::Hit),
        _ => None,
    }
}

/// The selected voice on one step, after the gesture the cell click and `enter` share: a plain gesture adds or takes the hit
/// off, an accented one adds an accent, accents a plain strike, or takes an accented one off.
///
/// This is the grid's whole editor, in one function, so the pointer and the keyboard cannot answer differently. It writes the
/// masks and `on`, through `Step`'s own writers; the 303's pitch and level on the step are left alone.
pub fn toggle_hit(mut step: Step, drum: Drum, accent: bool) -> Step {
    if step.is_hit(drum) {
        if accent && !step.is_accented(drum) {
            step.hit(drum, true);
        } else {
            step.clear_hit(drum);
        }
    } else {
        step.hit(drum, accent);
    }
    step
}

/// Write `drum` on `step`, through [`toggle_hit`]: the one writer the cell click and `enter` both use, so the pointer and
/// the keyboard cannot answer differently.
///
/// It reads the pattern from `SharedState` as it is this instant, never the frame's mirror — the engine may have written the
/// step since the panel last painted, and a write from a stale copy would silently undo it — then mirrors it for the face,
/// marks the session dirty and disarms a pending `CLEAR`, the rule every pattern edit keeps. The index is clamped, as every
/// index on this face is.
pub fn strike_step(
    state: &mut PanelState,
    shared: &SharedState,
    step: usize,
    drum: Drum,
    accent: bool,
) {
    let index = step.min(STEPS_PER_PATTERN - 1);
    let updated = toggle_hit(shared.pattern().steps[index], drum, accent);
    shared.set_step(index, updated);
    state.pattern = shared.pattern();
    state.dirty = true;
    super::clear::disarm(state);
}

/// One of the seven dials, in the order the face draws them.
///
/// The 303's knob slots are repurposed, left to right, so the row reads `ATTACK DECAY SUSTAIN RELEASE CUT OFF RESONANCE TUNE`
/// exactly where the 303 puts its seven. [`Dial::for_knob`] is the one mapping between the two catalogues; every arm is written
/// out, so a knob added to `Knob::ALL` fails to compile here rather than silently wearing another dial's field.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dial {
    Attack,
    Decay,
    Sustain,
    Release,
    Cutoff,
    Resonance,
    Tune,
}

impl Dial {
    /// The dial a 303 knob's slot is repurposed as.
    ///
    /// This is the face's one order: there is no `Dial::ALL` beside `Knob::ALL`, because a second positional list is the bug
    /// this repo has paid for twice. A test walks `Knob::ALL.map(Dial::for_knob)` instead.
    pub fn for_knob(knob: Knob) -> Dial {
        match knob {
            Knob::Waveform => Dial::Attack,
            Knob::Tuning => Dial::Decay,
            Knob::Cutoff => Dial::Sustain,
            Knob::Resonance => Dial::Release,
            Knob::EnvMod => Dial::Cutoff,
            Knob::Decay => Dial::Resonance,
            Knob::Accent => Dial::Tune,
        }
    }

    /// The silkscreen caption.
    pub fn label(self) -> &'static str {
        match self {
            Dial::Attack => "ATTACK",
            Dial::Decay => "DECAY",
            Dial::Sustain => "SUSTAIN",
            Dial::Release => "RELEASE",
            Dial::Cutoff => "CUT OFF",
            Dial::Resonance => "RESONANCE",
            Dial::Tune => "TUNE",
        }
    }

    /// The voice's value as a knob position.
    pub fn read(self, voice: &KitVoice) -> f32 {
        match self {
            Dial::Attack => (voice.attack_ms / ATTACK_RANGE_MS).clamp(0.0, 1.0),
            Dial::Decay => (voice.decay_ms / DECAY_RANGE_MS).clamp(0.0, 1.0),
            Dial::Sustain => voice.sustain.clamp(0.0, 1.0),
            Dial::Release => (voice.release_ms / RELEASE_RANGE_MS).clamp(0.0, 1.0),
            Dial::Cutoff => voice.cutoff.clamp(0.0, 1.0),
            Dial::Resonance => voice.resonance.clamp(0.0, 1.0),
            Dial::Tune => ((voice.tune / TUNE_RANGE_SEMITONES) + 1.0) / 2.0,
        }
        .clamp(0.0, 1.0)
    }

    /// Write one field of a voice from a knob position — the inverse of [`Dial::read`], and the only writer.
    pub fn write(self, voice: &mut KitVoice, position: f32) {
        let position = position.clamp(0.0, 1.0);
        match self {
            Dial::Attack => voice.attack_ms = position * ATTACK_RANGE_MS,
            Dial::Decay => voice.decay_ms = position * DECAY_RANGE_MS,
            Dial::Sustain => voice.sustain = position,
            Dial::Release => voice.release_ms = position * RELEASE_RANGE_MS,
            Dial::Cutoff => voice.cutoff = position,
            Dial::Resonance => voice.resonance = position,
            Dial::Tune => voice.tune = (position * 2.0 - 1.0) * TUNE_RANGE_SEMITONES,
        }
    }
}

/// The three controls beside the dials: the filter key, the device's `SHUFFLE`, and `HIT`.
///
/// A catalogue because the row's cells are counted from it — the dial row is `Knob::ALL`'s length plus this one's, and adding
/// a key without deciding where it sits fails to compile.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DrumKey {
    Filter,
    Shuffle,
    Hit,
}

impl DrumKey {
    /// Every key, in drawing order: filter, then shuffle, then `HIT`, as the spec's mockup draws the row.
    pub const ALL: [DrumKey; 3] = [DrumKey::Filter, DrumKey::Shuffle, DrumKey::Hit];

    /// Where the key sits after the dials.
    pub fn index(self) -> usize {
        match self {
            DrumKey::Filter => 0,
            DrumKey::Shuffle => 1,
            DrumKey::Hit => 2,
        }
    }
}

/// The band the dials live in: the top of the 303's knob row.
pub fn dial_area(l: &layout::PanelLayout) -> Rect {
    let band = l.knobs.area;
    Rect::from_min_max(
        band.min,
        Pos2::new(band.right(), band.top() + band.height() * DIAL_BAND),
    )
}

/// The dials row: the 303's seven cells plus the three keys, under the voice's name.
pub fn dial_cells(l: &layout::PanelLayout) -> Vec<Rect> {
    let area = dial_area(l);
    let row = Rect::from_min_max(
        Pos2::new(area.left(), area.top() + area.height() * CAPTION),
        area.max,
    );
    layout::knob_cells(row, Knob::ALL.len() + DrumKey::ALL.len())
}

/// The grid's band: from under the dials to the switches row, taking the bands the drum face does not use.
pub fn grid_area(l: &layout::PanelLayout) -> Rect {
    Rect::from_min_max(
        Pos2::new(l.knobs.area.left(), dial_area(l).bottom()),
        Pos2::new(l.knobs.area.right(), l.steps.bottom()),
    )
}

/// One voice's visible row band, inset from its pitch so sixteen rows read apart.
pub fn row_band(l: &layout::PanelLayout, drum: Drum) -> Rect {
    let grid = grid_area(l);
    let pitch = grid.height() / Drum::ALL.len() as f32;
    let inset = pitch * ROW_INSET;
    let top = grid.top() + pitch * drum.index() as f32;
    Rect::from_min_max(
        Pos2::new(grid.left(), top + inset),
        Pos2::new(grid.right(), top + pitch - inset),
    )
}

/// The voice's label, at the row's left: the gutter the caption and its plate live in.
pub fn label_rect(l: &layout::PanelLayout, drum: Drum) -> Rect {
    let row = row_band(l, drum);
    Rect::from_min_max(
        row.min,
        Pos2::new(row.left() + grid_area(l).width() * LABEL, row.bottom()),
    )
}

/// One cell of the grid: a voice on a step.
pub fn cell_rect(l: &layout::PanelLayout, drum: Drum, step: usize) -> Rect {
    row_cells(l, drum)[step]
}

/// One voice's step cells, in step order — the row's geometry, built once per row rather than once per cell.
///
/// Sixteen cells a row and sixteen rows is 256 rects a frame; building the row's `Vec` once per cell (as the first cut did)
/// was 256 transient allocations a frame on the UI thread, while a hand drags a dial.
pub fn row_cells(l: &layout::PanelLayout, drum: Drum) -> [Rect; STEPS_PER_PATTERN] {
    let row = row_band(l, drum);
    let label = label_rect(l, drum);
    let steps = Rect::from_min_max(
        Pos2::new(label.right() + theme::GAP, row.top()),
        Pos2::new(row.right(), row.bottom()),
    );
    let mut cells = [Rect::NOTHING; STEPS_PER_PATTERN];
    cells.copy_from_slice(&layout::knob_cells(steps, STEPS_PER_PATTERN));
    cells
}

/// One step's column: the sixteen cells under each other, from the first row's band to the last's.
///
/// Derived from the rows rather than from the cells, so the column keeps the row inset's shape at every scale. It is the step
/// selection's mark, the way the 303's border is drawn round its step key — this face has no step row to put one in.
pub fn column_rect(l: &layout::PanelLayout, step: usize) -> Rect {
    let step = step.min(STEPS_PER_PATTERN - 1);
    let first = cell_rect(l, Drum::ALL[0], step);
    let top = row_band(l, Drum::ALL[0]).top();
    let bottom = row_band(l, Drum::ALL[Drum::ALL.len() - 1]).bottom();
    Rect::from_min_max(
        Pos2::new(first.left(), top),
        Pos2::new(first.right(), bottom),
    )
}

/// One of the three keys beside the dials.
pub fn key_cell(l: &layout::PanelLayout, key: DrumKey) -> Rect {
    dial_cells(l)[Knob::ALL.len() + key.index()]
}

/// One segment of the filter key: `LP`, `BP` or `HP`.
pub fn filter_cell(l: &layout::PanelLayout, kind: FilterKind) -> Rect {
    let cell = key_cell(l, DrumKey::Filter);
    let width = cell.width() / FilterKind::ALL.len() as f32;
    let left = cell.left() + width * kind.index() as f32;
    Rect::from_min_max(
        Pos2::new(left, cell.top()),
        Pos2::new(left + width, cell.bottom()),
    )
}

/// A palette colour at a fraction of its opacity: the row band's faint accent, derived from the palette and never a literal.
fn faint(colour: egui::Color32, alpha: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(colour.r(), colour.g(), colour.b(), alpha)
}

/// Draw the whole drum face into the bands it owns.
pub fn draw(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
    p: &theme::Palette,
) {
    // The grid is drawn first: a click on a label selects the voice, and the dials below must show the new voice in the frame
    // that selected it, not a frame later.
    draw_grid(state, shared, ui, l, s, p);
    draw_dials(state, shared, ui, l, s, p);
}

fn draw_grid(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
    p: &theme::Palette,
) {
    let muted = shared.drum_mute.load(Ordering::Relaxed);
    let selected = state.drum.selected;
    let band = faint(p.led_accent, 36);
    let label_font = |row: Rect| theme::legend((row.height() * 0.72).max(6.0));

    for drum in Drum::ALL {
        let row = row_band(l, drum);
        let label = label_rect(l, drum);
        let is_selected = drum == selected;
        let is_muted = muted & u32::from(drum.bit()) != 0;

        if is_selected {
            ui.painter().rect_filled(row, 2.0 * s, band);
            // The plate: accent fill, the face's own ink for the caption — the same drawing a lit key uses, which inverts
            // with the scheme because both colours come from the palette.
            ui.painter().rect_filled(label, 2.0 * s, p.led_accent);
        }
        ui.painter().text(
            label.center(),
            Align2::CENTER_CENTER,
            drum.label(),
            label_font(row),
            if is_selected {
                p.panel
            } else if is_muted {
                p.silkscreen_dim
            } else {
                p.silkscreen
            },
        );

        let hit = ui.interact(
            label,
            ui.scope_id().with(("drum-label", drum.index())),
            egui::Sense::click(),
        );
        if hit.clicked() {
            let shift = ui.input(|input| input.modifiers.shift);
            if shift {
                // The live mute is the state's mask, not the kit's flags: the kit's are the starting mask a load seeds, and
                // rewriting them would let the panel's mute travel into a saved kit.
                let bit = u32::from(drum.bit());
                shared.drum_mute.store(muted ^ bit, Ordering::Relaxed);
            } else {
                state.drum.selected = drum;
                // A selection is not a hit. The label picks the voice the dials edit; `HIT` is how it is heard, and a
                // `SHIFT`-click is the mute — neither moves the pointer to the voice under it into a sound.
            }
        }

        for (step, cell) in row_cells(l, drum).iter().enumerate() {
            let cell = *cell;
            let painted_step = state.pattern.steps[step];
            let struck = painted_step.is_hit(drum);
            let accented = painted_step.is_accented(drum);
            let fill = if struck {
                if accented && !is_muted {
                    p.led_accent
                } else {
                    // A muted row keeps its strikes readable and loses only the accent, so a muted voice can be found and
                    // unmuted by eye.
                    p.silkscreen_dim
                }
            } else {
                p.well
            };
            ui.painter().rect_filled(cell, 1.5 * s, fill);
            if step % 4 == 0 {
                // Every fourth column framed in silk: the beat is findable without counting sixteen squares.
                ui.painter().rect_stroke(
                    cell,
                    1.5 * s,
                    p.silkscreen_stroke(s),
                    egui::StrokeKind::Inside,
                );
            }

            let response = ui.interact(
                cell,
                // Ids are behaviour: a drag routed to a renamed cell would land on its neighbour. The voice and the step
                // are the cell's identity, so a renamed Id here is a face change and must be argued with.
                ui.scope_id().with(("drum-cell", drum.index(), step)),
                egui::Sense::click(),
            );
            if response.clicked() {
                let shift = ui.input(|input| input.modifiers.shift);
                strike_step(state, shared, step, drum, shift);
            }
        }
    }

    // The step selection: one column, the mark the 303 draws round its step key. Drawn after the cells so it reads over
    // their edges, and it is the only mark a step has on this face — the playhead belongs to the 303's step row, which a
    // drum face does not draw.
    let column = column_rect(l, state.selected);
    ui.painter().rect_stroke(
        column,
        2.0 * s,
        egui::Stroke::new(1.5 * s, p.led_accent),
        egui::StrokeKind::Inside,
    );
    // A gutter in the face's own colour outside the accent: an accented hit is filled with the same ink, so without this the
    // column's edge would vanish for the height of every accent it crossed.
    ui.painter().rect_stroke(
        column,
        2.0 * s,
        egui::Stroke::new(1.5 * s, p.panel),
        egui::StrokeKind::Outside,
    );
}

fn draw_dials(
    state: &mut PanelState,
    shared: &SharedState,
    ui: &mut egui::Ui,
    l: &layout::PanelLayout,
    s: f32,
    p: &theme::Palette,
) {
    let selected = state.drum.selected;
    let area = dial_area(l);
    // The voice's name over its dials: what is being edited is readable at a glance.
    ui.painter().text(
        Pos2::new(area.left(), area.top() + area.height() * CAPTION * 0.5),
        Align2::LEFT_CENTER,
        selected.label(),
        theme::legend(13.0 * s),
        p.led_accent,
    );

    // A learn in progress is picking a control, not turning one: the 303's dials stand still and a click picks instead
    // (`row_knobs`), and a drum dial that turned here would copy a factory kit under a new name because the hand meant to
    // teach. There is no `CcTarget` for a drum voice this phase, so there is nothing here to pick either — the dial holds
    // its position and the kit is left alone.
    let choosing = state.learn == super::learn::Step::Choosing;
    let kit = shared.kit();
    // The kit the machine is playing, at the other end of the caption line. A drum machine is born on the `Init` kit, which
    // has no samples, so a face that never says the kit's name reads as a broken machine rather than as a kit with nothing in
    // it — and a face that says `KIT 808` cannot disagree with the card's row, because both read the state's kit.
    let caption = format!("KIT {}", kit.name);
    let caption = super::text::fit(
        ui.painter(),
        &caption,
        theme::legend(11.0 * s),
        area.width() * 0.5,
    );
    ui.painter().text(
        Pos2::new(area.right(), area.top() + area.height() * CAPTION * 0.5),
        Align2::RIGHT_CENTER,
        caption,
        theme::legend(11.0 * s),
        p.silkscreen_dim,
    );
    let voice = &kit.voices[selected.index()];
    let cells = dial_cells(l);
    for (knob, cell) in Knob::ALL.iter().zip(cells.iter()) {
        let knob = *knob;
        let dial = Dial::for_knob(knob);
        let slot = knob.index();
        // The dial's position is the selected voice's field, re-read every frame the way the 303's dials are re-read from
        // their atomics: the kit is the truth between frames, not this array.
        let before = dial.read(voice);
        state.knobs[slot] = before;
        let diameter = cell.width().min(cell.height() * 0.78);
        let square = Rect::from_center_size(cell.center(), egui::vec2(diameter, cell.height()));
        let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(square));
        cell_ui.set_clip_rect(*cell);
        let mut value = before;
        let mut scratch = before;
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("drum-dial", slot)),
            dial.label(),
            if choosing { &mut scratch } else { &mut value },
            diameter,
            p,
        );
        value = value.clamp(0.0, 1.0);
        state.knobs[slot] = value;
        if !choosing && (value - before).abs() > 1e-6 {
            edit_voice(state, shared, selected, |voice| dial.write(voice, value));
        }
    }

    // The filter key: the 303's waveform switch, three faces, the current one lit.
    let current = shared.kit().voices[selected.index()].filter;
    for kind in FilterKind::ALL {
        let rect = filter_cell(l, kind);
        let lit = kind == current;
        let face = if lit { p.panel_raised } else { p.key_face };
        ui.painter()
            .rect_filled(rect.shrink(1.0 * s), 2.0 * s, face);
        ui.painter().rect_stroke(
            rect.shrink(1.0 * s),
            2.0 * s,
            p.silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            rect.center(),
            Align2::CENTER_CENTER,
            kind.label(),
            theme::legend(11.0 * s),
            if lit { p.lit_ink } else { p.silkscreen },
        );
        let response = ui.interact(
            rect,
            ui.scope_id().with(("drum-filter", kind.index())),
            egui::Sense::click(),
        );
        if response.clicked() && !choosing {
            edit_voice(state, shared, selected, |voice| voice.filter = kind);
        }
    }

    // SHUFFLE is the device's swing, not the voice's: the same atomic the 303's performance row writes, so both machines hear
    // one shuffle. It keeps a dial because it is a knob on the 303 too.
    let cell = key_cell(l, DrumKey::Shuffle);
    let diameter = cell.width().min(cell.height() * 0.78);
    let square = Rect::from_center_size(cell.center(), egui::vec2(diameter, cell.height()));
    let mut cell_ui = ui.new_child(egui::UiBuilder::new().max_rect(square));
    cell_ui.set_clip_rect(cell);
    let before = shared.swing.get();
    let mut value = before;
    let mut scratch = before;
    let response = knob::knob(
        &mut cell_ui,
        egui::Id::unique("drum-shuffle"),
        "SHUFFLE",
        if choosing { &mut scratch } else { &mut value },
        diameter,
        p,
    );
    tips::hover(&response, state.tooltips, tips::Control::Shuffle);
    if super::learn::highlight(state.learn, crate::midi::cc::CcTarget::Swing) {
        super::panel::learn_highlight(ui.painter(), cell, s, p);
    }
    if choosing {
        // The swing is rack-wide and has a real target, so this dial is the one knob on the drum face a controller *can* be
        // taught — the 303's `SHUFFLE` behaves the same way, and the two write the same atomic.
        super::controls::learn_pick(
            ui,
            state,
            shared,
            "learn-drum-shuffle",
            cell,
            crate::midi::cc::CcTarget::Swing,
        );
    } else if (value - before).abs() > 1e-6 {
        shared.swing.set(value.clamp(0.0, 1.0));
        state.dirty = true;
    }

    // HIT auditions the selected voice with the transport stopped.
    let hit =
        transport::transport_button(ui, "drum-hit", key_cell(l, DrumKey::Hit), "HIT", false, p);
    if hit.clicked() {
        shared.request_audition(selected);
    }
}

/// Clone-on-write one voice, through the kit browser's one seam: the kit is the one truth, and the panel never keeps a
/// second copy of a voice's envelope.
///
/// A factory kit cannot be edited in place — it is compiled into the binary — so the first edit of one makes a copy under a
/// free name inside [`kits::edit`], once, however many frames a dial drag lasts.
fn edit_voice(
    state: &mut PanelState,
    shared: &SharedState,
    drum: Drum,
    edit: impl FnOnce(&mut KitVoice),
) {
    kits::edit(state, shared, |kit| edit(&mut kit.voices[drum.index()]));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::Scheme;
    use crate::ui::theme::DESIGN_SIZE;

    fn l() -> layout::PanelLayout {
        layout::layout(Rect::from_min_size(Pos2::ZERO, DESIGN_SIZE))
    }

    #[test]
    fn every_dial_maps_to_one_knob_and_one_voice_field() {
        // The face's order comes from `Knob::ALL` through the one mapping; a `Dial::ALL` beside it would be the second
        // positional list this repo has paid for twice.
        assert_eq!(
            Knob::ALL.map(|knob| Dial::for_knob(knob).label()),
            [
                "ATTACK",
                "DECAY",
                "SUSTAIN",
                "RELEASE",
                "CUT OFF",
                "RESONANCE",
                "TUNE"
            ]
        );
        for knob in Knob::ALL {
            let dial = Dial::for_knob(knob);
            assert!(dial.label().is_ascii(), "{dial:?} is not ASCII");
        }
    }

    #[test]
    fn the_tuning_dial_travels_the_whole_semitone_range() {
        let mut voice = KitVoice::default();
        Dial::Tune.write(&mut voice, 0.0);
        assert_eq!(voice.tune, -TUNE_RANGE_SEMITONES);
        Dial::Tune.write(&mut voice, 0.5);
        assert_eq!(voice.tune, 0.0, "centre is no detune");
        Dial::Tune.write(&mut voice, 1.0);
        assert_eq!(voice.tune, TUNE_RANGE_SEMITONES);
    }

    #[test]
    fn the_grid_is_derived_from_the_bands_and_the_dials_do_not_cover_it() {
        let l = l();
        let grid = grid_area(&l);
        assert!(grid.top() >= l.knobs.area.top());
        assert!(grid.top() >= dial_area(&l).bottom() - 0.01);
        assert!((grid.bottom() - l.steps.bottom()).abs() < 0.01);
        // The grid's first row starts where the mockup puts it: just under the dials, not below the whole knob band.
        let first = row_band(&l, Drum::Kick);
        assert!(first.top() - l.knobs.area.top() > l.knobs.area.height() * 0.5);
        assert!(first.bottom() < l.knobs.area.bottom());
    }

    #[test]
    fn the_three_keys_sit_after_the_seven_dials() {
        let l = l();
        let cells = dial_cells(&l);
        assert_eq!(cells.len(), Knob::ALL.len() + DrumKey::ALL.len());
        for (index, key) in DrumKey::ALL.iter().enumerate() {
            assert_eq!(key.index(), index);
            assert!(
                (key_cell(&l, *key).center().x - cells[Knob::ALL.len() + index].center().x).abs()
                    < 0.01
            );
        }
        // The filter's three faces divide its cell and never leave it.
        let cell = key_cell(&l, DrumKey::Filter);
        for kind in FilterKind::ALL {
            let segment = filter_cell(&l, kind);
            assert!(cell.contains_rect(segment), "{kind:?} escapes the key");
        }
    }

    #[test]
    fn every_palette_colour_is_used_through_the_palette() {
        // A cheap guard against a literal: the face's three roles come from one scheme and the same frame in another scheme
        // must paint different ink. The live test proves it on shapes; this proves the table honours it.
        let black = theme::palette(Scheme::Black);
        let light = theme::palette(Scheme::ChoirPastel);
        assert_ne!(black.led_accent, light.led_accent);
        assert_ne!(faint(black.led_accent, 36), faint(light.led_accent, 36));
    }

    #[test]
    fn the_face_keys_are_the_walk_the_step_and_the_write() {
        // One mapping for the face's keyboard. Up the grid is one row earlier in `Drum::ALL`, which is the order the rows
        // are drawn in, so the sign and the catalogue cannot disagree; the step's arrows are the 303's plain pair, `enter`
        // writes with shift as its accent, `h` strikes, and everything else is not this face's key.
        assert_eq!(
            face_key(egui::Key::ArrowUp, true),
            Some(FaceKey::MoveVoice(-1))
        );
        assert_eq!(
            face_key(egui::Key::ArrowDown, true),
            Some(FaceKey::MoveVoice(1))
        );
        assert_eq!(
            face_key(egui::Key::ArrowLeft, false),
            Some(FaceKey::MoveStep(-1))
        );
        assert_eq!(
            face_key(egui::Key::ArrowRight, false),
            Some(FaceKey::MoveStep(1))
        );
        assert_eq!(
            face_key(egui::Key::Enter, false),
            Some(FaceKey::Write { accented: false })
        );
        assert_eq!(
            face_key(egui::Key::Enter, true),
            Some(FaceKey::Write { accented: true }),
            "shift+enter is the accent gesture"
        );
        assert_eq!(face_key(egui::Key::H, false), Some(FaceKey::Hit));
        assert_eq!(face_key(egui::Key::H, true), None, "shift+h is not the hit");
        assert_eq!(
            face_key(egui::Key::B, false),
            None,
            "the other letters are silent on this face"
        );
        assert_eq!(
            face_key(egui::Key::ArrowUp, false),
            None,
            "an unshifted up arrow is the tempo's"
        );
        assert_eq!(
            face_key(egui::Key::ArrowLeft, true),
            None,
            "shift+left is the pattern's, not a second way to move the step"
        );
        assert_eq!(face_key(egui::Key::ArrowRight, true), None);
    }

    #[test]
    fn the_hit_gesture_adds_toggles_and_accents() {
        // The grid's whole editor, in one function: a plain gesture adds or takes off, a shifted one accents, and an
        // accented hit is taken off rather than accented twice.
        let empty = Step::rest();
        let plain = toggle_hit(empty, Drum::Snare, false);
        assert!(plain.is_hit(Drum::Snare));
        assert!(!plain.is_accented(Drum::Snare));
        assert!(!toggle_hit(plain, Drum::Snare, false).is_hit(Drum::Snare));

        let accented = toggle_hit(empty, Drum::Snare, true);
        assert!(accented.is_hit(Drum::Snare));
        assert!(accented.is_accented(Drum::Snare));
        let lifted = toggle_hit(accented, Drum::Snare, true);
        assert!(!lifted.is_hit(Drum::Snare), "a second shift takes it off");
        let from_plain = toggle_hit(plain, Drum::Snare, true);
        assert!(
            from_plain.is_hit(Drum::Snare) && from_plain.is_accented(Drum::Snare),
            "a shift on a plain strike accents it"
        );
    }

    #[test]
    fn the_column_is_one_step_wide_and_sixteen_rows_tall() {
        // The painted test compares against `column_rect`, so this is the one that says the rect is the column and not a
        // cell: one step wide, from the first row's band to the last's, clamped like every other index on the face.
        let l = l();
        let column = column_rect(&l, 6);
        let first = cell_rect(&l, Drum::Kick, 6);
        assert_eq!(column.left(), first.left());
        assert_eq!(column.right(), first.right());
        assert_eq!(column.top(), row_band(&l, Drum::Kick).top());
        assert_eq!(column.bottom(), row_band(&l, Drum::Shaker).bottom());
        assert_eq!(column_rect(&l, 99), column_rect(&l, STEPS_PER_PATTERN - 1));
    }
}
