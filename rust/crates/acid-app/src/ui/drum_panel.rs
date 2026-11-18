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
use acid_core::pattern::STEPS_PER_PATTERN;
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
        let is_muted = muted & (1u32 << drum.index()) != 0;

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
                let bit = 1u32 << drum.index();
                shared.drum_mute.store(muted ^ bit, Ordering::Relaxed);
            } else {
                state.drum.selected = drum;
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
                // The pattern as it is this instant, never the frame's mirror: the engine may have written this step since
                // the panel last painted, and a write from a stale copy would silently undo it (Review Focus 1).
                let mut updated = shared.pattern().steps[step];
                if updated.is_hit(drum) {
                    if shift && !updated.is_accented(drum) {
                        updated.hit(drum, true);
                    } else {
                        updated.clear_hit(drum);
                    }
                } else {
                    updated.hit(drum, shift);
                }
                shared.set_step(step, updated);
                state.pattern = shared.pattern();
                state.dirty = true;
            }
        }
    }
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

    let kit = shared.kit();
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
        knob::knob(
            &mut cell_ui,
            egui::Id::unique(("drum-dial", slot)),
            dial.label(),
            &mut value,
            diameter,
            p,
        );
        value = value.clamp(0.0, 1.0);
        state.knobs[slot] = value;
        if (value - before).abs() > 1e-6 {
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
        if response.clicked() {
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
    let response = knob::knob(
        &mut cell_ui,
        egui::Id::unique("drum-shuffle"),
        "SHUFFLE",
        &mut value,
        diameter,
        p,
    );
    tips::hover(&response, state.tooltips, tips::Control::Shuffle);
    if (value - before).abs() > 1e-6 {
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
}
