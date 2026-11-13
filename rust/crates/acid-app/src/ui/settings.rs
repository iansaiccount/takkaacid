//! The settings card: tooltips, autosave, MIDI in, sound out — everything the top strip and the bottom line used to hold,
//! in one place that opens with F2 or the gear and closes when you are done.
//!
//! The two device lists are collapsed sections with the device in use named above them: a machine with a dozen outputs
//! must not let its list eat the screen. An open section scrolls once it has more rows than fit.
//!
//! Pure painting plus a request enum, like the songs and help cards: `main` owns the ports, the streams and the probes.

use eframe::egui::{self, Align2, Pos2, Rect, Ui, UiBuilder};

use crate::audio::status as audio_status;

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
    /// Every MIDI input the machine offers, as names. The list the card shows when it opens; a port that arrives later
    /// shows on the next opening, because enumerating is a round trip per opening and not per frame.
    pub midi_ports: Vec<String>,
    /// The ports the app is holding open, by name. This is the tick on each row, and it survives a port going away: a
    /// controller with its cable out keeps its name, and its row, until the player turns it off.
    pub midi_enabled: Vec<String>,
    /// The ports that answer right now, named above the list so the answer is readable while the list is shut. Not the
    /// same question as `midi_enabled`: an unplugged port is still held, and still has a row, but nothing answers it.
    pub midi_current: Vec<String>,
    /// Every audio output the machine offers.
    pub outputs: Vec<String>,
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
    /// The ports that are open right now.
    pub midi_current: &'a [&'a str],
    /// The ports the app is holding open: the tick on each row, whether or not the port answers.
    pub midi_enabled: &'a [&'a str],
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
        self.midi_current = live
            .midi_current
            .iter()
            .map(|name| name.to_string())
            .collect();
        self.midi_enabled = live
            .midi_enabled
            .iter()
            .map(|name| name.to_string())
            .collect();
        self.output_current = live.output_current.map(str::to_string);
        self.midi_status = live.midi_status.to_string();
        self.midi_given_up = live.midi_given_up;
        self.audio_status = live.audio_status.to_string();
    }

    /// The rows the MIDI list shows, in order: the ports the machine reports, then any port the app still holds that the
    /// machine does not. A controller with its cable out keeps its row, because the app is still trying to open it, and
    /// turning it off is the only way out of that — a row that vanishes leaves nothing to click.
    pub fn ports(&self) -> Vec<&str> {
        let mut rows: Vec<&str> = self.midi_ports.iter().map(String::as_str).collect();
        for name in &self.midi_enabled {
            if !rows.contains(&name.as_str()) {
                rows.push(name);
            }
        }
        rows
    }

    /// Whether the app is holding this port open: the tick on its row.
    pub fn is_enabled(&self, port: &str) -> bool {
        self.midi_enabled.iter().any(|name| name == port)
    }

    /// Whether the machine still reports this port. A port that is held but not reported keeps its row, and says so.
    pub fn is_listed(&self, port: &str) -> bool {
        self.midi_ports.iter().any(|name| name == port)
    }
}

/// What the card asked `main` to do: the things only the app can do, because it owns the ports, the streams and the
/// probes. The card's own state — its two folds and its CLOSE — is [`Own`] and never travels.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    /// Flip the hover help.
    Tooltips,
    /// Flip autosave.
    Autosave,
    /// Open this port, or close it. The `bool` is the state the row is asking for: a row is a toggle, not a choice.
    MidiPort(String, bool),
    /// Move the stream to this output.
    SoundOut(String),
    /// Ask the supervisor to probe again.
    Retry,
}

/// What the card does to itself. These are panel state, so they never leave the panel — and because they are not
/// [`Request`]s, `main` has no arm for them that could never run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Own {
    /// Fold or unfold the MIDI panel.
    ToggleMidi,
    /// Fold or unfold the sound panel.
    ToggleSound,
    Close,
}

/// One thing a click on the card asked for: `main`'s work, or the card's own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Click {
    Ask(Request),
    Own(Own),
}

/// What `main` does with a request: one total mapping, so every arm of the match in `main` can run and a test can prove
/// the mapping without the binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    ToggleTooltips,
    ToggleAutosave,
    Retry,
    /// Open or close one MIDI input, by name.
    TogglePort(String, bool),
    /// Move the stream to this output.
    SoundOut(String),
}

impl Request {
    /// What this request means. The only place that answers it, so `main` adds no second list of cases.
    pub fn action(&self) -> Action {
        match self {
            Request::Tooltips => Action::ToggleTooltips,
            Request::Autosave => Action::ToggleAutosave,
            Request::Retry => Action::Retry,
            Request::MidiPort(name, on) => Action::TogglePort(name.clone(), *on),
            Request::SoundOut(name) => Action::SoundOut(name.clone()),
        }
    }
}
/// Where the fixed parts of the card sit. Rows live inside the two panels and are laid out by the scroll area, so a
/// panel's rows are `ports` (one rect per port) and nothing else: a test clicks a fold header, a switch, or a port row.
pub struct Rows {
    pub tooltips: Rect,
    pub autosave: Rect,
    pub midi_header: Rect,
    pub midi_panel: Rect,
    /// One rect per port row, in the order the card paints them. Empty while the panel is folded, because a folded panel
    /// paints no rows and so has none to click.
    pub ports: Vec<Rect>,
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
pub fn rows(panel: Rect, scale: f32, settings: &Settings) -> Rows {
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
    // One row per port, and no `none` row any more: "no ports" is every row unticked, which is a state the list can show.
    let port_count = settings.ports().len();
    let midi_panel = row(
        y,
        width * 0.72,
        if settings.midi_open {
            panel_height(port_count)
        } else {
            0.0
        },
    );
    y += midi_panel.height();
    let retry_rect = settings.midi_given_up.then(|| {
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
        if settings.sound_open {
            panel_height(settings.outputs.len())
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
    let ports = if settings.midi_open {
        row_bands(midi_panel, scale, port_count)
    } else {
        Vec::new()
    };
    Rows {
        tooltips: switches,
        autosave,
        midi_header,
        midi_panel,
        ports,
        sound_header,
        sound_panel,
        retry: retry_rect,
        close,
    }
}

/// One rect per row of a list, in the order the scroll area lays them out, before any scrolling: a test clicks a row's
/// centre, and the card paints the same band unless the list has been scrolled.
fn row_bands(area: Rect, scale: f32, count: usize) -> Vec<Rect> {
    (0..count)
        .map(|index| {
            Rect::from_min_size(
                Pos2::new(area.left(), area.top() + index as f32 * (ROW + GAP) * scale),
                egui::vec2(area.width(), ROW * scale),
            )
        })
        .collect()
}

/// Paint the card and say what was clicked, if anything. Only one control can be clicked in a frame, so the last request
/// set is the one that answers; every other control leaves it alone.
pub fn paint(
    ui: &mut Ui,
    panel: Rect,
    scale: f32,
    settings: &Settings,
    tooltips: bool,
    autosave: bool,
) -> Option<Click> {
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
    let rows = rows(panel, scale, settings);
    let mut request: Option<Click> = None;

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
        request = Some(Click::Ask(Request::Tooltips));
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
        request = Some(Click::Ask(Request::Autosave));
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
        request = Some(Click::Own(Own::ToggleMidi));
    }
    ui.painter().text(
        Pos2::new(card.left() + padding, rows.midi_header.bottom()),
        Align2::LEFT_TOP,
        current_line(&settings.midi_current),
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
    // Every port is its own row with its own tick: what the player turns on is a set, not a choice, and a port the machine
    // has lost keeps its row so it can be turned off without hunting for the cable.
    let ports = settings.ports();
    if settings.midi_open
        && let Some(clicked) = scrollable(ui, rows.midi_panel, scale, ports.len(), |index| {
            let port = ports[index];
            let on = settings.is_enabled(port);
            (
                port_label(port, on, settings.is_listed(port)),
                on,
                Request::MidiPort(port.to_string(), !on),
            )
        })
    {
        request = Some(Click::Ask(clicked));
    }
    if let Some(retry) = rows.retry
        && transport::transport_button(ui, "settings-retry", retry, "RETRY", true).clicked()
    {
        request = Some(Click::Ask(Request::Retry));
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
        request = Some(Click::Own(Own::ToggleSound));
    }
    ui.painter().text(
        Pos2::new(card.left() + padding, rows.sound_header.bottom()),
        Align2::LEFT_TOP,
        output_line(settings.output_current.as_deref()),
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
        request = Some(Click::Ask(clicked));
    }

    if transport::transport_button(ui, "settings-close", rows.close, "CLOSE", false).clicked() {
        request = Some(Click::Own(Own::Close));
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

/// The ports in use, named above the list so the answer is readable while the list is shut. There is more than one answer
/// now, so it names them all; a long list is cut by `shorten`, which is the one place the face may use an ellipsis.
fn current_line(current: &[String]) -> String {
    match current {
        [] => "IN USE: none".to_string(),
        [one] => format!("IN USE: {}", shorten(one, 40)),
        many => format!("IN USE: {}", shorten(&many.join(", "), 40)),
    }
}

/// The output in use: one stream, so one name, in the same words the ports line uses.
fn output_line(current: Option<&str>) -> String {
    match current {
        Some(name) => format!("IN USE: {}", shorten(name, 40)),
        None => "IN USE: none".to_string(),
    }
}

/// A port row: its tick, its name, and a word when the machine no longer reports it. The tick is the state, the word is the
/// reason, and the row is the only place either is said.
fn port_label(port: &str, on: bool, listed: bool) -> String {
    let mark = if on { "x" } else { " " };
    if listed {
        format!("[{mark}] {}", shorten(port, 34))
    } else {
        format!("[{mark}] {} (missing)", shorten(port, 24))
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
    // egui's default bar *floats* over the content, which is why every row ran underneath it. A bar that allocates its
    // own column gives the rows a right edge to stop at — and the width is taken from the style here rather than read
    // back from the layout, because egui animates the bar's appearance in over a few frames and a row would grow its
    // first pixel under the bar on the opening frame.
    child.style_mut().spacing.scroll.floating = false;
    // Only a list too long for the panel scrolls; a short one keeps the full width. The bar's width comes from the
    // child's style: `ui`'s still says the bar floats, and a floating bar allocates nothing.
    let bar = if count > VISIBLE_ROWS {
        child.style().spacing.scroll.allocated_width()
    } else {
        0.0
    };
    let row_width = (area.width() - bar).max(0.0);
    egui::ScrollArea::vertical()
        .max_height(area.height())
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            for index in 0..count {
                let (label, lit, asked) = row(index);
                let top_left = ui.cursor().min;
                // The row ends where the content area ends, not at the panel's edge: a scroll bar is laid over the right
                // of `area`, so `area.width()` ran every row underneath it.
                let rect = Rect::from_min_size(top_left, egui::vec2(row_width, ROW * scale));
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
