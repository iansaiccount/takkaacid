//! The drum machine's voices, as one catalogue.
//!
//! Sixteen is not a round number chosen for the grid: it is exactly the width of a `u16`, and a step's `hits` mask has one bit
//! per voice. A seventeenth voice is a third mask, which is why the catalogue lives apart from the mask — that change is a type
//! here, not a search through the drawing, the sequencer and the save file.
//!
//! The voices are fixed for every kit. A kit that has no cowbell mutes the cowbell; it does not renumber the ones after it.

use std::sync::Arc;

use crate::envelope::Envelope;
use crate::filter::MultimodeFilter;
use crate::param;
use crate::pattern::{Pattern, Step};
use crate::sequencer::{Performer, Sequencer};
use crate::voice::{ACCENT_DECAY_MS, ACCENT_GAIN, ACCENT_SWEEP_OCTAVES};

/// One voice of the drum machine, in grid order — the order the face draws and the mask's bits count in.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Drum {
    Kick,
    Snare,
    /// The 808's rimshot and the 909's rim click share the slot: one sound per voice, chosen by the kit.
    Rim,
    Clap,
    /// A short hat. Choked by [`Drum::OpenHat`] and choking it — see [`Drum::choke_group`].
    ClosedHat,
    /// A long hat, cut off when a closed hat lands.
    OpenHat,
    LowTom,
    MidTom,
    /// `HTOM` rather than `HIT`: the knob row's audition key is already called `HIT`, and one face cannot carry two.
    HighTom,
    Crash,
    Ride,
    Cowbell,
    /// The 808's four extras. A 909 kit mutes them; the catalogue keeps the slots so a kit can grow into them.
    LowConga,
    HighConga,
    Clave,
    Shaker,
}

impl Drum {
    /// Every voice, in grid order.
    pub const ALL: [Drum; 16] = [
        Drum::Kick,
        Drum::Snare,
        Drum::Rim,
        Drum::Clap,
        Drum::ClosedHat,
        Drum::OpenHat,
        Drum::LowTom,
        Drum::MidTom,
        Drum::HighTom,
        Drum::Crash,
        Drum::Ride,
        Drum::Cowbell,
        Drum::LowConga,
        Drum::HighConga,
        Drum::Clave,
        Drum::Shaker,
    ];

    /// Which bit of a step's `hits`/`accents` mask this voice is, and which row of the grid.
    pub fn index(self) -> usize {
        match self {
            Drum::Kick => 0,
            Drum::Snare => 1,
            Drum::Rim => 2,
            Drum::Clap => 3,
            Drum::ClosedHat => 4,
            Drum::OpenHat => 5,
            Drum::LowTom => 6,
            Drum::MidTom => 7,
            Drum::HighTom => 8,
            Drum::Crash => 9,
            Drum::Ride => 10,
            Drum::Cowbell => 11,
            Drum::LowConga => 12,
            Drum::HighConga => 13,
            Drum::Clave => 14,
            Drum::Shaker => 15,
        }
    }

    /// The bit this voice owns in a step's `hits`/`accents` mask — and in the mute mask, which counts the same sixteen
    /// voices in the same order. One place, so a reorder moves a voice's bit with it rather than leaving a second list
    /// behind: this was `pattern::drum_bit`, and it moved here when the drum machine became its fourth caller.
    pub fn bit(self) -> u16 {
        1u16 << self.index() as u16
    }

    /// The caption on the grid's row. Four characters at most, ASCII: the gutter is narrow and the face may not print glyphs.
    pub fn label(self) -> &'static str {
        match self {
            Drum::Kick => "KICK",
            Drum::Snare => "SNR",
            Drum::Rim => "RIM",
            Drum::Clap => "CLAP",
            Drum::ClosedHat => "CHH",
            Drum::OpenHat => "OHH",
            Drum::LowTom => "LOT",
            Drum::MidTom => "MID",
            Drum::HighTom => "HTOM",
            Drum::Crash => "CRS",
            Drum::Ride => "RDE",
            Drum::Cowbell => "CWB",
            Drum::LowConga => "LCO",
            Drum::HighConga => "HCO",
            Drum::Clave => "CLV",
            Drum::Shaker => "SHK",
        }
    }

    /// Which voices a hit on this one silences. `0` is "chokes nothing".
    ///
    /// This is the whole of a drum machine's realism in one number: a hat pair that rings together sounds like a synth pad,
    /// and one that cuts sounds like a drummer.
    pub fn choke_group(self) -> u8 {
        match self {
            Drum::ClosedHat | Drum::OpenHat => 1,
            Drum::Kick
            | Drum::Snare
            | Drum::Rim
            | Drum::Clap
            | Drum::LowTom
            | Drum::MidTom
            | Drum::HighTom
            | Drum::Crash
            | Drum::Ride
            | Drum::Cowbell
            | Drum::LowConga
            | Drum::HighConga
            | Drum::Clave
            | Drum::Shaker => 0,
        }
    }
}

/// Which side of the filter a voice passes. The knob row's three-way key prints these.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FilterKind {
    Low,
    Band,
    High,
}

impl FilterKind {
    pub const ALL: [FilterKind; 3] = [FilterKind::Low, FilterKind::Band, FilterKind::High];

    pub fn index(self) -> usize {
        match self {
            FilterKind::Low => 0,
            FilterKind::Band => 1,
            FilterKind::High => 2,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FilterKind::Low => "LP",
            FilterKind::Band => "BP",
            FilterKind::High => "HP",
        }
    }
}

/// The name of the kit a device holds before any kit is loaded. The card prints it, so it is a word and not an index.
pub const DEFAULT_KIT_NAME: &str = "Init";

/// The default voice's shape: a short percussive hit rather than a held note, which is what a drum is, and a filter as close to
/// the sample as recorded as the filter path allows — wide open, flat, low-pass.
///
/// Every one of these is a decision task 6's listening pass overturns per kit, which is why they are named here rather than spelled
/// in the constructor: changing a default is changing this list, not finding five literals.
const DEFAULT_TUNE: f32 = 0.0;
const DEFAULT_ATTACK_MS: f32 = 2.0;
const DEFAULT_DECAY_MS: f32 = 300.0;
const DEFAULT_SUSTAIN: f32 = 0.0;
const DEFAULT_RELEASE_MS: f32 = 20.0;
const DEFAULT_CUTOFF: f32 = 1.0;
const DEFAULT_RESONANCE: f32 = 0.0;

/// One voice of a kit: the sample it plays, what shapes it, and whether it starts silent.
///
/// The field order is the kit document's contract — task 6 writes `kit.json` from these names in this order, so a rename or a
/// reorder costs a migration. Provenance is deliberately not here: `file`, `source` and `credit` describe where a sample came
/// from, which the document records beside the file, not in the voice that plays it.
#[derive(Clone, Debug)]
pub struct KitVoice {
    /// Mono, at the engine's rate, already resampled. Empty means the voice has no sample: a silent voice, not a missing one.
    pub sample: Arc<[f32]>,
    /// Detune in semitones, ±12. This is the `TUNE` knob's range, not its position.
    pub tune: f32,
    pub attack_ms: f32,
    pub decay_ms: f32,
    pub sustain: f32,
    pub release_ms: f32,
    /// Which side of the voice's filter the sample passes.
    pub filter: FilterKind,
    /// Knob positions in `0.0..=1.0`, mapped like the 303's by `param::cutoff_hz` and `param::resonance`.
    pub cutoff: f32,
    pub resonance: f32,
    /// The kit's starting mute. A kit with no cowbell mutes the cowbell; it does not renumber the voices after it.
    pub mute: bool,
}

impl Default for KitVoice {
    /// A voice with no sample, untouched: the default kit is sixteen of these. Written out rather than derived because every
    /// number is one of the decisions above.
    fn default() -> Self {
        // `Arc<[f32]>`'s `Default` is the empty slice, so this allocates nothing and cannot fail.
        Self {
            sample: Arc::default(),
            tune: DEFAULT_TUNE,
            attack_ms: DEFAULT_ATTACK_MS,
            decay_ms: DEFAULT_DECAY_MS,
            sustain: DEFAULT_SUSTAIN,
            release_ms: DEFAULT_RELEASE_MS,
            filter: FilterKind::Low,
            cutoff: DEFAULT_CUTOFF,
            resonance: DEFAULT_RESONANCE,
            mute: false,
        }
    }
}

/// A kit: the name the card prints, and one voice per drum in catalogue order.
///
/// A kit is the whole sound of the drum machine, so it is swapped whole behind an `ArcSwap` — the audio thread reads one kit or
/// the next, never a voice table with half a load applied to it.
#[derive(Clone, Debug)]
pub struct Kit {
    pub name: String,
    pub voices: [KitVoice; Drum::ALL.len()],
}

impl Kit {
    /// The kit's mutes packed in catalogue order: bit `Drum::index()` set means that voice starts silent.
    ///
    /// The pack and the unpack live here together and nowhere else, so the state's `drum_mute` — what the audio thread reads once
    /// per block — and the kit's own flags cannot disagree about which voice is bit five. `Drum::index` is the catalogue, so
    /// moving a voice in the grid moves its bit with it.
    pub fn mute_mask(&self) -> u16 {
        let mut mask = 0u16;
        for drum in Drum::ALL {
            if self.voices[drum.index()].mute {
                mask |= drum.bit();
            }
        }
        mask
    }
}

/// The kit a device starts with: sixteen voices, none of them with a sample.
///
/// A device that has loaded no kit is silent, and that is the honest state — the kit is the sound, and there is nothing to play
/// before one arrives. Task 6 puts a factory kit in its place on the first run.
pub fn default_kit() -> Kit {
    Kit {
        name: DEFAULT_KIT_NAME.to_string(),
        voices: std::array::from_fn(|_| KitVoice::default()),
    }
}

/// One voice's playing state: where in its sample it has got to, and what shapes it.
///
/// The kit is what a voice *plays* and this is the performance, and the two are deliberately separate: a kit swapped in
/// while a hit is sounding replaces the sample and leaves the position, the envelope and the filter where they were. A dial
/// turn on the panel publishes a new kit on the frames a hand is on it, and retriggering every sounding voice for each of
/// them would make the drum face unusable.
struct DrumVoice {
    /// Fractional position in the kit voice's sample. Fractional because TUNE moves the read rate.
    position: f32,
    /// Whether there is sample left to play. A voice with nothing to play renders silence.
    running: bool,
    env: Envelope,
    filter: MultimodeFilter,
    /// The sample's read rate, `2^(semitones/12)`, from the kit's TUNE and the device's transpose.
    increment: f32,
    /// The voice's cutoff with no accent on it, in hertz.
    cutoff_hz: f32,
    /// Whether the hit that is sounding carried the accent.
    accent: bool,
    /// The accent's filter kick, `1.0` at the hit and decaying each sample: the 303's accent pulse into the cut-off, on a
    /// drum — which is what makes an accented hat read as an accent rather than as a louder hat.
    accent_pulse: f32,
    accent_pulse_coeff: f32,
}

impl DrumVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            position: 0.0,
            running: false,
            env: Envelope::new(sample_rate),
            filter: MultimodeFilter::new(sample_rate),
            increment: 1.0,
            cutoff_hz: param::cutoff_hz(DEFAULT_CUTOFF),
            accent: false,
            accent_pulse: 0.0,
            accent_pulse_coeff: (-1.0 / (ACCENT_DECAY_MS * 0.001 * sample_rate)).exp(),
        }
    }

    /// Take the kit's shape for this voice.
    ///
    /// Called at every hit and at the top of every block, so a dial turned while a hit is sounding is heard on the next block
    /// without restarting it. That is safe because the envelope's setters are no-ops when their value has not moved — a release
    /// re-stated every block would otherwise restart from wherever it had got to — and because the filter's setters hold no
    /// ramp state of their own: re-stating one rebuilds a coefficient and nothing else.
    fn shape(&mut self, voice: &KitVoice, transpose: i32) {
        // A kit's samples are decoded at the engine's own rate (task 6), so the read rate is one sample per sample plus
        // whatever TUNE and the device's transpose ask for.
        self.increment = 2f32.powf((voice.tune + transpose as f32) / 12.0);
        self.cutoff_hz = param::cutoff_hz(voice.cutoff);
        self.filter.set_kind(voice.filter);
        self.filter.set_resonance(param::resonance(voice.resonance));
        // The cutoff is set once here rather than once a sample; while an accent's pulse is running, the render loop owns it
        // and this waits for the pulse to end — otherwise a block boundary would snap the filter back to the plain cutoff for
        // a sample.
        if self.accent_pulse == 0.0 {
            self.filter.set_cutoff_hz(self.cutoff_hz);
        }
        self.env.set_attack_ms(voice.attack_ms);
        self.env.set_decay_seconds(voice.decay_ms * 0.001);
        self.env.set_sustain(voice.sustain);
        self.env.set_release_ms(voice.release_ms);
    }

    /// Strike this voice: back to the sample's first value, the envelope gated, and the accent pulse charged if the hit
    /// carries one.
    fn hit(&mut self, voice: &KitVoice, transpose: i32, accented: bool) {
        // The accent first, so `shape` knows whether the filter may take the plain cutoff: an accented hit lets the per-sample
        // pulse own the cutoff, and a plain one must not inherit the pulse the previous hit left decaying.
        self.accent = accented;
        self.accent_pulse = if accented { 1.0 } else { 0.0 };
        self.shape(voice, transpose);
        self.position = 0.0;
        // A voice with no sample is silent, not a slice out of bounds: the default kit is sixteen of those, and a new drum
        // device holds exactly it.
        self.running = !voice.sample.is_empty();
        self.env.gate_on();
    }

    /// Fold the voice away through its release. A choke and a stop both ask for this rather than for a cut, which is what
    /// keeps them from clicking.
    fn release(&mut self) {
        self.env.gate_off();
    }

    fn next(&mut self, sample: &[f32]) -> f32 {
        if !self.running {
            return 0.0;
        }
        let position = self.position;
        let index = position as usize;
        if index >= sample.len() {
            self.running = false;
            // A sample can run out while an accent's pulse is still open. The pulse is zeroed with the voice so `shape` never
            // finds a dead hit's pulse and keeps skipping CUT OFF; nothing observes it today, because the filter is not
            // processed while the voice is stopped and `hit` resets the pulse anyway.
            self.accent_pulse = 0.0;
            return 0.0;
        }
        // Linear interpolation between the two samples either side of a fractional position, which is all a TUNE of a few
        // semitones needs; the last sample is its own neighbour so the end of the buffer is never read past.
        let next = sample[(index + 1).min(sample.len() - 1)];
        let fraction = position - index as f32;
        let raw = sample[index] + (next - sample[index]) * fraction;

        // The accent's kick: the filter opens and falls away over `ACCENT_DECAY_MS`, and only while the pulse is worth anything
        // is the cutoff rebuilt per sample — `shape` owns it below that.
        if self.accent_pulse > 1e-4 {
            let octaves = ACCENT_SWEEP_OCTAVES * self.accent_pulse;
            self.filter
                .set_cutoff_hz(self.cutoff_hz * 2f32.powf(octaves));
            self.accent_pulse *= self.accent_pulse_coeff;
        } else {
            // The pulse is spent: it is drawn down to exactly zero, which is what lets `shape` take the plain cutoff back at the
            // next block. (`> 1e-4` already keeps this loop out of the branch; the zero is for `shape`, which asks `== 0.0`.)
            self.accent_pulse = 0.0;
        }

        let level = self.env.next();
        // There is no ACCENT knob on a drum face, so an accented hit takes the whole of the 303's lift.
        let gain = if self.accent { ACCENT_GAIN } else { 1.0 };
        let value = self.filter.process(raw) * level * gain;
        // A float WAV holds whatever its writer put there, and task 5 passed its samples through as recorded. A non-finite
        // sample, or a finite one so large that a resonant filter's own state overflows, would leave that state out of range
        // for the rest of the session; the filter is reset and the sample that did it plays as silence, so the voice is there
        // for the next hit. A merely loud sample is bounded by the rails below instead.
        let value = if value.is_finite() {
            value.clamp(-1.0, 1.0)
        } else {
            self.filter.reset();
            0.0
        };
        self.position += self.increment;
        // Flush denormals, as the 303's voice does: a released hit leaves the filter ringing at 1e-30 magnitudes.
        if value.abs() < 1e-20 { 0.0 } else { value }
    }
}

/// The sixteen voices, the kit they play and the mask over them — everything a step touches, and nothing that knows what a
/// clock is.
///
/// This is the [`Performer`] the sequencer renders through. It is a separate type from [`Drums`] for a borrow's sake rather
/// than for taste: `Sequencer::render` holds the clock for the length of the block while it borrows the performer, so the
/// performer has to be a field of the machine rather than the machine itself.
struct DrumMachine {
    kit: Arc<Kit>,
    voices: [DrumVoice; Drum::ALL.len()],
    /// One bit per voice, packed by [`Kit::mute_mask`].
    mute_mask: u16,
    /// The device's transpose in semitones, on top of each voice's own TUNE. Not saved, as the 303's is not.
    transpose: i32,
    /// A hit with no step under it — the face's `HIT` — served at the next render. One slot: a second request before the
    /// next block replaces the first, because a hand cannot press twice inside ten milliseconds and a queue would have to
    /// allocate.
    audition: Option<Drum>,
}

impl DrumMachine {
    fn new(sample_rate: f32, kit: Arc<Kit>) -> Self {
        // The **load** path seeds the mask from the kit, as `SharedState::adopt_kit` does on the panel's side: a kit that
        // mutes its cowbell starts silent on the cowbell. `set_kit` is the edit path and deliberately does not.
        let mute_mask = kit.mute_mask();
        Self {
            kit,
            voices: std::array::from_fn(|_| DrumVoice::new(sample_rate)),
            mute_mask,
            transpose: 0,
            audition: None,
        }
    }

    /// The one place a drum is struck.
    fn hit(&mut self, drum: Drum, accented: bool) {
        let group = drum.choke_group();
        if group != 0 {
            // A closed hat cuts an open one. Folding the other voice through its release rather than stopping it is what
            // keeps the cut from clicking — and it is inside the group only, so nothing else is touched.
            for other in Drum::ALL {
                if other != drum && other.choke_group() == group {
                    self.voices[other.index()].release();
                }
            }
        }
        let index = drum.index();
        self.voices[index].hit(&self.kit.voices[index], self.transpose, accented);
    }
}

impl Performer for DrumMachine {
    /// A step strikes every voice in its `hits` mask, accented where the `accents` mask says so. The 303's note is not read:
    /// a step is both worlds' type and this machine only ever asks about the drums.
    fn step(&mut self, step: &Step) {
        for drum in Drum::ALL {
            if step.is_hit(drum) {
                self.hit(drum, step.is_accented(drum));
            }
        }
    }

    /// A stop closes every gate. The samples keep running and the envelopes fold them away, so a two-second sample is cut
    /// where the stop says rather than at the end of its buffer.
    fn release(&mut self) {
        for voice in &mut self.voices {
            voice.release();
        }
    }

    fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        // One load per block, not one per sample: the `Arc` is what keeps a kit swapped out mid-block alive for the block
        // that is still playing it, and reading it this way holds no guard at all.
        let kit = &self.kit;
        let mask = self.mute_mask;
        let transpose = self.transpose;
        for drum in Drum::ALL {
            let index = drum.index();
            let voice = &mut self.voices[index];
            let sound = &kit.voices[index];
            voice.shape(sound, transpose);
            // A muted voice still plays: the mask silences its output, not the sequencer, so its choke of a neighbour happens
            // and unmuting it mid-hit shows the tail it would have had.
            let muted = mask & drum.bit() != 0;
            for slot in out.iter_mut() {
                let sample = voice.next(&sound.sample);
                if !muted {
                    *slot += sample;
                }
            }
        }
    }

    /// Nothing to clear. This machine's per-run state is each voice's own and every hit replaces it, unlike the 303's slide
    /// history — which is exactly why the trait asks the machine instead of deciding for it.
    fn arm(&mut self) {}
}

/// The drum machine: sixteen sample voices on the 303's clock.
///
/// The clock is the 303's, unchanged and shared: the bar, `REVERSE`, swing, pause, the bar line and the sample-accurate
/// start all live in [`Sequencer`], so a drum bar and a 303 bar started on the same frame are the same bar for ever. What is
/// this machine's is the sound — sixteen voices, a kit, and the mask over them.
pub struct Drums {
    sequencer: Sequencer,
    machine: DrumMachine,
}

impl Drums {
    pub fn new(sample_rate: f32, kit: Arc<Kit>) -> Self {
        Self {
            sequencer: Sequencer::new(sample_rate),
            machine: DrumMachine::new(sample_rate, kit),
        }
    }

    /// Swap the kit: what every voice plays. Positions, envelopes and filter states survive, so a dial turn is heard at once
    /// and retriggers nothing.
    ///
    /// The mute mask is **not** reseeded here. `Drums::new` loads a kit and seeds the mask from it, which is the same rule
    /// `SharedState::adopt_kit` follows; an edit that reseeded would undo a mute the player had just set.
    pub fn set_kit(&mut self, kit: Arc<Kit>) {
        self.machine.kit = kit;
    }

    /// Which voices are silent, one bit per [`Drum`] — the same pack [`Kit::mute_mask`] makes.
    pub fn set_mute_mask(&mut self, mask: u16) {
        self.machine.mute_mask = mask;
    }

    /// Strike `drum` with no step under it: the face's `HIT`, audible with the transport stopped and cut by the same `STOP`
    /// everything else is. Served at the next [`Drums::render`].
    pub fn audition(&mut self, drum: Drum) {
        self.machine.audition = Some(drum);
    }

    /// Fill `out` with the next block.
    pub fn render(&mut self, out: &mut [f32]) {
        if let Some(drum) = self.machine.audition.take() {
            self.machine.hit(drum, false);
        }
        // The clock and the voices are two fields of one machine; `Sequencer::render` needs both at once.
        let Self { sequencer, machine } = self;
        sequencer.render(machine, out);
    }

    pub fn set_pattern(&mut self, pattern: Arc<Pattern>) {
        self.sequencer.set_pattern(pattern);
    }

    /// TEMPO, in BPM; the clock clamps it to the panel's range.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.sequencer.set_bpm(bpm);
    }

    /// SWING, `0.0` straight to `1.0` full shuffle — the same clock control the 303 has, so a shuffle set on the drums is
    /// the shuffle the rest of the rack hears.
    pub fn set_swing(&mut self, swing: f32) {
        self.sequencer.set_swing(swing);
    }

    /// The step being played, `0..16`.
    pub fn position(&self) -> usize {
        self.sequencer.position()
    }

    /// The direction this machine is actually playing: `set_reverse` takes a request at once when the line is stopped and
    /// at the bar line when it is running, so this is the one a playhead may draw from.
    pub fn is_reverse(&self) -> bool {
        self.sequencer.is_reverse()
    }

    pub fn bar_line(&self) -> Option<usize> {
        self.sequencer.bar_line()
    }

    pub fn start(&mut self) {
        self.sequencer.start();
    }

    pub fn start_at(&mut self, step: usize) {
        self.sequencer.start_at(step);
    }

    pub fn start_later(&mut self, step: usize, silence: usize) {
        self.sequencer.start_later(step, silence);
    }

    /// Stop the transport: every voice releases, so a long sample is cut where the stop says.
    ///
    /// A direction asked for mid-bar is taken with the stop, as the 303's engine takes it — the next start returns to the top
    /// of the bar, so that bar is the first chance the direction will get.
    pub fn stop(&mut self) {
        self.hush();
        self.sequencer.stop();
        self.sequencer.settle_direction();
    }

    /// Stop the clock and keep the step, so [`Drums::resume`] carries on from it.
    pub fn pause(&mut self) {
        self.hush();
        self.sequencer.stop();
    }

    /// Carry on from the step a pause kept. Unlike [`Drums::start`] this does not return to the top of the bar.
    ///
    /// **Not for coming out of a stop.** The clock keeps its step through a stop as well, so the machine cannot tell the two
    /// states apart: a caller that resumes from `Stopped` gets the stale step rather than the bar's first. Play from
    /// `Stopped` is [`Drums::start`]; the rack's transport is what knows which of the two the player pressed.
    pub fn resume(&mut self) {
        self.sequencer.resume();
    }

    pub fn set_reverse(&mut self, on: bool) {
        self.sequencer.set_reverse(on);
    }

    /// The rack's performance transpose, in semitones: every voice reads its sample faster or slower, and nothing is saved.
    pub fn set_transpose(&mut self, semitones: i32) {
        self.machine.transpose = semitones.clamp(-12, 12);
    }

    /// Silence the machine now: every voice folds away through its release, and a hit asked for but not yet served is
    /// forgotten.
    ///
    /// A **stop** and a **pause** are the same silence on this side of the seam, and a hit can sound with no transport under
    /// it at all — the audition — so the machine is silenced here as well as by the clock, which closes only a gate it had
    /// opened. `Envelope::gate_off` starts the same ramp from the same level, so the clock's own close at the next render
    /// costs nothing. **Call it on the thread that renders the machine**: it writes all sixteen voices' envelopes.
    fn hush(&mut self) {
        self.machine.audition = None;
        self.machine.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sixteen_voices_are_labelled_numbered_and_chase_in_one_group() {
        assert_eq!(Drum::ALL.len(), 16, "one voice per bit of the step mask");

        for (index, drum) in Drum::ALL.iter().enumerate() {
            assert_eq!(drum.index(), index, "{drum:?} has the wrong slot");
            assert_eq!(
                Drum::ALL[drum.index()],
                *drum,
                "{drum:?} does not round trip"
            );
            assert!(drum.label().is_ascii(), "{drum:?} is not ASCII");
            assert!(
                !drum.label().is_empty() && drum.label().len() <= 4,
                "{drum:?} has a label the face has no room for"
            );
        }

        // The captions themselves, in grid order: the gutter prints these, so a rename, a duplicate or the old `HIT` for
        // `HighTom` is a face change and must fail here rather than in a screenshot.
        assert_eq!(
            Drum::ALL.map(Drum::label),
            [
                "KICK", "SNR", "RIM", "CLAP", "CHH", "OHH", "LOT", "MID", "HTOM", "CRS", "RDE",
                "CWB", "LCO", "HCO", "CLV", "SHK"
            ],
            "the grid's gutter prints these, in this order"
        );

        // One choke group, and it is the hats: a closed hat cuts an open one, which is what makes a hat pattern a pattern.
        let chasing: Vec<Drum> = Drum::ALL
            .iter()
            .copied()
            .filter(|drum| drum.choke_group() != 0)
            .collect();
        assert_eq!(
            chasing,
            vec![Drum::ClosedHat, Drum::OpenHat],
            "the hats share a choke group and nothing else does"
        );
        assert_eq!(
            Drum::ClosedHat.choke_group(),
            Drum::OpenHat.choke_group(),
            "the two hats must name the same group"
        );
    }

    #[test]
    fn the_filter_has_three_faces_with_ascii_names() {
        assert_eq!(FilterKind::ALL.len(), 3);
        for (index, kind) in FilterKind::ALL.iter().enumerate() {
            assert_eq!(kind.index(), index, "{kind:?} has the wrong slot");
            assert!(kind.label().is_ascii(), "{kind:?} is not ASCII");
        }
        assert_eq!(
            FilterKind::ALL.map(FilterKind::label),
            ["LP", "BP", "HP"],
            "the face prints these three, in this order"
        );
    }

    #[test]
    fn a_default_kit_has_sixteen_voices_and_no_samples() {
        let kit = default_kit();
        assert_eq!(
            kit.voices.len(),
            Drum::ALL.len(),
            "one voice per drum, or the mask has a bit with nothing behind it"
        );
        assert_eq!(
            kit.name, "Init",
            "the card prints this for a device with no kit loaded"
        );

        let default = KitVoice::default();
        for drum in Drum::ALL {
            let voice = &kit.voices[drum.index()];
            assert!(
                voice.sample.is_empty(),
                "{drum:?} ships a sample, and the default kit has no source to ship"
            );
            assert!(
                !voice.mute,
                "{drum:?} starts silent, but a fresh device is audible"
            );
            assert_eq!(voice.tune, default.tune, "{drum:?} starts detuned");
            assert_eq!(
                voice.filter, default.filter,
                "{drum:?} starts on the wrong side"
            );
            assert_eq!(voice.attack_ms, default.attack_ms, "{drum:?}");
            assert_eq!(voice.decay_ms, default.decay_ms, "{drum:?}");
            assert_eq!(voice.sustain, default.sustain, "{drum:?}");
            assert_eq!(voice.release_ms, default.release_ms, "{drum:?}");
            assert_eq!(voice.cutoff, default.cutoff, "{drum:?}");
            assert_eq!(voice.resonance, default.resonance, "{drum:?}");
        }

        // The numbers themselves, pinned. A default that drifts — a decay so short the hit disappears, a filter closed onto the
        // sample — is a change to the machine's sound, and it should fail here rather than in a listening test nobody runs.
        assert_eq!(default.tune, 0.0, "TUNE at centre: no detune");
        assert_eq!(
            default.attack_ms, 2.0,
            "long enough not to click, short enough to be a hit"
        );
        assert_eq!(default.decay_ms, 300.0, "a hit, not a held note");
        assert_eq!(default.sustain, 0.0, "a drum decays to nothing");
        assert_eq!(default.release_ms, 20.0);
        assert_eq!(default.cutoff, 1.0, "wide open: the sample as recorded");
        assert_eq!(
            default.resonance, 0.0,
            "flat: no peak the kit did not ask for"
        );
    }

    #[test]
    fn a_kit_packs_its_mutes_in_catalogue_order() {
        let mut kit = default_kit();
        assert_eq!(kit.mute_mask(), 0, "a fresh kit mutes nothing");

        // Both ends of the catalogue and a middle voice: bit 0, bit 14 and bit 15. A reversed order, an off-by-one and a mask
        // packed from a hand-kept list all show up here.
        for drum in [Drum::Kick, Drum::Clave, Drum::Shaker] {
            kit.voices[drum.index()].mute = true;
        }
        assert_eq!(
            kit.mute_mask(),
            0b1100_0000_0000_0001,
            "bit `Drum::index()` is a voice's bit, in catalogue order"
        );

        // The mask a *machine* holds is the live one (`Drums::set_mute_mask` writes it, and `SharedState::adopt_kit` seeds
        // it from this pack); the kit's own flags are the starting mask a load carries and nothing unpacks into them, so the
        // inverse lives on the machine and not here.
        assert_eq!(
            default_kit().mute_mask(),
            0,
            "a fresh kit is silent, not muted"
        );
    }
}
