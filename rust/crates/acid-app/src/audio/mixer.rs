//! The mix bus: one stream, an engine per device, one ceiling.
//!
//! `Rig` opens a stream once and calls whatever `RenderFn` it was handed, so the mixer *is* that closure. It loads
//! [`BusState`] once per block and keeps a [`Slot`] per published device — an [`AudioRenderer`], which owns that device's
//! engine, plus a mute ramp. The summing happens here rather than in the operating system because that is where a DAW puts
//! its mix bus, and because the app is the only thing that can hold a ceiling over the sum.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use acid_core::limiter::Limiter;
use acid_core::param::Smooth;
use arc_swap::ArcSwap;

use super::renderer::AudioRenderer;
use crate::device::BusState;
use crate::state::SharedState;

/// How fast a mute is a fade rather than a cut: the same 10 ms the VOLUME knob uses, so the two sound alike.
const MUTE_MS: f32 = 10.0;

/// One device as the audio thread holds it.
struct Slot {
    state: Arc<SharedState>,
    /// Owns this device's engine: its sequencer, its clock, its drive and its own limiter.
    renderer: AudioRenderer,
    /// The mute gain. Ramped, so muting is a fade over `MUTE_MS` and not a click.
    mute: Smooth,
}

/// Every device, rendered and summed.
pub struct Mixer {
    sample_rate: f32,
    max_block: usize,
    bus: Arc<ArcSwap<BusState>>,
    slots: Vec<Slot>,
    /// One block of scratch, sized once at open, so a block never allocates.
    scratch: Vec<f32>,
    /// The sum's ceiling. Each device's own limiter only holds its own output, so two devices can reach twice it.
    master: Limiter,
    /// How many *engines* have ever been built. A steady rack builds each device once: the reconcile keeps the renderer of
    /// every device that is still in the list, which is what stops an unrelated add from restarting a transport.
    ///
    rebuilds: usize,
}

impl Mixer {
    pub fn new(sample_rate: f32, max_block: usize, bus: Arc<ArcSwap<BusState>>) -> Self {
        let max_block = max_block.max(1);
        Self {
            sample_rate,
            max_block,
            bus,
            slots: Vec::new(),
            scratch: vec![0.0; max_block],
            master: Limiter::new(sample_rate),
            rebuilds: 0,
        }
    }

    /// Slots being rendered. Agrees with the published list after the first block.
    pub fn slots(&self) -> usize {
        self.slots.len()
    }

    pub fn rebuilds(&self) -> usize {
        self.rebuilds
    }

    /// Render one mono block: every device, summed, under one ceiling.
    pub fn render_mono(&mut self, out: &mut [f32]) {
        let published = self.bus.load();
        self.reconcile(&published);

        // Borrowed out of `self` so the loop can call `&mut self` methods while it reads here — the same trick
        // `AudioRenderer::render_into` uses.
        let mut scratch = std::mem::take(&mut self.scratch);
        for chunk in out.chunks_mut(self.max_block) {
            chunk.fill(0.0);
            let frame = &mut scratch[..chunk.len()];
            for slot in &mut self.slots {
                // Muting publishes nothing: the device's own atomic is read here, once per block.
                let target = if slot.state.is_muted() { 0.0 } else { 1.0 };
                slot.renderer.render_mono(frame);
                for (sample, device) in chunk.iter_mut().zip(frame.iter()) {
                    *sample += device * slot.mute.process(target);
                }
            }
            for sample in chunk.iter_mut() {
                *sample = self.master.process(*sample);
            }
        }
        self.scratch = scratch;
    }

    /// Bring the slots in line with what the rack published.
    ///
    /// The fast path — same length, same devices, same order — is what a still rack costs: two comparisons per device. A
    /// mismatch rebuilds the list, which is the one allocation on the audio path; it is named here rather than hidden, the
    /// way `renderer.rs` names its retired `Arc`. A device that was already being rendered keeps its `AudioRenderer`, so an
    /// unrelated add does not restart its transport.
    fn reconcile(&mut self, published: &BusState) {
        if published.slots.len() == self.slots.len()
            && self
                .slots
                .iter()
                .zip(published.slots.iter())
                .all(|(slot, state)| Arc::ptr_eq(&slot.state, state))
        {
            return;
        }

        let reference = published.slots.first().map(|state| Journey {
            step: state.position.load(Ordering::Relaxed),
            // Paused counts: a device that joins a paused rack has to land on the step the rack is holding, or the next play
            // starts it at zero while every other device resumes mid-bar.
            playing: state.transport() != crate::device::Transport::Stopped,
        });
        let mut kept = std::mem::take(&mut self.slots);
        let mut slots = Vec::with_capacity(published.slots.len());
        for (index, state) in published.slots.iter().enumerate() {
            let existing = kept
                .iter()
                .position(|slot| Arc::ptr_eq(&slot.state, state))
                .map(|found| kept.remove(found));
            let slot = match existing {
                Some(slot) => slot,
                None => {
                    let mut renderer =
                        AudioRenderer::new(self.sample_rate, 1, Arc::clone(state), self.max_block);
                    // A device joining a running rack comes in on the bar the rack is on, not on its own step 0. The
                    // reference device is already there, so it needs no seek.
                    if index > 0
                        && let Some(reference) = reference
                        && reference.playing
                    {
                        renderer.seek(reference.step);
                    }
                    self.rebuilds += 1;
                    let initial = if state.is_muted() { 0.0 } else { 1.0 };
                    Slot {
                        state: Arc::clone(state),
                        renderer,
                        mute: Smooth::new(initial, MUTE_MS, self.sample_rate),
                    }
                }
            };
            slots.push(slot);
        }
        self.slots = slots;
    }
}

/// Where the reference device is, at the moment a new slot is built.
#[derive(Copy, Clone)]
struct Journey {
    step: usize,
    playing: bool,
}
