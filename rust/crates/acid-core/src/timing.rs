//! Tempo and swing: how long one step of a grid lasts, and how the odd ones are delayed.
//!
//! A pattern is one 4/4 bar, so the **bar** is the tempo's alone and the grid only says how finely it is divided: sixteen
//! steps of sixteenths, thirty-two of thirty-seconds and twelve eighth-note triplets are the same number of samples, which
//! is what keeps every device's bar line landing with every other's.
//!
//! Every duration is `f64` samples, so ten minutes of playback accumulates no drift. Swing is expressed as a
//! pair-preserving trade — even steps longer, odd steps shorter — so the loop length never changes at any swing setting.

/// Slowest tempo the panel offers.
pub const MIN_BPM: f32 = 40.0;
/// Fastest tempo the panel offers.
pub const MAX_BPM: f32 = 300.0;
/// Tempo at launch.
pub const DEFAULT_BPM: f32 = 130.0;

/// Clamp a tempo to the panel's range.
pub fn clamp_bpm(bpm: f32) -> f32 {
    bpm.clamp(MIN_BPM, MAX_BPM)
}

/// Samples in one 4/4 bar: `sample_rate * 60 / bpm * 4`.
///
/// The bar is the number every grid agrees on; a step is this divided by the grid's count.
pub fn samples_per_bar(bpm: f32, sample_rate: f32) -> f64 {
    sample_rate as f64 * 60.0 / clamp_bpm(bpm) as f64 * 4.0
}

/// Samples in one step of a bar divided into `steps_per_bar`.
///
/// A zero is floored to one rather than dividing by it: this is on the audio path, where a panic becomes silence for
/// everything else in the rack.
pub fn samples_per_step(bpm: f32, sample_rate: f32, steps_per_bar: usize) -> f64 {
    samples_per_bar(bpm, sample_rate) / steps_per_bar.max(1) as f64
}

/// Samples in one specific step, with swing applied.
///
/// Even steps get `1 + 0.5·swing` of a step, odd steps `1 - 0.5·swing`. A pair therefore always lasts exactly two steps, so
/// the bar and the loop stay the same length at every swing setting and at every grid.
pub fn step_duration_samples(
    step_index: usize,
    bpm: f32,
    sample_rate: f32,
    swing: f32,
    steps_per_bar: usize,
) -> f64 {
    let swing = swing.clamp(0.0, 1.0) as f64;
    let factor = if step_index.is_multiple_of(2) {
        1.0 + 0.5 * swing
    } else {
        1.0 - 0.5 * swing
    };
    samples_per_step(bpm, sample_rate, steps_per_bar) * factor
}
