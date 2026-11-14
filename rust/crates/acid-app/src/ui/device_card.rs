//! The device card: one device's name, its role, the ports it hears, its channel, its MUTE and its removal, over the panel.
//!
//! The card paints and asks. It holds no device state of its own except what is being typed: `main` fills the mirror the
//! frame the card opens (`Card::offer`, from the rack and the machine's port list) and answers every row through
//! `PanelState::device_request`, because what a card shows is a reading of the machine and the panel does not own the rack.
//!
//! One card at a time: opening this one puts help, songs and settings away (`PanelState::open_device_card`), and it is
//! painted last, so nothing draws over it.

use eframe::egui::{self, Align2, Pos2, Rect};

use super::{PanelState, modal, text, theme, transport};
use crate::device::Request;
use crate::midi::route::{Channel, InputFilter, Role};

/// The card's padding, row height, gap and title, in design pixels: the settings card's own measurements, so the two cards
/// read as the same object.
const PADDING: f32 = 12.0;
const ROW: f32 = 22.0;
const GAP: f32 = 2.0;
const TITLE: f32 = 18.0;
/// The label column: every row's caption, left of its control.
const LABEL: f32 = 78.0;
/// How wide the card is, as a fraction of the panel's width, and the narrowest it may get.
const WIDTH: f32 = 0.46;
const MIN_WIDTH: f32 = 360.0;
/// The room the rows are laid out in: enough for a machine that reports this many ports. Nothing is capped by it — the
/// face is painted as tall as the rows actually reach — it only says where the card's top edge sits.
const ROOM_FOR_PORTS: usize = 6;
/// Air between the port list and the key rows, so `CHANNEL` does not read as another port.
const AIR: f32 = 8.0;

/// What the card is about, and what a hand has typed into it.
///
/// A mirror of the device plus the two things only the card owns: the string being typed and whether REMOVE is armed.
#[derive(Clone, Debug)]
pub struct Card {
    /// The device this card is about, or `None` while it is closed.
    pub open: Option<usize>,
    /// What is in the text field.
    pub name: String,
    /// Whether that device plays what it hears.
    pub role: Role,
    /// Every port the machine reports, and whether this device hears it. Filled from the device's own `Ports`, so the ticks
    /// are the filter's answer and not the widget's memory.
    pub ports: Vec<(String, bool)>,
    /// The channel the device listens on, in the store's numbering (`0..=15`, shown as `1..=16`).
    pub channel: Channel,
    /// Whether REMOVE is waiting for its second click.
    pub armed_remove: bool,
}

impl Default for Card {
    /// A card that is closed and has read nothing. Written out rather than derived so the permissive `PlayAndControl` /
    /// `All` pair stays a decision of the view: `midi::route` answers what a device hears, and it does not need a default
    /// that exists for a UI mirror nobody has filled yet.
    fn default() -> Self {
        Self {
            open: None,
            name: String::new(),
            role: Role::PlayAndControl,
            ports: Vec::new(),
            channel: Channel::All,
            armed_remove: false,
        }
    }
}
impl Card {
    /// Fill the card from a device: its name, its role and route, and the ports the machine reports.
    ///
    /// `main` calls this the frame the card opens, because the panel cannot read the rack and enumerating the ports costs a
    /// round trip. `armed_remove` falls back to disarmed: the arming belongs to one opening, never to the next.
    pub fn offer(&mut self, name: &str, inputs: &InputFilter, ports: Vec<String>) {
        self.name = name.to_string();
        self.role = inputs.role;
        self.channel = inputs.channel;
        self.armed_remove = false;
        self.ports = ports
            .into_iter()
            .map(|port| {
                let heard = inputs.ports.holds(&port);
                (port, heard)
            })
            .collect();
    }
}

/// The card's rect inside `panel`: centred, and wide enough for a name and a row of ports.
///
/// The height is the room the rows are laid out in, not the height that gets painted: `paint` draws the face as tall as the
/// rows it has, so a card with no ports is shorter than this rect and a machine with more ports than `ROOM_FOR_PORTS` grows
/// past it downward. Either way the rows are where `rows` puts them, which is what a test clicks.
pub fn card_rect(panel: Rect, s: f32) -> Rect {
    let rows = 3 + ROOM_FOR_PORTS + 1;
    let height = PADDING + TITLE + rows as f32 * (ROW + GAP) + AIR + ROW + PADDING;
    Rect::from_center_size(
        panel.center(),
        egui::vec2(
            panel.width().mul_add(WIDTH, 0.0).max(MIN_WIDTH * s),
            height * s,
        ),
    )
}

/// One rect per row of the card, in the order it paints them.
///
/// A test clicks the same place the card draws, so every rect here is where the row is painted. With no more ports than
/// `ROOM_FOR_PORTS` they are all inside `card_area`; past that the list keeps growing and the key rows fall below it, out
/// of the rect, which is why nothing caps the list and `paint` draws the face as tall as the rows reach.
pub struct Rows {
    pub name: Rect,
    pub role: Rect,
    pub all_ports: Rect,
    pub ports: Vec<Rect>,
    pub channel_down: Rect,
    pub channel_up: Rect,
    pub mute: Rect,
    pub remove: Rect,
    pub close: Rect,
}

pub fn rows(card_area: Rect, s: f32, card: &Card) -> Rows {
    let left = card_area.left() + PADDING * s;
    let right = card_area.right() - PADDING * s;
    let control_left = left + LABEL * s;
    let width = right - control_left;
    let row =
        |y: f32, w: f32| Rect::from_min_size(Pos2::new(control_left, y), egui::vec2(w, ROW * s));

    let mut y = card_area.top() + (PADDING + TITLE) * s;
    let name = row(y, width);
    y += (ROW + GAP) * s;
    let role = row(y, width * 0.45);
    y += (ROW + GAP) * s;
    let all_ports = row(y, width * 0.60);
    y += (ROW + GAP) * s;
    let ports: Vec<Rect> = (0..card.ports.len())
        .map(|index| row(y + index as f32 * (ROW + GAP) * s, width * 0.80))
        .collect();
    y += ports.len() as f32 * (ROW + GAP) * s + AIR * s;

    // The channel stepper: the words `-` and `+`, with the value between them. Triangles would be a glyph the bundled font
    // may not have, and the face may not use one.
    let channel_down = row(y, 30.0 * s);
    let channel_up = Rect::from_min_size(
        Pos2::new(channel_down.right() + 96.0 * s, y),
        egui::vec2(30.0 * s, ROW * s),
    );
    y += (ROW + GAP) * s + PADDING * s;

    let mute = row(y, 66.0 * s);
    let remove = Rect::from_min_size(
        Pos2::new(mute.right() + GAP * 2.0 * s, y),
        egui::vec2(84.0 * s, ROW * s),
    );
    let close = Rect::from_min_size(
        Pos2::new(right - 80.0 * s, y),
        egui::vec2(80.0 * s, ROW * s),
    );

    Rows {
        name,
        role,
        all_ports,
        ports,
        channel_down,
        channel_up,
        mute,
        remove,
        close,
    }
}

/// Paint the card over the panel, and ask for whatever a click meant.
///
/// The shade and the dismissal click are registered before the card's own rows, so a click on a row is the row's; the card
/// is painted after everything else, so nothing draws over it.
pub fn paint(ui: &mut egui::Ui, state: &mut PanelState, panel: Rect, s: f32) {
    let Some(index) = state.device_card.open else {
        return;
    };
    if index >= state.strip.cells.len() {
        // `main` owns the device list and the panel may be a frame behind a removal, so an index past the end is a stale
        // card rather than a device with nothing to show: it closes instead of reading a device that is gone.
        state.close_device_card();
        return;
    }

    let area = card_rect(panel, s);
    let rows = rows(area, s, &state.device_card);
    let shield = modal::shield(ui, ui.max_rect(), "device-card-shield");
    let face = Rect::from_min_size(
        area.min,
        egui::vec2(area.width(), rows.close.bottom() + PADDING * s - area.top()),
    );
    // The face is registered after the shade and swallows a click on it: a card is dismissed by CLOSE, Escape or a click
    // outside it, never by a click on one of its own rows — the rows that do not answer yet must not be dismissal targets
    // either, or a player aiming at MUTE loses the card.
    let _face = ui.interact(
        face,
        ui.scope_id().with("device-card-face"),
        egui::Sense::click(),
    );
    let title = if state.device_card.name.is_empty() {
        "DEVICE".to_string()
    } else {
        format!("DEVICE {}", state.device_card.name)
    };
    {
        let painter = ui.painter();
        painter.rect_filled(face, 6.0 * s, theme::PANEL_RAISED);
        painter.rect_stroke(
            face,
            6.0 * s,
            theme::silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
        // The title is the device's name, and a name is never truncated in the store (`Rack::rename` leaves that to the face
        // too), so it is fitted here: a long name ends in the one ellipsis the bundled font has, inside the card.
        let font = theme::legend(13.0 * s);
        let title = text::fit(
            painter,
            &title,
            font.clone(),
            face.width() - 2.0 * PADDING * s,
        );
        painter.text(
            Pos2::new(face.left() + PADDING * s, face.top() + PADDING * s),
            Align2::LEFT_TOP,
            title,
            font,
            theme::SILKSCREEN,
        );
    }

    let card = state.device_card.clone();
    for (caption, rect, value) in [
        ("ROLE", rows.role, role_label(card.role).to_string()),
        ("PORTS", rows.all_ports, "ALL PORTS".to_string()),
        ("CHANNEL", rows.channel_down, channel_label(card.channel)),
    ] {
        caption_row(ui.painter(), caption, rect, &value, s);
    }
    // The name is the one row that is a field rather than a caption. It is a widget, so it is registered after the face and
    // takes the clicks that land on it; Enter is what asks the rack, because a click that dismisses the card must not also
    // rename a device — you cannot take a rename back.
    // The caption says Enter because nothing else can: a field has no tooltip, and the card is not dismissed by clicking
    // away from it, so a player who types and clicks the face would otherwise never learn what applies the name.
    caption_row(ui.painter(), "NAME (ENTER)", rows.name, "", s);
    // The field sits in a display well: the pattern and tempo readouts wear the same one, so a name that can be typed into
    // reads as a readout of the device rather than as loose text on the card's face.
    {
        let painter = ui.painter();
        painter.rect_filled(rows.name, 3.0 * s, theme::WELL);
        painter.rect_stroke(
            rows.name,
            3.0 * s,
            theme::silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
    }
    let field = ui.put(
        rows.name,
        egui::TextEdit::singleline(&mut state.device_card.name)
            .id(ui.scope_id().with("device-card-name"))
            .hint_text("device name")
            .frame(egui::Frame::NONE)
            .text_color(theme::TEXT)
            .align(egui::Align2::LEFT_CENTER)
            .font(theme::legend(12.0 * s))
            .margin(egui::Margin::symmetric(6, 2))
            .desired_width(rows.name.width()),
    );
    if field.lost_focus()
        && ui.input(|input| input.key_pressed(egui::Key::Enter))
        && let Some(open) = state.device_card.open
    {
        state.device_request = Some(Request::Rename(open, state.device_card.name.clone()));
    }
    for (index, port) in card.ports.iter().enumerate() {
        let Some(rect) = rows.ports.get(index) else {
            break;
        };
        let tick = if port.1 { "[x]" } else { "[ ]" };
        caption_row(ui.painter(), "", *rect, &format!("{tick} {}", port.0), s);
    }
    caption_row(
        ui.painter(),
        "",
        rows.mute,
        mute_label(device_muted(&state.strip, index)),
        s,
    );

    // CLOSE is the card's own lifetime, so it is the one key that answers in this commit; the rest are drawn and wait for
    // the tasks that make them ask.
    let close = transport::transport_button(ui, "device-card-close", rows.close, "CLOSE", false);
    if shield.clicked() || close.clicked() {
        state.device_request = Some(Request::Close);
    }
}

/// The MUTE key's caption: what that device's mute is set to, not what a click would do.
fn mute_label(muted: bool) -> &'static str {
    if muted { "MUTE ON" } else { "MUTE OFF" }
}

/// Whether the device on the card is muted, read from the strip's mirror of the rack — the panel's only reading of it.
fn device_muted(strip: &super::strip::Strip, index: usize) -> bool {
    strip.cells.get(index).is_some_and(|cell| cell.muted)
}

/// One row: its caption in the label column, and its value in the control column.
fn caption_row(painter: &egui::Painter, caption: &str, control: Rect, value: &str, s: f32) {
    painter.text(
        Pos2::new(control.left() - LABEL * s + 2.0 * s, control.center().y),
        Align2::LEFT_CENTER,
        caption,
        theme::legend(10.0 * s),
        theme::SILKSCREEN_DIM,
    );
    painter.text(
        control.left_center(),
        Align2::LEFT_CENTER,
        value,
        theme::legend(11.0 * s),
        theme::SILKSCREEN,
    );
}

/// A role's caption on the card: what it does, not what the enum is called.
fn role_label(role: Role) -> &'static str {
    match role {
        Role::PlayAndControl => "PLAY + CTRL",
        Role::ControlOnly => "CTRL ONLY",
    }
}

/// A channel's caption on the card. The store is `0..=15` and a player counts from one, so `One(n)` is shown as `n + 1`.
fn channel_label(channel: Channel) -> String {
    match channel {
        Channel::All => "CH ANY".to_string(),
        Channel::One(only) => format!("CH {}", u16::from(only) + 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::vec2;

    fn panel() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(980.0, 670.0))
    }

    fn filled(ports: Vec<&str>) -> Card {
        let mut card = Card {
            open: Some(0),
            ..Card::default()
        };
        card.offer(
            "BASS",
            &InputFilter::everything(),
            ports.into_iter().map(str::to_string).collect(),
        );
        card
    }

    /// The rows are laid out top to bottom and the key row is last: a longer port list pushes MUTE, REMOVE and CLOSE down
    /// instead of drawing them over the ports, and with no more ports than the room `card_rect` allows they stay in the card.
    #[test]
    fn the_rows_sit_inside_the_card_and_the_key_rows_sit_below_the_ports() {
        let s = 1.0;
        let area = card_rect(panel(), s);
        let few = rows(area, s, &filled(vec!["Launchkey MIDI"]));
        let many = rows(area, s, &filled(vec!["a", "b"]));

        for rect in [
            few.name,
            few.role,
            few.all_ports,
            few.channel_down,
            few.channel_up,
            few.mute,
            few.remove,
            few.close,
        ] {
            assert!(area.contains_rect(rect), "{rect:?} is not in the card");
        }
        assert_eq!(few.ports.len(), 1);
        assert!(
            many.mute.top() > few.mute.top(),
            "a longer port list pushes the keys down instead of drawing over them"
        );
        assert!(many.channel_down.top() > few.channel_down.top());
        assert!(
            many.close.top() == many.mute.top() && many.remove.top() == many.mute.top(),
            "MUTE, REMOVE and CLOSE share one row"
        );
        assert!(
            many.mute.right() <= many.remove.left() && many.remove.right() <= many.close.left(),
            "and they do not overlap: {:?} {:?} {:?}",
            many.mute,
            many.remove,
            many.close
        );
    }

    /// The ticks are the filter's answer, not the widget's: `offer` asks `Ports` and nothing else.
    #[test]
    fn a_ports_tick_follows_the_devices_filter() {
        let mut card = Card::default();
        card.offer(
            "BASS",
            &InputFilter {
                ports: crate::midi::route::Ports::Named(vec!["nanoKONTROL".into()]),
                ..InputFilter::everything()
            },
            vec!["Launchkey MIDI".into(), "nanoKONTROL".into()],
        );
        assert_eq!(
            card.ports,
            vec![
                ("Launchkey MIDI".to_string(), false),
                ("nanoKONTROL".to_string(), true),
            ]
        );

        // A device that hears everything ticks every port the machine reports, without a list of its own.
        card.offer(
            "ACID 1",
            &InputFilter::everything(),
            vec!["Launchkey MIDI".into()],
        );
        assert_eq!(card.ports, vec![("Launchkey MIDI".to_string(), true)]);
        assert_eq!(
            card.name, "ACID 1",
            "and the name comes back from the store"
        );
    }

    /// The card shows the store's numbering: the engine counts channels from zero, the face counts from one.
    #[test]
    fn the_channel_is_shown_one_higher_than_it_is_stored() {
        assert_eq!(channel_label(Channel::All), "CH ANY");
        assert_eq!(channel_label(Channel::One(0)), "CH 1");
        assert_eq!(channel_label(Channel::One(15)), "CH 16");
    }
}
