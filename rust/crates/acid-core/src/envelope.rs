//! Linear segmented envelope.
//!
//! Four stages, each a straight ramp, with the increment recomputed whenever a stage is entered. Linear
//! is deliberate: the 303's capacitor-exponential curves are Phase 12 backlog, and a linear ramp is what
//! makes the decay and release times exactly measurable in tests.

/// Which part of the envelope is running.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Stage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

/// Default attack — short enough to be percussive, long enough not to click.
pub const DEFAULT_ATTACK_MS: f32 = 2.0;
/// Default decay.
pub const DEFAULT_DECAY_SECONDS: f32 = 0.3;
/// Default sustain level.
pub const DEFAULT_SUSTAIN: f32 = 0.35;
/// Default release.
pub const DEFAULT_RELEASE_MS: f32 = 20.0;

pub struct Envelope {
    sample_rate: f32,
    attack_ms: f32,
    decay_seconds: f32,
    sustain: f32,
    release_ms: f32,
    stage: Stage,
    level: f32,
    /// Per-sample magnitude of the current ramp.
    step: f32,
}

impl Envelope {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            attack_ms: DEFAULT_ATTACK_MS,
            decay_seconds: DEFAULT_DECAY_SECONDS,
            sustain: DEFAULT_SUSTAIN,
            release_ms: DEFAULT_RELEASE_MS,
            stage: Stage::Idle,
            level: 0.0,
            step: 0.0,
        }
    }

    pub fn set_attack_ms(&mut self, ms: f32) {
        self.attack_ms = ms.max(0.01);
        if self.stage == Stage::Attack {
            self.step = self.attack_increment();
        }
    }

    pub fn set_decay_seconds(&mut self, seconds: f32) {
        self.decay_seconds = seconds.max(0.001);
        if self.stage == Stage::Decay {
            self.step = self.decay_increment();
        }
    }

    pub fn set_sustain(&mut self, level: f32) {
        self.sustain = level.clamp(0.0, 1.0);
    }

    pub fn set_release_ms(&mut self, ms: f32) {
        self.release_ms = ms.max(0.01);
        if self.stage == Stage::Release {
            self.step = self.release_increment();
        }
    }

    /// Start (or retrigger) the envelope from wherever it currently is, so no click is emitted.
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
        self.step = self.attack_increment();
    }

    /// Begin the release from the current level.
    pub fn gate_off(&mut self) {
        if self.stage == Stage::Idle {
            return;
        }
        self.stage = Stage::Release;
        self.step = self.release_increment();
    }

    pub fn stage(&self) -> Stage {
        self.stage
    }

    pub fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    pub fn level(&self) -> f32 {
        self.level
    }

    /// Advance one sample and return the new level.
    // Named `next` because the plan and every DSP call site use that word; an `Envelope` is not an
    // iterator — it has no `Iterator` semantics, no `Item`, and no end.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => self.level = 0.0,
            Stage::Attack => {
                self.level += self.step;
                if self.level >= 1.0 {
                    self.level = 1.0;
                    self.stage = Stage::Decay;
                    self.step = self.decay_increment();
                }
            }
            Stage::Decay => {
                self.level -= self.step;
                if self.level <= self.sustain {
                    self.level = self.sustain;
                    self.stage = Stage::Sustain;
                }
            }
            Stage::Sustain => self.level = self.sustain,
            Stage::Release => {
                self.level -= self.step;
                if self.level <= 0.0 {
                    self.level = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.level
    }

    fn samples(&self, seconds: f32) -> f32 {
        (seconds * self.sample_rate).max(1.0)
    }

    fn attack_increment(&self) -> f32 {
        1.0 / self.samples(self.attack_ms * 0.001)
    }

    fn decay_increment(&self) -> f32 {
        let span = (1.0 - self.sustain).max(1e-6);
        span / self.samples(self.decay_seconds)
    }

    fn release_increment(&self) -> f32 {
        self.level / self.samples(self.release_ms * 0.001)
    }
}
