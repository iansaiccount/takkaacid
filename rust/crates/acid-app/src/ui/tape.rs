//! The tape deck in the top band: six keys drawn as shapes, and the two tempo nudge keys beside the readout.
//!
//! Shapes, not characters: this face may not use a font glyph, so a play key is a triangle the painter builds rather than a
//! `▶` from whatever font the machine happens to have. The catalogue here is the same idea as `Knobs` or `Switch` — one place
//! that says which keys exist, in which order, and whether each is wired — and the layout's `TAPE_COUNT` derives from it, so a
//! seventh key is a variant and a cell, never a second positional list.

use eframe::egui::{self, Pos2, Rect, Shape, Stroke, StrokeKind};

use super::{panel, theme, tips, transport};
use crate::state::SharedState;
use eframe::egui::Ui;

use super::PanelState;

/// The deck's keys, left to right, the way a tape deck reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TapeKey {
    Back,
    Play,
    Pause,
    Stop,
    Forward,
    Record,
}

impl TapeKey {
    pub const ALL: [TapeKey; 6] = [
        TapeKey::Back,
        TapeKey::Play,
        TapeKey::Pause,
        TapeKey::Stop,
        TapeKey::Forward,
        TapeKey::Record,
    ];

    /// What the key does, for a tooltip. A key that is not wired says so in its own words rather than promising.
    pub fn tip(self) -> tips::Control {
        match self {
            TapeKey::Back => tips::Control::TapeBack,
            TapeKey::Play => tips::Control::Run,
            TapeKey::Pause => tips::Control::TapePause,
            TapeKey::Stop => tips::Control::Stop,
            TapeKey::Forward => tips::Control::TapeForward,
            TapeKey::Record => tips::Control::TapeRecord,
        }
    }

    /// Whether pressing it does anything yet. The unwired three are drawn in the unlit face and their tip says "not wired".
    pub fn wired(self) -> bool {
        match self {
            TapeKey::Play | TapeKey::Pause | TapeKey::Stop => true,
            TapeKey::Back | TapeKey::Forward | TapeKey::Record => false,
        }
    }

    /// The request a press asks for, if it asks for one.
    fn request(self) -> Option<transport::Request> {
        match self {
            TapeKey::Play => Some(transport::Request::Play),
            TapeKey::Pause => Some(transport::Request::Pause),
            TapeKey::Stop => Some(transport::Request::Stop),
            TapeKey::Back | TapeKey::Forward | TapeKey::Record => None,
        }
    }
}

/// Paint the deck and the tempo stepper. Returns what a press asked for, if anything.
pub fn draw(
    ui: &mut Ui,
    state: &mut PanelState,
    shared: &SharedState,
    top: Rect,
    scale: f32,
    p: &theme::Palette,
) -> Option<transport::Request> {
    let mut asked = None;
    let transport = shared.transport();
    for (key, rect) in TapeKey::ALL.iter().zip(panel::tape_cells(top)) {
        let response = key_key(ui, *key, rect, scale, p, transport, shared);
        tips::hover(&response, state.tooltips, key.tip());
        if response.clicked() {
            asked = key.request();
        }
    }

    let stepper = panel::tempo_stepper_cells(top);
    for (which, rect) in [(0usize, stepper[0]), (2usize, stepper[2])] {
        let id = if which == 0 {
            "tape-tempo-down"
        } else {
            "tape-tempo-up"
        };
        let response = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
        let painter = ui.painter();
        painter.rect_filled(rect, 3.0 * scale, p.key_face);
        painter.rect_stroke(
            rect,
            3.0 * scale,
            p.silkscreen_stroke(scale),
            StrokeKind::Inside,
        );
        // A `−` and a `+` drawn as bars: two characters this face may not use, so they are strokes.
        let thick = 1.6 * scale;
        let arm = rect.width() * 0.28;
        painter.line_segment(
            [
                Pos2::new(rect.center().x - arm, rect.center().y),
                Pos2::new(rect.center().x + arm, rect.center().y),
            ],
            Stroke::new(thick, p.silkscreen),
        );
        if which == 2 {
            painter.line_segment(
                [
                    Pos2::new(rect.center().x, rect.center().y - arm),
                    Pos2::new(rect.center().x, rect.center().y + arm),
                ],
                Stroke::new(thick, p.silkscreen),
            );
        }
        if response.hovered() {
            painter.rect_stroke(
                rect,
                3.0 * scale,
                Stroke::new(1.5 * scale, p.led_accent),
                StrokeKind::Inside,
            );
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        tips::hover(
            &response,
            state.tooltips,
            if which == 0 {
                tips::Control::TempoDown
            } else {
                tips::Control::TempoUp
            },
        );
        if response.clicked() {
            asked = Some(transport::Request::NudgeTempo(if which == 0 {
                -1
            } else {
                1
            }));
        }
    }
    asked
}

/// One key: its face, its symbol, and the hover ring. The symbol is the only thing that says which key it is, so the three
/// unwired keys are drawn dim — a bright triangle that does nothing would be a promise.
fn key_key(
    ui: &mut Ui,
    key: TapeKey,
    rect: Rect,
    scale: f32,
    p: &theme::Palette,
    transport: crate::device::Transport,
    _shared: &SharedState,
) -> egui::Response {
    let id = format!("tape-{key:?}").to_lowercase();
    let response = ui.interact(rect, ui.scope_id().with(id), egui::Sense::click());
    let lit = match key {
        TapeKey::Play => transport == crate::device::Transport::Playing,
        TapeKey::Pause => transport == crate::device::Transport::Paused,
        _ => false,
    };
    let face = if lit { p.panel_raised } else { p.key_face };
    let ink = if !key.wired() {
        p.silkscreen_dim
    } else if lit {
        p.lit_ink
    } else {
        p.silkscreen
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 3.0 * scale, face);
    painter.rect_stroke(
        rect,
        3.0 * scale,
        p.silkscreen_stroke(scale),
        StrokeKind::Inside,
    );
    symbol(painter, key, rect, ink);
    if response.hovered() {
        painter.rect_stroke(
            rect,
            3.0 * scale,
            Stroke::new(1.5 * scale, if lit { p.lit_ink } else { p.led_accent }),
            StrokeKind::Inside,
        );
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// The six symbols, drawn inside `rect`. Triangles for play and the two directions, bars for pause, a square for stop, a
/// filled circle for record — the shapes a deck has used for fifty years.
///
/// Every arrow goes through [`arrow`], which takes the direction as a sign: the first version built the two rewind/fast-forward
/// triangles by hand and drew them pointing the wrong way — the screenshot showed a deck whose back key pointed forward.
fn symbol(painter: &egui::Painter, key: TapeKey, rect: Rect, ink: egui::Color32) {
    let c = rect.center();
    let unit = rect.width() * 0.30;
    match key {
        TapeKey::Play => {
            painter.add(arrow(c.x + unit * 0.55, c.y, unit, 1.0, ink));
        }
        // Back and forward are the same arrow twice, pointing the way the key moves the playhead.
        TapeKey::Back => {
            for shift in [0.0, unit * 1.05] {
                painter.add(arrow(c.x - unit * 0.55 + shift, c.y, unit, -1.0, ink));
            }
        }
        TapeKey::Forward => {
            for shift in [0.0, unit * 1.05] {
                painter.add(arrow(c.x + unit * 0.55 - shift, c.y, unit, 1.0, ink));
            }
        }
        TapeKey::Pause => {
            let w = unit * 0.42;
            for side in [-1.0f32, 1.0] {
                let x = c.x + side * unit * 0.45 - w * 0.5;
                painter.rect_filled(
                    Rect::from_min_size(Pos2::new(x, c.y - unit), egui::vec2(w, unit * 2.0)),
                    0.0,
                    ink,
                );
            }
        }
        TapeKey::Stop => {
            painter.rect_filled(
                Rect::from_center_size(c, egui::vec2(unit * 1.6, unit * 1.6)),
                0.0,
                ink,
            );
        }
        TapeKey::Record => {
            painter.circle_filled(c, unit * 0.85, ink);
        }
    }
}

/// One arrowhead with its apex at `x` and its body behind it, `direction` being `1.0` for "points right" and `-1.0` for
/// "points left". The sign is the only thing that decides which way it looks, so the two directions cannot disagree.
fn arrow(x: f32, y: f32, unit: f32, direction: f32, ink: egui::Color32) -> Shape {
    let apex = Pos2::new(x, y);
    let base = x - direction * unit * 0.95;
    Shape::convex_polygon(
        vec![
            apex,
            Pos2::new(base, y - unit * 0.8),
            Pos2::new(base, y + unit * 0.8),
        ],
        ink,
        Stroke::NONE,
    )
}
