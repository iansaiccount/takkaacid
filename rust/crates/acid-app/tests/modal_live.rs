//! Every card over the panel behaves the same: the face dims behind it, and a click outside takes it away.
//!
//! One scrim and one shield (`ui::modal`), so a fourth card cannot invent a third idiom — and so the panel keeps reading
//! as the thing behind the card rather than as a picture of one.

use acid_app::state::SharedState;
use acid_app::ui::banks::Banks;
use acid_app::ui::{PanelState, draw_panel, theme};
use eframe::egui::{
    Color32, Context, Event, FullOutput, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape,
    vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, vec2(980.0, 560.0))
}

fn run_on(
    ctx: &Context,
    panel: &mut PanelState,
    shared: &SharedState,
    events: Vec<Event>,
) -> FullOutput {
    let input = RawInput {
        screen_rect: Some(screen()),
        events,
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    output
}

/// A full click needs the widget to exist one frame earlier, so the pointer settles first.
fn click(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) {
    let button = |pressed: bool| Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::default(),
    };
    run_on(ctx, panel, shared, vec![Event::PointerMoved(at)]);
    run_on(ctx, panel, shared, vec![button(true)]);
    run_on(ctx, panel, shared, vec![button(false)]);
}

/// Every filled rectangle egui drew this frame, with its colour.
fn filled_rects(output: &FullOutput) -> Vec<(Rect, Color32)> {
    fn walk(shape: &Shape, out: &mut Vec<(Rect, Color32)>) {
        match shape {
            Shape::Rect(rect) => out.push((rect.rect, rect.fill)),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| walk(shape, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    for clipped in &output.shapes {
        walk(&clipped.shape, &mut out);
    }
    out
}

/// Whether the window itself was darkened: a dark fill over the whole window, which is the scrim and nothing else — the
/// face and the frame are inset from the window.
fn face_is_dimmed(output: &FullOutput) -> bool {
    filled_rects(output).iter().any(|(rect, fill)| {
        *rect == screen()
            && fill.a() > 0
            && theme::luminance(*fill) < theme::luminance(theme::BLACK.panel)
    })
}

/// One panel per card, each open, each built from `PanelState` alone so the test needs no window.
///
/// Every card, not the three there used to be: the kit browser is panel-sized and opaque, so a missing entry here is a card a
/// player can be stuck behind.
fn every_open_card() -> [(&'static str, PanelState); 5] {
    let with_cell = || {
        let mut panel = PanelState::default();
        panel.strip.cells = vec![acid_app::ui::strip::StripCell {
            name: "DRUMS 1".to_string(),
            number: 1,
            scheme: acid_app::device::Scheme::Black,
            running: false,
            muted: false,
            kind: acid_app::device::DeviceKind::Drums,
        }];
        panel
    };
    let mut songs = PanelState::default();
    songs.songs.card = Some(Banks::default());
    let mut device = with_cell();
    device.device_card.open = Some(0);
    let mut kits = with_cell();
    kits.kits.open = true;
    [
        (
            "help",
            PanelState {
                help_visible: true,
                ..PanelState::default()
            },
        ),
        ("songs", songs),
        (
            "settings",
            PanelState {
                settings_open: true,
                ..PanelState::default()
            },
        ),
        ("device", device),
        ("kits", kits),
    ]
}

#[test]
fn every_card_dims_the_face_behind_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    for (name, mut panel) in every_open_card() {
        let output = run_on(&ctx, &mut panel, &shared, Vec::new());
        assert!(
            face_is_dimmed(&output),
            "the {name} card must dim the face behind it: the light on the panel is what says a card is over it"
        );
    }
}

#[test]
fn a_card_that_is_shut_does_not_dim_anything() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let output = run_on(&ctx, &mut panel, &shared, Vec::new());
    assert!(
        !face_is_dimmed(&output),
        "the living room should be lit when there is nothing over it"
    );
}

#[test]
fn every_card_dismisses_on_a_click_outside_the_panel() {
    // The corner is outside the panel's frame, so this is the letterbox and the margin as well as the dark scrim.
    let at = Pos2::new(4.0, 4.0);
    for (name, mut panel) in every_open_card() {
        let ctx = Context::default();
        let shared = SharedState::new();
        click(&ctx, &mut panel, &shared, at);
        // Every card asks for its own dismissal in its own way: three are panel state and close here, the kit browser goes
        // through `kits::close`, and the device card asks `main` — the card is a reading of the rack, so the rack is what
        // closes it. The answer is still the same click.
        let still_up = match name {
            "help" => panel.help_visible,
            "songs" => panel.songs.card.is_some(),
            "settings" => panel.settings_open,
            "device" => panel.device_request != Some(acid_app::device::Request::Close),
            "kits" => panel.kits.open,
            other => panic!("{other} is not a card this test knows"),
        };
        assert!(
            !still_up,
            "the {name} card must close from outside: {still_up:?}"
        );
        assert!(
            panel.device_request.is_none() || name == "device",
            "a click meant for the {name} card must not also reach a device"
        );
    }
}
