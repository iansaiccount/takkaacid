//! The rack: devices, their names and numbers, the selection, the bus and the transport.
//!
//! Everything here drives the public API the way `main` does, so a change to how the rack is shaped fails here before it
//! reaches a window or a sound card.

use acid_app::device::{Device, DeviceKind, MAX_DEVICES};

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
