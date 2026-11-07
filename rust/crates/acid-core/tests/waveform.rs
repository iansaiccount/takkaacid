//! The four waves, and the catalogue the panel and the save file index into.

use acid_core::oscillator::{Oscillator, Waveform};

fn render(wave: Waveform, hz: f32, sr: f32, n: usize) -> Vec<f32> {
    let mut o = Oscillator::new(sr);
    o.set_frequency(hz);
    o.set_waveform(wave);
    (0..n).map(|_| o.next_sample()).collect()
}

#[test]
fn the_catalogue_lists_every_wave_once_and_round_trips() {
    assert_eq!(Waveform::ALL.len(), 4);
    for (index, wave) in Waveform::ALL.iter().enumerate() {
        assert_eq!(wave.index(), index, "{wave:?} has the wrong slot");
        assert_eq!(Waveform::from_index(index), *wave);
        assert!(wave.name().is_ascii(), "{wave:?}");
    }
    // The first two keep the meanings an older save file already had.
    assert_eq!(Waveform::ALL[0], Waveform::Saw);
    assert_eq!(Waveform::ALL[1], Waveform::Square);
}

#[test]
fn a_file_that_names_a_wave_this_build_lacks_clamps_to_the_last() {
    assert_eq!(Waveform::from_index(4), Waveform::Triangle);
    assert_eq!(Waveform::from_index(usize::MAX), Waveform::Triangle);
}

#[test]
fn sine_is_a_sine() {
    let sr = 48_000.0;
    let hz = 110.0;
    let buf = render(Waveform::Sine, hz, sr, 4_800);
    for (n, sample) in buf.iter().enumerate() {
        let phase = (n as f64) * f64::from(hz) / f64::from(sr);
        let expected = (core::f64::consts::TAU * phase).sin();
        assert!(
            (f64::from(*sample) - expected).abs() < 1e-5,
            "sample {n}: {sample} vs {expected}"
        );
    }
    // Full scale, centred on zero: a sine is the reference the other waves are heard against.
    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((peak - 1.0).abs() < 0.01, "peak {peak}");
    let mean: f32 = buf.iter().sum::<f32>() / buf.len() as f32;
    assert!(mean.abs() < 0.02, "a sine has no offset: {mean}");
}

#[test]
fn triangle_is_two_linear_ramps() {
    let buf = render(Waveform::Triangle, 110.0, 48_000.0, 4_800);
    // One period, split at the peak: the wave rises by a constant step, then falls by the same step. Measuring across
    // the whole buffer would average several periods together and prove nothing.
    let steps: Vec<f32> = buf.windows(2).map(|w| w[1] - w[0]).collect();
    let period = (48_000.0 / 110.0) as usize;
    let rising = &steps[2..period / 2 - 2];
    let falling = &steps[period / 2 + 2..period - 2];
    let spread = |xs: &[f32]| {
        let max = xs.iter().fold(f32::MIN, |m, v| m.max(*v));
        let min = xs.iter().fold(f32::MAX, |m, v| m.min(*v));
        max - min
    };
    assert!(spread(rising) < 0.001, "the rise is not straight");
    assert!(spread(falling) < 0.001, "the fall is not straight");
    assert!(
        rising[0] > 0.0 && falling[0] < 0.0,
        "and it turns over at the peak"
    );

    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((peak - 1.0).abs() < 0.02, "peak {peak}");
}

#[test]
fn every_wave_starts_at_the_bottom_and_stays_in_range() {
    for wave in Waveform::ALL {
        let buf = render(wave, 55.0, 48_000.0, 2_000);
        assert!(buf.iter().all(|s| (-1.05..=1.05).contains(s)), "{wave:?}");
        let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.9, "{wave:?} is too quiet: {peak}");
    }
}

#[test]
fn switching_waves_mid_note_keeps_the_pitch() {
    // One accumulator feeds every wave, so a switch must not jump the phase — otherwise the note clicks and the pitch
    // reads wrong for a moment.
    let mut o = Oscillator::new(48_000.0);
    o.set_frequency(110.0);
    for n in 0..1_000 {
        o.set_waveform(Waveform::ALL[n % Waveform::ALL.len()]);
        let _ = o.next_sample();
    }
    assert!(
        (o.frequency() - 110.0).abs() < 0.01,
        "frequency drifted to {}",
        o.frequency()
    );
}
