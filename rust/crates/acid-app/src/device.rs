//! A device, and the rack of them.
//!
//! A [`Device`] is one machine: its own engine state ([`SharedState`]), its own pattern bank, its own number and name. A
//! [`Rack`] owns the list, the selection and the transport, and publishes what the other two threads read.

use std::sync::Arc;

use std::sync::atomic::{AtomicBool, Ordering};

use acid_core::timing::clamp_bpm;
use arc_swap::ArcSwap;

use acid_core::bank::PatternBank;

use crate::state::{AtomicF32, SharedState};

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
    /// What this device hears: which ports, which channel, and whether it plays notes or only answers controllers. A device
    /// that has just been added hears nothing until it is given a port.
    pub inputs: crate::midi::route::InputFilter,
    /// Stable for the device's life: taken from the rack's counter when it is created, never reused and never renumbered.
    pub number: usize,
    /// What the strip and the cards call it. Editable, and distinct from its neighbours.
    pub name: String,
}

impl Device {
    /// A brand-new device: the starter phrase in `A1`, the launch patch, and the number it was given, on a tempo cell of
    /// its own.
    pub fn new(kind: DeviceKind, number: usize) -> Self {
        Self::on(kind, number, SharedState::new())
    }

    /// A brand-new device built on somebody else's state — how a device joins a rack, where the state holds the rack's own
    /// tempo cell so there is one tempo for every engine.
    pub fn on(kind: DeviceKind, number: usize, state: Arc<SharedState>) -> Self {
        // The same pair `persist::new_song` builds, so a new device and a new song open on the same phrase.
        let mut bank = PatternBank::new_empty();
        bank.replace(0, acid_core::factory::starter());
        let mut device = Self::adopted(kind, number, state, bank);
        // A device that arrives hearing nothing cannot be layered under a keyboard that is already playing.
        device.inputs = crate::midi::route::InputFilter::silent();
        device
    }

    /// A device wrapped around state and patterns that already exist — what a session loads.
    pub fn adopted(
        kind: DeviceKind,
        number: usize,
        state: Arc<SharedState>,
        bank: PatternBank,
    ) -> Self {
        // The engine plays what `SharedState` holds, not what the bank holds. A device whose state was never handed its
        // pattern renders silence, which the mixer's sum test found the day a second device existed.
        state.set_pattern(Arc::new(*bank.current_pattern()));
        state.set_pattern_index(bank.current());
        Self {
            kind,
            number,
            name: Self::default_name(kind, number),
            // A loaded device hears everything, which is what the factory device does: a session that never opens the device
            // card cannot tell phase 2 happened.
            inputs: crate::midi::route::InputFilter::everything(),
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
    /// One entry per device, in the same order: what each device hears. Read by the MIDI handlers, ignored by the mixer.
    pub routes: Vec<crate::midi::route::InputFilter>,
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

/// What the panel asks the rack to do.
///
/// The panel paints and asks; `main` owns the list and answers. A catalogue: `main`'s drain matches it in full, so a new
/// request fails to compile where a decision is owed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Request {
    /// Show this device on the panel.
    Select(usize),
    /// Mute or unmute this device — the index the key was drawn for, never the selected one.
    SetMute(usize, bool),
    /// Add a device of this kind.
    Add(DeviceKind),
}

/// A list of devices, the one the panel is showing, and what the other threads read.
pub struct Rack {
    devices: Vec<Device>,
    selected: usize,
    /// The number the next device will be given. A high-water mark: a removal never frees one, so two devices never share a
    /// default name and "the one I muted" is still the one that was muted.
    next_number: usize,
    /// The rack's transport. Every device follows it, so RUN starts them all on one frame — the downbeats coincide rather
    /// than drifting apart by a block.
    playing: AtomicBool,
    /// The rack's tempo: one cell, and the same cell in every device's `SharedState`. The panel, an external clock and a
    /// tempo controller all write it, and every engine reads it at its next block, so no fan-out and no frame stands between
    /// a hand and the sound. Swing stays each device's own.
    tempo_bpm: Arc<AtomicF32>,
    bus: Arc<ArcSwap<BusState>>,
}

impl Rack {
    /// The session's rack: one device holding the bank and the state the save file carried, at the tempo it was saved at.
    pub fn from_saved(saved: &crate::persist::SavedState) -> Self {
        let device = Device::adopted(
            DeviceKind::Acid,
            1,
            crate::persist::saved_to_shared(saved),
            saved.bank.clone(),
        );
        Self::with_device(device, saved.bpm)
    }

    fn with_device(device: Device, tempo_bpm: f32) -> Self {
        // The rack's tempo is the first device's own cell: one atomic, which every device that joins is built holding, so a
        // tempo written by the panel or the MIDI thread is heard by every engine at its next block without a fan-out.
        device.state.tempo_bpm.set(clamp_bpm(tempo_bpm));
        let tempo_bpm = Arc::clone(&device.state.tempo_bpm);
        let next_number = device.number + 1;
        let rack = Self {
            devices: vec![device],
            selected: 0,
            next_number,
            playing: AtomicBool::new(false),
            tempo_bpm,
            bus: Arc::new(ArcSwap::from_pointee(BusState {
                slots: Vec::new(),
                routes: Vec::new(),
            })),
        };
        rack.sync_transport();
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
        // Built in one pass, so the two lists cannot drift: every reader assumes `routes[i]` belongs to `slots[i]`.
        let mut slots = Vec::with_capacity(self.devices.len());
        let mut routes = Vec::with_capacity(self.devices.len());
        for device in &self.devices {
            slots.push(Arc::clone(&device.state));
            routes.push(device.inputs.clone());
        }
        self.bus.store(Arc::new(BusState { slots, routes }));
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

    /// The selected device, mutably, for the callers that need its state and its bank at once.
    ///
    /// `f(&device.state, &mut device.bank)` is two disjoint field borrows of one `&mut Device`, which the compiler allows;
    /// `f(rack.state(), rack.bank_mut())` is two borrows of the same `rack`, which it does not.
    pub fn device_mut(&mut self) -> &mut Device {
        let index = self.selected;
        &mut self.devices[index]
    }

    /// The device a session saves: the first one, always.
    ///
    /// The save file is one device (the spec's rule, and the frozen format), and devices beyond the first live in memory
    /// until the save-the-rack round. A save must not follow the selection, or adding a device and saving would rewrite the
    /// open song with the new device's single starter pattern.
    pub fn saved_device(&self) -> &Device {
        &self.devices[0]
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
        // Built on the rack's own tempo cell, so the new device's engine follows the tempo without being told it.
        self.devices.push(Device::on(
            kind,
            number,
            SharedState::with_tempo(Arc::clone(&self.tempo_bpm)),
        ));
        // A device that arrives while the rack is playing joins the transport rather than sitting silent.
        self.sync_transport();
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

    /// Whether the rack's transport is running. No device may disagree: the devices' own flags are mirrors of this one.
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Start or stop every device, on the same frame. This is what makes the downbeats coincide.
    pub fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
        self.sync_transport();
    }

    pub fn toggle_playing(&self) -> bool {
        let next = !self.is_playing();
        self.set_playing(next);
        next
    }

    pub fn tempo_bpm(&self) -> f32 {
        self.tempo_bpm.get()
    }

    /// Set the tempo. Every device's state already holds this cell, so there is nothing to fan out: the engines read it at
    /// their next block.
    pub fn set_tempo_bpm(&self, bpm: f32) {
        self.tempo_bpm.set(clamp_bpm(bpm));
    }

    /// The tempo cell, for the MIDI thread's clock follower and for a tempo controller. It is the cell every device's
    /// `SharedState` holds, so whatever writes it is heard by every engine at the next block — no copy, no fan-out, and
    /// nothing a frame can put back.
    pub fn tempo_cell(&self) -> Arc<AtomicF32> {
        Arc::clone(&self.tempo_bpm)
    }

    /// Fan the transport out to every device.
    ///
    /// Only `playing` needs it: the tempo is one cell every device already holds, so there is no tempo to carry. Called when
    /// the rack's transport changes and when a device joins, never per frame.
    pub fn sync_transport(&self) {
        let playing = self.is_playing();
        for device in &self.devices {
            device.state.playing.store(playing, Ordering::Relaxed);
        }
    }
}

impl Rack {
    /// What a device hears. Publishes, because the MIDI handlers read it from the bus and not from here.
    pub fn set_inputs(&mut self, index: usize, inputs: crate::midi::route::InputFilter) {
        if let Some(device) = self.devices.get_mut(index) {
            device.inputs = inputs;
            self.publish();
        }
    }

    /// What a device hears, if there is one at that index.
    pub fn inputs(&self, index: usize) -> Option<&crate::midi::route::InputFilter> {
        self.devices.get(index).map(|device| &device.inputs)
    }
}
