//! A device, and the rack of them.
//!
//! A [`Device`] is one machine: its own engine state ([`SharedState`]), its own pattern bank, its own number and name. A
//! [`Rack`] owns the list, the selection and the transport, and publishes what the other two threads read.

use std::sync::Arc;

use arc_swap::ArcSwap;

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

/// What the audio callback and the MIDI handlers read: the rack's devices, in rack order.
///
/// One published value rather than several atomics, so a reader can never see a half-changed list. `Rack::publish` writes it;
/// the mixer loads it once per block and each port's router loads it once per message.
#[derive(Debug)]
pub struct BusState {
    /// One entry per device, in rack order. The mixer reconciles its engines against this list and nothing else.
    pub slots: Vec<Arc<SharedState>>,
}

/// The rack refused to take another device.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum AddError {
    /// [`MAX_DEVICES`] is already holding devices.
    AtCapacity,
}

/// The rack refused to give up a device.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum RemoveError {
    /// A rack always holds at least one device: there is nothing to play a pattern on without one.
    LastDevice,
    /// There is no device at that index.
    OutOfRange,
}

/// A list of devices, the one the panel is showing, and what the other threads read.
pub struct Rack {
    devices: Vec<Device>,
    selected: usize,
    /// The number the next device will be given. A high-water mark: a removal never frees one, so two devices never share a
    /// default name and "the one I muted" is still the one that was muted.
    next_number: usize,
    bus: Arc<ArcSwap<BusState>>,
}

impl Rack {
    /// The session's rack: one device holding the bank and the state the save file carried.
    pub fn from_saved(saved: &crate::persist::SavedState) -> Self {
        let device = Device::adopted(
            DeviceKind::Acid,
            1,
            crate::persist::saved_to_shared(saved),
            saved.bank.clone(),
        );
        Self::from_device(device)
    }

    /// A rack of one device that already belongs to somebody else.
    pub fn from_device(device: Device) -> Self {
        let next_number = device.number + 1;
        let rack = Self {
            devices: vec![device],
            selected: 0,
            next_number,
            bus: Arc::new(ArcSwap::from_pointee(BusState { slots: Vec::new() })),
        };
        rack.publish();
        rack
    }

    /// The bus, for the audio thread and the MIDI handlers to hold a clone of.
    pub fn bus(&self) -> Arc<ArcSwap<BusState>> {
        Arc::clone(&self.bus)
    }

    /// Publish the device list. Adding or removing a device ends here; a selection does not, because nothing but the panel
    /// draws one. An `ArcSwap::store` never blocks a reader.
    pub fn publish(&self) {
        let slots = self
            .devices
            .iter()
            .map(|device| Arc::clone(&device.state))
            .collect();
        self.bus.store(Arc::new(BusState { slots }));
    }

    /// What the other threads currently see.
    pub fn hold(&self) -> Arc<BusState> {
        self.bus.load_full()
    }

    pub fn devices(&self) -> &[Device] {
        &self.devices
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Show a device. An index that is not there is refused rather than clamped: the caller decides what to show instead.
    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.devices.len() {
            return false;
        }
        self.selected = index;
        true
    }

    /// The device the panel is showing.
    pub fn device(&self) -> &Device {
        &self.devices[self.selected]
    }

    pub fn state(&self) -> &SharedState {
        &self.device().state
    }

    pub fn state_arc(&self) -> Arc<SharedState> {
        Arc::clone(&self.device().state)
    }

    pub fn bank(&self) -> &PatternBank {
        &self.device().bank
    }

    pub fn bank_mut(&mut self) -> &mut PatternBank {
        let index = self.selected;
        &mut self.devices[index].bank
    }

    /// A new device, numbered from the high-water mark, placed last and published. It does not take the selection: that is
    /// the panel's request to make, so the rack never moves under a hand that was not asking.
    pub fn add(&mut self, kind: DeviceKind) -> Result<usize, AddError> {
        if self.devices.len() >= MAX_DEVICES {
            return Err(AddError::AtCapacity);
        }
        let number = self.next_number;
        self.next_number += 1;
        self.devices.push(Device::new(kind, number));
        self.publish();
        Ok(self.devices.len() - 1)
    }

    /// Give up a device. The last one stays, and the selection never points past the end.
    pub fn remove(&mut self, index: usize) -> Result<(), RemoveError> {
        if index >= self.devices.len() {
            return Err(RemoveError::OutOfRange);
        }
        if self.devices.len() == 1 {
            return Err(RemoveError::LastDevice);
        }
        self.devices.remove(index);
        if self.selected >= self.devices.len() {
            self.selected = self.devices.len() - 1;
        }
        self.publish();
        Ok(())
    }

    /// Give a device a name of its own, and answer the name it landed on.
    ///
    /// It never fails: a blank falls back to the device's default name, and a name another device already holds is suffixed
    /// `(2)` — the rule `persist::free_song_name` uses for songs — because two `BASS` cells would make "which one is muted"
    /// unanswerable. Truncation is the strip's business (`ui::text::fit`), not the store's. An index that is not there
    /// answers with the empty string and changes nothing.
    pub fn rename(&mut self, index: usize, typed: &str) -> String {
        if index >= self.devices.len() {
            return String::new();
        }
        let wanted = match typed.trim() {
            "" => Device::default_name(self.devices[index].kind, self.devices[index].number),
            trimmed => trimmed.to_string(),
        };
        let mut candidate = wanted.clone();
        let mut copy = 2;
        while self
            .devices
            .iter()
            .enumerate()
            .any(|(other, device)| other != index && device.name == candidate)
        {
            candidate = format!("{wanted} ({copy})");
            copy += 1;
        }
        self.devices[index].name.clone_from(&candidate);
        candidate
    }
}
