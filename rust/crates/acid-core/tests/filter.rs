use acid_core::filter::LadderFilter;

fn run(f: &mut LadderFilter, input: impl Fn(usize) -> f32, n: usize) -> Vec<f32> {
    (0..n).map(|i| f.process(input(i))).collect()
}

fn sine_hz_rms(fc: f32, freq: f32, sr: f32, res: f32) -> f32 {
    let mut f = LadderFilter::new(sr);
    f.set_cutoff_hz(fc);
    f.set_resonance(res);
    let n = 48_000;
    // Unit amplitude: a 0.5-amplitude sine can never exceed 0.354 RMS, which the "passes" assertion
    // in the plan asked for.
    let out = run(
        &mut f,
        |i| (core::f32::consts::TAU * freq * i as f32 / sr).sin(),
        n,
    );
    acid_core::analysis::rms(&out[n / 2..])
}

#[test]
fn passes_low_frequencies_and_attenuates_above_cutoff() {
    let sr = 48_000.0;
    let pass = sine_hz_rms(1_000.0, 50.0, sr, 0.0);
    let stop = sine_hz_rms(1_000.0, 8_000.0, sr, 0.0);
    assert!(pass > 0.45, "50 Hz should pass, got {pass}");
    assert!(
        stop < 0.5 * pass / 8.0,
        "8 kHz should be far down, got {stop}"
    );
}

#[test]
fn rolloff_is_about_24_db_per_octave() {
    let sr = 48_000.0;
    // Two octaves above a low cutoff, so the measurement stays clear of Nyquist warping: a bilinear
    // one-pole approaches its asymptote only well below `sr/2`, and the plan's 3.2k/6.4k pair reads
    // ~14 dB/octave purely because of that warp.
    let a = sine_hz_rms(200.0, 800.0, sr, 0.0);
    let b = sine_hz_rms(200.0, 1_600.0, sr, 0.0);
    let db_per_oct = 20.0 * (a / b.max(1e-9)).log10();
    assert!((db_per_oct - 24.0).abs() < 8.0, "got {db_per_oct} dB/oct");
}

#[test]
fn resonance_lifts_the_cutoff_region() {
    let sr = 48_000.0;
    let flat = sine_hz_rms(800.0, 800.0, sr, 0.0);
    let peaky = sine_hz_rms(800.0, 800.0, sr, 0.9);
    assert!(peaky > flat * 1.5, "flat {flat} peaky {peaky}");
}

#[test]
fn stays_finite_at_maximum_resonance_and_top_cutoff() {
    let sr = 44_100.0;
    let mut f = LadderFilter::new(sr);
    f.set_cutoff_hz(20_000.0);
    f.set_resonance(1.0);
    let out = run(&mut f, |i| if i % 96 < 48 { 0.9 } else { -0.9 }, 96_000);
    assert!(
        out.iter().all(|s| s.is_finite() && s.abs() < 50.0),
        "filter blew up"
    );
}
