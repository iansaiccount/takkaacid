//! The settings card: tooltips, autosave, MIDI in, sound out — everything the top strip and the bottom line used to hold,
//! in one place that opens with F2 or the gear and closes when you are done.
//!
//! The two device lists are collapsed sections with the device in use named above them: a machine with a dozen outputs
//! must not let its list eat the screen. An open section scrolls once it has more rows than fit.
//!
//! Pure painting plus a request enum, like the songs and help cards: `main` owns the ports, the streams and the probes.

use eframe::egui::{self, Align2, Pos2, Rect, Ui, UiBuilder};

use crate::audio::status as audio_status;
use crate::midi::ports::PortChoice;

use super::theme;
use super::tips;
use super::transport;
/// How many device rows an open section shows before it scrolls.
const VISIBLE_ROWS: usize = 4;

/// What the card shows besides the two switches. The two lists are what the machine offered when the card opened;
/// everything else on the card mirrors the machine and is copied while the card is up, because a device chosen, unplugged
/// or failed must show without closing and reopening it.
#[derive(Clone, Default)]
pub struct Settings {
    /// Every MIDI input the machine offers, as names.
    pub midi_ports: Vec<String>,
    /// Every audio output the machine offers.
    pub outputs: Vec<String>,
    /// The port actually connected, named above the panel so the answer is visible while the list is shut. Not the
    /// preference: on a first run the preference is "any" while a controller is playing, and the card must say which.
    pub midi_current: Option<String>,
    /// The output the stream is on, named above the panel for the same reason.
    pub output_current: Option<String>,
    /// True while the supervisor has stopped retrying: the card then offers RETRY, where the status line used to.
    pub midi_given_up: bool,
    /// The supervisor's own words, shown beside the MIDI IN heading.
    pub midi_status: String,
    /// The audio supervisor's own words, shown beside the SOUND OUT heading: the one place a failed stream is visible
    /// now that the bottom line is gone.
    pub audio_status: String,
    /// Whether the MIDI panel is unfolded. Collapsed by default: the card is visited for one thing at a time.
    pub midi_open: bool,
    /// Whether the sound panel is unfolded.
    pub sound_open: bool,
}

/// What is happening to the two devices right now. `main` builds this every frame the card is open; the card only
/// copies it, so nothing here queries a device.
pub struct Live<'a> {
    pub midi_current: Option<&'a str>,
    pub output_current: Option<&'a str>,
    pub midi_status: &'a str,
    pub midi_given_up: bool,
    pub audio_status: &'a str,
}

impl Settings {
    /// Offer what the machine has. This enumerates ports and devices, a round trip each, so `main` does it once per
    /// opening — on the rising edge of the card — and never per frame.
    pub fn offer(&mut self, midi_ports: Vec<String>, outputs: Vec<String>) {
        self.midi_ports = midi_ports;
        self.outputs = outputs;
    }

    /// Copy what is happening to the devices and their two supervisors. Cheap, and it must be every frame: a stream
    /// that fails, a supervisor that gives up 30 s after the card opened, or an output the player just chose must show
    /// on the card that is already up.
    pub fn follow(&mut self, live: Live<'_>) {
        self.midi_current = live.midi_current.map(str::to_string);
        self.output_current = live.output_current.map(str::to_string);
        self.midi_status = live.midi_status.to_string();
        self.midi_given_up = live.midi_given_up;
        self.audio_status = live.audio_status.to_string();
    }
}

/// What the card asked for. `main` does the work; the two folds are panel state and are also applied here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Flip the hover help.
    Tooltips,
    /// Flip autosave.
    Autosave,
    /// Use this input, or `None` for "any port that looks like a controller".
    MidiIn(Option<String>),
    /// Move the stream to this output.
    SoundOut(String),
    /// Ask the supervisor to probe again.
    Retry,
    /// Fold or unfold the MIDI panel.
    ToggleMidi,
    /// Fold or unfold the sound panel.
    ToggleSound,
    Close,
}

/// The device a row chose, in the shape the picker keeps. `main` writes it into `PickerState`, which the stream handling
/// already consumes — the card is a new surface on the same mechanism, not a second one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Pick {
    Midi(PortChoice),
    Sound(String),
}

impl Request {
    /// The device this request chooses, or `None` for the ones that flip something or close the card. This is the only
    /// place that answers "what does this row choose", which is why it lives beside the request rather than in `main`:
    /// `main` is a binary, so a mapping kept there could not be tested.
    pub fn pick(&self) -> Option<Pick> {
        match self {
            Request::MidiIn(Some(name)) => Some(Pick::Midi(PortChoice::Named(name.clone()))),
            // The `none` row is the app picking a controller for itself.
            Request::MidiIn(None) => Some(Pick::Midi(PortChoice::Any)),
            Request::SoundOut(name) => Some(Pick::Sound(name.clone())),
            Request::Tooltips
            | Request::Autosave
            | Request::Retry
            | Request::ToggleMidi
            | Request::ToggleSound
            | Request::Close => None,
        }
    }
}
/// Where the fixed parts of the card sit. Device rows live inside the two panels and are laid out by the scroll area, so
/// they are not here; a test clicks the fold headers and the switches, which do not move.
pub struct Rows {
    pub tooltips: Rect,
    pub autosave: Rect,
    pub midi_header: Rect,
    pub midi_panel: Rect,
    pub sound_header: Rect,
    pub sound_panel: Rect,
    pub retry: Option<Rect>,
    pub close: Rect,
}

const PADDING: f32 = 12.0;
const ROW: f32 = 22.0;
const GAP: f32 = 2.0;
const HEADING: f32 = 18.0;
const CURRENT: f32 = 14.0;

/// The card's own rect inside `panel`.
pub fn card_rect(panel: Rect, scale: f32) -> Rect {
    panel.shrink(theme::MARGIN * scale)
}

/// Lay the card out. An open panel is as tall as its rows, up to `VISIBLE_ROWS`, and whatever does not fit scrolls.
pub fn rows(
    panel: Rect,
    scale: f32,
    ports: usize,
    outputs: usize,
    retry: bool,
    midi_open: bool,
    sound_open: bool,
) -> Rows {
    let card = card_rect(panel, scale);
    let left = card.left() + PADDING * scale;
    let width = card.width() - PADDING * 2.0 * scale;
    let row =
        |y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(left, y), egui::vec2(w, h * scale));
    let panel_height = |count: usize| {
        let shown = count.clamp(1, VISIBLE_ROWS);
        shown as f32 * (ROW + GAP) * scale
    };

    let mut y = card.top() + PADDING * scale + HEADING * scale;
    let switches = row(y, width * 0.30 - PADDING * scale, ROW);
    let autosave = Rect::from_min_size(
        Pos2::new(switches.right() + PADDING * scale, y),
        switches.size(),
    );
    y += ROW * scale + PADDING * scale;

    let midi_header = row(y, width * 0.60, HEADING);
    y += HEADING * scale + CURRENT * scale;
    // The input list has one extra row: `none`, which lets the app pick a controller for itself.
    let midi_panel = row(
        y,
        width * 0.72,
        if midi_open {
            panel_height(ports + 1)
        } else {
            0.0
        },
    );
    y += midi_panel.height();
    let retry_rect = retry.then(|| {
        let rect = row(y, width * 0.18, ROW);
        y += ROW * scale + GAP * scale;
        rect
    });
    y += PADDING * scale;

    let sound_header = row(y, width * 0.60, HEADING);
    y += HEADING * scale + CURRENT * scale;
    let sound_panel = row(
        y,
        width * 0.72,
        if sound_open {
            panel_height(outputs)
        } else {
            0.0
        },
    );

    let close = Rect::from_min_size(
        Pos2::new(
            card.right() - PADDING * scale - 90.0 * scale,
            card.bottom() - PADDING * scale - ROW * scale,
        ),
        egui::vec2(90.0 * scale, ROW * scale),
    );
    Rows {
        tooltips: switches,
        autosave,
        midi_header,
        midi_panel,
        sound_header,
        sound_panel,
        retry: retry_rect,
        close,
    }
}

/// Paint the card and say what was clicked, if anything. The first request of the frame wins.
pub fn paint(
    ui: &mut Ui,
    panel: Rect,
    scale: f32,
    settings: &Settings,
    tooltips: bool,
    autosave: bool,
) -> Option<Request> {
    let card = card_rect(panel, scale);
    {
        let painter = ui.painter();
        painter.rect_filled(card, 6.0 * scale, theme::PANEL_RAISED);
        painter.rect_stroke(
            card,
            6.0 * scale,
            theme::silkscreen_stroke(scale),
            egui::StrokeKind::Inside,
        );
    }

    let padding = PADDING * scale;
    let rows = rows(
        panel,
        scale,
        settings.midi_ports.len(),
        settings.outputs.len(),
        settings.midi_given_up,
        settings.midi_open,
        settings.sound_open,
    );
    let mut request = None;

    ui.painter().text(
        Pos2::new(card.left() + padding, card.top() + padding),
        Align2::LEFT_TOP,
        "SETTINGS",
        theme::legend(13.0 * scale),
        theme::SILKSCREEN,
    );

    // The two switches are the controls the deleted TIPS and AUTO keys were, so they explain themselves the way those
    // keys did — the tip follows the control that was actually drawn, and is silent when help is off.
    let tooltips_switch = transport::transport_button(
        ui,
        "settings-tooltips",
        rows.tooltips,
        &switch_label("TOOLTIPS", tooltips),
        tooltips,
    );
    tips::hover(&tooltips_switch, tooltips, tips::Control::Tooltips);
    if tooltips_switch.clicked() {
        request = Some(Request::Tooltips);
    }
    let autosave_switch = transport::transport_button(
        ui,
        "settings-autosave",
        rows.autosave,
        &switch_label("AUTOSAVE", autosave),
        autosave,
    );
    tips::hover(&autosave_switch, tooltips, tips::Control::Autosave);
    if autosave_switch.clicked() {
        request = Some(Request::Autosave);
    }

    // MIDI IN: the fold, the device in use named above it, then the ports themselves, scrolling if there are many.
    if transport::transport_button(
        ui,
        "settings-midi-fold",
        rows.midi_header,
        &fold_label("MIDI IN", settings.midi_open),
        settings.midi_open,
    )
    .clicked()
    {
        request = Some(Request::ToggleMidi);
    }
    ui.painter().text(
        Pos2::new(card.left() + padding, rows.midi_header.bottom()),
        Align2::LEFT_TOP,
        current_line(settings.midi_current.as_deref()),
        theme::legend(10.0 * scale),
        theme::SILKSCREEN_DIM,
    );
    ui.painter().text(
        Pos2::new(card.right() - padding, rows.midi_header.bottom()),
        Align2::RIGHT_TOP,
        &settings.midi_status,
        theme::legend(10.0 * scale),
        if settings.midi_given_up {
            theme::LED_ACCENT
        } else {
            theme::SILKSCREEN_DIM
        },
    );
    if settings.midi_open
        && let Some(clicked) = scrollable(
            ui,
            rows.midi_panel,
            scale,
            settings.midi_ports.len() + 1,
            |index| {
                let (label, lit, request) = if index < settings.midi_ports.len() {
                    let port = &settings.midi_ports[index];
                    (
                        shorten(port, 34),
                        settings.midi_current.as_deref() == Some(port.as_str()),
                        Request::MidiIn(Some(port.clone())),
                    )
                } else {
                    (
                        "none".to_string(),
                        settings.midi_current.is_none(),
                        Request::MidiIn(None),
                    )
                };
                (label, lit, request)
            },
        )
    {
        request = Some(clicked);
    }
    if let Some(retry) = rows.retry
        && transport::transport_button(ui, "settings-retry", retry, "RETRY", true).clicked()
    {
        request = Some(Request::Retry);
    }

    // SOUND OUT: the same shape. No `none` row: an audio stream needs a device to land on.
    if transport::transport_button(
        ui,
        "settings-sound-fold",
        rows.sound_header,
        &fold_label("SOUND OUT", settings.sound_open),
        settings.sound_open,
    )
    .clicked()
    {
        request = Some(Request::ToggleSound);
    }
    ui.painter().text(
        Pos2::new(card.left() + padding, rows.sound_header.bottom()),
        Align2::LEFT_TOP,
        current_line(settings.output_current.as_deref()),
        theme::legend(10.0 * scale),
        theme::SILKSCREEN_DIM,
    );
    // The stream's own words, beside the output in use. This is the card's half of what the deleted bottom line said:
    // a machine with a busy or hot-unplugged device looks healthy on the face and is silent, so it must say so here.
    ui.painter().text(
        Pos2::new(card.right() - padding, rows.sound_header.bottom()),
        Align2::RIGHT_TOP,
        &settings.audio_status,
        theme::legend(10.0 * scale),
        if audio_status::is_failed(&settings.audio_status) {
            theme::LED_ACCENT
        } else {
            theme::SILKSCREEN_DIM
        },
    );
    if settings.sound_open
        && let Some(clicked) = scrollable(
            ui,
            rows.sound_panel,
            scale,
            settings.outputs.len(),
            |index| {
                let output = &settings.outputs[index];
                (
                    shorten(output, 34),
                    settings.output_current.as_deref() == Some(output.as_str()),
                    Request::SoundOut(output.clone()),
                )
            },
        )
    {
        request = Some(clicked);
    }

    if transport::transport_button(ui, "settings-close", rows.close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }
    request
}

/// A folded section's heading: the name, and an ASCII fold mark, because the face may not use font glyphs.
fn fold_label(name: &str, open: bool) -> String {
    format!("{name}  [{}]", if open { "-" } else { "+" })
}

/// A switch's caption: what it is *and* where it stands. A switch that always prints ON misreports the state it is in —
/// the deleted TIPS and AUTO keys said both words, and the card has to say what they said.
fn switch_label(name: &str, on: bool) -> String {
    format!("{name} {}", if on { "ON" } else { "OFF" })
}

/// The device in use, named above its list so the answer is readable while the list is shut.
fn current_line(current: Option<&str>) -> String {
    match current {
        Some(name) => format!("IN USE: {}", shorten(name, 40)),
        None => "IN USE: none".to_string(),
    }
}

/// Paint `count` rows into `area` behind a vertical scroll, and answer the request the player clicked, if any. The rows are
/// built lazily by `row`, which returns the label, whether it is the one in use, and the request it asks for.
fn scrollable(
    ui: &mut Ui,
    area: Rect,
    scale: f32,
    count: usize,
    row: impl Fn(usize) -> (String, bool, Request),
) -> Option<Request> {
    let mut request = None;
    let mut child = ui.new_child(UiBuilder::new().max_rect(area));
    egui::ScrollArea::vertical()
        .max_height(area.height())
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for index in 0..count {
                let (label, lit, asked) = row(index);
                let top_left = ui.cursor().min;
                let rect = Rect::from_min_size(top_left, egui::vec2(area.width(), ROW * scale));
                ui.allocate_space(rect.size());
                if transport::transport_button(
                    ui,
                    &format!("settings-row-{index}"),
                    rect,
                    &label,
                    lit,
                )
                .clicked()
                {
                    request = Some(asked);
                }
                ui.add_space(GAP * scale);
            }
        });
    request
}

/// A device name as much of it as a row can hold, with an ellipsis when it was cut.
fn shorten(name: &str, max: usize) -> String {
    let mut short: String = name.chars().take(max).collect();
    if name.chars().count() > max {
        short.push('…');
    }
    short
}
