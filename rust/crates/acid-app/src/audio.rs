//! Opening the sound card.
//!
//! One output stream, pulled from a mono render closure. The callback owns its scratch buffer (sized
//! once at open), never allocates, never locks, and duplicates each mono sample to every channel.


use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig, SupportedStreamConfig};

/// Preferred sample rates, best first. 48 kHz matches most interfaces; 44.1 kHz matches most files.
pub const PREFERRED_RATES: [u32; 3] = [48_000, 44_100, 96_000];

/// Largest block the callback is prepared for; bigger device buffers are rendered in chunks.
const MAX_BLOCK: usize = 8_192;

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
    let usable: Vec<&SupportedStreamConfig> = configs.iter().filter(|c| c.channels() >= 1).collect();
    if usable.is_empty() {
        return None;
    }
    let rates: Vec<u32> = usable.iter().map(|c| c.sample_rate()).collect();
    let rate = pick_rate(&rates);
    if let Some(rate) = rate {
        if let Some(stereo) = usable
            .iter()
            .find(|c| c.sample_rate() == rate && c.channels() >= 2)
        {
            return Some(*(*stereo));
        }
        if let Some(any) = usable.iter().find(|c| c.sample_rate() == rate) {
            return Some(*(*any));
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

/// Open the default output device and pull mono frames from `render_mono` forever.
pub fn start<F>(render_mono: F) -> Result<AudioHandle>
where
    F: FnMut(&mut [f32]) + Send + 'static,
{
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

    let sample_rate = chosen.sample_rate();
    let channels = chosen.channels();
    let format = chosen.sample_format();
    eprintln!("takka-acid: output '{}' — {}", name, describe(&chosen));

    let config = StreamConfig {
        channels,
        sample_rate,
        buffer_size: cpal::BufferSize::Default,
    };

    let mut mono = vec![0.0f32; MAX_BLOCK];
    let mut render = render_mono;
    let ch = channels as usize;
    let error_callback = |err| eprintln!("takka-acid: audio stream error: {err}");

    let stream = match format {
        SampleFormat::F32 => device.build_output_stream(
            config,
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
            error_callback,
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

/// A silent stream, used when the app must keep running without a device.
pub fn start_silent() -> Result<AudioHandle> {
    start(|buf| buf.fill(0.0))
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
