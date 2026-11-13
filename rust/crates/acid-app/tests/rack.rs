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
    // This is the MIDI thread's path: the clock follower writes the cell, and the next sync carries it.
    let mut rack = one();
    rack.add(DeviceKind::Acid).expect("a second");
    rack.tempo_cell().set(129.0);
    rack.sync_transport();
    for device in rack.devices() {
        assert_eq!(device.state.tempo_bpm.get(), 129.0);
    }
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
