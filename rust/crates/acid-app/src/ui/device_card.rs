//! The device card: one device's name, its role, the ports it hears, its channel, its MUTE and its removal, over the panel.
//!
//! The card paints and asks. It holds no device state of its own except what is being typed: `main` fills it the frame it
//! opens (`Card::offer`, from the rack and the machine's port list — a round trip, so not per frame) and re-reads the *route*
//! every frame (`Card::route_from`, which costs nothing), and every row answers through `PanelState::device_request`, because
//! what a card shows is a reading of the machine and the panel does not own the rack.
//!
//! One card at a time: opening this one puts help, songs and settings away (`PanelState::open_device_card`), and it is
//! One card at a time: opening this one puts help, songs, settings and the picker away (`PanelState::open_device_card`), and
//! it is painted over everything but the picker.

use eframe::egui::{self, Align2, Pos2, Rect};

use super::{PanelState, clear, modal, text, theme, tips, transport};
use crate::device::Request;
use crate::midi::route::{Channel, InputFilter, Ports, Role};

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
    /// Which theme the device wears. A reading like the route: the key shows this, and a click on it opens the list.
    pub scheme: crate::device::Scheme,
    /// The theme list that key opens, and where its window sits in the catalogue.
    pub themes: Themes,
    /// When REMOVE was first clicked: the same two-click switch `PanelState::clear_armed_at` is, and spent the same way. `None`
    /// means the key says `REMOVE`; a fresh `Some` means it says `CONFIRM?` and the next click asks the rack.
    pub armed_remove: Option<std::time::Instant>,
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
            scheme: crate::device::Scheme::Black,
            themes: Themes::default(),
            armed_remove: None,
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
        self.armed_remove = None;
        self.ports = ports.into_iter().map(|port| (port, false)).collect();
        // A port the filter names but the machine no longer reports keeps its row. Unplugging a controller must not quietly
        // drop it from the filter the next time any tick is clicked — the click rebuilds the whole set — and a player has to
        // be able to see what the device is still listening for, and to untick it.
        if let Ports::Named(named) = &inputs.ports {
            for port in named {
                if !self.ports.iter().any(|(listed, _)| listed == port) {
                    self.ports.push((port.clone(), false));
                }
            }
        }
        self.route_from(inputs);
    }

    /// Adopt the route the rack holds: the role, the channel, and one tick per port the card is showing.
    ///
    /// `paint` reads the tick from here and asks; nothing on the card remembers a tick the filter does not hold, so a
    /// reopened card cannot show a tick that was never set (Review Focus 2). `main` calls this every frame while the card
    /// is open — it costs no allocation and no port enumeration — and `offer` calls it when the card opens.
    pub fn route_from(&mut self, inputs: &InputFilter) {
        self.role = inputs.role;
        self.channel = inputs.channel;
        for (port, heard) in &mut self.ports {
            *heard = inputs.ports.holds(port);
        }
    }

    /// The card's reading of the device's colour. Not part of a route, but a reading all the same: the key shows what the rack
    /// holds, so a click that changes it moves the key on the next frame and not before.
    pub fn scheme_from(&mut self, scheme: crate::device::Scheme) {
        self.scheme = scheme;
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

/// The picker: a card that offers the kinds of device there are, with one honest line about what is not saved yet.
///
/// It is its own value rather than `device_card`'s flag because it is a different card — its own geometry, its own footnote
/// and its own answer (`Add`), and the two are never up together.
/// The theme list: a drop-down over the card, because twenty-two themes do not fit one key.
///
/// It lives on the card rather than in a card of its own: it is the same device's question, it is painted over the card's
/// face, and it dies with the card. `top` is the first row drawn, so the wheel and the clamp are the only two things that
/// move it and the geometry is the only place the window is decided.
#[derive(Clone, Debug, Default)]
pub struct Themes {
    pub open: bool,
    pub top: usize,
}

#[derive(Clone, Debug, Default)]
pub struct Picker {
    pub open: bool,
}

/// How wide the picker is, as a fraction of the panel's width, and the narrowest it may get. Narrower than the device card:
/// a kind's name is three or four letters and the footnote is one line.
const PICKER_WIDTH: f32 = 0.32;
const PICKER_MIN_WIDTH: f32 = 280.0;
/// The blank line the footnote sits on, and the type it is set in.
const FOOTNOTE: f32 = 14.0;
/// The sentence the round keeps: what a restart costs. It disappears in the round that adds `SavedState::devices`.
const NOT_SAVED: &str = "NAMES AND EXTRA DEVICES ARE NOT SAVED YET";

/// The picker's rect inside `panel`: centred, and as tall as its rows, its footnote and its one key need.
pub fn picker_rect(panel: Rect, s: f32) -> Rect {
    let kinds = crate::device::DeviceKind::ALL.len();
    let height = PADDING + TITLE + kinds as f32 * (ROW + GAP) + FOOTNOTE + ROW + PADDING;
    Rect::from_center_size(
        panel.center(),
        egui::vec2(
            panel
                .width()
                .mul_add(PICKER_WIDTH, 0.0)
                .max(PICKER_MIN_WIDTH * s),
            height * s,
        ),
    )
}

/// One rect per kind, in `DeviceKind::ALL` order, and the CLOSE key under them.
pub struct PickerRows {
    pub kinds: Vec<Rect>,
    pub close: Rect,
}

/// Lay the picker out. The count comes from `DeviceKind::ALL`, so a second kind is a variant and a row here.
pub fn picker_rows(area: Rect, s: f32) -> PickerRows {
    let left = area.left() + PADDING * s;
    let width = area.width() - PADDING * 2.0 * s;
    let top = area.top() + (PADDING + TITLE) * s;
    let kinds: Vec<Rect> = (0..crate::device::DeviceKind::ALL.len())
        .map(|index| {
            Rect::from_min_size(
                Pos2::new(left, top + index as f32 * (ROW + GAP) * s),
                egui::vec2(width, ROW * s),
            )
        })
        .collect();
    let close = Rect::from_min_size(
        Pos2::new(
            area.right() - PADDING * s - 80.0 * s,
            area.top() + area.height() - PADDING * s - ROW * s,
        ),
        egui::vec2(80.0 * s, ROW * s),
    );
    PickerRows { kinds, close }
}

/// Paint the picker and ask for whatever a click meant. Like the device card it is a shield, a face, its rows and one key;
/// unlike the device card it closes itself, because it holds nothing that `main` had to read.
pub fn paint_picker(ui: &mut egui::Ui, state: &mut PanelState, panel: Rect, s: f32) {
    if !state.device_picker.open {
        return;
    }
    let area = picker_rect(panel, s);
    let rows = picker_rows(area, s);
    let shield = modal::shield(ui, ui.max_rect(), "device-picker-shield");
    {
        let painter = ui.painter();
        painter.rect_filled(area, 6.0 * s, theme::BLACK.panel_raised);
        painter.rect_stroke(
            area,
            6.0 * s,
            theme::BLACK.silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
        painter.text(
            Pos2::new(area.left() + PADDING * s, area.top() + PADDING * s),
            Align2::LEFT_TOP,
            "ADD DEVICE",
            theme::legend(13.0 * s),
            theme::BLACK.silkscreen,
        );
    }

    let mut asked: Option<Request> = None;
    let count = state.strip.cells.len();
    for (index, kind) in crate::device::DeviceKind::ALL.iter().enumerate() {
        let Some(rect) = rows.kinds.get(index).copied() else {
            return;
        };
        // A greyed row is the unlit face: recessed and plain instead of raised and amber. It still answers a click, so the
        // refusal is `Rack::add`'s to say — the same reason the strip's `+` stays clickable at the cap.
        let fits = kind.enabled(count);
        if transport::card_button(
            ui,
            &format!("device-picker-kind-{index}"),
            rect,
            kind.name(),
            fits,
        )
        .clicked()
        {
            asked = Some(Request::Add(*kind));
        }
    }

    // The footnote is the promise this round keeps, so it is a line on the card and not a notice that clears itself.
    ui.painter().text(
        Pos2::new(
            area.left() + PADDING * s,
            rows.kinds.last().map_or(area.top(), |last| last.bottom()) + FOOTNOTE * 0.5 * s,
        ),
        Align2::LEFT_CENTER,
        NOT_SAVED,
        theme::legend(10.0 * s),
        theme::BLACK.silkscreen_dim,
    );

    if transport::card_button(ui, "device-picker-close", rows.close, "CLOSE", false).clicked()
        || shield.clicked()
    {
        // Closing is this card's own state and says nothing to the rack: it has no mirror to drop.
        asked = None;
        state.device_picker.open = false;
    }
    if let Some(request) = asked {
        // A row that was clicked has answered the question the card asked, so the card goes.
        state.device_picker.open = false;
        state.device_request = Some(request);
    }
}

/// The theme list's geometry: the popup and one rect per visible row, plus the window's first row after clamping.
///
/// The list opens away from the key that opened it — over the card's own face, never under the key itself — and is capped by
/// the panel, so it is the panel's height and not the catalogue that decides how many rows are on screen.
pub struct ThemeRows {
    pub list: Rect,
    pub rows: Vec<Rect>,
    pub top: usize,
}

pub fn theme_rows(rows: &Rows, panel: Rect, s: f32, top: usize) -> ThemeRows {
    let count = crate::device::Scheme::ALL.len();
    let row_height = (ROW + GAP) * s;
    let pad = 4.0 * s;
    let width = rows.scheme.width().max(190.0 * s);
    let above = rows.scheme.top() - panel.top() - 12.0 * s;
    let below = panel.bottom() - rows.scheme.bottom() - 12.0 * s;
    let (up, space) = if above >= below {
        (true, above)
    } else {
        (false, below)
    };
    let visible = (((space - pad * 2.0) / row_height).floor() as usize).clamp(3, count);
    let height = visible as f32 * row_height + pad * 2.0;
    // A gap, not an abutment: the list must not even touch the key, or the key's own click area would sit under its edge.
    let y = if up {
        rows.scheme.top() - GAP * s - height
    } else {
        rows.scheme.bottom() + GAP * s
    };
    let list = Rect::from_min_size(Pos2::new(rows.scheme.left(), y), egui::vec2(width, height));
    let top = top.min(count.saturating_sub(visible));
    let rows = (0..visible)
        .map(|index| {
            Rect::from_min_size(
                Pos2::new(
                    list.left() + pad,
                    list.top() + pad + index as f32 * row_height,
                ),
                egui::vec2(width - pad * 2.0, ROW * s),
            )
        })
        .collect();
    ThemeRows { list, rows, top }
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
    /// The `COLOUR` key: which way round the device paints its face, and a click asks for the other one.
    pub scheme: Rect,
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
    y += (ROW + GAP) * s;

    // COLOUR: one key, captioned with the scheme the device is on, because there are two and the row already says which.
    let scheme = row(y, width * 0.45);
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
        scheme,
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
    // What this frame's clicks asked for. It is one slot, so a frame can only mean one thing; the card collects it here and
    // writes it once, at the end, so that "any other key puts the arming away" can be one rule rather than eight.
    let mut asked: Option<Request> = None;
    // One clock reading for the frame: the arming window is measured against the same instant the click is answered at.
    let now = std::time::Instant::now();
    let shield = modal::shield(ui, ui.max_rect(), "device-card-shield");
    let face = Rect::from_min_size(
        area.min,
        egui::vec2(area.width(), rows.close.bottom() + PADDING * s - area.top()),
    );
    // The face is registered after the shade and swallows a click on it: a card is dismissed by CLOSE, Escape or a click
    // outside it, never by a click on one of its own rows, or a player aiming at a key that misses it by a pixel would lose
    // the card.
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
        painter.rect_filled(face, 6.0 * s, theme::BLACK.panel_raised);
        painter.rect_stroke(
            face,
            6.0 * s,
            theme::BLACK.silkscreen_stroke(s),
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
            theme::BLACK.silkscreen,
        );
    }

    // Every row's caption lives in the label column; the controls themselves are widgets, registered below so they win
    // the clicks that land on them.
    for (text, rect) in [
        ("NAME (ENTER)", rows.name),
        ("ROLE", rows.role),
        ("PORTS", rows.all_ports),
        ("CHANNEL", rows.channel_down),
        ("COLOUR", rows.scheme),
    ] {
        caption(ui.painter(), text, rect, s);
    }

    // The name is the one row that is a field rather than a key. It is a widget, so it is registered after the face and
    // takes the clicks that land on it; Enter is what asks the rack, because a click that dismisses the card must not also
    // rename a device — you cannot take a rename back. The caption says Enter because nothing else can: a field has no
    // tooltip, and clicking the card's own face keeps the card up, so a player would otherwise never learn what applies it.
    //
    // The field sits in a display well: the pattern and tempo readouts wear the same one, so a name that can be typed into
    // reads as a readout of the device rather than as loose text on the card's face.
    {
        let painter = ui.painter();
        painter.rect_filled(rows.name, 3.0 * s, theme::BLACK.well);
        painter.rect_stroke(
            rows.name,
            3.0 * s,
            theme::BLACK.silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
    }
    let field = ui.put(
        rows.name,
        egui::TextEdit::singleline(&mut state.device_card.name)
            .id(ui.scope_id().with("device-card-name"))
            .hint_text("device name")
            .frame(egui::Frame::NONE)
            .text_color(theme::BLACK.text)
            .align(egui::Align2::LEFT_CENTER)
            .font(theme::legend(12.0 * s))
            .margin(egui::Margin::symmetric(6, 2))
            .desired_width(rows.name.width()),
    );
    if field.lost_focus()
        && ui.input(|input| input.key_pressed(egui::Key::Enter))
        && let Some(open) = state.device_card.open
    {
        asked = Some(Request::Rename(open, state.device_card.name.clone()));
    }

    // ROLE: what the device does with what it hears. `CTRL ONLY` is the switched-on state, because it is the one that
    // changes what the panel does — a controller-only device stops answering the keyboard with notes.
    let role = state.device_card.role;
    let role_key = transport::card_button(
        ui,
        "device-card-role",
        rows.role,
        role_label(role),
        role == Role::ControlOnly,
    );
    if role_key.clicked() {
        asked = Some(Request::SetRole(index, next_role(role)));
    }

    // PORTS: `ALL PORTS` is the factory filter and the way back to it; each row below is one of the machine's ports with the
    // tick the device's own `Ports` gives it, so nothing here remembers a tick the filter does not hold.
    // `ALL PORTS` is lit only when there is something to hear and this device hears all of it: a machine with no ports
    // reported says nothing about whether a device hears everything, and lighting the key would claim it does.
    let all_ticked = !state.device_card.ports.is_empty()
        && state.device_card.ports.iter().all(|(_, heard)| *heard);
    let all_ports = transport::card_button(
        ui,
        "device-card-all-ports",
        rows.all_ports,
        "ALL PORTS",
        all_ticked,
    );
    if all_ports.clicked() {
        asked = Some(Request::SetPorts(index, Ports::All));
    }
    for port_index in 0..state.device_card.ports.len() {
        let Some(rect) = rows.ports.get(port_index).copied() else {
            break;
        };
        let (heard, label) = {
            let (port, heard) = &state.device_card.ports[port_index];
            // Fitted, because a port's name is the machine's and can be as long as it likes: a DAW's forwarding name
            // overran the row and spilled onto the shade. The font is the one the key will draw it in.
            let label = format!("{} {port}", if *heard { "[x]" } else { "[ ]" });
            let font = transport::key_font(rect);
            (
                *heard,
                text::fit(ui.painter(), &label, font, rect.width() - 8.0 * s),
            )
        };
        let key = transport::card_button(
            ui,
            &format!("device-card-port-{port_index}"),
            rect,
            &label,
            heard,
        );
        if key.clicked() {
            let next = toggled(&state.device_card.ports, port_index);
            asked = Some(Request::SetPorts(index, next));
        }
    }

    // CHANNEL: two keys with the value between them, because the face may not use a triangle from a system font.
    let channel = state.device_card.channel;
    let down = transport::card_button(
        ui,
        "device-card-channel-down",
        rows.channel_down,
        "-",
        false,
    );
    if down.clicked() {
        asked = Some(Request::SetChannel(index, prev_channel(channel)));
    }
    ui.painter().text(
        Pos2::new(
            rows.channel_down.right() + 8.0 * s,
            rows.channel_down.center().y,
        ),
        Align2::LEFT_CENTER,
        channel_label(channel),
        theme::legend(11.0 * s),
        theme::BLACK.silkscreen,
    );
    let up = transport::card_button(ui, "device-card-channel-up", rows.channel_up, "+", false);
    if up.clicked() {
        asked = Some(Request::SetChannel(index, next_channel(channel)));
    }

    // COLOUR: the theme this device wears, and the key that opens the list of them. A key that shows the current theme and
    // opens one list beats a key that cycles: twenty-two themes is a lottery with twenty-two tickets.
    let scheme = state.device_card.scheme;
    let colour = transport::card_button(
        ui,
        "device-card-colour",
        rows.scheme,
        scheme.name(),
        state.device_card.themes.open,
    );
    if colour.clicked() {
        state.device_card.themes.open = !state.device_card.themes.open;
        if state.device_card.themes.open {
            // Open on the theme in use: the window is this list's, and a list that opened at its first row would hide the one
            // row the player can be sure they want to see.
            let window = theme_rows(&rows, panel, s, 0).rows.len();
            let count = crate::device::Scheme::ALL.len();
            state.device_card.themes.top = scheme
                .index()
                .saturating_sub(window / 2)
                .min(count.saturating_sub(window));
        }
    }

    // MUTE: this device's own mute, read from the strip's mirror of the rack — the panel's only reading of it.
    let muted = device_muted(&state.strip, index);
    let mute = transport::card_button(ui, "device-card-mute", rows.mute, mute_label(muted), muted);
    if mute.clicked() {
        asked = Some(Request::SetMute(index, !muted));
    }
    tips::hover(&mute, state.tooltips, tips::Control::DeviceMute);

    // REMOVE: the second key on the face that takes two clicks, and the same two-click rule as CLEAR — `clear::click` is
    // the one place "is this armed?" is answered, so a pause longer than the window makes the next click a question again
    // rather than an answer.
    let armed = clear::is_armed(state.device_card.armed_remove, now);
    let remove = transport::card_button(
        ui,
        "device-card-remove",
        rows.remove,
        if armed { "CONFIRM?" } else { "REMOVE" },
        armed,
    );
    if remove.clicked() {
        match clear::click(state.device_card.armed_remove, now) {
            clear::Click::Armed => {
                state.device_card.armed_remove = Some(now);
                asked = Some(Request::AskRemove(index));
            }
            clear::Click::Cleared => {
                state.device_card.armed_remove = None;
                asked = Some(Request::ConfirmRemove(index));
            }
        }
    }

    // CLOSE is the card's own lifetime.
    let close = transport::card_button(ui, "device-card-close", rows.close, "CLOSE", false);
    if shield.clicked() || close.clicked() {
        asked = Some(Request::Close);
    }

    paint_themes(ui, state, panel, s, &rows, &mut asked, index);

    // One write, and the arming's one rule: any other key on the card answers a different question, so it puts the question
    // away. A click that asked nothing leaves whatever the frame already held — a strip cell's click must survive the card.
    if let Some(request) = asked {
        if !matches!(request, Request::AskRemove(_) | Request::ConfirmRemove(_)) {
            state.device_card.armed_remove = None;
        }
        state.device_request = Some(request);
    }
}

/// Paint the theme list and answer a pick.
///
/// Its shade is registered last, so while the list is up a click anywhere else is the list's: it puts the list away and does
/// not reach the card behind it. That is the one rule a drop-down needs — the key that opened it must not also be the thing
/// that a click meant for a row lands on.
fn paint_themes(
    ui: &mut egui::Ui,
    state: &mut PanelState,
    panel: Rect,
    s: f32,
    rows: &Rows,
    asked: &mut Option<Request>,
    device: usize,
) {
    if !state.device_card.themes.open {
        return;
    }
    let current = state.device_card.scheme;
    let count = crate::device::Scheme::ALL.len();
    let window = theme_rows(rows, panel, s, state.device_card.themes.top);
    state.device_card.themes.top = window.top;

    // The wheel scrolls the window and stops at either end. Read only while the pointer is over the list, so a wheel over the
    // panel behind it keeps doing whatever the panel does with a wheel.
    let over = ui
        .input(|input| input.pointer.hover_pos())
        .is_some_and(|at| window.list.contains(at));
    if over {
        let scrolled = ui.input(|input| input.smooth_scroll_delta.y);
        if scrolled != 0.0 {
            // Only the floor is clamped here: the ceiling belongs to `theme_rows`, which is the one place that decides what
            // window exists. Two clamps would mean mutating either one is invisible to every test.
            let step = (scrolled / ((ROW + GAP) * s)).round() as i64;
            state.device_card.themes.top = (window.top as i64 - step).max(0) as usize;
        }
    }
    let window = theme_rows(rows, panel, s, state.device_card.themes.top);
    state.device_card.themes.top = window.top;

    let shield = modal::shield(ui, ui.max_rect(), "device-card-themes-shield");
    let _face = ui.interact(
        window.list,
        ui.scope_id().with("device-card-themes"),
        egui::Sense::click(),
    );
    {
        let painter = ui.painter();
        painter.rect_filled(window.list, 4.0 * s, theme::BLACK.panel_raised);
        painter.rect_stroke(
            window.list,
            4.0 * s,
            theme::BLACK.silkscreen_stroke(s),
            egui::StrokeKind::Inside,
        );
        // The window is about half the catalogue, and nothing else on the list says there is more of it below: a thumb on
        // the right edge, as tall as the window's share and as far down as the window has scrolled. No glyph, and no click
        // of its own — the wheel and the rows are the list's input.
        if window.rows.len() < count.max(1) {
            let share = window.rows.len() as f32 / count as f32;
            let track = (window.list.height() - 8.0 * s).max(1.0);
            let thumb = (track * share).max(8.0 * s);
            let travel = track - thumb;
            let last = count.saturating_sub(window.rows.len());
            let at = match last {
                0 => 0.0,
                last => window.top as f32 / last as f32,
            };
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(
                        window.list.right() - 3.0 * s,
                        window.list.top() + 4.0 * s + travel * at,
                    ),
                    egui::vec2(2.0 * s, thumb),
                ),
                1.0 * s,
                theme::BLACK.silkscreen_dim,
            );
        }
    }
    for (offset, rect) in window.rows.iter().enumerate() {
        let Some(scheme) = crate::device::Scheme::ALL.get(window.top + offset).copied() else {
            break;
        };
        let key = ui.interact(
            *rect,
            ui.scope_id()
                .with(("device-card-theme", window.top + offset)),
            egui::Sense::click(),
        );
        let painter = ui.painter();
        if key.hovered() {
            painter.rect_filled(rect.shrink(s), 3.0 * s, theme::BLACK.well);
        }
        // The swatch is the theme's own face, bordered in its own dim legend: a theme whose face and raised plate are close
        // together still reads as a square, which is why the border is the theme's and not the card's.
        let palette = theme::palette(scheme);
        let swatch = Rect::from_center_size(
            Pos2::new(rect.left() + 10.0 * s, rect.center().y),
            egui::vec2(12.0 * s, 12.0 * s),
        );
        painter.rect_filled(swatch, 2.0 * s, palette.panel);
        painter.rect_stroke(
            swatch,
            2.0 * s,
            egui::Stroke::new(s, palette.silkscreen_dim),
            egui::StrokeKind::Inside,
        );
        let font = theme::legend(11.0 * s);
        let name = text::fit(
            painter,
            scheme.name(),
            font.clone(),
            rect.width() - 34.0 * s,
        );
        painter.text(
            Pos2::new(rect.left() + 22.0 * s, rect.center().y),
            Align2::LEFT_CENTER,
            name,
            font,
            theme::BLACK.silkscreen,
        );
        if scheme == current {
            // A dot, not a tick: the face may not use a font glyph, and the theme in use is the one thing this list has to say
            // without being clicked.
            painter.circle_filled(
                Pos2::new(rect.right() - 8.0 * s, rect.center().y),
                3.5 * s,
                theme::BLACK.led_on,
            );
        }
        if key.clicked() {
            *asked = Some(Request::SetScheme(device, scheme));
            state.device_card.themes.open = false;
        }
    }
    if shield.clicked() {
        state.device_card.themes.open = false;
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

/// One row's caption, in the label column to the left of its control.
fn caption(painter: &egui::Painter, caption: &str, control: Rect, s: f32) {
    painter.text(
        Pos2::new(control.left() - LABEL * s + 2.0 * s, control.center().y),
        Align2::LEFT_CENTER,
        caption,
        theme::legend(10.0 * s),
        theme::BLACK.silkscreen_dim,
    );
}

/// The role the role key asks for: the one the device does not do now.
fn next_role(role: Role) -> Role {
    match role {
        Role::PlayAndControl => Role::ControlOnly,
        Role::ControlOnly => Role::PlayAndControl,
    }
}

/// The channel one step up: past `ANY` comes channel 1, and past the last channel comes `ANY` again, so leaving `ANY` is
/// never a one-way door.
fn next_channel(channel: Channel) -> Channel {
    match channel {
        Channel::All => Channel::One(0),
        Channel::One(15) => Channel::All,
        Channel::One(only) => Channel::One(only + 1),
    }
}

/// The channel one step down, the other way round: below channel 1 is `ANY`, and below `ANY` is the last channel.
fn prev_channel(channel: Channel) -> Channel {
    match channel {
        Channel::All => Channel::One(15),
        Channel::One(0) => Channel::All,
        Channel::One(only) => Channel::One(only - 1),
    }
}

/// The port set a click on one row asks for: every port's tick as the card shows it, with this one flipped, in the machine's
/// order. A tick clicked from `ALL PORTS` therefore produces the list of every port but that one, which is the honest reading
/// of "I do not want this one"; a click that leaves nothing ticked produces `Named(vec![])`, "silent", which is the state a
/// device added by `+` starts in.
fn toggled(ports: &[(String, bool)], index: usize) -> Ports {
    Ports::Named(
        ports
            .iter()
            .enumerate()
            .filter(
                |(other, (_, heard))| {
                    if *other == index { !heard } else { *heard }
                },
            )
            .map(|(_, (port, _))| port.clone())
            .collect(),
    )
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
            few.scheme,
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

    /// A port the filter still names after the machine stopped reporting it keeps its row. The click on any row rebuilds the
    /// whole set from the rows, so without this an unplugged controller would be dropped from the filter silently — the
    /// device would stop listening for the cable the moment the player routed a *different* port.
    #[test]
    fn a_port_the_machine_no_longer_reports_keeps_its_row() {
        let mut card = Card::default();
        card.offer(
            "BASS",
            &InputFilter {
                ports: crate::midi::route::Ports::Named(vec![
                    "Launchkey MIDI".into(),
                    "gone with the cable".into(),
                ]),
                ..InputFilter::everything()
            },
            vec!["Launchkey MIDI".into()],
        );
        assert_eq!(
            card.ports,
            vec![
                ("Launchkey MIDI".to_string(), true),
                ("gone with the cable".to_string(), true),
            ],
            "the machine's list first, then what the filter still holds"
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
