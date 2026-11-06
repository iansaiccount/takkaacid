//! Tempo and swing: how long a 16th note lasts, and how the odd ones are delayed.
//!
//! Every duration is `f64` samples, so ten minutes of playback accumulates no drift. Swing is expressed
//! as a pair-preserving trade — even steps longer, odd steps shorter — so the loop length never changes.

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

/// Samples in one 16th note: `sample_rate * 60 / (bpm * 4)`.
pub fn samples_per_step(bpm: f32, sample_rate: f32) -> f64 {
    sample_rate as f64 * 60.0 / (clamp_bpm(bpm) as f64 * 4.0)
}

/// Samples in one specific step, with swing applied.
///
/// Even steps get `1 + 0.5·swing` of a step, odd steps `1 - 0.5·swing`. A pair therefore always lasts
/// exactly two steps, so the bar and the loop stay the same length at every swing setting.
pub fn step_duration_samples(step_index: usize, bpm: f32, sample_rate: f32, swing: f32) -> f64 {
    let swing = swing.clamp(0.0, 1.0) as f64;
    let factor = if step_index.is_multiple_of(2) {
        1.0 + 0.5 * swing
    } else {
        1.0 - 0.5 * swing
    };
    samples_per_step(bpm, sample_rate) * factor
}
