use acid_core::analysis::dominant_hz;
use acid_core::engine::Engine;
use acid_core::oscillator::Waveform;

fn pitch_of(e: &mut Engine, hz: f32) -> f32 {
    e.note_on(hz, false);
    let mut buf = vec![0.0; 65_536];
    e.render(&mut buf);
    dominant_hz(&buf[65_536 - 32_768..], e.sample_rate())
}

#[test]
fn tuning_knob_bends_the_pitch_by_an_octave_each_way() {
    let mut e = Engine::new(48_000.0);
    e.set_tuning(0.5);
    let base = pitch_of(&mut e, 110.0);
    e.note_off();
    e.set_tuning(1.0);
    let up = pitch_of(&mut e, 110.0);
    e.note_off();
    e.set_tuning(0.0);
    let down = pitch_of(&mut e, 110.0);
    assert!((base - 110.0).abs() < 2.0, "base {base}");
    assert!((up - 220.0).abs() < 5.0, "up {up}");
    assert!((down - 55.0).abs() < 2.0, "down {down}");
}

#[test]
fn waveform_switch_changes_timbre_not_pitch() {
    let mut e = Engine::new(48_000.0);
    e.set_tuning(0.5);
    e.set_waveform(Waveform::Saw);
    let saw = pitch_of(&mut e, 82.4);
    e.note_off();
    e.set_waveform(Waveform::Square);
    let square = pitch_of(&mut e, 82.4);
    assert!((saw - square).abs() < 2.0, "saw {saw} square {square}");
}
