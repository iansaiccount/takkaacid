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
