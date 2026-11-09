//! The settings card: tooltips, autosave, MIDI in, sound out — everything the top strip and the bottom line used to hold,
//! in one place that opens with F2 or the gear and closes when you are done.
//!
//! Pure painting plus a request enum, like the songs and help cards: `main` owns the ports, the streams and the probes.

use eframe::egui::{self, Align2, Pos2, Rect, Ui};

use super::theme;
use super::transport;

/// What the card shows besides the two switches. `main` fills this when the card opens, because it owns the lists.
#[derive(Clone, Default)]
pub struct Settings {
    /// Every MIDI input the machine offers, as names.
    pub midi_ports: Vec<String>,
    /// Every audio output the machine offers.
    pub outputs: Vec<String>,
    /// The port in use, so its row is the lit one.
    pub midi_current: Option<String>,
    /// The output in use.
    pub output_current: Option<String>,
    /// True while the supervisor has stopped retrying: the card then offers RETRY, where the status line used to.
    pub midi_given_up: bool,
    /// The supervisor's own words, shown under the MIDI IN heading.
    pub midi_status: String,
}

/// What the card asked for. `main` does the work.
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
    Close,
}

/// Where every row of the card sits, so the card paints from the same arithmetic a test clicks.
pub struct Rows {
    pub tooltips: Rect,
    pub autosave: Rect,
    pub midi: Vec<Rect>,
    pub sound: Vec<Rect>,
    pub retry: Option<Rect>,
    pub close: Rect,
}

const PADDING: f32 = 12.0;
const ROW: f32 = 22.0;
const GAP: f32 = 2.0;
const HEADING: f32 = 18.0;

/// The card's own rect inside `panel`.
pub fn card_rect(panel: Rect, scale: f32) -> Rect {
    panel.shrink(theme::MARGIN * scale)
}

/// Lay the rows out. One row per device, then a `none` row for the input: the two lists the status lines used to open.
pub fn rows(panel: Rect, scale: f32, ports: usize, outputs: usize, retry: bool) -> Rows {
    let card = card_rect(panel, scale);
    let left = card.left() + PADDING * scale;
    let width = card.width() - PADDING * 2.0 * scale;
    let row = |y: f32, w: f32| Rect::from_min_size(Pos2::new(left, y), egui::vec2(w, ROW * scale));
    let mut y = card.top() + PADDING * scale + HEADING * scale;

    let switches = row(y, width * 0.30 - PADDING * scale);
    let autosave = Rect::from_min_size(
        Pos2::new(switches.right() + PADDING * scale, y),
        switches.size(),
    );
    y += ROW * scale + PADDING * scale + HEADING * scale;

    let mut midi = Vec::with_capacity(ports + 1);
    for _ in 0..=ports {
        midi.push(row(y, width * 0.72));
        y += ROW * scale + GAP * scale;
    }
    let retry_rect = retry.then(|| {
        let rect = row(y, width * 0.18);
        y += ROW * scale + GAP * scale;
        rect
    });
    y += HEADING * scale;

    let mut sound = Vec::with_capacity(outputs);
    for _ in 0..outputs {
        sound.push(row(y, width * 0.72));
        y += ROW * scale + GAP * scale;
    }

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
        midi,
        sound,
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
    );
    let mut request = None;

    ui.painter().text(
        Pos2::new(card.left() + padding, card.top() + padding),
        Align2::LEFT_TOP,
        "SETTINGS",
        theme::legend(13.0 * scale),
        theme::SILKSCREEN,
    );
    ui.painter().text(
        Pos2::new(card.right() - padding, card.top() + padding + 2.0 * scale),
        Align2::RIGHT_TOP,
        format!(
            "SOUND OUT: {}",
            settings.output_current.as_deref().unwrap_or("none")
        ),
        theme::legend(10.0 * scale),
        theme::SILKSCREEN_DIM,
    );

    if transport::transport_button(
        ui,
        "settings-tooltips",
        rows.tooltips,
        "TOOLTIPS ON",
        tooltips,
    )
    .clicked()
    {
        request = Some(Request::Tooltips);
    }
    if transport::transport_button(
        ui,
        "settings-autosave",
        rows.autosave,
        "AUTOSAVE ON",
        autosave,
    )
    .clicked()
    {
        request = Some(Request::Autosave);
    }

    // MIDI IN: every port, the one in use lit, and a `none` row that lets the app choose.
    let y = rows
        .midi
        .first()
        .map_or(rows.tooltips.top(), |rect| rect.top())
        - HEADING * scale;
    ui.painter().text(
        Pos2::new(card.left() + padding, y),
        Align2::LEFT_TOP,
        "MIDI IN",
        theme::legend(11.0 * scale),
        theme::SILKSCREEN,
    );
    ui.painter().text(
        Pos2::new(card.right() - padding, y),
        Align2::RIGHT_TOP,
        &settings.midi_status,
        theme::legend(10.0 * scale),
        if settings.midi_given_up {
            theme::LED_ACCENT
        } else {
            theme::SILKSCREEN_DIM
        },
    );
    for (index, port) in settings.midi_ports.iter().enumerate() {
        let lit = settings.midi_current.as_deref() == Some(port.as_str());
        if transport::transport_button(
            ui,
            &format!("settings-midi-{index}"),
            rows.midi[index],
            &shorten(port, 34),
            lit,
        )
        .clicked()
        {
            request = Some(Request::MidiIn(Some(port.clone())));
        }
    }
    let none_row = rows.midi[settings.midi_ports.len()];
    if transport::transport_button(
        ui,
        "settings-midi-none",
        none_row,
        "none",
        settings.midi_current.is_none(),
    )
    .clicked()
    {
        request = Some(Request::MidiIn(None));
    }
    if let Some(retry) = rows.retry
        && transport::transport_button(ui, "settings-retry", retry, "RETRY", true).clicked()
    {
        request = Some(Request::Retry);
    }

    // SOUND OUT: the same shape, without a `none`: an audio stream needs a device to land on.
    let y = rows
        .sound
        .first()
        .map_or(none_row.bottom(), |rect| rect.top())
        - HEADING * scale;
    ui.painter().text(
        Pos2::new(card.left() + padding, y),
        Align2::LEFT_TOP,
        "SOUND OUT",
        theme::legend(11.0 * scale),
        theme::SILKSCREEN,
    );
    for (index, output) in settings.outputs.iter().enumerate() {
        let lit = settings.output_current.as_deref() == Some(output.as_str());
        if transport::transport_button(
            ui,
            &format!("settings-out-{index}"),
            rows.sound[index],
            &shorten(output, 34),
            lit,
        )
        .clicked()
        {
            request = Some(Request::SoundOut(output.clone()));
        }
    }

    if transport::transport_button(ui, "settings-close", rows.close, "CLOSE", false).clicked() {
        request = Some(Request::Close);
    }
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

// `y` is recomputed per section above, so the layout is the single source of truth and this card holds no state of its own.
