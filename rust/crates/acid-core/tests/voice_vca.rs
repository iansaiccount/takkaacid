use acid_core::analysis::rms;
use acid_core::voice::Voice;

#[test]
fn an_accented_note_is_about_forty_percent_louder() {
    let sr = 48_000.0;
    let plain = {
        let mut v = Voice::new(sr);
        v.note_on(110.0, false);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        rms(&b[2_400..])
    };
    let accented = {
        let mut v = Voice::new(sr);
        v.set_accent_knob(1.0);
        v.note_on(110.0, true);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        rms(&b[2_400..])
    };
    let ratio = accented / plain;
    assert!(ratio > 1.25 && ratio < 1.55, "accent ratio {ratio}");
}

#[test]
fn a_shorter_decay_settles_lower_by_half_a_second() {
    let sr = 48_000.0;
    let level_at_500ms = |decay_knob: f32| {
        let mut v = Voice::new(sr);
        v.set_decay_knob(decay_knob);
        v.note_on(110.0, false);
        let mut b = vec![0.0; 28_800];
        v.render(&mut b);
        rms(&b[24_000..])
    };
    let short = level_at_500ms(0.0);
    let long = level_at_500ms(0.9);
    assert!(short < long * 0.9, "short {short} long {long}");
}

#[test]
fn the_voice_goes_quiet_after_note_off_without_being_retriggered() {
    let mut v = Voice::new(44_100.0);
    v.note_on(55.0, false);
    v.render(&mut vec![0.0; 4_410]);
    v.note_off();
    let mut b = vec![0.0; 8_820];
    v.render(&mut b);
    assert!(rms(&b[4_410..]) < 0.001);
    assert!(!v.is_active());
}
