//! Opening the sound card.
//!
//! Two phases: [`open_default`] picks the device and configuration, then [`Opened::spawn`] builds the
//! stream once the caller knows the sample rate and can create its renderer at that rate. The callback
//! owns its scratch buffer (sized once at open), never allocates, never locks, and duplicates each mono
//! sample to every channel.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Instant;

use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, SupportedStreamConfig};

pub mod supervisor;
pub mod renderer;

pub use supervisor::{Action, Supervisor};

/// Preferred sample rates, best first. 48 kHz matches most interfaces; 44.1 kHz matches most files.
pub const PREFERRED_RATES: [u32; 3] = [48_000, 44_100, 96_000];

/// Largest block the callback is prepared for; bigger device buffers are rendered in chunks.
pub const MAX_BLOCK: usize = 8_192;

/// A mono render closure the callback can own.
pub type RenderFn = Box<dyn FnMut(&mut [f32]) + Send>;

/// Pick the best rate available: 48k, then 44.1k, then 96k, then whatever exists.
pub fn pick_rate(rates: &[u32]) -> Option<u32> {
    for preferred in PREFERRED_RATES {
        if rates.contains(&preferred) {
            return Some(preferred);
        }
    }
    rates.first().copied()
}

/// Pick a usable stream config: the preferred rate where possible, stereo before mono.
pub fn pick_config(configs: &[SupportedStreamConfig]) -> Option<SupportedStreamConfig> {
    let usable: Vec<&SupportedStreamConfig> =
        configs.iter().filter(|c| c.channels() >= 1).collect();
    if usable.is_empty() {
        return None;
    }
    let rates: Vec<u32> = usable.iter().map(|c| c.sample_rate()).collect();
    if let Some(rate) = pick_rate(&rates) {
        if let Some(stereo) = usable
            .iter()
            .find(|c| c.sample_rate() == rate && c.channels() >= 2)
        {
            return Some(**stereo);
        }
        if let Some(any) = usable.iter().find(|c| c.sample_rate() == rate) {
            return Some(**any);
        }
    }
    Some(*usable[0])
}

/// One-line description used in the startup log and the panel's status strip.
pub fn describe(cfg: &SupportedStreamConfig) -> String {
    format!(
        "{} Hz, {} channel(s), {:?}",
        cfg.sample_rate(),
        cfg.channels(),
        cfg.sample_format()
    )
}

/// A device and configuration, resolved but not yet playing.
pub struct Opened {
    device: cpal::Device,
    name: String,
    config: SupportedStreamConfig,
}

impl Opened {
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate()
    }

    pub fn sample_rate_f32(&self) -> f32 {
        self.config.sample_rate() as f32
    }

    pub fn channels(&self) -> u16 {
        self.config.channels()
    }

    pub fn device_name(&self) -> &str {
        &self.name
    }

    /// Build and start the stream, pulling mono frames from `render_mono`.
    ///
    /// Stream errors are reported on `errors` — the app's supervisor decides what to do next.
    pub fn spawn<F>(self, render_mono: F, errors: Sender<()>) -> Result<AudioHandle>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let Opened {
            device,
            name,
            config: chosen,
        } = self;
        let sample_rate = chosen.sample_rate();
        let channels = chosen.channels();
        let format = chosen.sample_format();
        eprintln!("takka-acid: output '{}' — {}", name, describe(&chosen));

        let stream_config = StreamConfig {
            channels,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let mut mono = vec![0.0f32; MAX_BLOCK];
        let mut render = render_mono;
        let ch = channels as usize;
        let on_error = move |err: cpal::Error| {
            eprintln!("takka-acid: audio stream error: {err}");
            let _ = errors.send(());
        };

        let stream = match format {
            SampleFormat::F32 => device.build_output_stream(
                stream_config,
                move |data: &mut [f32], _info: &cpal::OutputCallbackInfo| {
                    let frames = data.len() / ch;
                    let mut done = 0;
                    while done < frames {
                        let n = (frames - done).min(mono.len());
                        render(&mut mono[..n]);
                        let range = done * ch..(done + n) * ch;
                        for (frame, out) in data[range].chunks_mut(ch).enumerate() {
                            out.fill(mono[frame]);
                        }
                        done += n;
                    }
                },
                on_error,
                None,
            )?,
            other => return Err(anyhow!("unsupported sample format {other:?}")),
        };
        stream.play().context("start audio stream")?;

        Ok(AudioHandle {
            _stream: stream,
            sample_rate,
            channels,
            device: name,
        })
    }
}

/// Resolve the default output device and a usable configuration.
pub fn open_default() -> Result<Opened> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no default output device"))?;
    let name = device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "unknown device".to_string());

    let default_cfg = device.default_output_config().ok();
    let chosen = match default_cfg {
        Some(cfg) if PREFERRED_RATES.contains(&cfg.sample_rate()) => cfg,
        other => {
            let cfgs: Vec<SupportedStreamConfig> = device
                .supported_output_configs()
                .map(|ranges| ranges.map(|r| r.with_max_sample_rate()).collect())
                .unwrap_or_default();
            pick_config(&cfgs)
                .or(other)
                .ok_or_else(|| anyhow!("no usable output configuration"))?
        }
    };

    Ok(Opened {
        device,
        name,
        config: chosen,
    })
}

/// Open the default device and play whatever `render_mono` produces.
pub fn start<F>(render_mono: F, errors: Sender<()>) -> Result<AudioHandle>
where
    F: FnMut(&mut [f32]) + Send + 'static,
{
    open_default()?.spawn(render_mono, errors)
}

/// A live output stream. Dropping it stops audio.
pub struct AudioHandle {
    _stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
    device: String,
}

// Read by the status strip in Phase 11 Task 55; until then the handle is only kept alive.
#[allow(dead_code)]
impl AudioHandle {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn sample_rate_f32(&self) -> f32 {
        self.sample_rate as f32
    }

    pub fn channels(&self) -> u16 {
        self.channels
    }

    pub fn device(&self) -> &str {
        &self.device
    }
}

/// Owns the stream, rebuilds the renderer at the device's rate, reopens after a failure, and exposes
/// a one-line status for the UI.
pub struct Rig {
    /// Called with the device's sample rate after every successful open, to build a fresh renderer.
    open: Box<dyn Fn(f32) -> RenderFn>,
    handle: Option<AudioHandle>,
    errors: Option<Receiver<()>>,
    supervisor: Supervisor,
    retry_at: Option<Instant>,
    status: String,
}

impl Rig {
    pub fn new(open: impl Fn(f32) -> RenderFn + 'static) -> Self {
        let mut rig = Self {
            open: Box::new(open),
            handle: None,
            errors: None,
            supervisor: Supervisor::new(),
            retry_at: None,
            status: "starting".to_string(),
        };
        rig.attempt();
        rig
    }

    /// Drain stream errors and reopen when the retry delay has elapsed. Never blocks the caller.
    pub fn poll(&mut self) {
        let mut failed = false;
        if let Some(rx) = &self.errors {
            while rx.try_recv().is_ok() {
                failed = true;
            }
        }
        if failed {
            self.handle = None;
            self.errors = None;
            match self.supervisor.on_error() {
                Action::GiveUp => {
                    self.status = "failed: audio device lost".to_string();
                    self.retry_at = None;
                }
                Action::Retry => {
                    self.status = format!(
                        "reconnecting (attempt {}/{})",
                        self.supervisor.attempts(),
                        supervisor::MAX_RETRIES
                    );
                    self.retry_at = Some(Instant::now() + supervisor::RETRY_DELAY);
                }
            }
        }
        if let Some(at) = self.retry_at
            && Instant::now() >= at
        {
            self.retry_at = None;
            self.attempt();
        }
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    fn attempt(&mut self) {
        let opened = match open_default() {
            Ok(o) => o,
            Err(err) => {
                self.status = format!("failed: {err}");
                self.handle = None;
                self.errors = None;
                return;
            }
        };
        let (tx, rx) = channel();
        let render = (self.open)(opened.sample_rate_f32());
        match opened.spawn(render, tx) {
            Ok(handle) => {
                self.status = format!("{} Hz - {}", handle.sample_rate(), handle.device());
                self.supervisor.on_success();
                self.handle = Some(handle);
                self.errors = Some(rx);
            }
            Err(err) => {
                self.status = format!("failed: {err}");
                self.handle = None;
                self.errors = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_48k_then_44k1_then_the_rest() {
        let rates = [96_000u32, 44_100, 48_000];
        assert_eq!(pick_rate(&rates), Some(48_000));
        assert_eq!(pick_rate(&[96_000, 44_100]), Some(44_100));
        assert_eq!(pick_rate(&[192_000, 96_000]), Some(96_000));
        assert_eq!(pick_rate(&[]), None);
    }
}
