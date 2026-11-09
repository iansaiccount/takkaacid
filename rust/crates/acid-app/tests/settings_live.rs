//! The settings card: opened by F2 or the gear, folded and unfolded by its own headers, and dismissed only by CLOSE, Escape
//! or a click outside it — never by a click on one of its own rows.

use acid_app::state::SharedState;
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::settings::{self, Request};
use acid_app::ui::{PanelState, dispatch, draw_panel};
use acid_core::bank::PatternBank;
use eframe::egui::{
    Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2,
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

/// Every filled rectangle egui drew this frame: a row's face is one, and so is the bar's track.
fn filled_rects(output: &FullOutput) -> Vec<Rect> {
    fn walk(shape: &Shape, out: &mut Vec<Rect>) {
        match shape {
            Shape::Rect(rect) => out.push(rect.rect),
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

/// Every string egui drew this frame: a test asserts on what the card *says*, not on where it says it.
fn text_in(output: &FullOutput) -> Vec<String> {
    fn walk(shape: &Shape, out: &mut Vec<String>) {
        match shape {
            Shape::Text(text) => out.push(text.galley.text().to_string()),
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

/// Paint one frame with no input and answer everything it said.
fn painted(ctx: &Context, panel: &mut PanelState, shared: &SharedState) -> Vec<String> {
    let input = RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
    output.textures_delta.clear();
    text_in(&output)
}

fn says(texts: &[String], word: &str) -> bool {
    texts.iter().any(|text| text == word)
}

/// One key press, through the same two calls `main` makes: the frame's events become presses, and each press is
/// resolved. Shortcuts are not handled inside `draw_panel` — `main` does that — so this is the seam a test presses.
fn press(
    panel: &mut PanelState,
    shared: &SharedState,
    bank: &mut PatternBank,
    key: Key,
    shift: bool,
) {
    let events = vec![Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift,
            ..Modifiers::default()
        },
    }];
    for press in dispatch::presses(&events, shift) {
        dispatch::handle(press, panel, shared, bank);
    }
}

/// Hold the pointer still over `at` for a few frames, so egui's tooltip delay passes, and answer what was painted.
fn hovered(ctx: &Context, panel: &mut PanelState, shared: &SharedState, at: Pos2) -> Vec<String> {
    let mut seen = Vec::new();
    for step in 0..5 {
        // The pointer settles on the first frame and then stays: an egui tooltip waits for the hand to stop moving, so a
        // PointerMoved every frame at one position keeps it away and a test hovers forever and sees nothing.
        let events = if step == 0 {
            vec![Event::PointerMoved(at)]
        } else {
            Vec::new()
        };
        let input = RawInput {
            screen_rect: Some(screen()),
            time: Some(f64::from(step)),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| draw_panel(panel, shared, ui));
        output.textures_delta.clear();
        seen.extend(text_in(&output));
    }
    seen
}

/// A panel with the settings card open and a couple of devices on offer.
fn open_settings(midi_ports: &[&str], outputs: &[&str]) -> PanelState {
    let mut panel = PanelState {
        settings_open: true,
        ..PanelState::default()
    };

    panel.settings.midi_ports = midi_ports.iter().map(|s| s.to_string()).collect();
    panel.settings.outputs = outputs.iter().map(|s| s.to_string()).collect();
    panel
}

fn rows(panel: &PanelState) -> settings::Rows {
    let l = layout(screen());
    settings::rows(
        l.panel,
        scale(screen()),
        panel.settings.midi_ports.len(),
        panel.settings.outputs.len(),
        panel.settings.midi_given_up,
        panel.settings.midi_open,
        panel.settings.sound_open,
    )
}

/// The bug this pins: the dismissal shield was registered after the card, so it won the hit test and every click inside the
/// card — a fold header most visibly — shut the card instead of doing what it said.
#[test]
fn folding_a_panel_does_not_close_the_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers", "HDMI"]);
    assert!(!panel.settings.midi_open, "the sections start folded");

    let at = rows(&panel).midi_header.center();
    click(&ctx, &mut panel, &shared, at);

    assert!(panel.settings_open, "clicking the fold must not dismiss");
    assert!(panel.settings.midi_open, "it unfolds the panel");
}

/// The same click again folds it, and the card still stands.
#[test]
fn folding_a_panel_shut_keeps_the_card_open() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers"]);
    panel.settings.midi_open = true;

    let at = rows(&panel).midi_header.center();
    click(&ctx, &mut panel, &shared, at);

    assert!(panel.settings_open);
    assert!(!panel.settings.midi_open);
}

/// A click on the dark area outside the card is a dismissal — the shield still does its job.
#[test]
fn a_click_outside_the_card_closes_it() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &[]);

    click(&ctx, &mut panel, &shared, Pos2::new(6.0, 6.0));

    assert!(!panel.settings_open, "clicking away closes the card");
}

/// Both switches ask for their own flip, and neither click takes the card away with it.
#[test]
fn the_switches_ask_for_their_flip() {
    let ctx = Context::default();
    let shared = SharedState::new();

    let mut tooltips = open_settings(&[], &[]);
    let at = rows(&tooltips).tooltips.center();
    click(&ctx, &mut tooltips, &shared, at);
    assert_eq!(tooltips.settings_request, Some(Request::Tooltips));
    assert!(tooltips.settings_open);

    let mut autosave = open_settings(&[], &[]);
    let at = rows(&autosave).autosave.center();
    click(&ctx, &mut autosave, &shared, at);
    assert_eq!(autosave.settings_request, Some(Request::Autosave));
    assert!(autosave.settings_open);
}

/// The sound section folds and unfolds on its own, independently of the MIDI one.
#[test]
fn the_two_sections_fold_independently() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers", "HDMI"]);

    let at = rows(&panel).sound_header.center();
    click(&ctx, &mut panel, &shared, at);
    assert!(panel.settings.sound_open);
    assert!(!panel.settings.midi_open, "the MIDI panel stayed shut");

    let screen_rows = rows(&panel);
    assert!(
        screen_rows.sound_panel.height() > 0.0,
        "an open panel needs a panel to paint in"
    );
    assert_eq!(
        screen_rows.midi_panel.height(),
        0.0,
        "a shut panel takes no space"
    );
    let _ = screen_rows.retry;
}

/// An open panel never grows past four rows, however long the machine's list is: what does not fit scrolls.
#[test]
fn an_open_panel_is_four_rows_tall_at_most() {
    let l = layout(screen());
    let s = scale(screen());
    let short = settings::rows(l.panel, s, 2, 2, false, true, true);
    let long = settings::rows(l.panel, s, 12, 12, false, true, true);

    let four_rows = 4.0 * (22.0 + 2.0) * s;
    assert!(short.midi_panel.height() <= four_rows);
    assert!(
        long.midi_panel.height() <= four_rows,
        "twelve outputs still show four rows"
    );
    assert!(
        long.midi_panel.height() < l.panel.height() * 0.35,
        "twelve outputs must not fill the card"
    );
}

/// A device row writes the same choice the old status-line picker wrote: the card is a new surface, not a new mechanism.
#[test]
fn a_midi_row_writes_the_input_choice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep", "IAC Driver"], &[]);
    panel.settings.midi_open = true;

    let rows = rows(&panel);
    let first_row = rows.midi_panel.min + vec2(12.0, 8.0);
    click(&ctx, &mut panel, &shared, first_row);

    assert_eq!(
        panel.settings_request,
        Some(Request::MidiIn(Some("KeyStep".to_string()))),
        "the row asks for the port it names"
    );
    assert!(panel.settings_open, "choosing a device is not a dismissal");
}

/// The sound rows do the same for the output, and the card stays up while the stream moves.
#[test]
fn a_sound_row_writes_the_output_choice() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &["Speakers", "HDMI"]);
    panel.settings.sound_open = true;

    let rows = rows(&panel);
    let second_row = rows.sound_panel.min + vec2(12.0, 22.0 + 2.0 + 8.0);
    click(&ctx, &mut panel, &shared, second_row);

    assert_eq!(
        panel.settings_request,
        Some(Request::SoundOut("HDMI".to_string())),
        "the row asks for the output it names"
    );
    assert!(panel.settings_open);
}

/// F2 is the card's front door, and a second press is its back door.
#[test]
fn f2_opens_and_closes_the_settings_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut bank = PatternBank::factory();
    let mut panel = PanelState::default();
    assert!(!says(&painted(&ctx, &mut panel, &shared), "SETTINGS"));

    press(&mut panel, &shared, &mut bank, Key::F2, false);
    assert!(panel.settings_open, "F2 opens the card");
    assert!(
        says(&painted(&ctx, &mut panel, &shared), "SETTINGS"),
        "and the card paints what the key opened"
    );

    press(&mut panel, &shared, &mut bank, Key::F2, false);
    assert!(!panel.settings_open, "F2 closes it again");
    assert!(
        !says(&painted(&ctx, &mut panel, &shared), "SETTINGS"),
        "a closed card paints nothing"
    );
}

/// The gear is the pointer's way in.
#[test]
fn the_gear_opens_settings() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = PanelState::default();
    let l = layout(screen());
    let at = acid_app::ui::panel::gear_rect(l.top, scale(screen())).center();

    click(&ctx, &mut panel, &shared, at);

    assert!(panel.settings_open, "clicking the gear opens the card");
    assert!(says(&painted(&ctx, &mut panel, &shared), "SETTINGS"));
}

/// There is room for one card: opening settings puts the other two away.
#[test]
fn opening_settings_closes_help_and_the_songs_card() {
    let shared = SharedState::new();
    let mut bank = PatternBank::factory();
    let mut panel = PanelState {
        help_visible: true,
        ..PanelState::default()
    };
    panel.songs.card = Some(acid_app::ui::songs::Songs::default());

    press(&mut panel, &shared, &mut bank, Key::F2, false);

    assert!(panel.settings_open);
    assert!(!panel.help_visible, "help must not stay up behind settings");
    assert!(panel.songs.card.is_none(), "nor the songs card");
}

/// And the same the other way round: F2 while help is up replaces it in one keypress.
#[test]
fn f2_with_help_open_replaces_help() {
    let shared = SharedState::new();
    let mut bank = PatternBank::factory();
    let mut panel = PanelState::default();

    press(&mut panel, &shared, &mut bank, Key::F1, false);
    assert!(panel.help_visible);

    press(&mut panel, &shared, &mut bank, Key::F2, false);
    assert!(panel.settings_open);
    assert!(!panel.help_visible, "two cards at once is not a state");
}

/// The mirror held a bug: help opened over a still-open settings card, so Escape closed the card nobody could see first.
#[test]
fn the_help_key_clears_the_settings_card() {
    let shared = SharedState::new();
    let mut bank = PatternBank::factory();

    for (key, shift) in [(Key::F1, false), (Key::Questionmark, true)] {
        let mut panel = PanelState::default();
        press(&mut panel, &shared, &mut bank, Key::F2, false);
        assert!(panel.settings_open);

        press(&mut panel, &shared, &mut bank, key, shift);

        assert!(panel.help_visible, "{key:?} opens help");
        assert!(
            !panel.settings_open,
            "{key:?} must close the settings card, or help paints over it"
        );
    }
}

/// RETRY is the supervisor's way back, so it is on the card exactly when the supervisor has stopped trying.
#[test]
fn retry_appears_only_while_the_supervisor_has_given_up() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &[]);

    assert!(
        rows(&panel).retry.is_none(),
        "a retrying supervisor needs no key"
    );
    assert!(
        !says(&painted(&ctx, &mut panel, &shared), "RETRY"),
        "and the card offers none"
    );

    panel.settings.midi_given_up = true;

    assert!(
        rows(&panel).retry.is_some(),
        "a supervisor that has given up offers a way back, and the row moves the sections below it"
    );
    assert!(
        says(&painted(&ctx, &mut panel, &shared), "RETRY"),
        "the card says so"
    );
}

/// A switch whose caption always says ON misreports the state it is in; the deleted TIPS/AUTO keys said both.
#[test]
fn the_switches_print_their_state() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &[]);
    assert!(panel.tooltips, "help starts on");
    assert!(!panel.autosave, "and saving starts manual");

    let texts = painted(&ctx, &mut panel, &shared);
    assert!(says(&texts, "TOOLTIPS ON"), "{texts:?}");
    assert!(
        says(&texts, "AUTOSAVE OFF"),
        "autosave is off, so the switch must say so: {texts:?}"
    );

    panel.tooltips = false;
    panel.autosave = true;
    let texts = painted(&ctx, &mut panel, &shared);
    assert!(says(&texts, "TOOLTIPS OFF"), "{texts:?}");
    assert!(says(&texts, "AUTOSAVE ON"), "{texts:?}");
}

/// The card mirrors the machine while it is open: a stream that fails, or a device that is chosen, must show without
/// closing and reopening it.
#[test]
fn the_card_follows_the_devices_while_it_is_open() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &["Speakers", "HDMI"]);

    panel.settings.follow(settings::Live {
        midi_current: Some("KeyStep"),
        output_current: Some("Speakers"),
        midi_status: "MIDI: KeyStep",
        midi_given_up: false,
        audio_status: "48 kHz - Speakers",
    });
    let texts = painted(&ctx, &mut panel, &shared);
    assert!(says(&texts, "IN USE: Speakers"), "{texts:?}");
    assert!(says(&texts, "48 kHz - Speakers"), "{texts:?}");

    panel.settings.follow(settings::Live {
        midi_current: Some("KeyStep"),
        output_current: Some("HDMI"),
        midi_status: "MIDI: KeyStep",
        midi_given_up: false,
        audio_status: "48 kHz - HDMI",
    });
    let texts = painted(&ctx, &mut panel, &shared);
    assert!(
        says(&texts, "IN USE: HDMI"),
        "the card must say where the sound is going now, not where it went when it opened: {texts:?}"
    );

    assert_eq!(
        panel.settings.outputs.len(),
        2,
        "following the devices never re-enumerates them: that is the costly half, and it belongs to the opening"
    );
}

/// A device pulled out while the card is open: the row that was lit stops being lit, the name above the list stops
/// claiming it, and the rows that remain still work — the list is only re-read on the next opening.
#[test]
fn a_device_unplugged_while_the_card_is_open_leaves_the_rows_working() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep", "IAC Driver"], &[]);
    panel.settings.midi_open = true;
    panel.settings.follow(settings::Live {
        midi_current: Some("KeyStep"),
        output_current: None,
        midi_status: "MIDI: KeyStep",
        midi_given_up: false,
        audio_status: "48 kHz - Speakers",
    });
    assert!(says(&painted(&ctx, &mut panel, &shared), "IN USE: KeyStep"));

    // The port goes away: `main` reports the connection it has, which is now none, and the code that makes the words
    // has already said so. Nothing may panic on the name that is no longer there.
    panel.settings.follow(settings::Live {
        midi_current: None,
        output_current: None,
        midi_status: "MIDI: none (retrying)",
        midi_given_up: false,
        audio_status: "48 kHz - Speakers",
    });
    let texts = painted(&ctx, &mut panel, &shared);
    assert!(says(&texts, "IN USE: none"), "{texts:?}");
    assert!(says(&texts, "MIDI: none (retrying)"), "{texts:?}");

    let rows = rows(&panel);
    let second_row = rows.midi_panel.min + vec2(12.0, 22.0 + 2.0 + 8.0);
    click(&ctx, &mut panel, &shared, second_row);

    assert_eq!(
        panel.settings_request,
        Some(Request::MidiIn(Some("IAC Driver".to_string()))),
        "the rows that are left still choose, whatever the card was told before"
    );
}

/// A failed audio stream is the one thing the old bottom line said that nothing else does: silence must not look
/// healthy on a device-busy or hot-unplugged machine.
#[test]
fn an_audio_failure_is_visible_in_the_settings_card() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &["Speakers"]);
    panel.settings.follow(settings::Live {
        midi_current: None,
        output_current: None,
        midi_status: "MIDI: none (retrying)",
        midi_given_up: false,
        audio_status: "failed: device busy",
    });

    let texts = painted(&ctx, &mut panel, &shared);

    assert!(
        texts.iter().any(|text| text == "failed: device busy"),
        "a silent app that looks healthy is the worst state it can be in: {texts:?}"
    );
}

/// Escape gives up on one thing at a time, nearest first, and a learn in progress is the last thing it touches.
#[test]
fn escape_closes_one_thing_at_a_time_and_the_card_comes_first() {
    let shared = SharedState::new();
    let mut panel = PanelState {
        settings_open: true,
        learn: acid_app::ui::learn::Step::Choosing,
        ..PanelState::default()
    };

    assert!(
        dispatch::escape(&mut panel, &shared),
        "there was something to close"
    );
    assert!(!panel.settings_open, "the card goes first");
    assert_eq!(
        panel.learn,
        acid_app::ui::learn::Step::Choosing,
        "a learn in progress keeps going: the card was in front of it, not instead of it"
    );

    panel.help_visible = true;
    assert!(dispatch::escape(&mut panel, &shared));
    assert!(!panel.help_visible, "then help");

    panel.songs.card = Some(acid_app::ui::songs::Songs::default());
    assert!(dispatch::escape(&mut panel, &shared));
    assert!(panel.songs.card.is_none(), "then the songs card");

    assert!(dispatch::escape(&mut panel, &shared));
    assert_eq!(
        panel.learn,
        acid_app::ui::learn::Step::Idle,
        "and last, the learn"
    );

    assert!(
        !dispatch::escape(&mut panel, &shared),
        "with nothing open Escape has nothing to do, so a test can tell"
    );
}

/// A device row's request becomes the action `main` takes. The mapping lives here, beside the request, because `main` is
/// a binary and no test can call it — and it is total, so `main` has no arm that cannot run.
#[test]
fn a_device_row_asks_for_the_action_main_takes() {
    use acid_app::midi::ports::PortChoice;
    use acid_app::ui::settings::{Action, Pick};

    assert_eq!(
        Request::MidiIn(Some("KeyStep".to_string())).action(),
        Action::Choose(Pick::Midi(PortChoice::Named("KeyStep".to_string())))
    );
    assert_eq!(
        Request::MidiIn(None).action(),
        Action::Choose(Pick::Midi(PortChoice::Any)),
        "the `none` row is the app picking a controller for itself"
    );
    assert_eq!(
        Request::SoundOut("HDMI".to_string()).action(),
        Action::Choose(Pick::Sound("HDMI".to_string()))
    );
    assert_eq!(Request::Tooltips.action(), Action::ToggleTooltips);
    assert_eq!(Request::Autosave.action(), Action::ToggleAutosave);
    assert_eq!(Request::Retry.action(), Action::Retry);
}

/// The card folds and closes itself: its own state never travels to `main`, so there is no arm there that cannot run.
#[test]
fn the_card_keeps_its_own_folds_and_its_close() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&["KeyStep"], &[]);

    let at = rows(&panel).close.center();
    click(&ctx, &mut panel, &shared, at);

    assert!(!panel.settings_open, "CLOSE closes the card");
    assert_eq!(
        panel.settings_request, None,
        "and asks main for nothing at all"
    );
}

/// The card's two switches are the controls the deleted TIPS and AUTO keys were, so they explain themselves the way
/// those keys did — and they say nothing at all when the help is turned off.
#[test]
fn the_settings_switches_explain_themselves() {
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&[], &[]);
    let at = rows(&panel).tooltips.center();

    let seen = hovered(&ctx, &mut panel, &shared, at);
    assert!(
        seen.iter()
            .any(|text| text.contains("Turns these hover hints on and off")),
        "the switch has to say what it does: {seen:?}"
    );

    // A fresh context for the second half: egui's clock only runs forwards, and a fresh one also proves the silence is
    // the switch's doing rather than leftover hover state.
    let quiet = Context::default();
    panel.tooltips = false;
    let seen = hovered(&quiet, &mut panel, &shared, at);
    assert!(
        !seen
            .iter()
            .any(|text| text.contains("Turns these hover hints")),
        "help that is off must not talk: {seen:?}"
    );
}

/// A list longer than the panel scrolls, and the scroll bar has the right edge to itself. This is measured on what is
/// *painted*, not on where clicks land: egui already clips a click to the scroll area's viewport, so a click test passes
/// even while every row face runs underneath the bar.
#[test]
fn a_scrolling_list_gives_the_scrollbar_its_gutter() {
    fn widest_row_face(ports: &[&str]) -> (f32, f32) {
        let ctx = Context::default();
        let shared = SharedState::new();
        let mut panel = open_settings(ports, &[]);
        panel.settings.midi_open = true;
        let band = rows(&panel).midi_panel;
        let output = run_on(&ctx, &mut panel, &shared, Vec::new());
        let row_height = 22.0;
        let widest = filled_rects(&output)
            .into_iter()
            .filter(|rect| {
                (rect.height() - row_height * scale(screen())).abs() < 1.0
                    && rect.top() >= band.top() - 1.0
                    && rect.bottom() <= band.bottom() + 1.0
            })
            .map(|rect| rect.right())
            .fold(f32::MIN, f32::max);
        (widest - band.left(), band.width())
    }

    let few: Vec<&str> = vec!["KeyStep", "IAC Driver"];
    let many: Vec<String> = (0..12).map(|index| format!("Port {index}")).collect();
    let many: Vec<&str> = many.iter().map(String::as_str).collect();

    let (short_right, short_width) = widest_row_face(&few);
    let (scrolling_right, scrolling_width) = widest_row_face(&many);

    assert!(
        short_right > short_width - 2.0,
        "a list that fits should use the whole panel: {short_right} of {short_width}"
    );
    assert!(
        scrolling_right <= scrolling_width - 4.0,
        "a scrolling list has to leave the bar its gutter: {scrolling_right} of {scrolling_width}"
    );

    // Narrower rows are still rows: a click in the middle of the first one chooses its port.
    let ctx = Context::default();
    let shared = SharedState::new();
    let mut panel = open_settings(&many, &[]);
    panel.settings.midi_open = true;
    let first_row = rows(&panel).midi_panel.min + vec2(40.0, 8.0);
    click(&ctx, &mut panel, &shared, first_row);
    assert_eq!(
        panel.settings_request,
        Some(Request::MidiIn(Some("Port 0".to_string()))),
        "a row that stops short of the bar is still clickable"
    );
}
