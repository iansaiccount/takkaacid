//! A device, and the rack of them.
//!
//! A [`Device`] is one machine: its own engine state ([`SharedState`]), its own pattern bank, its own number and name. A
//! [`Rack`] owns the list, the selection and the transport, and publishes what the other two threads read.

use std::sync::Arc;

use std::sync::atomic::{AtomicU8, Ordering};

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
/// Two entries today — the bass and the drum machine — and `match`es over it are written out in full on purpose: a third kind
/// has to fail to compile wherever a decision is owed.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DeviceKind {
    /// The 303 clone: one voice, one bank, one shuffle.
    Acid,
    /// The drum machine: sixteen sample voices on the same bar, bank, transport and shuffle.
    Drums,
}

impl DeviceKind {
    /// Every kind the picker offers, in the order it offers them.
    pub const ALL: [DeviceKind; 2] = [DeviceKind::Acid, DeviceKind::Drums];

    /// The kind `name` names, by the caption `name()` spells — the one mapping between a catalogue entry and the word a file
    /// stores. Derived from `ALL`, so a variant added or renamed needs no second list here.
    pub fn from_name(name: &str) -> Option<DeviceKind> {
        DeviceKind::ALL
            .iter()
            .copied()
            .find(|kind| kind.name() == name)
    }

    /// The caption on a picker row, and the stem of a default name: `ACID 3`, `DRUMS 2`.
    pub fn name(self) -> &'static str {
        match self {
            DeviceKind::Acid => "ACID",
            DeviceKind::Drums => "DRUMS",
        }
    }

    /// Whether this kind's sound is a kit of samples — the question a card's `KIT` row and the rate-change reload both ask.
    ///
    /// Written out rather than compared with `== DeviceKind::Drums`: a third kind must fail to compile here and say whether it
    /// has a kit, not quietly take the `false` arm and get no row and no reload.
    pub fn has_kit(self) -> bool {
        match self {
            DeviceKind::Acid => false,
            DeviceKind::Drums => true,
        }
    }

    /// Whether another device of this kind fits in a rack of `count` devices.
    ///
    /// The strip's `+` and the picker's rows both grey out from this, so the two cannot disagree about when the rack is
    /// full. A greyed row still answers a click: the refusal is `Rack::add`'s to say, and a dead key would say nothing.
    pub fn enabled(self, count: usize) -> bool {
        // Every kind shares the rack's cap — the cap is the panel's width and the voice count, not the kind's.
        match self {
            DeviceKind::Acid | DeviceKind::Drums => count < MAX_DEVICES,
        }
    }
}

/// Which theme a device paints its face in — its own, so four machines are told apart at a glance, not the rack's.
///
/// A catalogue: `match`es over it are written out in full, so a theme added or removed fails to compile wherever a decision
/// is owed. The colours are `ui::theme`'s (`theme::palette`), derived from the five-colour palettes on
/// <https://www.media.io/color-palette/music-color-palette.html>; each one's five source colours live in `theme`'s table,
/// which is the one place they are written down.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Scheme {
    /// The face the instrument has always had: near-black slab, amber silkscreen.
    #[default]
    Black,
    /// Page 1.
    VinylNoir,
    /// Page 2.
    StageSpotlight,
    /// Page 3.
    JazzLounge,
    /// Page 4.
    SynthNeon,
    /// Page 5.
    AcousticSunrise,
    /// Page 6.
    FestivalCitrus,
    /// Page 7.
    IndieDusk,
    /// Page 8.
    OperaVelvet,
    /// Page 9.
    LoFiRain,
    /// Page 10.
    DrumlineConcrete,
    /// Page 11.
    ChoirPastel,
    /// Page 12.
    BasslineEmber,
    /// Page 13.
    PopBubblegum,
    /// Page 14.
    OrchestralIce,
    /// Page 15.
    GarageGrunge,
    /// Page 16.
    DiscoMirrorball,
    /// Page 17.
    AmbientMoss,
    /// Page 18.
    KPopCandy,
    /// Page 19.
    StudioSandalwood,
    /// Page 20.
    EdmLaser,
    /// Page 21.
    RetroCassette,
}

impl Scheme {
    /// The theme `name` names, by the caption `name()` spells — the mapping a saved device needs and nothing else does.
    /// Derived from `ALL` for the same reason `DeviceKind::from_name` is: a second list of twenty-two spellings would be the
    /// bug this repo has paid for twice.
    pub fn from_name(name: &str) -> Option<Scheme> {
        Scheme::ALL
            .iter()
            .copied()
            .find(|scheme| scheme.name() == name)
    }

    /// Every theme there is, in the order the card's list offers them.
    pub const ALL: [Scheme; 22] = [
        Scheme::Black,
        Scheme::VinylNoir,
        Scheme::StageSpotlight,
        Scheme::JazzLounge,
        Scheme::SynthNeon,
        Scheme::AcousticSunrise,
        Scheme::FestivalCitrus,
        Scheme::IndieDusk,
        Scheme::OperaVelvet,
        Scheme::LoFiRain,
        Scheme::DrumlineConcrete,
        Scheme::ChoirPastel,
        Scheme::BasslineEmber,
        Scheme::PopBubblegum,
        Scheme::OrchestralIce,
        Scheme::GarageGrunge,
        Scheme::DiscoMirrorball,
        Scheme::AmbientMoss,
        Scheme::KPopCandy,
        Scheme::StudioSandalwood,
        Scheme::EdmLaser,
        Scheme::RetroCassette,
    ];

    /// Where this theme sits in `ALL`. Found rather than written down: a second positional list is the bug this repo has
    /// paid for twice, and twenty-two arms of `Scheme::X => 12` would be exactly that.
    pub fn index(self) -> usize {
        Scheme::ALL
            .iter()
            .position(|scheme| *scheme == self)
            .expect("every scheme is in ALL")
    }

    /// The caption on the card's key and in its list.
    pub fn name(self) -> &'static str {
        match self {
            Scheme::Black => "BLACK",
            Scheme::VinylNoir => "VINYL NOIR",
            Scheme::StageSpotlight => "STAGE SPOTLIGHT",
            Scheme::JazzLounge => "JAZZ LOUNGE",
            Scheme::SynthNeon => "SYNTH NEON",
            Scheme::AcousticSunrise => "ACOUSTIC SUNRISE",
            Scheme::FestivalCitrus => "FESTIVAL CITRUS",
            Scheme::IndieDusk => "INDIE DUSK",
            Scheme::OperaVelvet => "OPERA VELVET",
            Scheme::LoFiRain => "LO FI RAIN",
            Scheme::DrumlineConcrete => "DRUMLINE CONCRETE",
            Scheme::ChoirPastel => "CHOIR PASTEL",
            Scheme::BasslineEmber => "BASSLINE EMBER",
            Scheme::PopBubblegum => "POP BUBBLEGUM",
            Scheme::OrchestralIce => "ORCHESTRAL ICE",
            Scheme::GarageGrunge => "GARAGE GRUNGE",
            Scheme::DiscoMirrorball => "DISCO MIRRORBALL",
            Scheme::AmbientMoss => "AMBIENT MOSS",
            Scheme::KPopCandy => "K POP CANDY",
            Scheme::StudioSandalwood => "STUDIO SANDALWOOD",
            Scheme::EdmLaser => "EDM LASER",
            Scheme::RetroCassette => "RETRO CASSETTE",
        }
    }
}

/// One machine: its own sound, its own patterns, its own identity.
pub struct Device {
    /// The kind is not a field here: the state carries it, and [`Device::kind`] reads it, so a device's face and its sound
    /// cannot disagree about which machine it is.
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
    /// Which way round this device paints its face. In memory only, like the name: the save file is frozen at 9 this phase,
    /// so a colour is gone on restart until the round that saves the whole rack.
    pub scheme: Scheme,
}

impl Device {
    /// What this device is.
    ///
    /// Read from the state, which is the one place the answer lives: `AudioRenderer` builds the machine a device renders
    /// from the same field, so a device's face and its sound cannot disagree about which machine it is.
    pub fn kind(&self) -> DeviceKind {
        self.state.kind
    }

    /// A brand-new device: the starter phrase in `A1`, the launch patch, and the number it was given, on a tempo cell of
    /// its own.
    pub fn new(kind: DeviceKind, number: usize) -> Self {
        Self::on(number, SharedState::for_kind(kind))
    }

    /// A brand-new device built on somebody else's state — how a device joins a rack, where the state holds the rack's own
    /// tempo cell so there is one tempo for every engine. The state already says which machine it is: the rack builds it
    /// with [`SharedState::with_tempo_for`].
    pub fn on(number: usize, state: Arc<SharedState>) -> Self {
        // The same pair `persist::new_song` builds, so a new device and a new song open on the same phrase.
        //
        // A drum device opens on this same pattern with both masks zero, which is an empty grid: the 303's notes are not
        // drum hits, and a starter phrase for the drums is the factory banks' work (plan 01, task 9). What it *sounds* is the
        // drum machine reading those masks, so the grid and the sound agree: nothing on the grid, nothing heard.
        let mut bank = PatternBank::new_empty();
        bank.replace(0, acid_core::factory::starter());
        let mut device = Self::adopted(number, state, bank);
        // A device that arrives hearing nothing cannot be layered under a keyboard that is already playing.
        device.inputs = crate::midi::route::InputFilter::silent();
        device
    }

    /// A device wrapped around state and patterns that already exist — what a session loads. Its kind is the state's.
    pub fn adopted(number: usize, state: Arc<SharedState>, bank: PatternBank) -> Self {
        let device = Self {
            number,
            name: Self::default_name(state.kind, number),
            scheme: Scheme::Black,
            // A loaded device hears everything, which is what the factory device does: a session that never opens the device
            // card cannot tell phase 2 happened.
            inputs: crate::midi::route::InputFilter::everything(),
            state,
            bank,
        };
        device.publish_pattern();
        device
    }

    /// Give this device a whole bank of patterns, and open on its first: what loading a factory bank does.
    ///
    /// A bank is patterns only — the kit, the knobs, the name and the route are not its business, so a machine handed a
    /// groove keeps the sound it was edited to make. *Which* bank a device takes is its caller's question, and it is the
    /// caller's to answer: a drum device loads the drum banks, a 303 a song's, because a bank of masks played by the 303
    /// is silence rather than a line.
    pub fn load_bank(&mut self, bank: PatternBank) {
        self.bank = bank;
        self.bank.select(0);
        self.publish_pattern();
    }

    /// The engine plays what `SharedState` holds, not what the bank holds: say which pattern the bank is on. A device
    /// whose state was never handed its pattern renders silence, which the mixer's sum test found the day a second
    /// device existed.
    ///
    /// One spelling, because a bank arrives two ways — a session load and a factory bank — and the second caller is how
    /// the first one's rule comes to be forgotten.
    fn publish_pattern(&self) {
        self.state
            .set_pattern(Arc::new(*self.bank.current_pattern()));
        self.state.set_pattern_index(self.bank.current());
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
    /// Open this device's card. A request rather than panel state for the same reason the settings card's lists are: what
    /// a card shows is a reading of the machine, and `main` is the only thing allowed to take that reading.
    Open(usize),
    /// Close whichever card is open.
    Close,
    /// Add a device of this kind.
    Add(DeviceKind),
    /// The `+` was clicked: offer the kinds there are. Nothing is added until a row is clicked.
    OpenPicker,
    /// Rename this device. The panel asks with what was typed and shows what the rack answers: a collision lands as
    /// `BASS (2)`, and the panel must display the store's answer, not the hand's.
    Rename(usize, String),
    /// Whether this device plays what it hears, or only answers controllers.
    SetRole(usize, crate::midi::route::Role),
    /// Which of the opened ports may address this device.
    SetPorts(usize, crate::midi::route::Ports),
    /// Which channel on those ports.
    SetChannel(usize, crate::midi::route::Channel),
    /// Which way round this device paints its face. The card's `COLOUR` key asks for the other scheme.
    SetScheme(usize, Scheme),
    /// The REMOVE key: arm it. Nothing goes until the second click.
    AskRemove(usize),
    /// The second click on REMOVE: this device may go, unless it is the last one.
    ConfirmRemove(usize),
}

/// A list of devices, the one the panel is showing, and what the other threads read.
/// The three things a transport can be. Stopped and Paused are both silent; they differ in what a play does next.
///
/// This is the rack's, and each device's copy is a mirror of it (`SharedState::transport`). The audio thread reads the mirror
/// and decides what a change means for the engine: a play out of `Paused` resumes on the kept step, a play out of `Stopped`
/// starts the bar again. Below the engine there is only one silence, which is why this is where the distinction lives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Transport {
    #[default]
    Stopped = 0,
    Playing = 1,
    Paused = 2,
}

impl Transport {
    /// The catalogue: every state, in the order the deck's keys and the byte numbering follow.
    pub const ALL: [Transport; 3] = [Transport::Stopped, Transport::Playing, Transport::Paused];

    /// The byte an atomic carries, written out in full so a fourth state fails to compile *here* rather than arriving at the
    /// audio thread as `Stopped` with the panel lit. The catalogue owns the numbering.
    pub fn byte(self) -> u8 {
        match self {
            Transport::Stopped => 0,
            Transport::Playing => 1,
            Transport::Paused => 2,
        }
    }

    /// The state a byte carries, read through [`Transport::ALL`] so the decode cannot drift from the encode. An unknown byte
    /// reads as `Stopped`: a transport is not a thing to guess at.
    pub fn from_mirror(byte: u8) -> Self {
        Self::ALL
            .iter()
            .copied()
            .find(|state| state.byte() == byte)
            .unwrap_or(Transport::Stopped)
    }

    /// Whether the clock runs. False for a pause: a paused deck is not playing, it is merely still on its step.
    pub fn is_playing(self) -> bool {
        self == Transport::Playing
    }
}

pub struct Rack {
    devices: Vec<Device>,
    selected: usize,
    /// The number the next device will be given. A high-water mark: a removal never frees one, so two devices never share a
    /// default name and "the one I muted" is still the one that was muted.
    next_number: usize,
    /// The rack's transport, as one of three states. An atomic because the rack is read from the panel and from the MIDI
    /// thread; the devices' own copies are mirrors of it, never a second opinion. Every device follows it, so a play starts
    /// them all on one frame — the downbeats coincide rather than drifting apart by a block.
    transport: AtomicU8,
    /// The rack's tempo: one cell, and the same cell in every device's `SharedState`. The panel, an external clock and a
    /// tempo controller all write it, and every engine reads it at its next block, so no fan-out and no frame stands between
    /// a hand and the sound. Swing stays each device's own.
    tempo_bpm: Arc<AtomicF32>,
    bus: Arc<ArcSwap<BusState>>,
}

impl Rack {
    /// The session's rack: every machine the file carries, at the tempo it was saved at. A document that has not been through
    /// `sanitize` — a default one, or a test's — describes its single machine in the flat fields, which is the one device
    /// `SavedDevice::from_mirror` builds.
    pub fn from_saved(saved: &crate::persist::SavedState) -> Self {
        let tempo_bpm = Arc::new(AtomicF32::new(clamp_bpm(saved.bpm)));
        let devices = Self::devices_for(saved, &tempo_bpm);
        let next_number = devices
            .iter()
            .map(|device| device.number)
            .max()
            .unwrap_or(0)
            + 1;
        let rack = Self {
            devices,
            selected: 0,
            next_number,
            transport: AtomicU8::new(Transport::Stopped.byte()),
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

    /// Replace every machine with the ones `saved` carries, keeping this rack's **tempo cell**, transport and bus: the audio
    /// thread holds the bus and every engine holds the cell, so a load may not swap either of them out. The panel shows slot 0
    /// afterwards — the file records no selection, and "where I was" means nothing once the rack it pointed into is gone.
    ///
    /// The document's own `bpm` is **not** read. This is the seam a document with no clock goes through — a rack file — and a
    /// song's load adds its tempo on top, in [`Self::replace_from_saved`].
    pub fn replace_devices_from(&mut self, saved: &crate::persist::SavedState) {
        self.devices = Self::devices_for(saved, &self.tempo_bpm);
        self.selected = 0;
        self.next_number = self
            .devices
            .iter()
            .map(|device| device.number)
            .max()
            .unwrap_or(0)
            + 1;
        self.sync_transport();
        self.publish();
    }

    /// A song load: the machines, and the document's tempo. The tempo is set before the machines are built on the one cell,
    /// so the bus is published with the clock the file asked for rather than the one the session had.
    pub fn replace_from_saved(&mut self, saved: &crate::persist::SavedState) {
        self.set_tempo_bpm(saved.bpm);
        self.replace_devices_from(saved);
    }

    /// Every machine a document describes, each holding `tempo` — the rack's one cell, so a tempo written by the panel or the
    /// MIDI thread is heard by every engine without a fan-out.
    fn devices_for(saved: &crate::persist::SavedState, tempo: &Arc<AtomicF32>) -> Vec<Device> {
        let mirror;
        let described: &[crate::persist::SavedDevice] = if saved.devices.is_empty() {
            mirror = crate::persist::SavedDevice::from_mirror(saved);
            std::slice::from_ref(&mirror)
        } else {
            &saved.devices
        };
        described
            .iter()
            .map(|one| {
                let mut device = Device::adopted(
                    one.number,
                    crate::persist::saved_device_to_shared(one, Arc::clone(tempo)),
                    one.bank.clone(),
                );
                device.name = one.name.clone();
                device.scheme = Scheme::from_name(&one.scheme).unwrap_or_default();
                device.inputs = crate::midi::route::InputFilter {
                    ports: one.inputs.ports.clone(),
                    channel: one.inputs.channel,
                    role: one.inputs.role,
                };
                device
            })
            .collect()
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
            number,
            SharedState::with_tempo_for(kind, Arc::clone(&self.tempo_bpm)),
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
    /// `(2)` — the rule `persist::distinct_name` gives every named thing, songs and kits included — because two `BASS` cells would make "which one is muted"
    /// unanswerable. Truncation is the strip's business (`ui::text::fit`), not the store's. An index that is not there
    /// answers with the empty string and changes nothing.
    pub fn rename(&mut self, index: usize, typed: &str) -> String {
        if index >= self.devices.len() {
            return String::new();
        }
        let wanted = match typed.trim() {
            "" => Device::default_name(self.devices[index].kind(), self.devices[index].number),
            trimmed => trimmed.to_string(),
        };
        let name = crate::persist::distinct_name(&wanted, |candidate| {
            self.devices
                .iter()
                .enumerate()
                .any(|(other, device)| other != index && device.name == candidate)
        });
        self.devices[index].name.clone_from(&name);
        name
    }

    /// The rack's transport. No device may disagree: their copies are mirrors of this one.
    pub fn transport(&self) -> Transport {
        Transport::from_mirror(self.transport.load(Ordering::Relaxed))
    }

    /// Whether the clock is running. False for a pause.
    pub fn is_playing(&self) -> bool {
        self.transport().is_playing()
    }

    /// Move every device to the same transport state, on the same frame. This is what makes the downbeats coincide.
    ///
    /// Pausing a stopped rack is refused: the engine keeps its step through a stop, so a pause taken from there would resume
    /// mid-bar on the next play, and the plan's key table says a pause from `Stopped` does nothing. The refusal lives here
    /// because here is where the two states are both known, and it **answers**, so the one caller that has to say something
    /// about it — the panel's `PAUSE` key — reads the rule instead of keeping a copy of it. Every other change is always
    /// taken, which is why a caller that does not care can ignore the answer.
    pub fn set_transport(&self, transport: Transport) -> bool {
        if transport == Transport::Paused && self.transport() == Transport::Stopped {
            return false;
        }
        self.transport.store(transport.byte(), Ordering::Relaxed);
        self.sync_transport();
        true
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
    /// Only the transport needs it: the tempo is one cell every device already holds, and the transpose is each device's own,
    /// so neither is carried. Called when the rack's transport changes and when a device joins, never per frame.
    pub fn sync_transport(&self) {
        let transport = self.transport();
        for device in &self.devices {
            device.state.set_transport(transport);
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

    /// Which way round a device paints its face, if there is one at that index.
    ///
    /// Nothing to publish: the audio thread and the MIDI handlers cannot see a colour, so the bus has no field for it and no
    /// reader has to be told.
    pub fn set_scheme(&mut self, index: usize, scheme: Scheme) {
        if let Some(device) = self.devices.get_mut(index) {
            device.scheme = scheme;
        }
    }
}
