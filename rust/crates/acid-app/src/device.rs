//! A device, and the rack of them.
//!
//! A [`Device`] is one machine: its own engine state ([`SharedState`]), its own pattern bank, its own number and name. A
//! [`Rack`] owns the list, the selection and the transport, and publishes what the other two threads read.

use std::sync::Arc;

use acid_core::bank::PatternBank;

use crate::state::SharedState;

/// The most devices a rack may hold.
///
/// A strip of four cells fills the panel's width, and four acid lines is already a lot of voice. A hard cap rather than a
/// growing list: the strip has to show every device at once, and a rack that outgrows its own strip is a layout problem this
/// round does not want. `+` greys out at the cap instead of failing silently.
pub const MAX_DEVICES: usize = 4;

/// What a device is, in the catalogue the picker reads.
///
/// One entry today, and `match`es over it are written out in full on purpose: a second kind has to fail to compile wherever a
/// decision is owed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    /// The 303 clone: one voice, one bank, one shuffle.
    Acid,
}

impl DeviceKind {
    /// Every kind the picker offers, in the order it offers them.
    pub const ALL: [DeviceKind; 1] = [DeviceKind::Acid];

    /// The caption on a picker row.
    pub fn name(self) -> &'static str {
        match self {
            DeviceKind::Acid => "ACID",
        }
    }
}

/// One machine: its own sound, its own patterns, its own identity.
pub struct Device {
    pub kind: DeviceKind,
    /// The engine state this device's thread pair shares: its knobs, its pattern, its mute, its learn map, its position.
    pub state: Arc<SharedState>,
    /// The sixty-four patterns this device is working through.
    pub bank: PatternBank,
    /// Stable for the device's life: taken from the rack's counter when it is created, never reused and never renumbered.
    pub number: usize,
    /// What the strip and the cards call it. Editable, and distinct from its neighbours.
    pub name: String,
}

impl Device {
    /// A brand-new device: the starter phrase in `A1`, the launch patch, and the number it was given.
    pub fn new(kind: DeviceKind, number: usize) -> Self {
        // The same pair `persist::new_song` builds, so a new device and a new song open on the same phrase.
        let mut bank = PatternBank::new_empty();
        bank.replace(0, acid_core::factory::starter());
        Self::adopted(kind, number, SharedState::new(), bank)
    }

    /// A device wrapped around state and patterns that already exist — what a session loads.
    pub fn adopted(
        kind: DeviceKind,
        number: usize,
        state: Arc<SharedState>,
        bank: PatternBank,
    ) -> Self {
        Self {
            kind,
            number,
            name: Self::default_name(kind, number),
            state,
            bank,
        }
    }

    /// The name a device carries until a hand types one: `ACID 3`.
    pub fn default_name(kind: DeviceKind, number: usize) -> String {
        format!("{} {number}", kind.name())
    }
}
