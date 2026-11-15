//! The rack: devices, their names and numbers, the selection, the bus and the transport.
//!
//! Everything here drives the public API the way `main` does, so a change to how the rack is shaped fails here before it
//! reaches a window or a sound card.

use acid_app::device::{AddError, Device, DeviceKind, MAX_DEVICES, Rack, RemoveError};
use acid_app::persist;
use acid_core::bank::PatternBank;

fn one() -> Rack {
    Rack::from_saved(&persist::SavedState::default())
}

#[test]
fn a_new_device_is_numbered_named_and_has_something_to_play() {
    let device = Device::new(DeviceKind::Acid, 2);
    assert_eq!(device.number, 2);
    assert_eq!(device.name, "ACID 2");
    assert_eq!(Device::default_name(DeviceKind::Acid, 3), "ACID 3");
    // A device that arrives silent is a bug report; the starter phrase is what a new song opens on.
    assert!(
        device
            .bank
            .current_pattern()
            .steps
            .iter()
            .any(|step| step.on)
    );
    // Its own engine state, not a shared one: muting it cannot mute its neighbour.
    assert!(!device.state.is_muted());
    assert_eq!(MAX_DEVICES, 4);
    assert_eq!(DeviceKind::ALL, [DeviceKind::Acid]);
}

#[test]
fn a_rack_opens_on_one_device_and_publishes_it() {
    let rack = one();
    assert_eq!(rack.devices().len(), 1);
    assert_eq!(rack.selected(), 0);
    assert_eq!(rack.devices()[0].name, "ACID 1");
    let held = rack.hold();
    assert_eq!(held.slots.len(), 1);
    assert!(std::sync::Arc::ptr_eq(
        &held.slots[0],
        &rack.devices()[0].state
    ));
}

/// One rule for "the rack has no room": the strip's `+` and the picker's rows both ask the catalogue, so they cannot disagree
/// about when a rack is full. A kind that is not enabled is still *clickable* — the refusal is the rack's to say.
#[test]
fn a_kind_is_offered_until_the_rack_holds_its_last_device() {
    for kind in DeviceKind::ALL {
        assert!(kind.enabled(0));
        assert!(kind.enabled(MAX_DEVICES - 1), "{kind:?} at the last seat");
        assert!(!kind.enabled(MAX_DEVICES), "{kind:?} when the rack is full");
        assert!(!kind.enabled(MAX_DEVICES + 3));
    }
}

#[test]
fn adding_publishes_in_rack_order_and_the_bus_follows_the_list() {
    let mut rack = one();
    let second = rack.add(DeviceKind::Acid).expect("room for a second");
    assert_eq!(second, 1);
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.slots.len(), 2);
    for (index, slot) in held.slots.iter().enumerate() {
        assert!(std::sync::Arc::ptr_eq(slot, &rack.devices()[index].state));
    }
}

#[test]
fn the_selection_never_leaves_the_list() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second device");
    assert!(rack.select(1));
    assert_eq!(rack.selected(), 1);
    assert!(
        !rack.select(7),
        "an out-of-range pick is refused, not clamped"
    );
    assert_eq!(rack.selected(), 1);

    // Removing the selected device leaves the selection on a device that exists.
    rack.remove(1).expect("a first of two may go");
    assert_eq!(rack.devices().len(), 1);
    assert_eq!(rack.selected(), 0);
    let held = rack.hold();
    assert_eq!(held.slots.len(), 1);
    assert!(std::sync::Arc::ptr_eq(
        &held.slots[0],
        &rack.devices()[0].state
    ));
}

#[test]
fn the_last_device_cannot_go_and_the_cap_holds() {
    let mut rack = one();
    assert_eq!(rack.remove(0), Err(RemoveError::LastDevice));
    assert_eq!(rack.remove(9), Err(RemoveError::OutOfRange));

    for _ in 1..4 {
        rack.add(DeviceKind::Acid).expect("up to four");
    }
    assert_eq!(rack.devices().len(), 4);
    assert_eq!(rack.add(DeviceKind::Acid), Err(AddError::AtCapacity));
    assert_eq!(rack.devices().len(), 4, "a refusal changes nothing");
}

#[test]
fn a_rack_from_a_saved_session_is_the_session_it_was() {
    let saved = persist::SavedState {
        bank: PatternBank::factory(),
        knobs: persist::Knobs {
            cutoff: 0.25,
            ..Default::default()
        },
        ..Default::default()
    };
    let rack = Rack::from_saved(&saved);
    assert_eq!(rack.devices().len(), 1);
    assert!((rack.state().cutoff.get() - 0.25).abs() < 1e-6);
    assert_eq!(rack.bank().current(), saved.bank.current());
}
#[test]
fn a_device_number_is_never_reused() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second"); // ACID 2
    rack.add(DeviceKind::Acid).expect("a third"); // ACID 3
    rack.remove(1).expect("a device may go");
    let index = rack.add(DeviceKind::Acid).expect("room again");
    assert_eq!(
        rack.devices()[index].number,
        4,
        "the gap at 2 is not filled"
    );
    assert_eq!(rack.devices()[index].name, "ACID 4");
}

#[test]
fn renaming_lands_on_one_device_and_keeps_names_distinct() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");

    assert_eq!(
        rack.rename(0, "  BASS  "),
        "BASS",
        "the typed name is trimmed"
    );
    assert_eq!(rack.devices()[0].name, "BASS");
    assert_eq!(
        rack.devices()[1].name,
        "ACID 2",
        "the neighbour is untouched"
    );

    assert_eq!(
        rack.rename(1, "BASS"),
        "BASS (2)",
        "a collision is suffixed, not refused"
    );
    assert_eq!(rack.devices()[1].name, "BASS (2)");

    assert_eq!(
        rack.rename(1, "   "),
        "ACID 2",
        "blank falls back to the device's own name"
    );
    assert_eq!(
        rack.rename(0, "BASS"),
        "BASS",
        "renaming back to your own name is not a collision"
    );
}

#[test]
fn the_transport_is_the_racks_and_the_devices_follow_it() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");

    assert!(!rack.is_playing());
    rack.set_playing(true);
    assert!(rack.is_playing());
    for device in rack.devices() {
        assert!(
            device.state.is_playing(),
            "every device starts on the same frame"
        );
    }
    rack.set_playing(false);
    for device in rack.devices() {
        assert!(!device.state.is_playing());
    }

    rack.set_tempo_bpm(5.0);
    assert_eq!(
        rack.tempo_bpm(),
        acid_core::timing::MIN_BPM,
        "out of range is clamped, not stored"
    );
    rack.set_tempo_bpm(141.0);
    for device in rack.devices() {
        assert_eq!(device.state.tempo_bpm.get(), 141.0);
    }
}

#[test]
fn a_device_added_while_the_transport_runs_starts_in_it() {
    let mut rack = one();
    rack.set_playing(true);
    rack.set_tempo_bpm(152.0);
    let index = rack.add(DeviceKind::Acid).expect("a second");
    let device = &rack.devices()[index];
    assert!(
        device.state.is_playing(),
        "a new device joins the transport, it does not sit silent"
    );
    assert_eq!(device.state.tempo_bpm.get(), 152.0);
}

#[test]
fn a_tempo_written_straight_into_the_cell_reaches_every_device() {
    // This is the MIDI thread's path: the clock follower writes the cell, and every device is reading that cell already.
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.tempo_cell().set(129.0);
    for device in rack.devices() {
        assert_eq!(device.state.tempo_bpm.get(), 129.0);
    }
}

/// The save file is one device: the first, never the selected one. `+` and `Ctrl+2` made the selection movable, so this is
/// the rule that keeps SAVE from rewriting the open song with a device the format cannot hold.
#[test]
fn a_session_saves_the_first_device_whatever_is_selected() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    assert!(rack.select(1));
    assert_eq!(
        rack.saved_device().number,
        1,
        "the first device is the one on disk, not the one on the panel"
    );
}

/// The bug this pins, joined the way `main` joins the parts: a fader learned to the tempo wrote one device's `tempo_bpm`,
/// and the next `sync_transport` put the rack's value back — so factory CC 72 did nothing at all. One router, the rack's
/// own bus and tempo cell, and the tempo has to arrive in the cell the fan-out reads.
#[test]
fn a_tempo_fader_on_a_router_moves_the_rack_and_its_devices() {
    use acid_app::midi::route::Router;

    let rack = one();
    let bus = rack.bus();
    let mut router = Router::new("Launchkey MIDI", rack.tempo_cell());

    // CC 72 takes pickup: the first message says where the fader is, and the sweep up to the knob is what turns it.
    router.on_message(&bus.load(), &[0xB0, 72, 0]);
    assert_eq!(
        rack.tempo_bpm(),
        acid_core::timing::DEFAULT_BPM,
        "a parked fader leaves the tempo alone"
    );
    router.on_message(&bus.load(), &[0xB0, 72, 127]);

    assert_eq!(
        rack.tempo_bpm(),
        acid_core::timing::MAX_BPM,
        "the fader lands in the cell every device holds"
    );
    assert_eq!(
        rack.devices()[0].state.tempo_bpm.get(),
        acid_core::timing::MAX_BPM,
        "and the device is reading that same cell, so no frame is owed one"
    );
}

/// One tempo, one atomic: every device in the rack is built holding the rack's own cell, so nothing has to be copied into
/// anything and no fan-out can put an older tempo back under a moving fader.
#[test]
fn every_device_in_a_rack_holds_the_racks_tempo_cell() {
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    let cell = rack.tempo_cell();
    for device in rack.devices() {
        assert!(
            std::sync::Arc::ptr_eq(&cell, &device.state.tempo_bpm),
            "{} has a tempo of its own, which is a copy somebody has to fan out",
            device.name
        );
    }
}

/// The engine reads `SharedState::tempo_bpm` at the next block and the panel reads it every frame, so a tempo written by
/// the MIDI thread must already be there: no fan-out, no frame, no window in which the pickup sees an older value.
#[test]
fn a_tempo_the_midi_thread_writes_is_heard_without_a_frame() {
    use acid_app::midi::route::Router;

    let rack = one();
    let bus = rack.bus();
    let mut router = Router::new("Launchkey MIDI", rack.tempo_cell());
    router.on_message(&bus.load(), &[0xB0, 72, 0]);
    router.on_message(&bus.load(), &[0xB0, 72, 127]);

    assert_eq!(
        rack.tempo_bpm(),
        acid_core::timing::MAX_BPM,
        "the rack's own cell moved"
    );
    assert_eq!(
        rack.devices()[0].state.tempo_bpm.get(),
        acid_core::timing::MAX_BPM,
        "and the device's tempo is that same cell, so no frame is owed one"
    );
}

/// A sweep is many messages; the tempo must follow every one of them. With the fader writing the cell the pickup measures
/// against, a quick move cannot outrun a fan-out and drop the pin halfway.
#[test]
fn a_quick_fader_sweep_keeps_moving_the_tempo() {
    use acid_app::midi::route::Router;

    let rack = one();
    let bus = rack.bus();
    let mut router = Router::new("Launchkey MIDI", rack.tempo_cell());

    // From the bottom, past the knob at 130 bpm, and on to the top in steps no frame could keep up with.
    let mut seen = Vec::new();
    for value in (0..=127u8).step_by(4).chain(std::iter::once(127)) {
        router.on_message(&bus.load(), &[0xB0, 72, value]);
        seen.push(rack.tempo_bpm());
    }

    assert_eq!(
        rack.tempo_bpm(),
        acid_core::timing::MAX_BPM,
        "the sweep ends at the top"
    );
    let rising = seen.windows(2).filter(|pair| pair[1] > pair[0]).count();
    assert!(
        rising > 20,
        "the fader kept turning the whole way, not for a step or two: {seen:?}"
    );
    assert_eq!(rack.devices()[0].state.tempo_bpm.get(), rack.tempo_bpm());
}

/// The sound, not just the state: the renderer's next block pushes the snapshot into the engine. Driven through the
/// *clock*, which is the path that used to write only the rack's cell and leave the engine reading an old copy until a
/// frame fanned it out — so this is the test that would have caught the round's bug on its own.
#[test]
fn a_followed_tempo_reaches_the_engine_at_the_next_block() {
    use acid_app::audio::renderer;
    use acid_app::midi::route::Router;
    use acid_app::midi::tempo::PPQN;
    use acid_core::engine::Engine;

    let rack = one();
    let bus = rack.bus();
    let mut router = Router::new("Launchkey MIDI", rack.tempo_cell());
    // A steady 120 bpm stream: the follower believes a run of agreeing ticks, not one interval.
    let step = std::time::Duration::from_secs_f64(60.0 / (120.0 * f64::from(PPQN)));
    let start = std::time::Instant::now();
    for tick in 0..12 {
        router.on_message_at(&bus.load(), &[0xF8], start + step * tick);
    }

    let params = rack.devices()[0].state.snapshot();
    let mut engine = Engine::new(48_000.0);
    renderer::apply(&params, &mut engine);

    let heard = engine.bpm();
    assert!(
        (heard - 120.0).abs() < 5.0,
        "the engine should be playing the clock's tempo at its next block, was {heard}"
    );
}

#[test]
fn the_bus_carries_the_route_beside_the_slot() {
    use acid_app::midi::route::{Channel, InputFilter, Ports, Role};
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.publish();
    let held = rack.hold();
    assert_eq!(held.routes.len(), held.slots.len());
    assert_eq!(
        held.routes[0],
        InputFilter::everything(),
        "the factory device hears everything, as it always did"
    );
    assert_eq!(
        held.routes[1],
        InputFilter::silent(),
        "a device just added hears nothing"
    );

    rack.set_inputs(
        1,
        InputFilter {
            ports: Ports::Named(vec!["nanoKONTROL".into()]),
            channel: Channel::All,
            role: Role::ControlOnly,
        },
    );
    assert_eq!(
        rack.hold().routes[1].role,
        Role::ControlOnly,
        "setting a filter publishes it"
    );
    assert_eq!(rack.inputs(1), Some(&input_filter()));
    assert_eq!(rack.inputs(9), None);
}

fn input_filter() -> acid_app::midi::route::InputFilter {
    acid_app::midi::route::InputFilter {
        ports: acid_app::midi::route::Ports::Named(vec!["nanoKONTROL".into()]),
        channel: acid_app::midi::route::Channel::All,
        role: acid_app::midi::route::Role::ControlOnly,
    }
}

/// The seam the MIDI thread depends on: it is handed cells, never the rack. Two routers on two ports must write the same
/// tempo, and the fan-out must carry it to every device — that is what makes one clock move the whole rack.
#[test]
fn a_rack_hands_each_router_the_tempo_cell_it_shares() {
    let rack = one();
    let a = rack.tempo_cell();
    let b = rack.tempo_cell();
    a.set(144.0);
    assert_eq!(b.get(), 144.0, "one cell, two handles");
    assert!(
        std::sync::Arc::ptr_eq(&a, &b),
        "a clone of the rack's cell, never a copy of its value"
    );

    rack.sync_transport();
    assert_eq!(rack.devices()[0].state.tempo_bpm.get(), 144.0);
}

/// The theme catalogue: the instrument's own face first, then one per palette on the page, and a name for each that the
/// card's list can print.
#[test]
fn the_theme_catalogue_starts_with_black_and_names_every_theme() {
    use acid_app::device::Scheme;

    assert_eq!(Scheme::ALL.len(), 22, "BLACK and the page's twenty-one");
    assert_eq!(
        Scheme::ALL[0],
        Scheme::Black,
        "the instrument's own face first"
    );
    assert_eq!(Scheme::default(), Scheme::Black, "a new device is black");
    assert_eq!(Scheme::Black.name(), "BLACK");
    for (index, scheme) in Scheme::ALL.iter().enumerate() {
        // The index is found from ALL, not written down: a second positional list is the bug this repo has paid for twice.
        assert_eq!(scheme.index(), index, "{scheme:?} is not where ALL says");
        assert!(!scheme.name().is_empty(), "{scheme:?} has no caption");
        assert!(scheme.name().is_ascii(), "{scheme:?} is a panel caption");
        assert_eq!(
            scheme.name().to_uppercase(),
            scheme.name(),
            "{scheme:?} is set in capitals on the face"
        );
    }
}

/// A colour belongs to a machine: setting one leaves its neighbours alone, survives a selection change, and a device that
/// has just arrived is the instrument's own black.
#[test]
fn a_device_carries_its_own_scheme() {
    use acid_app::device::Scheme;

    let mut rack = Rack::from_saved(&persist::SavedState::default());
    rack.add(DeviceKind::Acid).expect("room for a second");
    assert_eq!(
        rack.devices()[1].scheme,
        Scheme::Black,
        "a new device is black"
    );

    rack.set_scheme(1, Scheme::SynthNeon);
    assert_eq!(rack.devices()[1].scheme, Scheme::SynthNeon);
    assert_eq!(rack.devices()[0].scheme, Scheme::Black, "and only that one");

    rack.select(0);
    assert_eq!(
        rack.devices()[1].scheme,
        Scheme::SynthNeon,
        "a colour is not a selection"
    );

    // An index the rack does not hold changes nothing and does not panic: the panel may be a frame behind a removal. The
    // scheme is the one device 1 is *not*, so a write that ignored the bounds would change it and be seen.
    rack.set_scheme(9, Scheme::Black);
    assert_eq!(
        rack.devices()[0].scheme,
        Scheme::Black,
        "a stale index must not repaint the first device"
    );
    assert_eq!(
        rack.devices()[1].scheme,
        Scheme::SynthNeon,
        "nor the one it was not about"
    );
}
