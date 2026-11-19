//! The drum machine, heard through its own surface: a kit, a bar, and the transport.
//!
//! Every drum test drives `Drums` — `render` and the transport — rather than the voices inside it, because the questions worth
//! pinning are the ones a player hears: a hit that stops at the end of its sample, a tail the envelope shapes, a stop that
//! cuts a two-second sample where the stop says, an accent that is louder *and* brighter, a closed hat that cuts an open one,
//! and two machines on one clock.

use std::sync::Arc;

use acid_core::analysis::{brightness, dominant_hz, peak, rms};
use acid_core::drums::{Drum, Drums, FilterKind, Kit, KitVoice, default_kit};
use acid_core::engine::Engine;
use acid_core::pattern::Pattern;
use acid_core::voice::ACCENT_DECAY_MS;

/// The engine's rate everywhere in this file.
const SR: f32 = 48_000.0;
/// One 16th at 120 BPM — the sample every step boundary in this file falls on.
const STEP: usize = 6_000;

// ---- fixtures ------------------------------------------------------------------------------------

fn sine(hz: f32, samples: usize) -> Arc<[f32]> {
    sine_at(hz, samples, 1.0)
}

/// The same tone at a chosen height: a quiet one leaves room for the accent's lift under the rails.
fn sine_at(hz: f32, samples: usize, amplitude: f32) -> Arc<[f32]> {
    (0..samples)
        .map(|i| amplitude * (core::f32::consts::TAU * hz * i as f32 / SR).sin())
        .collect::<Vec<f32>>()
        .into()
}

/// Repeatable pseudo-noise: white enough to have spectrum everywhere the filter can pass, and the same signal every run, so
/// the three faces are compared on one input.
fn noise(samples: usize) -> Arc<[f32]> {
    let mut state = 0x2545_f491u32;
    (0..samples)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / 8_388_608.0 - 1.0
        })
        .collect::<Vec<f32>>()
        .into()
}

/// A slow ramp across the whole sample, rising or falling: a signal whose value says *where* in the sample the voice is,
/// which is what a retrigger and a kit swap have to be read from.
fn ramp(samples: usize, rising: bool) -> Arc<[f32]> {
    (0..samples)
        .map(|i| {
            let x = i as f32 / samples as f32;
            if rising { x } else { 1.0 - x }
        })
        .collect::<Vec<f32>>()
        .into()
}

/// The default sixteen, with the voices named here replaced.
fn kit_with(voices: &[(Drum, KitVoice)]) -> Arc<Kit> {
    let mut kit = default_kit();
    for (drum, voice) in voices {
        kit.voices[drum.index()] = voice.clone();
    }
    Arc::new(kit)
}

/// A bar whose steps strike the voices named here; every other step is a rest.
fn pattern_of(hits: &[(usize, Drum, bool)]) -> Arc<Pattern> {
    let mut pattern = Pattern::empty();
    for &(step, drum, accented) in hits {
        pattern.steps[step].hit(drum, accented);
    }
    Arc::new(pattern)
}

/// A machine on the bar at 120 BPM with the transport already started.
fn playing(kit: Arc<Kit>, pattern: Arc<Pattern>) -> Drums {
    let mut drums = Drums::new(SR, kit);
    drums.set_bpm(120.0);
    drums.set_pattern(pattern);
    drums.start();
    drums
}

/// One hit of `voice`, accented or not, and the block it renders.
fn hit_of(voice: KitVoice, accented: bool) -> Vec<f32> {
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, accented)]),
    );
    let mut buf = vec![0.0; 4_800];
    drums.render(&mut buf);
    buf
}

/// How often the signal crosses upwards through zero: the cheap way to read a pitch off a sine.
fn zero_crossings(buf: &[f32]) -> usize {
    buf.windows(2)
        .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
        .count()
}

fn mean(buf: &[f32]) -> f32 {
    buf.iter().sum::<f32>() / buf.len() as f32
}

// ---- one hit, its tail, and where a stop cuts it -------------------------------------------------

#[test]
fn a_hit_plays_its_sample_once_and_stops() {
    // A 100 ms sample. The voice has a 300 ms decay, so the envelope is still open when the sample's last value goes by:
    // what silences the voice is the sample ending, not the envelope closing.
    let voice = KitVoice {
        sample: sine(1_000.0, 4_800),
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut buf = vec![0.0; 24_000];
    drums.render(&mut buf);

    assert!(
        peak(&buf[..4_800]) > 0.05,
        "the hit sounded at all: {}",
        peak(&buf[..4_800])
    );
    assert!(
        peak(&buf[STEP..]) < 1e-6,
        "the sample ran out and the voice stopped: {}",
        peak(&buf[STEP..])
    );
}

#[test]
fn the_envelope_shapes_the_tail() {
    // A two-second sample and a 200 ms release: the buffer after the stop covers the whole release and out the other side,
    // so what it shows is the ADSR's release rather than a cut. Sustain is 1.0, so the level is still up when the stop
    // arrives — otherwise the release would be releasing a level nobody could hear.
    let voice = KitVoice {
        sample: sine(200.0, 96_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        release_ms: 200.0,
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut head = vec![0.0; 4_800];
    drums.render(&mut head);
    assert!(
        rms(&head[2_400..]) > 0.2,
        "the hit is sounding before the stop: {}",
        rms(&head[2_400..])
    );

    drums.stop();
    let mut tail = vec![0.0; 12_000]; // 250 ms
    // In device-sized blocks, not one gulp: a release that is re-armed every block would never finish, and one render call
    // for the whole tail is the case that hides it.
    for block in tail.chunks_mut(480) {
        drums.render(block);
    }
    let first = rms(&tail[..2_400]); // the release's own start
    let middle = rms(&tail[7_200..9_600]); // 150-200 ms in, most of the way down
    let after = rms(&tail[10_560..]); // past the 200 ms the release was given

    assert!(
        first > 0.2,
        "the release starts from the level the stop found: {first}"
    );
    assert!(
        middle > 0.0 && middle < first * 0.8,
        "the release is a ramp down, not a held level: {first} -> {middle}"
    );
    assert!(
        after < first * 0.02,
        "the release ended when it said it would: {after}"
    );
}

#[test]
fn a_stop_cuts_a_long_sample_where_the_stop_says() {
    // The same two-second sample, stopped after 50 ms with the default 20 ms release: the block is half a second long and
    // the sample had 1.95 seconds left, so anything audible late in this block would be the sample playing past the stop.
    let voice = KitVoice {
        sample: sine(200.0, 96_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut head = vec![0.0; 2_400];
    drums.render(&mut head);
    assert!(peak(&head) > 0.3, "the hit is sounding: {}", peak(&head));

    drums.stop();
    let mut tail = vec![0.0; 24_000];
    // Block by block, the way the audio device asks for it: a release re-armed on every block would never reach silence.
    for block in tail.chunks_mut(480) {
        drums.render(block);
    }
    assert!(
        peak(&tail[..960]) > 0.01,
        "the release has not been cut to zero either: {}",
        peak(&tail[..960])
    );
    assert!(
        peak(&tail[1_500..]) < 1e-3,
        "the tail is gone inside the release, not at the end of the block: {}",
        peak(&tail[1_500..])
    );
}

#[test]
fn a_retrigger_restarts_the_sample() {
    // A rising ramp: the value read is the position in the sample, so a retrigger that restarts reads near zero and one
    // that carries on from the old tail reads on up.
    let voice = KitVoice {
        sample: ramp(15_000, true),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false), (1, Drum::Kick, false)]),
    );
    let mut buf = vec![0.0; 12_000];
    drums.render(&mut buf);

    let before = peak(&buf[5_600..5_990]);
    let after = peak(&buf[6_020..6_200]);
    assert!(
        before > 0.3,
        "the first hit reached most of the way up the ramp: {before}"
    );
    assert!(
        after < before * 0.2,
        "the second hit restarted at the sample's first value ({after}), not at the old tail ({before})"
    );
}

// ---- accent, filter, tuning ----------------------------------------------------------------------

#[test]
fn accent_is_louder_and_brighter() {
    // Louder: a quiet low tone, whose height is the accent's gain and nothing else — the filter's sweep cannot change how tall
    // a note is when it is already passing all of it. Half scale, so the 1.6x lift lands under the rails.
    let tone = KitVoice {
        sample: sine_at(200.0, 24_000, 0.5),
        cutoff: 1.0,
        ..KitVoice::default()
    };
    let (plain_peak, hot_peak) = (
        peak(&hit_of(tone.clone(), false)),
        peak(&hit_of(tone, true)),
    );
    assert!(
        hot_peak > plain_peak * 1.2,
        "an accent is louder: {hot_peak} against {plain_peak}"
    );

    // Brighter: noise through a filter that is open but not wide open, so the accent's lift has somewhere to go.
    let hiss = KitVoice {
        sample: noise(24_000),
        cutoff: 0.35,
        ..KitVoice::default()
    };
    let plain = hit_of(hiss.clone(), false);
    let hot = hit_of(hiss, true);
    let (hot_side, plain_side) = (
        brightness(&hot[1_000..3_000]),
        brightness(&plain[1_000..3_000]),
    );
    assert!(
        hot_side > plain_side * 1.2,
        "an accent is brighter as well as louder, or a level change passes for one: {hot_side} against {plain_side}"
    );
}

#[test]
fn the_filter_has_three_faces() {
    // One signal, one cutoff, three answers. A player switches `LP | BP | HP` expecting the same hit to read as three
    // different sounds: the same noise comes back brighter through the high face, and the band face passes one band
    // rather than either end.
    let through = |kind: FilterKind| {
        let voice = KitVoice {
            sample: noise(9_600),
            filter: kind,
            cutoff: 0.5,
            resonance: 0.0,
            ..KitVoice::default()
        };
        let mut drums = playing(
            kit_with(&[(Drum::Kick, voice)]),
            pattern_of(&[(0, Drum::Kick, false)]),
        );
        let mut buf = vec![0.0; 9_600];
        drums.render(&mut buf);
        buf
    };
    let low = through(FilterKind::Low);
    let band = through(FilterKind::Band);
    let high = through(FilterKind::High);

    // Past the 2 ms attack, so the measurement is the filter rather than the envelope's own ramp.
    let (low, band, high) = (&low[480..], &band[480..], &high[480..]);
    assert!(rms(low) > 1e-3 && rms(band) > 1e-6 && rms(high) > 1e-3);
    assert!(
        brightness(high) > brightness(low) * 2.0,
        "the high face passes what the low face keeps: {} against {}",
        brightness(high),
        brightness(low)
    );
    // The band face is the one that passes neither end: a tone well under the cutoff and a tone well over it both come
    // back quiet, while the face that owns that end passes them.
    let tone = |kind: FilterKind, hz: f32| {
        let voice = KitVoice {
            sample: sine(hz, 9_600),
            filter: kind,
            cutoff: 0.5,
            resonance: 0.0,
            sustain: 1.0,
            decay_ms: 1_000.0,
            ..KitVoice::default()
        };
        let mut drums = playing(
            kit_with(&[(Drum::Kick, voice)]),
            pattern_of(&[(0, Drum::Kick, false)]),
        );
        let mut buf = vec![0.0; 9_600];
        drums.render(&mut buf);
        // Short of the sample's last value, which is where the block ends.
        rms(&buf[480..9_400])
    };
    let (band_under, low_under) = (tone(FilterKind::Band, 100.0), tone(FilterKind::Low, 100.0));
    let (band_over, high_over) = (
        tone(FilterKind::Band, 10_000.0),
        tone(FilterKind::High, 10_000.0),
    );
    assert!(
        band_under < low_under * 0.5,
        "the band face passes little of what the low face passes: {band_under} against {low_under}"
    );
    assert!(
        band_over < high_over * 0.5,
        "and little of what the high face passes: {band_over} against {high_over}"
    );
}

#[test]
fn tuning_moves_the_pitch() {
    // A 1 kHz sample read at twice the rate is a 2 kHz sample: TUNE +12 doubles the zero-crossing rate.
    let voice = KitVoice {
        sample: sine(1_000.0, 48_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let at = |tune: f32| {
        let mut drums = playing(
            kit_with(&[(
                Drum::Kick,
                KitVoice {
                    tune,
                    ..voice.clone()
                },
            )]),
            pattern_of(&[(0, Drum::Kick, false)]),
        );
        let mut buf = vec![0.0; 4_800];
        drums.render(&mut buf);
        zero_crossings(&buf[1_000..3_000])
    };
    let plain = at(0.0);
    let up = at(12.0);
    assert!(plain > 20, "the window holds cycles to count: {plain}");
    let ratio = up as f32 / plain as f32;
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "+12 semitones should double the rate: {plain} -> {up} crossings ({ratio}x)"
    );
}

#[test]
fn the_rack_transpose_moves_every_voice() {
    // The transpose is the rehearsal, not the patch: it reaches the machine the way it reaches the 303's voice.
    let voice = KitVoice {
        sample: sine(1_000.0, 48_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let at = |transpose: i32| {
        let mut drums = playing(
            kit_with(&[(Drum::Kick, voice.clone())]),
            pattern_of(&[(0, Drum::Kick, false)]),
        );
        drums.set_transpose(transpose);
        let mut buf = vec![0.0; 4_800];
        drums.render(&mut buf);
        zero_crossings(&buf[1_000..3_000])
    };
    let plain = at(0);
    let up = at(12);
    let ratio = up as f32 / plain as f32;
    assert!(
        (ratio - 2.0).abs() < 0.2,
        "the transpose should reach the drums: {plain} -> {up} crossings ({ratio}x)"
    );
}

// ---- the choke group, the audition, the mask -----------------------------------------------------

#[test]
fn an_open_hat_is_choked_by_a_closed_one() {
    // The closed hat ships silent on purpose: then the only thing in the buffer is the open hat, and its level is the
    // envelope's. A 40 ms release means the choke is a fade — audible at 10 ms, gone by 60 — rather than a cut to zero,
    // which would click.
    let open = KitVoice {
        sample: noise(48_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        release_ms: 40.0,
        ..KitVoice::default()
    };
    let closed = KitVoice {
        sample: Arc::default(),
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::OpenHat, open), (Drum::ClosedHat, closed)]),
        pattern_of(&[(0, Drum::OpenHat, false), (1, Drum::ClosedHat, false)]),
    );
    let mut buf = vec![0.0; 12_000];
    drums.render(&mut buf);

    let before = rms(&buf[5_000..5_900]);
    let within_10ms = rms(&buf[STEP..STEP + 480]);
    let later = rms(&buf[8_880..11_880]);
    assert!(before > 0.1, "the open hat is ringing: {before}");
    assert!(
        within_10ms < before * 0.95,
        "the closed hat has begun to cut the open one within 10 ms: {before} -> {within_10ms}"
    );
    assert!(
        within_10ms > before * 0.3,
        "and it folds it away through the release rather than cutting it to zero: {within_10ms}"
    );
    assert!(
        later < before * 0.05,
        "the open hat is gone inside its release: {later}"
    );
}

#[test]
fn a_muted_hat_still_chokes_an_open_one() {
    // The mask silences output, not the sequencer: a drummer who mutes the closed hat must still get the open hat cut,
    // or the mute would change the pattern's feel instead of its level.
    let open = KitVoice {
        sample: noise(48_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        release_ms: 40.0,
        ..KitVoice::default()
    };
    let closed = KitVoice {
        sample: noise(4_800),
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::OpenHat, open), (Drum::ClosedHat, closed)]),
        pattern_of(&[(0, Drum::OpenHat, false), (1, Drum::ClosedHat, false)]),
    );
    // Only the closed hat is muted, so the open hat is the only voice that could be sounding after the choke.
    drums.set_mute_mask(Drum::ClosedHat.bit());
    let mut buf = vec![0.0; 12_000];
    drums.render(&mut buf);

    let before = rms(&buf[5_000..5_900]);
    let later = rms(&buf[8_880..11_880]);
    assert!(before > 0.1, "the open hat is ringing: {before}");
    assert!(
        later < before * 0.05,
        "the muted closed hat still cut the open one: {later}"
    );
}

#[test]
fn audition_sounds_with_the_transport_stopped() {
    // `HIT` on the face: a hit with no step under it, played while the line is stopped — and cut by the same `STOP`
    // everything else is.
    let voice = KitVoice {
        sample: sine(200.0, 48_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let mut drums = Drums::new(SR, kit_with(&[(Drum::Kick, voice)]));

    drums.audition(Drum::Kick);
    let mut buf = vec![0.0; 4_800];
    drums.render(&mut buf);
    assert!(
        peak(&buf) > 0.1,
        "the audition sounded without a transport: {}",
        peak(&buf)
    );

    drums.stop();
    let mut tail = vec![0.0; 4_800];
    for block in tail.chunks_mut(480) {
        drums.render(block);
    }
    assert!(
        peak(&tail[1_500..]) < 1e-3,
        "and the stop cut it: {}",
        peak(&tail[1_500..])
    );

    // And a hit asked for and then stopped inside the same block never sounds at all: the request is a slot to be served,
    // not a queue that survives the silence.
    drums.audition(Drum::Kick);
    drums.stop();
    let mut never = vec![0.0; 4_800];
    for block in never.chunks_mut(480) {
        drums.render(block);
    }
    assert_eq!(peak(&never), 0.0, "an audition survived the stop");
}

#[test]
fn a_muted_voice_is_silent_and_keeps_its_step() {
    let voice = KitVoice {
        sample: sine(1_000.0, 4_800),
        ..KitVoice::default()
    };
    let pattern = pattern_of(&[(0, Drum::Kick, false)]);
    let mut drums = playing(kit_with(&[(Drum::Kick, voice)]), pattern);

    drums.set_mute_mask(Drum::Kick.bit());
    let mut buf = vec![0.0; 9_600];
    drums.render(&mut buf);
    assert_eq!(peak(&buf), 0.0, "a muted voice makes no sound");

    // The step is still there — the mask never touched the pattern, which is the panel's — so unmuting plays it.
    drums.set_mute_mask(0);
    drums.start();
    let mut again = vec![0.0; 4_800];
    drums.render(&mut again);
    assert!(
        peak(&again) > 0.05,
        "the step survived the mute: {}",
        peak(&again)
    );
}

#[test]
fn every_voice_muted_is_silence_with_the_steps_still_on() {
    // Review Focus 5's first half: sixteen voices on one step, all muted. The machine is silent and the pattern still
    // says all sixteen struck — the mixer's ceiling is the other half, and task 8's.
    let voice = KitVoice {
        sample: noise(9_600),
        ..KitVoice::default()
    };
    let mut bar = Pattern::empty();
    let mut kit_voices = Vec::new();
    for drum in Drum::ALL {
        bar.steps[0].hit(drum, true);
        kit_voices.push((drum, voice.clone()));
    }
    let mut drums = playing(kit_with(&kit_voices), Arc::new(bar));

    drums.set_mute_mask(u16::MAX);
    let mut buf = vec![0.0; 9_600];
    drums.render(&mut buf);
    assert_eq!(peak(&buf), 0.0, "all sixteen muted is silence");

    // The steps are still on: unmuting brings all sixteen back, which is the half a mask could have eaten.
    drums.set_mute_mask(0);
    drums.start();
    let mut again = vec![0.0; 4_800];
    drums.render(&mut again);
    assert!(
        peak(&again) > 0.1,
        "unmuting brings all sixteen back: {}",
        peak(&again)
    );
}

// ---- the kit, and the clock ----------------------------------------------------------------------

#[test]
fn a_kit_swap_changes_the_sound_and_keeps_the_hit_going() {
    // A dial turn on the panel publishes a whole new kit. What a voice is playing changes at once; where it has got to
    // does not, or every turn of TUNE would retrigger the hit under it. Two ramps read the position off the sound: the
    // rising one is at 0.2 of its length when the falling one arrives, and the falling one's value there is 0.8.
    let rising = KitVoice {
        sample: ramp(24_000, true),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let falling = KitVoice {
        sample: ramp(24_000, false),
        ..rising.clone()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, rising.clone())]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut first = vec![0.0; 4_800];
    drums.render(&mut first);
    assert!(
        (mean(&first[2_400..]) - 0.15).abs() < 0.05,
        "the rising ramp was at its quarter: {}",
        mean(&first[2_400..])
    );

    drums.set_kit(kit_with(&[(Drum::Kick, falling)]));
    let mut second = vec![0.0; 4_800];
    drums.render(&mut second);
    let level = mean(&second[..200]);
    assert!(
        (level - 0.8).abs() < 0.03,
        "the new sample plays from where the voice had got to ({level}), not from its first sample"
    );
}

#[test]
fn a_new_machine_takes_its_mutes_from_its_kit_and_a_kit_swap_does_not() {
    // Loading a kit seeds the mask from the kit's own flags (the same rule `SharedState::adopt_kit` follows); editing one
    // leaves the mask the player set alone, or a dial turn would undo a mute.
    let sampled = |mute: bool| KitVoice {
        sample: sine(1_000.0, 4_800),
        mute,
        ..KitVoice::default()
    };
    let pattern = pattern_of(&[(0, Drum::Kick, false)]);

    let mut drums = playing(kit_with(&[(Drum::Kick, sampled(true))]), pattern.clone());
    let mut buf = vec![0.0; 9_600];
    drums.render(&mut buf);
    assert_eq!(peak(&buf), 0.0, "the kit's own mute started it silent");

    drums.set_kit(kit_with(&[(Drum::Kick, sampled(false))]));
    // Back to step 0, where the hit is: the block above has already walked past it, and a buffer with no hit in it could
    // not tell a muted voice from a quiet bar.
    drums.start();
    let mut second = vec![0.0; 9_600];
    drums.render(&mut second);
    assert_eq!(
        peak(&second),
        0.0,
        "a kit swapped in is the edit path: the mask is still the player's, so KICK stays muted"
    );

    drums.set_mute_mask(0);
    drums.start();
    let mut third = vec![0.0; 4_800];
    drums.render(&mut third);
    assert!(peak(&third) > 0.05, "unmuted, the voice plays");
}

#[test]
fn a_kit_with_no_samples_is_silence_and_a_hit_is_harmless() {
    // The default kit is sixteen voices with nothing to play — which is exactly what a new drum device holds. Striking
    // every one of them must be silence, not a panic and not a slice past the end.
    let mut bar = Pattern::empty();
    for drum in Drum::ALL {
        bar.steps[0].hit(drum, true);
    }
    let mut drums = playing(Arc::new(default_kit()), Arc::new(bar));
    let mut buf = vec![0.0; 9_600];
    drums.render(&mut buf);
    assert_eq!(peak(&buf), 0.0, "no samples is silence, not a panic");
}

#[test]
fn a_hostile_sample_is_silence_not_a_poisoned_voice() {
    // A float WAV holds whatever its writer put there, and task 5 passed those samples through as recorded, leaving the
    // ceiling to this machine. One non-finite sample would lodge in the filter's own state and silence the voice for the
    // rest of the session, so it plays as nothing instead — and the voice keeps sounding around it.
    let mut samples: Vec<f32> = (0..48_000)
        .map(|i| (core::f32::consts::TAU * 100.0 * i as f32 / SR).sin())
        .collect();
    samples[100] = f32::NAN;
    samples[200] = f32::INFINITY;
    let voice = KitVoice {
        sample: samples.into(),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut buf = vec![0.0; 24_000];
    drums.render(&mut buf);

    assert!(
        buf.iter().all(|sample| sample.is_finite()),
        "a non-finite sample reached the output"
    );
    assert!(
        peak(&buf[1_000..]) > 0.1,
        "and the voice still sounds around the hostile samples: {}",
        peak(&buf[1_000..])
    );
}

#[test]
fn an_enormous_sample_does_not_outlive_itself() {
    // A float WAV can hold samples near `f32::MAX`, and through a resonant filter that is enough to overflow the filter's own
    // state — which is a different failure from the sample being non-finite, because the sample is finite. The voice has to
    // come back: the hostile stretch plays as whatever the rails allow, and the tone after it is still there.
    let mut samples: Vec<f32> = (0..96_000)
        .map(|i| (core::f32::consts::TAU * 200.0 * i as f32 / SR).sin())
        .collect();
    for sample in &mut samples[..4_800] {
        *sample = f32::MAX;
    }
    let voice = KitVoice {
        sample: samples.into(),
        cutoff: 1.0,
        resonance: 1.0,
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, voice)]),
        pattern_of(&[(0, Drum::Kick, false)]),
    );
    let mut buf = vec![0.0; 24_000];
    drums.render(&mut buf);

    assert!(
        buf.iter().all(|sample| sample.is_finite()),
        "an enormous sample turned the output non-finite"
    );
    assert!(
        peak(&buf[6_000..]) > 0.1,
        "and the voice recovered: {} after the hostile stretch",
        peak(&buf[6_000..])
    );
}

#[test]
fn a_cut_off_turn_is_heard_on_the_next_block() {
    // The kit is re-read at the top of every block (trap 14's cousin): a dial turned while a hit is still sounding is heard
    // without retriggering it, and the filter's own state survives the change.
    //
    // The hit is **accented** on purpose. An accent's pulse only decays in `next` while the voice is playing, and it is drawn
    // down to zero when it is spent — so a hit without an accent never reaches the arm that does it, and a test that used one
    // would pass with that arm deleted.
    let bright = KitVoice {
        sample: noise(96_000),
        cutoff: 0.9,
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let dark = KitVoice {
        cutoff: 0.1,
        ..bright.clone()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, bright)]),
        pattern_of(&[(0, Drum::Kick, true)]),
    );
    // The accented hit, and then nine and a half of the pulse's time constants in device-sized blocks — long enough for it to be
    // spent, whatever the rate. Both windows are thrown away: measuring while the accent is open would make this test pass
    // without the dial turn landing at all.
    let mut accent = vec![0.0; 4_800];
    drums.render(&mut accent);
    // Nine and a half pulse time constants: `ln(1e4) ≈ 9.21` is where the pulse falls under the arm's threshold, and the
    // constant is the engine's own, so this stays right at any rate the kit is decoded at.
    let mut spent = vec![0.0; (9.5 * ACCENT_DECAY_MS * 0.001 * SR) as usize];
    for block in spent.chunks_mut(480) {
        drums.render(block);
    }

    // The baseline: the voice still playing at the plain cutoff, with no accent on it.
    let mut first = vec![0.0; 4_800];
    drums.render(&mut first);
    drums.set_kit(kit_with(&[(Drum::Kick, dark)]));
    let mut second = vec![0.0; 4_800];
    drums.render(&mut second);

    let (before, after) = (brightness(&first[480..]), brightness(&second[480..]));
    assert!(
        before > 0.0 && after < before * 0.5,
        "CUT OFF turned down should darken the voice it is still playing: {before} -> {after}"
    );
}

#[test]
fn a_resonant_voice_at_the_edges_stays_finite() {
    // RESONANCE at the top and CUT OFF at both ends: the damping is at its smallest and the coefficients are at their clamps,
    // which is where a state-variable filter would ring itself to infinity if it could. What this pins is that the voice's own
    // rails hold and nothing turns non-finite.
    for cutoff in [0.0, 1.0] {
        let voice = KitVoice {
            sample: noise(24_000),
            cutoff,
            resonance: 1.0,
            sustain: 1.0,
            decay_ms: 1_000.0,
            ..KitVoice::default()
        };
        let mut drums = playing(
            kit_with(&[(Drum::Kick, voice)]),
            pattern_of(&[(0, Drum::Kick, false)]),
        );
        let mut buf = vec![0.0; 24_000];
        drums.render(&mut buf);

        assert!(
            buf.iter().all(|sample| sample.is_finite()),
            "CUT OFF {cutoff} at full resonance went non-finite"
        );
        let loudest = peak(&buf);
        assert!(
            loudest <= 1.0,
            "one voice cannot pass the rails: {loudest} at CUT OFF {cutoff}"
        );
        if cutoff > 0.5 {
            assert!(
                loudest > 0.05,
                "and the filter is wide open here, so it sounds: {loudest}"
            );
        }
    }
}

#[test]
fn start_at_lands_on_the_step_it_names() {
    // A device joining a paused rack lands on the step the resume will use, so the step it names has to sound — not the top
    // of the bar, whose steps are rests here.
    let voice = KitVoice {
        sample: sine(1_000.0, 4_800),
        ..KitVoice::default()
    };
    let mut drums = Drums::new(SR, kit_with(&[(Drum::Kick, voice)]));
    drums.set_bpm(120.0);
    drums.set_pattern(pattern_of(&[(8, Drum::Kick, false)]));
    drums.start_at(8);

    let mut buf = vec![0.0; 4_800];
    drums.render(&mut buf);
    assert!(
        peak(&buf) > 0.05,
        "step 8's kick sounded at once: {}",
        peak(&buf)
    );
}

#[test]
fn reverse_reads_the_bar_from_its_other_end() {
    // REVERSE is a direction on the clock, not a second pattern: a step written on 15 is what step 0 sounds when the line
    // runs backwards.
    let voice = KitVoice {
        sample: sine(1_000.0, 4_800),
        ..KitVoice::default()
    };
    let mut drums = Drums::new(SR, kit_with(&[(Drum::Kick, voice)]));
    drums.set_bpm(120.0);
    drums.set_pattern(pattern_of(&[(15, Drum::Kick, false)]));
    drums.set_reverse(true);
    drums.start();

    let mut buf = vec![0.0; 4_800];
    drums.render(&mut buf);
    assert!(
        peak(&buf) > 0.05,
        "the last step sounded on the first: {}",
        peak(&buf)
    );
}

#[test]
fn the_bar_is_the_303s() {
    // Two machines started on the same frame, on one clock: same step, same bar line, at the same sample. The 133 BPM and
    // the swing are deliberate — a second clock would drift off a tempo this unround inside a bar.
    let mut engine = Engine::new(SR);
    engine.set_bpm(133.0);
    engine.set_swing(0.3);
    let mut drums = Drums::new(SR, kit_with(&[(Drum::Kick, KitVoice::default())]));
    drums.set_bpm(133.0);
    drums.set_swing(0.3);
    engine.start();
    drums.start();

    let mut mine = vec![0.0; 1_024];
    let mut theirs = vec![0.0; 1_024];
    let mut bars = 0;
    for _ in 0..200 {
        engine.render(&mut mine);
        drums.render(&mut theirs);
        assert_eq!(
            engine.position(),
            drums.position(),
            "the drum machine walked off the 303's step"
        );
        assert_eq!(
            engine.bar_line().is_some(),
            drums.bar_line().is_some(),
            "one machine announced a bar line and the other did not"
        );
        if let (Some(acid), Some(drum)) = (engine.bar_line(), drums.bar_line()) {
            assert_eq!(
                acid, drum,
                "the two machines put the bar line on different samples"
            );
            bars += 1;
        }
    }
    assert!(bars >= 2, "two bars went by: {bars} bar lines announced");
}

#[test]
fn a_late_start_waits_out_the_silence_it_is_given() {
    let voice = KitVoice {
        sample: sine(1_000.0, 4_800),
        ..KitVoice::default()
    };
    let mut drums = Drums::new(SR, kit_with(&[(Drum::Kick, voice)]));
    drums.set_bpm(120.0);
    drums.set_pattern(pattern_of(&[(0, Drum::Kick, false)]));
    drums.start_later(0, 1_000);

    let mut buf = vec![0.0; 4_800];
    drums.render(&mut buf);
    assert_eq!(
        peak(&buf[..1_000]),
        0.0,
        "the start waited for the sample it was told to begin on"
    );
    assert!(
        peak(&buf[1_000..]) > 0.05,
        "and began there: {}",
        peak(&buf[1_000..])
    );
}

#[test]
fn a_pause_keeps_the_step_and_a_resume_carries_on_from_it() {
    // A kick on step 0 and a snare on step 4, then a pause three steps in: the resume has to carry on into the snare.
    // Starting again from the top instead would sound the kick, which is a different pitch and a different test.
    let kick = KitVoice {
        sample: sine(200.0, 24_000),
        sustain: 1.0,
        decay_ms: 1_000.0,
        ..KitVoice::default()
    };
    let snare = KitVoice {
        sample: sine(2_000.0, 24_000),
        ..kick.clone()
    };
    let mut drums = playing(
        kit_with(&[(Drum::Kick, kick), (Drum::Snare, snare)]),
        pattern_of(&[(0, Drum::Kick, false), (4, Drum::Snare, false)]),
    );
    let mut head = vec![0.0; STEP * 3];
    drums.render(&mut head);
    assert_eq!(
        drums.position(),
        3,
        "three steps played and the line is on the fourth"
    );

    drums.pause();
    drums.render(&mut vec![0.0; 1_000]);
    drums.resume();
    let mut after = vec![0.0; STEP + 2_400];
    drums.render(&mut after);
    // Step 3 is a rest; step 4's snare lands on the next boundary.
    assert!(
        peak(&after[..STEP]) < 1e-3,
        "the rest on step 3 is silent: {}",
        peak(&after[..STEP])
    );
    let crossings = zero_crossings(&after[STEP + 200..STEP + 2_200]);
    assert!(
        crossings > 60,
        "the resume played the snare on step 4, not the kick on step 0: {crossings} crossings"
    );
}

#[test]
fn a_start_strikes_rather_than_gliding_from_the_previous_run() {
    // The slide history is per run: a step left sliding into nothing must not retune the first note of the next run
    // with the gate still shut. This is `Performer::arm`'s half of the trait, and the drum machine is where the 303's
    // slide state no longer lives.
    use acid_core::pattern::Step;
    use acid_core::pitch::Pitch;
    use acid_core::sequencer::Sequencer;
    use acid_core::voice::Voice;

    let mut seq = Sequencer::new(SR);
    let mut steps = [Step::rest(); 16];
    // Step 0 is C3 (130.8 Hz) and the run's last step slides into an A1 (55 Hz), so what the restart does is audible as a
    // pitch: struck, the note is 130.8 Hz from its first cycle; glided, it starts near 55 Hz and climbs for 60 ms.
    steps[0] = Step::note(Pitch::C, 3);
    steps[15] = Step {
        slide: true,
        ..Step::note(Pitch::A, 1)
    };
    seq.set_pattern(Arc::new(Pattern::from_steps(steps)));
    seq.set_bpm(120.0);
    let mut voice = Voice::new(SR);
    seq.start();
    // Fifteen steps and a little more: step 15's slide is the last thing the run does, and the run stops before the wrap can
    // strike step 0 and clear the slide itself.
    let mut bar = vec![0.0; STEP * 15 + 3_000];
    seq.render(&mut voice, &mut bar);
    seq.stop();
    let mut tail = vec![0.0; 2_400];
    seq.render(&mut voice, &mut tail);
    // The release is 20 ms, so a 50 ms tail is silence at its end: the run is over and the next one starts from nothing.
    let end = peak(&tail[2_000..]);
    assert!(
        end < 1e-3,
        "the run ended: {end} left at the end of the release"
    );

    seq.start();
    let mut again = vec![0.0; 4_800];
    seq.render(&mut voice, &mut again);
    let first = dominant_hz(&again[..1_200], SR);
    assert!(
        first > 100.0,
        "the first note of the run was struck at its own pitch, not glided from the last step of the one before: {first} Hz"
    );
}
