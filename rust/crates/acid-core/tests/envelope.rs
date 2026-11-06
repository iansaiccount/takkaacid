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
