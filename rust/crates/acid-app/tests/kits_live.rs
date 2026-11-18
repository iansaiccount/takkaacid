//! The kit browser and the card's kit row, driven through the real `draw_panel`: the browser is the banks card's sibling,
//! and the rule that a factory kit is edited by making a copy is a panel rule, so it is tested where a player's hand lands.

use std::sync::Arc;

use acid_app::device::{DeviceKind, Scheme};
use acid_app::kits::{default_name, factory_names, is_factory};
use acid_app::midi::route::InputFilter;
use acid_app::persist::{self, kit_names, save_kit};
use acid_app::state::SharedState;
use acid_app::ui::device_card;
use acid_app::ui::dispatch;
use acid_app::ui::drum_panel;
use acid_app::ui::kits::{self, Request};
use acid_app::ui::layout::{layout, scale};
use acid_app::ui::strip::StripCell;
use acid_app::ui::theme::DESIGN_SIZE;
use acid_app::ui::{PanelState, draw_panel};
use acid_core::bank::PatternBank;
use acid_core::drums::{Drum, default_kit};
use eframe::egui::{
    Context, Event, FullOutput, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2,
};

fn screen() -> Rect {
    Rect::from_min_size(Pos2::ZERO, DESIGN_SIZE)
}

fn run(
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

fn press(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::default(),
    }
}

fn release(at: Pos2) -> Event {
    Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::default(),
    }
}

/// Every line of text the frame drew — the card's captions, its rows and the status line's notice.
fn said(output: &FullOutput) -> Vec<String> {
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

/// A panel showing one drum device, with the kit list `main` would have read for it.
fn drum_panel_state(names: Vec<String>, current: &str) -> (PanelState, Arc<SharedState>) {
    let shared = SharedState::new();
    let mut kit = default_kit();
    kit.name = current.to_string();
    shared.set_kit(Arc::new(kit));

    let mut panel = PanelState::default();
    panel.strip.cells = vec![StripCell {
        name: "DRUMS 1".to_string(),
        number: 1,
        scheme: Scheme::Black,
        running: false,
        muted: false,
        kind: DeviceKind::Drums,
    }];
    panel.strip.selected = 0;
    panel.kits.names = names;
    panel.kits.open = true;
    (panel, shared)
}

/// A library with one kit of the player's own in it, and the list a browser reads from it.
fn library_with_one_kit() -> (tempfile::TempDir, Vec<String>) {
    let dir = tempfile::tempdir().unwrap();
    let mut kit = default_kit();
    kit.name = "My Kit".to_string();
    save_kit(&kit, dir.path(), 44_100.0).unwrap();
    let names = kit_names(dir.path());
    (dir, names)
}

#[test]
fn the_kit_browser_lists_the_factory_kits_first() {
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names, default_name());
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &shared, vec![]);
    let said = said(&output);

    assert!(
        said.iter().any(|line| line == "KITS"),
        "the card is captioned"
    );
    assert!(said.iter().any(|line| line == "FACTORY"));
    assert!(
        said.iter().any(|line| line == "MY KITS"),
        "the player's own kits have a heading of their own: {said:?}"
    );

    // Every name on offer is on the card, and the factory three come first.
    for name in factory_names() {
        assert!(
            said.iter().any(|line| line.starts_with(name)),
            "the browser does not list {name}: {said:?}"
        );
    }
    assert!(said.iter().any(|line| line.starts_with("My Kit")));
    let at = |needle: &str| said.iter().position(|line| line.starts_with(needle));
    assert!(
        at(factory_names().last().unwrap()) < at("My Kit"),
        "a factory kit is listed above the player's own"
    );
    assert!(
        at("FACTORY") < at(factory_names()[0]) && at("MY KITS") < at("My Kit"),
        "each heading comes before the list it names"
    );
}

#[test]
fn a_factory_kit_is_marked_read_only_and_selecting_it_loads_it() {
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names.clone(), "My Kit");
    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &shared, vec![]);

    // The row says what it is before it is clicked: a factory kit cannot be edited in place.
    let spoken = said(&output);
    let read_only: Vec<&String> = spoken
        .iter()
        .filter(|line| line.contains("READ ONLY"))
        .collect();
    assert_eq!(
        read_only.len(),
        factory_names().len(),
        "each factory row is marked, and no player's row is: {read_only:?}"
    );

    // Clicking one asks `main` for it, which is the seam that decodes it at the device's rate.
    let s = scale(screen());
    let l = layout(screen());
    let rows = kits::rows(l.panel, s, &names);
    let rect = rows.factory[0];
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(rect.center())],
    );
    run(&ctx, &mut panel, &shared, vec![press(rect.center())]);
    run(&ctx, &mut panel, &shared, vec![release(rect.center())]);
    assert_eq!(
        panel.kits_request,
        Some(Request::Load(factory_names()[0].to_string())),
        "a factory row asks for the kit that ships"
    );
}

#[test]
fn editing_a_factory_voice_copies_the_kit_under_a_free_name() {
    // The trap this test exists for: a dial drag publishes a kit every frame. If "editing a factory kit copies it" sat in
    // the publish path, one drag would leave `808 (2) (3) (4)` behind. It happens once, at the edit's entry.
    let names: Vec<String> = factory_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let (mut panel, shared) = drum_panel_state(names, "808");
    let ctx = Context::default();
    let l = layout(screen());
    let attack = drum_panel::dial_cells(&l)[acid_app::knobs::Knob::Waveform.index()].center();

    // A drag: a press, three moves and a release, which is four frames that each publish a kit.
    let before_attack = shared.kit().voices[Drum::Kick.index()].attack_ms;
    run(&ctx, &mut panel, &shared, vec![Event::PointerMoved(attack)]);
    run(&ctx, &mut panel, &shared, vec![press(attack)]);
    for step in 1..=3 {
        run(
            &ctx,
            &mut panel,
            &shared,
            vec![Event::PointerMoved(attack - vec2(0.0, 6.0 * step as f32))],
        );
    }
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![release(attack - vec2(0.0, 18.0))],
    );

    assert_eq!(
        shared.kit().name,
        "808 (2)",
        "the first edit of a factory kit copies it, under the first free name"
    );
    assert!(
        !is_factory(&shared.kit().name),
        "and the copy is the player's, not the kit that ships"
    );
    assert!(
        panel.dirty,
        "a kit whose name is in the save file makes the session unsaved"
    );
    assert_eq!(
        panel.kit_dirty,
        Some(1),
        "and the folder the edited device lives in is owed a write, recorded as that device's number"
    );
    assert!(
        panel.kits.refresh,
        "the list has a kit it did not have, so `main` reads it again"
    );
    let attack_ms = shared.kit().voices[Drum::Kick.index()].attack_ms;
    assert!(
        attack_ms > before_attack,
        "the edit that made the copy is the edit that landed: {before_attack} -> {attack_ms}"
    );

    // And the session names the copy: the kit travels as a name, so a restart comes back to this one.
    let saved = persist::shared_to_saved(&shared, PatternBank::factory());
    assert_eq!(
        saved.kit.as_deref(),
        Some("808 (2)"),
        "the session names the copy, not the kit it came from"
    );

    // A second drag edits the copy the player now owns, and makes nothing new.
    let decay = drum_panel::dial_cells(&l)[acid_app::knobs::Knob::Cutoff.index()].center();
    run(&ctx, &mut panel, &shared, vec![Event::PointerMoved(decay)]);
    run(&ctx, &mut panel, &shared, vec![press(decay)]);
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(decay - vec2(0.0, 12.0))],
    );
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![release(decay - vec2(0.0, 12.0))],
    );
    assert_eq!(
        shared.kit().name,
        "808 (2)",
        "the second edit is the player's own kit and needs no copy: one copy, not one a frame"
    );
}

#[test]
fn a_drum_cards_kit_row_names_the_kit_and_opens_the_browser() {
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names.clone(), "My Kit");
    panel.kits.open = false;
    panel.device_card.open = Some(0);
    // What `main` fills the card with on the frame it opens: the device's name and kind, the ports, and the kit.
    panel.device_card.offer(
        "DRUMS 1",
        DeviceKind::Drums,
        &InputFilter::everything(),
        Vec::new(),
    );
    panel.device_card.kit_from("My Kit");

    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &shared, vec![]);
    assert!(
        said(&output).iter().any(|line| line == "KIT"),
        "the card captions the row"
    );
    assert!(
        said(&output).iter().any(|line| line == "My Kit"),
        "and says which kit the device plays: {:?}",
        said(&output)
    );

    let s = scale(screen());
    let l = layout(screen());
    let rect = device_card::rows(
        device_card::card_rect(l.panel, s, panel.device_card.kind),
        s,
        &panel.device_card,
    )
    .kit
    .expect("a drum card carries the kit row");
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(rect.center())],
    );
    run(&ctx, &mut panel, &shared, vec![press(rect.center())]);
    run(&ctx, &mut panel, &shared, vec![release(rect.center())]);
    assert!(
        panel.kits.open,
        "the key opens the browser, which is where the other kits are"
    );
    assert!(
        panel.device_card.open.is_none(),
        "and the card that opened it goes: there is room for one"
    );
}

#[test]
fn an_acid_cards_card_has_no_kit_row() {
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names, "My Kit");
    panel.kits.open = false;
    panel.strip.cells[0].kind = DeviceKind::Acid;
    panel.device_card.open = Some(0);
    panel.device_card.offer(
        "ACID 1",
        DeviceKind::Acid,
        &InputFilter::everything(),
        Vec::new(),
    );

    let ctx = Context::default();
    let output = run(&ctx, &mut panel, &shared, vec![]);
    assert!(
        !said(&output).iter().any(|line| line == "KIT"),
        "an acid device has no kit, so its card has no row for one: {:?}",
        said(&output)
    );
    let s = scale(screen());
    let l = layout(screen());
    assert!(
        device_card::rows(
            device_card::card_rect(l.panel, s, panel.device_card.kind),
            s,
            &panel.device_card,
        )
        .kit
        .is_none()
    );
}

#[test]
fn a_copy_lands_beside_a_name_the_library_already_has() {
    // The list `main` reads is the wall the copy is named against. If it were empty on the frames the panel is usable — which
    // it was, until `Kits::default` started asking for a read — a copy could take `808 (2)` while a folder of that name already
    // held the player's work, and the next save would write over it with no notice at all.
    let (_library, mut names) = library_with_one_kit();
    names.insert(names.len() - 1, "808 (2)".to_string());
    let (mut panel, shared) = drum_panel_state(names, "808");
    assert!(
        panel.kits.names.iter().any(|name| name == "808 (2)"),
        "the library's taken name is on the list the copy rule reads"
    );

    let ctx = Context::default();
    let l = layout(screen());
    let attack = drum_panel::dial_cells(&l)[acid_app::knobs::Knob::Waveform.index()].center();
    run(&ctx, &mut panel, &shared, vec![Event::PointerMoved(attack)]);
    run(&ctx, &mut panel, &shared, vec![press(attack)]);
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack - vec2(0.0, 10.0))],
    );
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![release(attack - vec2(0.0, 10.0))],
    );

    assert_eq!(
        shared.kit().name,
        "808 (3)",
        "a copy steps past every name the library holds, taken ones included"
    );
}

#[test]
fn loading_a_kit_over_an_unsaved_edit_takes_two_clicks() {
    // The edit is not on disk, so one click would throw it away. The first click asks and says so on the card; the second is
    // the answer. Same rule as loading over an unsaved song, and the same two-click guard the book describes for it.
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names.clone(), "My Kit");
    panel.kit_dirty = Some(1);

    let ctx = Context::default();
    let s = scale(screen());
    let l = layout(screen());
    let row = kits::rows(l.panel, s, &names).mine[0];

    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(row.center())],
    );
    run(&ctx, &mut panel, &shared, vec![press(row.center())]);
    let asked = run(&ctx, &mut panel, &shared, vec![release(row.center())]);
    assert_eq!(
        panel.kits_request, None,
        "one click on a row whose kit has unsaved edits asks rather than loads"
    );
    assert!(
        panel.kits.load_armed.is_some(),
        "and the row is armed, waiting for the answer"
    );
    assert!(
        said(&asked).iter().any(|line| line.contains("click again")),
        "the card says what the second click will do: {:?}",
        said(&asked)
    );

    run(&ctx, &mut panel, &shared, vec![press(row.center())]);
    run(&ctx, &mut panel, &shared, vec![release(row.center())]);
    assert_eq!(
        panel.kits_request,
        Some(Request::Load("My Kit".to_string())),
        "the second click is the answer"
    );
    assert!(
        panel.kits.load_armed.is_none(),
        "and the question is put away"
    );
}

#[test]
fn a_fresh_panel_asks_for_its_kit_list() {
    // The copy rule names a factory kit's copy against the list of kits on offer, so a panel whose list has never been read
    // has to say so — otherwise the first edit of `808` could take a name a folder already holds.
    assert!(
        PanelState::default().kits.refresh,
        "a kit list nobody has read is not up to date, and the copy rule reads it"
    );
    assert!(
        PanelState::default().kits.names.is_empty(),
        "and nothing is listed until `main` reads the library"
    );
}

#[test]
fn opening_a_card_puts_the_kit_browser_away() {
    // Five cards and one rule. The browser was the card the rule forgot — and both of the ways in that forgot it were keys,
    // so the symptom was a key that looked dead: F2 opened the settings card underneath an opaque, panel-sized browser. The
    // rule lives in one method, and every path that opens a card calls it.
    let (_library, names) = library_with_one_kit();
    let (mut panel, shared) = drum_panel_state(names, "My Kit");
    let bank = PatternBank::factory();
    let press = |key: Key| dispatch::KeyPress {
        key,
        shift: false,
        ctrl: false,
    };
    assert!(panel.kits.open);

    panel.open_device_card(0);
    assert!(
        !panel.kits.open,
        "opening a device card puts the browser away"
    );
    assert_eq!(panel.device_card.open, Some(0));

    panel.open_device_picker();
    assert_eq!(panel.device_card.open, None, "and the picker does the same");

    // F2 and F1 are the two paths a pointer never takes, and both went through a hand-kept list of "the others".
    let mut bank = bank;
    kits::open(&mut panel);
    assert!(panel.kits.open);
    dispatch::handle(press(Key::F2), &mut panel, &shared, &mut bank);
    assert!(panel.settings_open);
    assert!(
        !panel.kits.open,
        "F2 opens settings over everything, so the browser goes first"
    );

    kits::open(&mut panel);
    dispatch::handle(press(Key::F1), &mut panel, &shared, &mut bank);
    assert!(panel.help_visible);
    assert!(!panel.settings_open, "F1 puts settings away");
    assert!(!panel.kits.open, "and the browser with it");

    // And a key behind a card is a key the panel refuses: with the browser up over an acid face, Enter must not write a step.
    let mut acid = drum_panel_state(vec!["808".to_string()], "808").0;
    acid.strip.cells[0].kind = DeviceKind::Acid;
    acid.kits.open = true;
    let steps_before = shared.pattern().steps;
    dispatch::handle(press(Key::Enter), &mut acid, &shared, &mut bank);
    assert_eq!(
        shared.pattern().steps,
        steps_before,
        "the step editor refuses its keys while a card is up, the browser included"
    );
    assert!(
        !acid.dirty,
        "so the session is not marked unsaved by a key nobody saw"
    )
}

#[test]
fn a_long_kit_list_is_capped_above_the_keys() {
    // A player accumulates kits: one per edit of a factory kit. The rows must stop before the footer, or IMPORT and CLOSE are
    // painted over and the list runs off the card — which is what a hand-kept list of forty would do.
    let mut names: Vec<String> = factory_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    for index in 0..40 {
        names.push(format!("Kit {index}"));
    }
    let s = scale(screen());
    let l = layout(screen());
    let laid = kits::rows(l.panel, s, &names);
    let card = kits::card_rect(l.panel, s);
    let painted = laid.factory.iter().chain(laid.mine.iter()).count();
    assert!(
        painted < names.len(),
        "the list is capped: {painted} of {} rows",
        names.len()
    );
    for rect in laid.factory.iter().chain(laid.mine.iter()) {
        assert!(
            card.contains_rect(*rect),
            "a row escapes the card: {rect:?}"
        );
        assert!(
            rect.bottom() <= laid.import.top(),
            "a row is painted over the footer keys: {rect:?}"
        );
    }
    assert!(card.contains_rect(laid.import) && card.contains_rect(laid.close));
}

#[test]
fn an_edit_records_which_device_is_owed_the_write() {
    // The device edited is not the device selected when the save happens: a player can turn a dial on the second drum machine,
    // click back to the bass and save. So the panel records the *number* of the device it edited, and `main` writes that one.
    let names: Vec<String> = factory_names()
        .iter()
        .map(|name| name.to_string())
        .collect();
    let (mut panel, shared) = drum_panel_state(names, "808");
    // Two machines, and the second one is the one on the panel.
    panel.strip.cells.push(StripCell {
        name: "DRUMS 2".to_string(),
        number: 2,
        scheme: Scheme::Black,
        running: false,
        muted: false,
        kind: DeviceKind::Drums,
    });
    panel.strip.selected = 1;

    let ctx = Context::default();
    let l = layout(screen());
    let attack = drum_panel::dial_cells(&l)[acid_app::knobs::Knob::Waveform.index()].center();
    run(&ctx, &mut panel, &shared, vec![Event::PointerMoved(attack)]);
    run(&ctx, &mut panel, &shared, vec![press(attack)]);
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![Event::PointerMoved(attack - vec2(0.0, 12.0))],
    );
    run(
        &ctx,
        &mut panel,
        &shared,
        vec![release(attack - vec2(0.0, 12.0))],
    );

    assert_eq!(
        panel.kit_dirty,
        Some(2),
        "the write is owed for the device that was edited, by its number, not for the first one"
    );
}
