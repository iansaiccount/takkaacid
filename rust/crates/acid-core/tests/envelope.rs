use acid_core::envelope::{Envelope, Stage};

#[test]
fn attack_peaks_in_about_two_ms() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.set_attack_ms(2.0);
    e.gate_on();
    let n = (0.002 * sr) as usize;
    let mut peak = 0.0f32;
    for _ in 0..n {
        peak = peak.max(e.next());
    }
    assert!(peak > 0.95, "peak only {peak}");
}

#[test]
fn decay_reaches_sustain_within_the_requested_time() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.set_attack_ms(1.0);
    e.set_decay_seconds(0.1);
    e.set_sustain(0.35);
    e.gate_on();
    for _ in 0..(0.002 * sr) as usize {
        e.next();
    }
    for _ in 0..(0.1 * sr) as usize {
        e.next();
    }
    let level = e.next();
    assert!((level - 0.35).abs() < 0.05, "level {level}");
    assert_eq!(e.stage(), Stage::Sustain);
}

#[test]
fn release_reaches_silence_and_goes_idle() {
    let sr = 44_100.0;
    let mut e = Envelope::new(sr);
    e.set_release_ms(20.0);
    e.gate_on();
    for _ in 0..(0.05 * sr) as usize {
        e.next();
    }
    e.gate_off();
    for _ in 0..(0.02 * sr) as usize + 16 {
        e.next();
    }
    assert!(e.next() < 0.001);
    assert!(e.is_idle());
}

#[test]
fn retrigger_restarts_from_the_current_level() {
    let sr = 48_000.0;
    let mut e = Envelope::new(sr);
    e.gate_on();
    for _ in 0..(0.05 * sr) as usize {
        e.next();
    }
    e.gate_off();
    for _ in 0..(0.005 * sr) as usize {
        e.next();
    }
    let before = e.next();
    e.gate_on();
    let after = e.next();
    assert!(
        after >= before - 0.01,
        "retrigger dropped the level: {before} -> {after}"
    );
    assert_eq!(e.stage(), Stage::Attack);
}

/// A machine that re-states its voice's settings every block must not restart the ramps it is describing: a release that
/// is re-armed every 2 ms never reaches silence, so the time it was given becomes a time constant instead.
#[test]
fn restating_a_time_does_not_restart_the_ramp() {
    let sr = 48_000.0;
    let ramp = |restate: bool| {
        let mut e = Envelope::new(sr);
        e.set_release_ms(20.0);
        e.gate_on();
        for _ in 0..(0.05 * sr) as usize {
            e.next();
        }
        e.gate_off();
        // Twenty blocks of 2 ms: forty milliseconds, twice the release the envelope was given.
        for _ in 0..20 {
            if restate {
                e.set_release_ms(20.0);
            }
            for _ in 0..(0.002 * sr) as usize {
                e.next();
            }
        }
        e
    };

    assert!(ramp(false).is_idle(), "a 20 ms release is over after 40 ms");
    let restated = ramp(true);
    assert!(
        restated.is_idle(),
        "re-stating the release restarted it: {} left after twice the time it was given",
        restated.level()
    );
}
