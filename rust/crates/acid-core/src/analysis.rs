//! Measurement helpers used by tests to pin audible properties without an ear.
//!
//! These are deliberately simple and deterministic: no FFT, no randomness, no I/O. If a DSP change is
//! real, one of these numbers moves.

/// Peak absolute sample value.
pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// Root-mean-square level.
pub fn rms(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let sum: f64 = buf.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    (sum / buf.len() as f64).sqrt() as f32
}

/// Fundamental frequency by normalised autocorrelation.
///
/// Waveform-agnostic on purpose: a zero-crossing counter reports half the true pitch for a sawtooth.
/// Searches lags for `40..=2000 Hz`, picks the strongest correlation, then refines it parabolically.
/// Returns `0.0` for silence, DC, or a buffer shorter than 1024 samples.
pub fn dominant_hz(buf: &[f32], sample_rate: f32) -> f32 {
    if buf.len() < 1024 || sample_rate <= 0.0 {
        return 0.0;
    }
    // Enough periods for a stable lag estimate, cheap enough to call from many tests.
    let n = buf.len().min(4096);
    let window = &buf[..n];
    let mean = window.iter().sum::<f32>() / n as f32;
    let ac: Vec<f32> = window.iter().map(|s| s - mean).collect();

    let min_lag = (sample_rate / 2000.0).max(1.0) as usize;
    let max_lag = ((sample_rate / 40.0) as usize).min(n / 2);
    if max_lag <= min_lag + 2 {
        return 0.0;
    }

    let corr = |lag: usize| -> f32 {
        let m = n - lag;
        let (mut sum, mut e1, mut e2) = (0.0f64, 0.0f64, 0.0f64);
        for i in 0..m {
            let a = ac[i] as f64;
            let b = ac[i + lag] as f64;
            sum += a * b;
            e1 += a * a;
            e2 += b * b;
        }
        let denom = (e1 * e2).sqrt();
        if denom > 1e-12 {
            (sum / denom) as f32
        } else {
            0.0
        }
    };

    // Correlation for every candidate lag, built once — the sub-octave guard re-reads it.
    let mut corr_at = vec![0.0f32; max_lag + 2];
    for (lag, slot) in corr_at
        .iter_mut()
        .enumerate()
        .take(max_lag + 1)
        .skip(min_lag)
    {
        *slot = corr(lag);
    }

    // A smooth waveform correlates near 1.0 at tiny lags, so the search starts only after the
    // correlation has first gone negative — one half period in, which no waveform can fake.
    let search_from = (min_lag..=max_lag)
        .find(|&lag| corr_at[lag] < 0.0)
        .map(|lag| lag + 1)
        .unwrap_or(min_lag);

    let mut best_lag = 0usize;
    let mut best = 0.0f32;
    for (lag, &c) in corr_at
        .iter()
        .enumerate()
        .take(max_lag + 1)
        .skip(search_from)
    {
        if c > best {
            best = c;
            best_lag = lag;
        }
    }
    if best_lag == 0 || best <= 0.0 {
        return 0.0;
    }

    // Autocorrelation peaks just as strongly at twice the period, so preferring the global maximum
    // reports an octave low. Take the earliest local peak that is nearly as strong instead.
    let threshold = best * 0.9;
    let mut chosen = best_lag;
    for lag in search_from.max(min_lag + 1)..max_lag {
        let c = corr_at[lag];
        if c >= threshold && c >= corr_at[lag - 1] && c >= corr_at[lag + 1] {
            chosen = lag;
            break;
        }
    }

    let (cm1, c0, cp1) = (corr_at[chosen - 1], corr_at[chosen], corr_at[chosen + 1]);
    let curvature = cm1 - 2.0 * c0 + cp1;
    let delta = if curvature.abs() > 1e-9 {
        0.5 * (cm1 - cp1) / curvature
    } else {
        0.0
    };
    let lag = chosen as f32 + delta.clamp(-1.0, 1.0);
    if lag <= 0.0 { 0.0 } else { sample_rate / lag }
}

/// Ratio of high-frequency energy to total energy — a cheap brightness proxy.
///
/// `rms(first difference) / rms(buf)`. Used by every brightness assertion in Phase 04; `0.0` for
/// silence and for buffers too short to difference.
pub fn brightness(buf: &[f32]) -> f32 {
    if buf.len() < 2 {
        return 0.0;
    }
    let mut diff_energy = 0.0f64;
    let mut energy = 0.0f64;
    for pair in buf.windows(2) {
        let d = (pair[1] - pair[0]) as f64;
        diff_energy += d * d;
    }
    for s in buf {
        energy += (*s as f64) * (*s as f64);
    }
    if energy <= 1e-20 {
        0.0
    } else {
        (diff_energy / energy).sqrt() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, sr: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (core::f32::consts::TAU * hz * i as f32 / sr).sin())
            .collect()
    }

    fn ramp(hz: f32, sr: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let phase = (hz * i as f32 / sr).fract();
                2.0 * phase - 1.0
            })
            .collect()
    }

    #[test]
    fn peak_and_rms_measure_what_they_say() {
        assert_eq!(peak(&[]), 0.0);
        assert_eq!(rms(&[]), 0.0);
        assert_eq!(peak(&[0.1, -0.5, 0.25]), 0.5);
        assert!((rms(&[1.0, -1.0, 1.0, -1.0]) - 1.0).abs() < 1e-6);
        let s = sine(440.0, 48_000.0, 48_000);
        assert!((rms(&s) - core::f32::consts::FRAC_1_SQRT_2).abs() < 0.01);
    }

    #[test]
    fn dominant_hz_finds_a_sine() {
        let sr = 48_000.0;
        for hz in [55.0, 110.0, 440.0, 880.0] {
            let got = dominant_hz(&sine(hz, sr, 16_384), sr);
            assert!((got - hz).abs() < hz * 0.01, "expected {hz}, got {got}");
        }
    }

    #[test]
    fn dominant_hz_finds_a_sawtooth_at_the_right_octave() {
        let sr = 44_100.0;
        let got = dominant_hz(&ramp(110.0, sr, 16_384), sr);
        assert!((got - 110.0).abs() < 2.0, "got {got}");
    }

    #[test]
    fn dominant_hz_is_silent_for_silence_dc_and_short_buffers() {
        assert_eq!(dominant_hz(&vec![0.0; 4096], 48_000.0), 0.0);
        assert_eq!(dominant_hz(&vec![0.5; 4096], 48_000.0), 0.0);
        assert_eq!(dominant_hz(&sine(440.0, 48_000.0, 100), 48_000.0), 0.0);
    }
}
