//! Offline ear check: renders a set of WAV files so the voice can be judged by ear.
//!
//! Run from `rust/`:
//!
//! ```text
//! cargo run --release -p acid-core --example earcheck
//! ```
//!
//! Writes `earcheck/*.wav` (16-bit mono PCM at 48 kHz) next to the crate's manifest directory and prints
//! peak/RMS for each take, so what you hear and what the tests assert come from the same render path.
//! The files are git-ignored on purpose — they are listening aids, not artefacts.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;
use acid_core::pitch::{step_hz, Pitch};

const SAMPLE_RATE: f32 = 48_000.0;
const BLOCK: usize = 512;

/// A named take: a name and a function that fills the engine's settings and plays something.
struct Take {
    name: &'static str,
    hint: &'static str,
    build: fn() -> Engine,
}

fn main() -> std::io::Result<()> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.join("earcheck"))
        .unwrap_or_else(|| PathBuf::from("earcheck"));
    fs::create_dir_all(&out_dir)?;
    println!("writing WAVs to {}", out_dir.display());

    for take in takes() {
        let mut engine = (take.build)();
        let samples = render_take(&mut engine, take.name);
        let path = out_dir.join(format!("{}.wav", take.name));
        write_wav(&path, &samples)?;
        println!(
            "  {:<28} {:>7.2} s  peak {:>5.2}  rms {:>5.3}   {}",
            take.name,
            samples.len() as f32 / SAMPLE_RATE,
            acid_core::analysis::peak(&samples),
            acid_core::analysis::rms(&samples),
            take.hint
        );
    }
    println!("\nListen in any player. Delete the directory whenever you like.");
    Ok(())
}

fn takes() -> Vec<Take> {
    vec![
        Take {
            name: "01-default",
            hint: "launch patch: saw, 55 Hz, cutoff 0.35, res 0.55, env mod 0.5",
            build: || {
                let mut e = Engine::new(SAMPLE_RATE);
                e.set_waveform(Waveform::Saw);
                e
            },
        },
        Take {
            name: "02-cutoff-0.10",
            hint: "very dark — pluck, almost no bite",
            build: || with_knobs(0.10, 0.55, 0.5, 0.35, 0.8),
        },
        Take {
            name: "03-cutoff-0.35",
            hint: "default — squelchy bass, darkens as it falls",
            build: || with_knobs(0.35, 0.55, 0.5, 0.35, 0.8),
        },
        Take {
            name: "04-cutoff-0.60",
            hint: "bright — the saw's edge is audible",
            build: || with_knobs(0.60, 0.55, 0.5, 0.35, 0.8),
        },
        Take {
            name: "05-cutoff-0.90",
            hint: "wide open — buzzy, should still not alias badly at 55 Hz",
            build: || with_knobs(0.90, 0.55, 0.5, 0.35, 0.8),
        },
        Take {
            name: "06-resonance-0.20",
            hint: "soft, no peak",
            build: || with_knobs(0.35, 0.20, 0.5, 0.35, 0.8),
        },
        Take {
            name: "07-resonance-0.60",
            hint: "the 303 'ow' — resonance starts to sing",
            build: || with_knobs(0.35, 0.60, 0.5, 0.35, 0.8),
        },
        Take {
            name: "08-resonance-0.95",
            hint: "edge of self-oscillation: loud, whistling, must not crackle",
            build: || with_knobs(0.35, 0.95, 0.5, 0.35, 0.8),
        },
        Take {
            name: "09-envmod-0.0",
            hint: "flat filter — no sweep, useful as a reference",
            build: || with_knobs(0.35, 0.55, 0.0, 0.35, 0.8),
        },
        Take {
            name: "10-envmod-1.0",
            hint: "full sweep — the classic 'bwaap' on every note",
            build: || with_knobs(0.35, 0.55, 1.0, 0.35, 0.8),
        },
        Take {
            name: "11-decay-short",
            hint: "plucky, filter closes fast",
            build: || with_knobs(0.35, 0.55, 0.5, 0.0, 0.8),
        },
        Take {
            name: "12-decay-long",
            hint: "long tail, resonance rings on",
            build: || with_knobs(0.35, 0.55, 0.5, 1.0, 0.8),
        },
        Take {
            name: "13-accent-alternating",
            hint: "plain and accented notes alternating: accents louder and brighter",
            build: || {
                let mut e = with_knobs(0.35, 0.55, 0.5, 0.35, 0.8);
                e.set_accent(1.0);
                e
            },
        },
        Take {
            name: "14-slide",
            hint: "55 Hz gliding to 110 Hz without retriggering",
            build: || with_knobs(0.35, 0.55, 0.5, 0.35, 0.8),
        },
        Take {
            name: "15-square",
            hint: "square wave: hollow, more harmonics than the saw",
            build: || {
                let mut e = with_knobs(0.35, 0.55, 0.5, 0.35, 0.8);
                e.set_waveform(Waveform::Square);
                e
            },
        },
        Take {
            name: "16-everything-max",
            hint: "worst case: max resonance, env mod, decay, accent, cutoff",
            build: || {
                let mut e = with_knobs(0.9, 1.0, 1.0, 1.0, 1.0);
                e.set_waveform(Waveform::Square);
                e
            },
        },
    ]
}

fn with_knobs(cutoff: f32, resonance: f32, env_mod: f32, decay: f32, accent: f32) -> Engine {
    let mut e = Engine::new(SAMPLE_RATE);
    e.set_waveform(Waveform::Saw);
    e.set_tuning(0.5);
    e.set_cutoff(cutoff);
    e.set_resonance(resonance);
    e.set_env_mod(env_mod);
    e.set_decay(decay);
    e.set_accent(accent);
    e
}

fn render_take(engine: &mut Engine, name: &str) -> Vec<f32> {
    let mut out = Vec::with_capacity((SAMPLE_RATE * 6.0) as usize);
    let note = step_hz(Pitch::C, 0);

    match name {
        "13-accent-alternating" => {
            for i in 0..6 {
                play_note(engine, note, i % 2 == 1, 0.45, &mut out);
                if i < 5 {
                    rest(engine, 0.10, &mut out);
                }
            }
        }
        "14-slide" | "16-everything-max" => {
            // Two gestures: a plain note, then one that slides up an octave on the same breath.
            play_note(engine, note, false, 0.8, &mut out);
            rest(engine, 0.35, &mut out);
            engine.note_on(note, false);
            render(engine, 0.5, &mut out);
            engine.glide_to(note * 2.0);
            render(engine, 1.0, &mut out);
            engine.note_off();
            render(engine, 0.3, &mut out);
        }
        _ => {
            play_note(engine, note, false, 1.6, &mut out);
            render(engine, 0.4, &mut out);
        }
    }
    out
}

fn play_note(engine: &mut Engine, hz: f32, accent: bool, seconds: f32, out: &mut Vec<f32>) {
    engine.note_on(hz, accent);
    render(engine, seconds, out);
    engine.note_off();
    render(engine, 0.25, out);
}

fn rest(engine: &mut Engine, seconds: f32, out: &mut Vec<f32>) {
    render(engine, seconds, out);
}

fn render(engine: &mut Engine, seconds: f32, out: &mut Vec<f32>) {
    let mut remaining = (seconds * SAMPLE_RATE) as usize;
    let mut block = vec![0.0f32; BLOCK];
    while remaining > 0 {
        let n = remaining.min(BLOCK);
        engine.render(&mut block[..n]);
        out.extend_from_slice(&block[..n]);
        remaining -= n;
    }
}

/// Minimal 16-bit mono PCM WAV writer — no dependencies, so the example stays buildable offline.
fn write_wav(path: &std::path::Path, samples: &[f32]) -> std::io::Result<()> {
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = SAMPLE_RATE as u32 * 2;
    let mut file = fs::File::create(path)?;
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36 + data_len).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    header.extend_from_slice(&1u16.to_le_bytes()); // PCM
    header.extend_from_slice(&1u16.to_le_bytes()); // mono
    header.extend_from_slice(&(SAMPLE_RATE as u32).to_le_bytes());
    header.extend_from_slice(&byte_rate.to_le_bytes());
    header.extend_from_slice(&2u16.to_le_bytes()); // block align
    header.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_len.to_le_bytes());
    file.write_all(&header)?;
    let mut body = Vec::with_capacity(samples.len() * 2);
    for s in samples {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        body.extend_from_slice(&v.to_le_bytes());
    }
    file.write_all(&body)
}
