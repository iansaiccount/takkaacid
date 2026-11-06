use acid_core::analysis::brightness;
use acid_core::voice::Voice;

#[test]
fn more_env_mod_is_brighter_on_the_attack() {
    let sr = 48_000.0;
    let brightness_for = |env_mod: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.25);
        v.set_env_mod_knob(env_mod);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        brightness(&b[0..2_400])
    };
    let dark = brightness_for(0.0);
    let bright = brightness_for(0.9);
    assert!(bright > dark * 1.2, "dark {dark} bright {bright}");
}

#[test]
fn a_higher_cutoff_knob_is_brighter() {
    let sr = 48_000.0;
    let brightness_for = |cutoff: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(cutoff);
        v.set_env_mod_knob(0.0);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 9_600];
        v.render(&mut b);
        brightness(&b[4_800..])
    };
    assert!(brightness_for(0.9) > brightness_for(0.2) * 1.5);
}

#[test]
fn accent_opens_the_filter_further_than_a_plain_note() {
    let sr = 48_000.0;
    let brightness_for = |accent: bool| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.2);
        v.set_env_mod_knob(0.5);
        v.set_accent_knob(1.0);
        v.note_on(55.0, accent);
        let mut b = vec![0.0; 4_800];
        v.render(&mut b);
        brightness(&b[0..2_400])
    };
    assert!(brightness_for(true) > brightness_for(false) * 1.1);
}

#[test]
fn resonance_knob_highlights_the_cutoff_region() {
    let sr = 48_000.0;
    let brightness_for = |res: f32| {
        let mut v = Voice::new(sr);
        v.set_cutoff_knob(0.4);
        v.set_resonance_knob(res);
        v.set_env_mod_knob(0.0);
        v.note_on(55.0, false);
        let mut b = vec![0.0; 9_600];
        v.render(&mut b);
        brightness(&b[4_800..])
    };
    assert!(brightness_for(0.95) > brightness_for(0.1) * 1.2);
}
