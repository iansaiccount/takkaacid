//! One mono synth voice.
//!
//! Phase 02 ships a placeholder sine behind the final public API. Phase 03 replaces the oscillator and
//! Phase 04 the filter/VCA — the methods here do not change shape when they do.

/// Gate and accent as plain floats, so the sequencer and the UI can carry them without enums.
#[derive(Copy, Clone, Debug, Default)]
#[allow(dead_code)] // fields are read once the sequencer drives the voice in Phase 05
pub struct VoiceParams {
    pub gate: f32,
    pub accent: f32,
}

const ATTACK_SECONDS: f32 = 0.005;
const RELEASE_SECONDS: f32 = 0.02;
const AMPLITUDE: f32 = 0.3;
const ACCENT_BOOST: f32 = 0.4;
const MIN_HZ: f32 = 16.0;
const MAX_HZ: f32 = 4_000.0;

pub struct Voice {
    sample_rate: f32,
    phase: f64,
    hz: f32,
    gate: bool,
    accent: bool,
    /// 0.0..=1.0 gain envelope edge, so notes start and stop without a click.
    level: f32,
    active: bool,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            hz: 55.0,
            gate: false,
            accent: false,
            level: 0.0,
            active: false,
        }
    }

    pub fn note_on(&mut self, hz: f32, accent: bool) {
        self.hz = hz.clamp(MIN_HZ, MAX_HZ);
        self.accent = accent;
        self.gate = true;
        self.active = true;
    }

    pub fn set_pitch(&mut self, hz: f32) {
        self.hz = hz.clamp(MIN_HZ, MAX_HZ);
    }

    pub fn note_off(&mut self) {
        self.gate = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn render(&mut self, out: &mut [f32]) {
        let attack_step = 1.0 / (ATTACK_SECONDS * self.sample_rate);
        let release_step = 1.0 / (RELEASE_SECONDS * self.sample_rate);
        let amp = AMPLITUDE * if self.accent { 1.0 + ACCENT_BOOST } else { 1.0 };
        let inc = self.hz as f64 / self.sample_rate as f64;

        for slot in out.iter_mut() {
            if self.gate {
                self.level = (self.level + attack_step).min(1.0);
            } else {
                self.level = (self.level - release_step).max(0.0);
                if self.level == 0.0 {
                    self.active = false;
                }
            }
            let sine = (self.phase * std::f64::consts::TAU).sin() as f32;
            *slot = (sine * amp * self.level).clamp(-0.999, 0.999);
            self.phase = (self.phase + inc).fract();
        }
    }
}
