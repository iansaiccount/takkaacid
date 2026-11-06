use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

#[test]
fn every_extreme_combination_stays_finite_and_in_range() {
    for sr in [44_100.0, 48_000.0, 96_000.0] {
        for wave in [Waveform::Saw, Waveform::Square] {
            for (cutoff, res, env_mod, decay, accent) in [
                (0.0, 1.0, 1.0, 0.0, 1.0),
                (1.0, 1.0, 1.0, 1.0, 1.0),
                (0.5, 0.0, 0.0, 0.5, 0.0),
                (0.0, 0.0, 1.0, 1.0, 1.0),
            ] {
                let mut e = Engine::new(sr);
                e.set_waveform(wave);
                e.set_cutoff(cutoff);
                e.set_resonance(res);
                e.set_env_mod(env_mod);
                e.set_decay(decay);
                e.set_accent(accent);
                e.note_on(27.5, true);
                let mut buf = vec![0.0; 48_000];
                e.render(&mut buf);
                assert!(buf.iter().all(|s| s.is_finite()), "NaN at sr {sr} {wave:?}");
                assert!(
                    buf.iter().all(|s| s.abs() <= 0.999),
                    "clip at sr {sr} {wave:?}"
                );
            }
        }
    }
}

#[test]
fn a_released_note_renders_exact_zeros() {
    let mut e = Engine::new(48_000.0);
    e.set_resonance(1.0);
    e.note_on(55.0, true);
    e.render(&mut vec![0.0; 4_800]);
    e.note_off();
    e.render(&mut vec![0.0; 4_800]);
    let mut tail = vec![1.0; 4_800];
    e.render(&mut tail);
    assert!(
        tail.iter().all(|s| *s == 0.0),
        "tail was not exact silence"
    );
}

#[test]
fn odd_block_sizes_produce_identical_output_to_one_long_block() {
    let render_blocks = |blocks: &[usize]| {
        let mut e = Engine::new(48_000.0);
        e.note_on(110.0, false);
        let mut out = Vec::new();
        for &n in blocks {
            let mut b = vec![0.0; n];
            e.render(&mut b);
            out.extend_from_slice(&b);
        }
        out
    };
    let one = render_blocks(&[16_384]);
    let many = render_blocks(&[1, 2, 3, 511, 1_000, 4_096, 10_771]);
    assert_eq!(one.len(), many.len());
    for (i, (a, b)) in one.iter().zip(many.iter()).enumerate() {
        assert!((a - b).abs() < 1e-6, "sample {i} differs: {a} vs {b}");
    }
}
