//! Ear check for the demo songs: renders each one to a WAV so it can be judged by ear.
//!
//! Run from `rust/`:
//!
//! ```text
//! cargo run --release -p acid-app --example earcheck-demos
//! ```
//!
//! Writes `earcheck/demos/<demo>.wav` (16-bit mono PCM at 48 kHz) and prints peak, RMS and brightness for every pattern in
//! every demo, so what you hear and what the demo tests assert come from the same render path. The files are git-ignored:
//! they are listening aids, not artefacts.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

use acid_app::demo::{self, Demo};
use acid_core::analysis;
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;
use acid_core::pattern::STEPS_PER_PATTERN;

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK: usize = 256;

fn main() -> std::io::Result<()> {
    let out_dir = out_dir();
    fs::create_dir_all(&out_dir)?;
    println!("writing demo WAVs to {}", out_dir.display());

    for demo in demo::songs() {
        let rendered = render(&demo);
        let path = out_dir.join(format!("{}.wav", file_name(demo.name)));
        write_wav(&path, &rendered.samples)?;
        println!(
            "{:<18} {:>6.2} s  peak {:.3}  rms {:.4}  brightness {:.3}",
            demo.name,
            rendered.seconds(),
            analysis::peak(&rendered.samples),
            analysis::rms(&rendered.samples),
            analysis::brightness(&rendered.samples),
        );
        for (slot, line) in rendered.per_pattern.iter().enumerate() {
            println!(
                "  A{}  rms {:.4}  peak {:.3}  {}",
                slot + 1,
                line.rms,
                line.peak,
                if line.rms > 0.002 {
                    ""
                } else {
                    "<- too quiet: this pattern is barely sounding"
                }
            );
        }
    }
    Ok(())
}

/// Where the WAVs go: `rust/earcheck/demos`, beside the voice ear check's own folder.
fn out_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|crates| crates.parent())
        .map(|rust| rust.join("earcheck").join("demos"))
        .unwrap_or_else(|| PathBuf::from("earcheck/demos"))
}

/// A file name that is safe on every platform, from a demo's display name.
fn file_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

struct Rendered {
    samples: Vec<f32>,
    per_pattern: Vec<Line>,
}

struct Line {
    peak: f32,
    rms: f32,
}

impl Rendered {
    fn seconds(&self) -> f32 {
        self.samples.len() as f32 / SAMPLE_RATE
    }
}

/// Render every pattern of a demo's bank, one bar each, into one take: the demo as the pattern knob plays it.
fn render(demo: &Demo) -> Rendered {
    let mut engine = engine_for(demo);
    let mut samples = Vec::new();
    let mut per_pattern = Vec::new();

    for slot in 0..8 {
        let pattern = *demo.state.bank.get(slot);
        engine.set_pattern(Arc::new(pattern));
        let bar = bar_samples(demo.state.bpm);
        let mut buffer = vec![0.0; bar];
        let mut offset = 0;
        while offset < bar {
            let take = (bar - offset).min(BLOCK);
            engine.render(&mut buffer[offset..offset + take]);
            offset += take;
        }
        per_pattern.push(Line {
            peak: analysis::peak(&buffer),
            rms: analysis::rms(&buffer),
        });
        samples.extend_from_slice(&buffer);
    }

    Rendered {
        samples,
        per_pattern,
    }
}

/// One bar of audio at this tempo, rounded to a whole buffer so the render loop never splits a block.
fn bar_samples(bpm: f32) -> usize {
    let sixteenths = 60.0 / bpm / 4.0;
    let samples = (sixteenths * STEPS_PER_PATTERN as f32 * SAMPLE_RATE) as usize;
    samples.next_multiple_of(BLOCK)
}

fn engine_for(demo: &Demo) -> Engine {
    let knobs = &demo.state.knobs;
    let mut engine = Engine::new(SAMPLE_RATE);
    engine.set_waveform(match knobs.waveform {
        1 => Waveform::Square,
        2 => Waveform::Sine,
        3 => Waveform::Triangle,
        _ => Waveform::Saw,
    });
    engine.set_tuning(knobs.tuning);
    engine.set_cutoff(knobs.cutoff);
    engine.set_resonance(knobs.resonance);
    engine.set_env_mod(knobs.env_mod);
    engine.set_decay(knobs.decay);
    engine.set_accent(knobs.accent);
    engine.set_volume(knobs.volume);
    engine.set_bpm(demo.state.bpm);
    engine.set_swing(demo.state.swing);
    engine.start();
    engine
}

/// 16-bit mono PCM at 48 kHz, the same shape the voice ear check writes.
fn write_wav(path: &std::path::Path, samples: &[f32]) -> std::io::Result<()> {
    let data_len = samples.len() * 2;
    let mut file = fs::File::create(path)?;
    file.write_all(b"RIFF")?;
    file.write_all(&(36 + data_len as u32).to_le_bytes())?;
    file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&(SAMPLE_RATE as u32).to_le_bytes())?;
    file.write_all(&(SAMPLE_RATE as u32 * 2).to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?;
    file.write_all(&16u16.to_le_bytes())?;
    file.write_all(b"data")?;
    file.write_all(&(data_len as u32).to_le_bytes())?;
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        file.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}
