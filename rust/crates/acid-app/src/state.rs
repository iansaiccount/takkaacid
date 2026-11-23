//! State shared between the UI thread and the audio thread.
//!
//! Everything here is either an atomic or a short-lived mutex the audio thread never touches, so the
//! audio callback can never be blocked by the user moving a knob.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU8, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::device::DeviceKind;
use crate::notes::NoteQueue;
use acid_core::drums::{Drum, Kit, default_kit};
use acid_core::engine::DEFAULT_TUNING_KNOB;
use acid_core::oscillator::Waveform;
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};
use acid_core::voice::{
    DEFAULT_ACCENT_KNOB, DEFAULT_CUTOFF_KNOB, DEFAULT_DECAY_KNOB, DEFAULT_ENV_MOD_KNOB,
    DEFAULT_RESONANCE_KNOB,
};
use arc_swap::ArcSwap;
/// Tempo used until the sequencer owns one (Phase 05's `timing` module takes this over).
pub const DEFAULT_BPM: f32 = 130.0;

/// Default master volume knob position.
pub const DEFAULT_VOLUME: f32 = 0.8;

/// An `f32` behind an atomic, for knobs read by the audio thread once per block.
#[derive(Debug, Default)]
pub struct AtomicF32(AtomicU32);

impl AtomicF32 {
    pub fn new(value: f32) -> Self {
        Self(AtomicU32::new(value.to_bits()))
    }

    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Ordering::Relaxed))
    }

    pub fn set(&self, value: f32) {
        self.0.store(value.to_bits(), Ordering::Relaxed);
    }
}

/// Controller take-over state for one learnable control: the last value a controller sent, and whether it owns the knob.
///
/// Two atomics rather than one packed word: the flag would have to hide inside the value's bits, and that trick costs
/// more in reading than the four bytes it saves.
#[derive(Debug)]
pub struct Pickup {
    /// The last value the controller sent, as f32 bits; `u32::MAX` until it sends one, because `0.0` is a real value.
    last: AtomicU32,
    /// True once the controller has reached the knob and may turn it.
    engaged: AtomicBool,
}

impl Pickup {
    pub const fn new() -> Self {
        Self {
            last: AtomicU32::new(u32::MAX),
            engaged: AtomicBool::new(false),
        }
    }

    pub fn last(&self) -> Option<f32> {
        match self.last.load(Ordering::Relaxed) {
            u32::MAX => None,
            bits => Some(f32::from_bits(bits)),
        }
    }

    pub fn engaged(&self) -> bool {
        self.engaged.load(Ordering::Relaxed)
    }

    /// Remember what the controller just sent, and whether that took the knob.
    pub fn record(&self, sent: f32, engaged: bool) {
        self.last.store(sent.to_bits(), Ordering::Relaxed);
        self.engaged.store(engaged, Ordering::Relaxed);
    }
}

impl Default for Pickup {
    fn default() -> Self {
        Self::new()
    }
}

/// What the audio stream is doing, for the panel's status line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AudioStatus {
    Starting,
    Running { device: String, sample_rate: u32 },
    Failed { reason: String },
}
/// Every control in one `Copy` value: what the audio callback reads once per block.
#[derive(Copy, Clone, Debug)]
pub struct Parameters {
    pub waveform: Waveform,
    pub tuning: f32,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_mod: f32,
    pub decay: f32,
    pub accent: f32,
    pub volume: f32,
    pub tempo_bpm: f32,
    pub swing: f32,
    pub transport: crate::device::Transport,
    /// The direction this device's line runs in — asked for, and taken at the bar line.
    pub reversed: bool,
    /// This device's performance transpose, on top of the saved TUNING knob. Deliberately not part of the patch.
    pub transpose: i32,
}

/// The seam between the two threads.
///
/// Every knob is an atomic read once per audio block; the pattern lives behind an `ArcSwap`, so an edit on
/// the UI thread publishes a new `Arc` and the audio thread's `load` never waits. `status` is a mutex, but
/// only the UI thread touches it.
#[derive(Debug)]
pub struct SharedState {
    /// Which machine this state drives. Chosen when the device is made and never changed, so the audio thread can build the
    /// right machine from the state alone — which is all an [`crate::audio::renderer::AudioRenderer`] is given. **The state
    /// does not carry it:** the kind is the device's, written per machine as `SavedDevice::kind` and read back by
    /// `DeviceKind::from_name`, so one state never has to say what it is to be loaded as.
    ///
    /// **The one answer to "what kind is this device".** `Device` keeps no field of its own beside this one — `Device::kind`
    /// reads it — so a device's face and its sound cannot disagree about which machine it is.
    pub kind: DeviceKind,
    /// The state this device is in. A device in a rack mirrors the rack's — `Rack::sync_transport` is its only writer — and
    /// a device on its own (every test, and the audio crate's own harness) owns its transport and moves it itself.
    pub transport: AtomicU8,
    /// A direction asked for: the engine's sequencer takes it at the bar line, so this is a request the audio thread reads.
    pub reversed: AtomicBool,
    /// The direction the sequencer is **playing**, published by the renderer each block. `reversed` is what the player asked
    /// for and a running line takes it at the bar line; this is what came of it. The panel draws its playhead from this one,
    /// because a marker on the request would jump a bar ahead of the sound. Never saved.
    pub reverse_active: AtomicBool,
    /// This device's performance transpose. Its own, never fanned out from another device, and never saved.
    pub transpose: std::sync::atomic::AtomicI32,
    /// True when the player has muted this device. A gain, not a stop: the audio thread reads it once per block and the
    /// sequencer keeps running underneath, so unmuting rejoins on the beat instead of restarting the pattern.
    pub mute: AtomicBool,
    /// Step the transport is on, `0..16`, published by the audio thread for the LEDs.
    pub position: AtomicUsize,
    /// Whether a tempo was written by something other than the panel since the panel last looked. The learned tempo fader
    /// writes the cell from the MIDI thread and cannot touch panel state, and the tempo is in the save file: the panel takes
    /// this once a frame and marks the session unsaved, so a fader-moved tempo is written by `AUTOSAVE` like a key-moved one.
    pub tempo_written: AtomicBool,
    /// The tempo this state's engine follows. In a rack it is the rack's own cell — the same `Arc` in every device — so the
    /// panel, the MIDI thread and the fan-out-free engines all read and write one atomic. A state built on its own owns one.
    pub tempo_bpm: Arc<AtomicF32>,
    pub swing: AtomicF32,
    /// `0` = saw, `1` = square.
    pub waveform: AtomicUsize,
    pub tuning: AtomicF32,
    pub cutoff: AtomicF32,
    pub resonance: AtomicF32,
    pub env_mod: AtomicF32,
    pub decay: AtomicF32,
    pub accent: AtomicF32,
    pub volume: AtomicF32,
    /// Frequency of the note a MIDI keyboard is holding, in Hz.
    pub midi_hz: AtomicF32,
    /// True while a MIDI note is held.
    pub midi_gate: AtomicBool,
    /// True when the held note was struck hard enough to accent.
    pub midi_accent: AtomicBool,
    /// How hard the held note was struck, `0..=127`. The audition plays at this velocity, so what the player hears is
    /// what the step will carry.
    pub midi_velocity: AtomicU32,
    /// The computer keyboard's octave offset, `-2..=2`.
    pub octave: AtomicI32,
    /// The controller each learnable knob listens to, in `midi::cc::LEARNABLE` order; `-1` means unmapped.
    pub cc_map: [AtomicI32; crate::midi::cc::LEARNABLE.len()],
    /// True while the panel is waiting for a controller to learn.
    pub learn_active: AtomicBool,
    /// The controller the MIDI thread saw while learning; `-1` until one arrives.
    pub learn_cc: AtomicI32,
    /// Which of the 64 patterns is playing, `0..64`. `main` owns the bank, so it keeps this in step; the MIDI thread
    /// only reads it, to know which bank a pattern message is pointing into.
    pub pattern_index: AtomicI32,
    /// The pattern a learned controller has selected, `0..64`, or `-1` when the selection follows the sound. A pending
    /// swap lives here for one bar, so the pickup can measure against what the fader is holding.
    pub pattern_selected: AtomicI32,
    /// A pattern a controller asked for, as an absolute number; `-1` when there is nothing to do. The MIDI thread may
    /// not touch the bank, so it leaves this and `main` drains it.
    pub pattern_request: AtomicI32,
    /// Take-over state, one per learnable control, in `cc_map` slot order: a controller has to reach a knob before it
    /// may turn it, so a fader parked somewhere else cannot slam the panel when a song loads.
    pub pickup: [Pickup; crate::midi::cc::LEARNABLE.len()],
    /// The MIDI notes the panel has not placed yet, oldest first. A queue rather than one slot: two notes can land
    /// inside a single frame, and the later one must not overwrite the earlier one's pitch or velocity.
    pub notes: NoteQueue,
    pub pattern: ArcSwap<Pattern>,
    /// The kit this device plays. A whole kit is swapped at once, so the audio thread reads one kit or the next and never a
    /// voice table with half of a load applied to it.
    pub kit: ArcSwap<Kit>,
    /// Which voices start silent, in catalogue order: bit `Drum::index()` set means that voice is muted. `Kit::mute_mask` and
    /// `Kit::mute_mask` packs it, so the state and the kit cannot disagree about which voice a bit names; the mask is never
    /// unpacked back into a kit's flags — the kit's are the starting mask a load seeds, and an edit keeps them where they were
    /// (see `SharedState::adopt_kit` against `set_kit`).
    /// Sixteen voices fit a `u16`, so the high half of this word is unused: the one bridge back narrows with `as u16` rather
    /// than masking bits by hand.
    pub drum_mute: AtomicU32,
    /// A voice the panel asked to hear with the transport stopped, as `Drum::index()`; `-1` when nothing is waiting. A request
    /// the renderer drains, not a voice that is sounding.
    pub audition: AtomicI32,
    pub status: Mutex<AudioStatus>,
}

impl SharedState {
    /// A state with a tempo cell of its own: a device on its own, or a test.
    ///
    /// It is the **303's**, because a state built without being asked for a kind is the bass — every test in the workspace
    /// and the mixer's own harness build one this way. A machine of another kind is asked for by name:
    /// [`SharedState::for_kind`].
    pub fn new() -> Arc<Self> {
        Self::with_tempo(Arc::new(AtomicF32::new(DEFAULT_BPM)))
    }

    /// A state for a device of `kind`, on a tempo cell of its own.
    pub fn for_kind(kind: DeviceKind) -> Arc<Self> {
        Self::with_tempo_for(kind, Arc::new(AtomicF32::new(DEFAULT_BPM)))
    }

    /// A state sharing `tempo`: how a device joins a rack, where there is one tempo for every engine. The cell is the
    /// rack's, so a tempo the panel or the MIDI thread writes is there before the next block reads it, and nothing has to be
    /// fanned out into a copy that a frame could turn stale.
    ///
    /// The 303's, like [`SharedState::new`]; [`SharedState::with_tempo_for`] is the same with the kind named.
    pub fn with_tempo(tempo: Arc<AtomicF32>) -> Arc<Self> {
        Self::with_tempo_for(DeviceKind::Acid, tempo)
    }

    /// A state for a device of `kind` sharing the rack's tempo cell — how a device joins a rack.
    pub fn with_tempo_for(kind: DeviceKind, tempo: Arc<AtomicF32>) -> Arc<Self> {
        // The kit this device holds and the mask it starts muted by are the same kit, packed here once so that a test and the
        // app cannot disagree about which kit a fresh device has.
        let kit = Arc::new(default_kit());
        let drum_mute = AtomicU32::new(u32::from(kit.mute_mask()));
        Arc::new(Self {
            kind,
            transport: AtomicU8::new(crate::device::Transport::Stopped.byte()),
            reversed: AtomicBool::new(false),
            reverse_active: AtomicBool::new(false),
            transpose: std::sync::atomic::AtomicI32::new(0),
            mute: AtomicBool::new(false),
            position: AtomicUsize::new(0),
            tempo_written: AtomicBool::new(false),
            tempo_bpm: tempo,
            swing: AtomicF32::new(0.0),
            waveform: AtomicUsize::new(0),
            // The launch patch is the engine's, so the panel and the engine cannot disagree about it.
            tuning: AtomicF32::new(DEFAULT_TUNING_KNOB),
            cutoff: AtomicF32::new(DEFAULT_CUTOFF_KNOB),
            resonance: AtomicF32::new(DEFAULT_RESONANCE_KNOB),
            env_mod: AtomicF32::new(DEFAULT_ENV_MOD_KNOB),
            decay: AtomicF32::new(DEFAULT_DECAY_KNOB),
            accent: AtomicF32::new(DEFAULT_ACCENT_KNOB),
            volume: AtomicF32::new(DEFAULT_VOLUME),
            pattern: ArcSwap::from_pointee(Pattern::empty()),
            kit: ArcSwap::from(kit),
            drum_mute,
            audition: AtomicI32::new(-1),
            midi_hz: AtomicF32::new(0.0),
            midi_gate: AtomicBool::new(false),
            midi_accent: AtomicBool::new(false),
            midi_velocity: AtomicU32::new(u32::from(acid_core::pattern::DEFAULT_VELOCITY)),
            octave: AtomicI32::new(0),
            cc_map: std::array::from_fn(|slot| {
                AtomicI32::new(i32::from(crate::midi::cc::DEFAULT_CC[slot]))
            }),
            learn_active: AtomicBool::new(false),
            learn_cc: AtomicI32::new(-1),
            pattern_index: AtomicI32::new(0),
            // Nothing is waiting for the bar, so the selection is the sound.
            pattern_selected: AtomicI32::new(-1),
            pattern_request: AtomicI32::new(-1),
            pickup: std::array::from_fn(|_| Pickup::new()),
            notes: NoteQueue::new(),
            status: Mutex::new(AudioStatus::Starting),
        })
    }

    /// The state the rack is in, as this device last heard it.
    pub fn transport(&self) -> crate::device::Transport {
        crate::device::Transport::from_mirror(self.transport.load(Ordering::Relaxed))
    }

    pub fn set_transport(&self, transport: crate::device::Transport) {
        self.transport.store(transport.byte(), Ordering::Relaxed);
    }

    /// Whether the clock is running. A pause is not playing.
    pub fn is_playing(&self) -> bool {
        self.transport().is_playing()
    }

    /// The direction this device has been asked for: the sequencer applies it at the bar line, never mid-bar.
    pub fn is_reversed(&self) -> bool {
        self.reversed.load(Ordering::Relaxed)
    }

    /// Whether the line is actually playing backwards — the direction the renderer published, not the one asked for. A
    /// playhead drawn from [`SharedState::is_reversed`] would turn at the key press and leave the bar sounding the old way
    /// behind it.
    pub fn is_reverse_active(&self) -> bool {
        self.reverse_active.load(Ordering::Relaxed)
    }

    pub fn set_reversed(&self, reversed: bool) {
        self.reversed.store(reversed, Ordering::Relaxed);
    }

    /// The device's performance transpose, in semitones. Not part of the patch and not saved: it is the machine's own, so a
    /// second device on the rack keeps its own.
    pub fn transpose(&self) -> i32 {
        self.transpose.load(Ordering::Relaxed)
    }

    /// Move this device by a semitone, clamped to an octave.
    pub fn set_transpose(&self, semitones: i32) {
        self.transpose
            .store(semitones.clamp(-12, 12), Ordering::Relaxed);
    }

    pub fn is_muted(&self) -> bool {
        self.mute.load(Ordering::Relaxed)
    }

    /// Take the "a controller moved the tempo" flag, and answer whether it was set.
    ///
    /// The panel is the only reader, so this is a take rather than a read: a frame that finds nothing leaves `dirty` alone,
    /// and each write dirties exactly one frame. It lives here and not in the CC handler because of the three-thread rule —
    /// the MIDI thread writes atomics and never panel state.
    pub fn take_tempo_written(&self) -> bool {
        self.tempo_written.swap(false, Ordering::Relaxed)
    }
    pub fn set_mute(&self, muted: bool) {
        self.mute.store(muted, Ordering::Relaxed);
    }

    /// Every control at once, as the audio callback wants them: eleven relaxed loads, no allocation.
    pub fn snapshot(&self) -> Parameters {
        Parameters {
            waveform: self.waveform(),
            tuning: self.tuning.get(),
            cutoff: self.cutoff.get(),
            resonance: self.resonance.get(),
            env_mod: self.env_mod.get(),
            decay: self.decay.get(),
            accent: self.accent.get(),
            volume: self.volume.get(),
            tempo_bpm: self.tempo_bpm.get(),
            swing: self.swing.get(),
            transport: self.transport(),
            reversed: self.is_reversed(),
            transpose: self.transpose(),
        }
    }

    pub fn set_waveform(&self, waveform: Waveform) {
        // The catalogue owns the numbering, so adding a wave cannot leave a stale `match` behind.
        self.waveform.store(waveform.index(), Ordering::Relaxed);
    }

    pub fn waveform(&self) -> Waveform {
        Waveform::from_index(self.waveform.load(Ordering::Relaxed))
    }

    /// Publish a whole pattern. The audio thread sees either the old or the new one, never a half-built one.
    pub fn set_pattern(&self, pattern: Arc<Pattern>) {
        self.pattern.store(pattern);
    }
    pub fn pattern(&self) -> Arc<Pattern> {
        self.pattern.load_full()
    }

    /// The kit this device plays, read like [`SharedState::pattern`]: an owned `Arc` out of the swap, no lock and no allocation,
    /// so the audio thread can hold it for a whole block while the panel swaps another kit in.
    pub fn kit(&self) -> Arc<Kit> {
        self.kit.load_full()
    }

    /// Publish a whole kit — the drum face's clone-on-write edit, the way [`SharedState::set_step`] edits one step.
    ///
    /// One truth, not a second copy of a voice's envelope in the 303's atomics: the panel clones the kit on the UI thread
    /// (~1.5 KB, only while a hand is turning a dial), writes the one field, and swaps the whole table, so the audio thread
    /// reads either the old kit or the new one and never a half-edited voice.
    pub fn set_kit(&self, kit: Arc<Kit>) {
        self.kit.store(kit);
    }

    /// Take `kit` as the kit this device plays, **and seed the mute mask from it** — the load path, where [`SharedState::set_kit`]
    /// is the edit path.
    ///
    /// The two are different questions and must stay two methods. A loaded kit's `mute` flags are the kit's *starting* mask, so
    /// a kit that ships with its cowbell off arrives with the cowbell off. An edit must not do that: a dial turn would then
    /// undo the player's shift-click mutes on every frame of a drag, and the mask the engine reads is the panel's, not the
    /// kit's. `Kit::mute_mask` is the one pack, so the mask and the flags cannot disagree about which voice is bit five.
    pub fn adopt_kit(&self, kit: Kit) {
        self.drum_mute
            .store(u32::from(kit.mute_mask()), Ordering::Relaxed);
        self.kit.store(Arc::new(kit));
    }

    /// Ask for a voice to be heard, stopped or running. The renderer drains it at the next block.
    pub fn request_audition(&self, drum: Drum) {
        // `Drum::index` is the only spelling of a voice's slot, and this is the one place it becomes the `-1`-sentinel number.
        self.audition.store(drum.index() as i32, Ordering::Relaxed);
    }

    /// Take the voice a panel asked to hear, if any: one swap, as [`SharedState::take_pattern_request`] does it.
    pub fn take_audition(&self) -> Option<Drum> {
        let requested = self.audition.swap(-1, Ordering::Relaxed);
        usize::try_from(requested)
            .ok()
            .and_then(|slot| Drum::ALL.get(slot).copied())
    }

    /// Which pattern is playing, `0..64`; `main` keeps this in step with the bank.
    pub fn pattern_index(&self) -> usize {
        self.pattern_index.load(Ordering::Relaxed).max(0) as usize
    }

    pub fn set_pattern_index(&self, index: usize) {
        self.pattern_index.store(index as i32, Ordering::Relaxed);
    }

    /// Take the pattern a controller asked for, if any. Drained by `main`, which owns the bank.
    pub fn take_pattern_request(&self) -> Option<usize> {
        let requested = self.pattern_request.swap(-1, Ordering::Relaxed);
        (requested >= 0).then_some(requested as usize)
    }

    /// The pattern a learned controller has selected but not yet heard.
    ///
    /// The swap waits for the bar, so between messages the sounding pattern (`pattern_index`) is still the old one while
    /// the fader has already moved on. A controller's pickup has to measure against what it is holding — the selection —
    /// or the second message of a sweep would look like a hand on the knob and be dropped.
    pub fn pattern_selected(&self) -> usize {
        let selected = self.pattern_selected.load(Ordering::Relaxed);
        if selected >= 0 {
            selected as usize
        } else {
            self.pattern_index()
        }
    }

    /// Record the pattern a controller has selected. `main` sets this when it takes a request and when it adopts a
    /// pattern, so the selection and the sound never disagree for longer than a bar.
    pub fn set_pattern_selected(&self, index: usize) {
        self.pattern_selected.store(index as i32, Ordering::Relaxed);
    }

    /// Follow the sounding pattern again: nothing is waiting for the bar.
    pub fn clear_pattern_selected(&self) {
        self.pattern_selected.store(-1, Ordering::Relaxed);
    }

    /// Clone-on-write one step. Stray indices are ignored: this is reachable from the UI.
    pub fn set_step(&self, index: usize, step: Step) {
        if index >= STEPS_PER_PATTERN {
            return;
        }
        self.pattern.rcu(|current| {
            let mut next = **current;
            next.steps[index] = step;
            Arc::new(next)
        });
    }

    pub fn status(&self) -> AudioStatus {
        match self.status.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn set_status(&self, status: AudioStatus) {
        match self.status.lock() {
            Ok(mut guard) => *guard = status,
            Err(poisoned) => *poisoned.into_inner() = status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_starts_unmuted_and_remembers_being_muted() {
        let s = SharedState::new();
        assert!(!s.is_muted(), "a fresh device is audible");
        s.set_mute(true);
        assert!(s.is_muted());
        s.set_mute(false);
        assert!(!s.is_muted());
    }

    /// The mirror is a state, not a bool: a paused device is silent and is not playing, and the byte survives the round trip
    /// that `Rack::sync_transport` performs on it every time the rack moves.
    #[test]
    fn the_transport_mirror_carries_all_three_states() {
        use crate::device::Transport;

        let s = SharedState::new();
        assert_eq!(s.transport(), Transport::Stopped);
        assert!(!s.is_playing());

        for state in Transport::ALL {
            s.set_transport(state);
            assert_eq!(s.transport(), state, "{state:?} must survive the mirror");
            assert_eq!(s.is_playing(), state == Transport::Playing, "{state:?}");
        }

        // A byte nobody wrote reads as Stopped rather than as some third state: a transport is not a thing to guess at.
        s.set_transport(Transport::Paused);
        s.transport.store(9, std::sync::atomic::Ordering::Relaxed);
        assert_eq!(s.transport(), Transport::Stopped);
    }

    #[test]
    fn status_round_trips() {
        let s = SharedState::new();
        assert_eq!(s.status(), AudioStatus::Starting);
        s.set_status(AudioStatus::Running {
            device: "PipeWire".to_string(),
            sample_rate: 48_000,
        });
        assert_eq!(
            s.status(),
            AudioStatus::Running {
                device: "PipeWire".to_string(),
                sample_rate: 48_000
            }
        );
    }

    #[test]
    fn defaults_sound_like_a_patch_not_a_silence() {
        let s = SharedState::new();
        assert!((s.tempo_bpm.get() - DEFAULT_BPM).abs() < 1e-6);
        assert!(s.volume.get() > 0.5);
    }

    /// A fresh device is audible, and the mask it starts with agrees with the kit it holds. What this cannot show: the default
    /// kit mutes nothing, so a constructor that seeded a bare `0` would pass this too. The pack itself is pinned by
    /// `drums::tests::a_kit_packs_its_mutes_in_catalogue_order`; this catches a non-zero seed and a mask that disagrees with the
    /// kit in the swap.
    #[test]
    fn the_drum_mute_mask_starts_from_the_kit() {
        let s = SharedState::new();
        assert_eq!(
            s.drum_mute.load(Ordering::Relaxed),
            u32::from(s.kit().mute_mask()),
            "the mask the engine reads is the mutes of the kit the device holds"
        );
        assert_eq!(
            s.drum_mute.load(Ordering::Relaxed),
            0,
            "a fresh device mutes nothing"
        );
    }

    /// A loaded kit's mutes are the device's starting mask, and an edit's are not: the two are different questions, which is
    /// why `adopt_kit` and `set_kit` are two methods. A kit that ships with a voice muted has to arrive muted; a dial turn must
    /// not reset the mutes a player set by hand.
    #[test]
    fn a_loaded_kit_seeds_the_mute_mask_and_an_edit_does_not() {
        let s = SharedState::new();
        let mut kit = default_kit();
        kit.name = "Mine".to_string();
        kit.voices[Drum::Kick.index()].mute = true;
        kit.voices[Drum::Shaker.index()].mute = true;
        s.adopt_kit(kit);

        let expected = (1u32 << Drum::Kick.index()) | (1u32 << Drum::Shaker.index());
        assert_eq!(
            s.drum_mute.load(Ordering::Relaxed),
            expected,
            "a loaded kit's own mutes are the mask the engine reads"
        );
        assert_eq!(s.kit().name, "Mine");

        // An edit publishes a kit and leaves the mask alone: this is the dial-drag path.
        let mut edited = (*s.kit()).clone();
        edited.voices[Drum::Kick.index()].decay_ms = 111.0;
        s.set_kit(Arc::new(edited));
        assert_eq!(
            s.drum_mute.load(Ordering::Relaxed),
            expected,
            "an edit is not a load, so the player's mutes survive it"
        );
    }

    /// The audition slot is a request, not a live voice: `-1` is the codebase's "nothing pending", and one swap drains it.
    #[test]
    fn nothing_is_auditioned_until_a_voice_asks() {
        let s = SharedState::new();
        assert_eq!(s.take_audition(), None, "-1 means nothing is waiting");

        s.request_audition(Drum::Kick);
        assert_eq!(
            s.take_audition(),
            Some(Drum::Kick),
            "the first voice is not a sentinel"
        );
        assert_eq!(s.take_audition(), None, "a request is drained once");

        s.request_audition(Drum::Shaker);
        assert_eq!(
            s.take_audition(),
            Some(Drum::Shaker),
            "the last voice is not past the end of the catalogue"
        );
        assert_eq!(s.take_audition(), None);
    }
}
