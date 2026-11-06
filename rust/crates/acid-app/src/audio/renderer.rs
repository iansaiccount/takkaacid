//! The audio callback's brain: one block, every parameter, no allocation.
//!
//! `apply` is the only place a parameter reaches the engine, so the mapping is unit-testable without a
//! sound card. `AudioRenderer` reads the shared state once per block, swaps the sequencer's pattern only
//! when the UI actually published a new one, and copies the mono result to every channel.

use std::sync::Arc;

use acid_core::engine::Engine;
use acid_core::pattern::Pattern;

use crate::state::{Parameters, SharedState};

/// Push a parameter snapshot into the engine's knobs and transport.
pub fn apply(params: &Parameters, engine: &mut Engine) {
    engine.set_waveform(params.waveform);
    engine.set_tuning(params.tuning);
    engine.set_cutoff(params.cutoff);
    engine.set_resonance(params.resonance);
    engine.set_env_mod(params.env_mod);
    engine.set_decay(params.decay);
    engine.set_accent(params.accent);
    engine.set_volume(params.volume);
    engine.set_bpm(params.tempo_bpm);
    engine.set_swing(params.swing);
    if params.playing != engine.is_playing() {
        if params.playing {
            engine.start();
        } else {
            engine.stop();
        }
    }
}

/// Owns the engine for the audio thread's lifetime and keeps it in step with the UI.
pub struct AudioRenderer {
    engine: Engine,
    state: Arc<SharedState>,
    /// Interleaving scratch, sized once at open so a block never allocates.
    mono: Vec<f32>,
    /// The pattern last handed to the engine, so an unchanged one costs a refcount compare.
    pattern: Arc<Pattern>,
    channels: usize,
    max_block: usize,
}

impl AudioRenderer {
    pub fn new(
        sample_rate: f32,
        channels: usize,
        state: Arc<SharedState>,
        max_block: usize,
    ) -> Self {
        let pattern = state.pattern();
        let mut engine = Engine::new(sample_rate);
        engine.set_pattern(Arc::clone(&pattern));
        Self {
            engine,
            state,
            mono: vec![0.0; max_block.max(1)],
            pattern,
            channels: channels.max(1),
            max_block: max_block.max(1),
        }
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Render one mono block: parameters first, then the pattern if it changed, then the engine.
    pub fn render_mono(&mut self, out: &mut [f32]) {
        let params = self.state.snapshot();
        apply(&params, &mut self.engine);

        let latest = self.state.pattern();
        if !Arc::ptr_eq(&latest, &self.pattern) {
            self.pattern = latest;
            self.engine.set_pattern(Arc::clone(&self.pattern));
        }

        let mut offset = 0;
        while offset < out.len() {
            let take = (out.len() - offset).min(self.max_block);
            self.engine.render(&mut out[offset..offset + take]);
            offset += take;
        }
        // Publish the step so the panel's LEDs can chase the audio rather than the wall clock.
        self.state
            .position
            .store(self.engine.position(), std::sync::atomic::Ordering::Relaxed);
    }

    /// Render one interleaved block, duplicating each mono frame to every channel.
    pub fn render_into(&mut self, out: &mut [f32]) {
        let frames = out.len() / self.channels;
        if frames == 0 {
            return;
        }
        // `mono` is borrowed out of `self` so the loop can call `&mut self` methods while it writes here.
        let mut mono = std::mem::take(&mut self.mono);
        let mut done = 0;
        while done < frames {
            let take = (frames - done).min(self.max_block);
            let buf = &mut mono[..take];
            self.render_mono(buf);
            for (frame, sample) in buf.iter().enumerate() {
                let base = (done + frame) * self.channels;
                out[base..base + self.channels].fill(*sample);
            }
            done += take;
        }
        self.mono = mono;
    }
}
