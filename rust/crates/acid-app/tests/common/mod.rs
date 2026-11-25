//! One geometry for the `BANKS` card, shared by the two live test files that drive it.
//!
//! Round 02's review found this card's arithmetic written twice — eighteen helpers in `banks_live.rs`, thirteen in
//! `songs_live.rs`, and two `songs_top()`s that did not even mean the same thing. Every position here is a call into
//! [`banks::Rects`] or into the panel's own layout; nothing in this module computes a coordinate of the card. A helper here
//! that recomputed one would be the second opinion the round exists to delete.
//!
//! `allow(dead_code)` because each test crate compiles its own copy of this module and uses the part it needs.

#![allow(dead_code)]

use acid_app::state::SharedState;
use acid_app::ui::banks::{self, Banks, Filter};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::switches::{self, Switch};
use acid_app::ui::{PanelState, SongState, draw_panel};
use eframe::egui::{
    Context, Event, Modifiers, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect, vec2,
};

/// The window the panel is laid out in: the design's own width, a little shorter, so the card is drawn below the design size
/// and a row that only fits at the design size has to be found by the band, not by a taller window.
pub fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

/// The panel the card is drawn over, as `layout` places it.
pub fn panel() -> Rect {
    layout(screen()).panel
}

pub fn scale_of() -> f32 {
    scale(screen())
}

/// The card's geometry with the song's strip: `ALL`, which is the chip the card opens on.
pub fn rects() -> banks::Rects {
    rects_for(Filter::All)
}

pub fn rects_for(filter: Filter) -> banks::Rects {
    banks::card_rects(panel(), scale_of(), filter)
}

pub fn run_on(ctx: &Context, panel: &mut PanelState, shared: &SharedState, events: Vec<Event>) {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
}

/// A primary click at `at`, over three frames: a frame that moves the pointer there, one that presses, one that releases.
pub fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    // A frame with the pointer over the widget before the press: egui only reports a click on a widget it had already seen
    // hovered, which is how a control that appears under a resting pointer cannot be clicked by accident.
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![button(true)]);
    run_on(ctx, panel, shared, vec![button(false)]);
}

/// A wheel of `dy` pixels over the list: the pointer onto the band, then the wheel, in two frames. A hand turns a wheel over
/// what it is looking at, and the card reads it only there.
pub fn wheel(ctx: &Context, panel: &mut PanelState, shared: &SharedState, dy: f32) {
    wheel_at(ctx, panel, shared, rects().band.center(), dy);
}

/// The same wheel, over a place of the test's choosing — the strip, to prove a door does not move under it.
pub fn wheel_at(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2, dy: f32) {
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(
        ctx,
        panel,
        shared,
        vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: vec2(0.0, dy),
            phase: eframe::egui::TouchPhase::Move,
            source: eframe::egui::MouseWheelSource::Wheel,
            modifiers: Modifiers::default(),
        }],
    );
}

/// The centre of row `n` of the window: the row's name, which loads what its tag says.
pub fn row_centre(n: usize) -> Pos2 {
    rects().row(n).center()
}

/// The centre of the delete cross at row `n`'s right end.
pub fn cross_centre(n: usize) -> Pos2 {
    rects().cross(n).center()
}

/// Where a chip is: the catalogue's order is the card's order, so this is a lookup rather than a second list.
pub fn chip_centre(filter: Filter) -> Pos2 {
    let index = Filter::ALL
        .iter()
        .position(|chip| *chip == filter)
        .expect("every chip is on the card");
    rects().chips[index].center()
}

pub fn search_centre() -> Pos2 {
    rects().search.center()
}

/// The key beside the strip's field. The strip follows the chip, and the chip is the card's own state — so the card is asked
/// rather than handed a chip a test could name and the painter not have.
pub fn save_centre(panel: &PanelState) -> Pos2 {
    strip_rects(panel).strip_save.center()
}

pub fn copy_centre(panel: &PanelState) -> Pos2 {
    strip_rects(panel)
        .strip_copy
        .expect("only the song's strip has a copy key, and only a song chip is asked for one")
        .center()
}

/// The card's geometry for the chip it is on.
fn strip_rects(panel: &PanelState) -> banks::Rects {
    let card = panel.songs.card.as_ref().expect("the card is open");
    rects_for(card.filter)
}

pub fn footer_new_centre() -> Pos2 {
    rects().footer_new.center()
}

pub fn footer_import_centre() -> Pos2 {
    rects().footer_import.center()
}

pub fn footer_export_centre() -> Pos2 {
    rects().footer_export.center()
}

pub fn footer_close_centre() -> Pos2 {
    rects().footer_close.center()
}

/// The `BANKS` key on the panel, which opens the card.
pub fn banks_key() -> Pos2 {
    switches::cell(layout(screen()).switches, Switch::Banks).center()
}

/// The one way these tests build a panel: the library the card lists, and whether the session it is over is unsaved.
pub fn open_card(songs: Banks, dirty: bool) -> PanelState {
    PanelState {
        songs: SongState {
            card: Some(songs),
            ..SongState::default()
        },
        dirty,
        ..PanelState::default()
    }
}
