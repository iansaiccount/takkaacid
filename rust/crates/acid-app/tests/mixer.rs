//! The mixer: every device rendered through one stream, summed, and held under one ceiling.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use acid_app::audio::MAX_BLOCK;
use acid_app::audio::mixer::Mixer;
use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{BusState, DeviceKind, Rack, Transport};
use acid_app::midi::route::InputFilter;
use acid_app::persist;
use acid_app::state::SharedState;
use acid_core::drums::{DEFAULT_KIT_NAME, Drum, Drums, Kit, default_kit};
use acid_core::pattern::{Pattern, STEPS_PER_PATTERN, Step};

/// The rate every renderer here is built at, so a drum machine's samples and the bare `Drums` beside it are the same rate.
const RATE: f32 = 48_000.0;

fn rack_of(n: usize) -> Rack {
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    for _ in 1..n {
        rack.add(DeviceKind::Acid).expect("room");
    }
    rack.publish();
    rack
}

/// A kit whose every voice is the same sine, at `amplitude` — one buffer, sixteen voices.
///
/// A test does not decode two megabytes of factory WAVs to ask what sixteen voices do to a sum: the same sample in every
/// voice is the worst case a ceiling has to hold (sixteen in phase), and it is the easiest to recognise in a mix.
fn stub_kit(seconds: f32, amplitude: f32) -> Kit {
    let mut kit = default_kit();
    kit.name = "Stub".to_string();
    let len = (RATE * seconds) as usize;
    let samples: Vec<f32> = (0..len)
        .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / RATE).sin() * amplitude)
        .collect();
    let sample: Arc<[f32]> = Arc::from(samples);
    for drum in Drum::ALL {
        kit.voices[drum.index()].sample = Arc::clone(&sample);
    }
    kit
}

/// A held envelope, for the ceiling test: sustain at the top so sixteen voices struck together stay at their worst instead
/// of decaying away from it.
fn sustained(mut kit: Kit) -> Kit {
    for voice in &mut kit.voices {
        voice.sustain = 1.0;
        voice.decay_ms = 500.0;
    }
    kit
}

/// One kick on the downbeat and one on the backbeat.
fn kick_bar() -> [Step; STEPS_PER_PATTERN] {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[0].hit(Drum::Kick, false);
    steps[8].hit(Drum::Kick, false);
    steps
}

/// All sixteen voices struck on one step.
fn every_voice_bar() -> [Step; STEPS_PER_PATTERN] {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    for drum in Drum::ALL {
        steps[0].hit(drum, false);
    }
    steps
}

/// Give the drum device at `index` a kit and a bar: what a test does after `Rack::add`, which opens a drum machine on the
/// default kit and the 303's starter pattern — silence, because a drum step sounds from its masks.
fn dress_drums(rack: &Rack, index: usize, kit: Kit, steps: [Step; STEPS_PER_PATTERN]) {
    let state = &rack.devices()[index].state;
    state.adopt_kit(kit);
    state.set_pattern(Arc::new(Pattern::from_steps(steps)));
}

/// A rack of one 303 and one drum machine, both playing, the drum machine holding `kit` and `steps`.
fn rack_with_drums(kit: Kit, steps: [Step; STEPS_PER_PATTERN]) -> Rack {
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    let index = rack.add(DeviceKind::Drums).expect("a drum machine fits");
    dress_drums(&rack, index, kit, steps);
    rack.set_transport(Transport::Playing);
    rack
}

/// Mute every voice the way the drum face does — the state's mask, one bit per voice — and leave the steps alone.
fn mute_every_voice(state: &SharedState) {
    let mut mask = 0u32;
    for drum in Drum::ALL {
        mask |= 1 << drum.index();
    }
    state.drum_mute.store(mask, Ordering::Relaxed);
}

#[test]
fn one_device_through_the_mixer_is_the_device_it_was() {
    // The regression that makes this whole phase safe: nothing about the sound of a single device may change.
    let rack = rack_of(1);
    let state = rack.state_arc();
    rack.set_transport(acid_app::device::Transport::Playing);

    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut mixed = vec![0.0; 4_096];
    mixer.render_mono(&mut mixed);

    // The renderer this replaces, driven from the same state and a fresh engine: both start on step 0.
    let mut bare = AudioRenderer::new(48_000.0, 1, state, MAX_BLOCK);
    let mut alone = vec![0.0; 4_096];
    bare.render_mono(&mut alone);

    assert!(
        mixed.iter().any(|sample| sample.abs() > 0.01),
        "the mixer must not be silent"
    );
    for (a, b) in mixed.iter().zip(&alone) {
        assert!((a - b).abs() < 1e-6, "mixed {a} differs from bare {b}");
    }
    assert_eq!(mixer.slots(), 1);
}

#[test]
fn two_devices_sum_to_something_louder_than_one() {
    // Quieten both devices so the sum stays under the ceiling: once the master limiter engages it holds *both* cases at
    // CEILING, and a test written against full volume would be measuring the limiter rather than the sum.
    let rack = rack_of(2);
    for device in rack.devices() {
        device.state.volume.set(0.3);
    }
    rack.set_transport(acid_app::device::Transport::Playing);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    mixer.render_mono(&mut out);
    let both = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));

    let one = rack_of(1);
    one.devices()[0].state.volume.set(0.3);
    one.set_transport(acid_app::device::Transport::Playing);
    let mut single = Mixer::new(48_000.0, MAX_BLOCK, one.bus());
    let mut out = vec![0.0; 4_096];
    single.render_mono(&mut out);
    let alone = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));

    assert!(
        both > alone * 1.5,
        "two identical devices should sum, not replace: {both} vs {alone}"
    );
}

#[test]
fn unmuting_a_running_device_is_audible_and_it_kept_time_meanwhile() {
    // Two racks built the same way render the same samples, so the only difference between these mixes is the mute.
    // Comparing one mixer against itself across two windows proved nothing: the pattern moved on between them, so any
    // difference satisfied the assertion — which is how a mutation that muted nothing survived this test.
    let loud = rack_of(2);
    let quiet = rack_of(2);
    for rack in [&loud, &quiet] {
        rack.set_transport(acid_app::device::Transport::Playing);
    }
    quiet.devices()[1].state.set_mute(true);
    let loud_b = std::sync::Arc::clone(&loud.devices()[1].state);
    let quiet_b = std::sync::Arc::clone(&quiet.devices()[1].state);

    let mut loud_mixer = Mixer::new(48_000.0, MAX_BLOCK, loud.bus());
    let mut quiet_mixer = Mixer::new(48_000.0, MAX_BLOCK, quiet.bus());
    // The ramp is 10 ms, so render past it before listening: what is measured is the mute, not the fade into it.
    let mut out = vec![0.0; 4_096];
    for _ in 0..8 {
        loud_mixer.render_mono(&mut out);
        quiet_mixer.render_mono(&mut out);
    }

    let before = quiet_b.position.load(std::sync::atomic::Ordering::Relaxed);
    let mut with_b = vec![0.0; 4_096];
    let mut without_b = vec![0.0; 4_096];
    for _ in 0..8 {
        loud_mixer.render_mono(&mut with_b);
        quiet_mixer.render_mono(&mut without_b);
    }
    let difference = with_b
        .iter()
        .zip(&without_b)
        .fold(0.0f32, |most, pair| most.max((pair.0 - pair.1).abs()));
    assert!(
        difference > 1e-3,
        "muting a running device must be audible: over the same window the mix moved by only {difference}"
    );

    // Mute is a gain, not a stop: the muted device's clock is the audible one's, step for step, over the same eight
    // blocks — so unmuting rejoins in time instead of repeating the step it was muted on.
    let a = quiet_b.position.load(std::sync::atomic::Ordering::Relaxed);
    assert_ne!(
        before, a,
        "a muted device keeps counting steps so unmuting rejoins in time"
    );
    assert_eq!(
        loud_b.position.load(std::sync::atomic::Ordering::Relaxed),
        a,
        "the muted device must be on the same step as the audible one it is twinned with"
    );
}

#[test]
fn a_device_born_muted_does_not_click_on_its_first_block() {
    let rack = rack_of(1);
    let state = std::sync::Arc::clone(&rack.devices()[0].state);
    rack.set_transport(acid_app::device::Transport::Playing);
    state.set_mute(true);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    mixer.render_mono(&mut out);
    let peak = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));
    assert!(
        peak < 1e-6,
        "a slot built for a muted device must start on the muted value: {peak}"
    );
}

#[test]
fn two_devices_cannot_reach_twice_the_ceiling() {
    let rack = rack_of(2);
    for device in rack.devices() {
        device.state.volume.set(1.0);
    }
    rack.set_transport(acid_app::device::Transport::Playing);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 48_000];
    mixer.render_mono(&mut out);
    let peak = out.iter().fold(0.0f32, |acc, sample| acc.max(sample.abs()));
    assert!(
        peak <= acid_core::limiter::CEILING + 1e-6,
        "the sum must not leave above the ceiling: {peak}"
    );
    assert!(peak > 0.1, "and it must not be silent either");
}

#[test]
fn a_steady_rack_rebuilds_no_machine() {
    let rack = rack_of(2);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 1_024];
    mixer.render_mono(&mut out);
    let built = mixer.rebuilds();
    assert_eq!(built, 2, "each device is built once");

    for _ in 0..16 {
        mixer.render_mono(&mut out);
    }
    assert_eq!(mixer.rebuilds(), built, "a steady rack reuses its engines");
}

#[test]
fn a_bus_of_the_same_length_with_a_different_device_is_not_treated_as_unchanged() {
    // `Rack` never swaps a device in place — `add` and `remove` both change the length — so this is the one bus that the
    // fast path must not mistake for "nothing changed": the same count, with one device that is not the one before. It is
    // the `Arc::ptr_eq` half of the fast path, and a mutation that drops it survives every other test here.
    let rack = rack_of(2);
    let published = rack.bus();
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, std::sync::Arc::clone(&published));
    let mut out = vec![0.0; 256];
    mixer.render_mono(&mut out);
    assert_eq!(mixer.rebuilds(), 2, "each device is built once");

    let kept = std::sync::Arc::clone(&rack.devices()[1].state);
    let routes = vec![InputFilter::everything(); 2];
    published.store(std::sync::Arc::new(BusState {
        slots: vec![SharedState::new(), kept],
        routes,
    }));
    mixer.render_mono(&mut out);

    assert_eq!(mixer.slots(), 2);
    assert_eq!(
        mixer.rebuilds(),
        3,
        "the swapped-in device needs its own engine; the one that stayed keeps the engine it had"
    );
}

#[test]
fn a_third_device_joins_without_disturbing_the_first_two() {
    let mut rack = rack_of(2);
    rack.set_transport(acid_app::device::Transport::Playing);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    let built = mixer.rebuilds();

    // `add` fans the transport out, so the new device is already playing; only its engine is new.
    rack.add(DeviceKind::Acid).expect("a third");
    mixer.render_mono(&mut out);

    assert_eq!(mixer.slots(), 3);
    assert_eq!(
        mixer.rebuilds(),
        built + 1,
        "only the new device is built; the two already playing are kept"
    );
}

#[test]
fn removing_a_device_stops_its_sound() {
    let mut rack = rack_of(2);
    rack.set_transport(acid_app::device::Transport::Playing);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    rack.remove(1).expect("a second of two may go");
    mixer.render_mono(&mut out);
    assert_eq!(mixer.slots(), 1);
}

fn position(rack: &Rack, index: usize) -> usize {
    rack.devices()[index]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed)
}

/// A device added mid-bar makes no sound and does not start until the rack's bar line, then begins on that sample in step
/// with everyone. Started where the others were, its clock would be a step out of phase until the bar wrapped — which is what
/// this test used to assert, before the player asked for the wait.
#[test]
fn a_device_added_mid_bar_waits_for_the_bar_and_lands_on_it() {
    let mut rack = rack_of(1);
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so a bar passes in a few blocks of 480
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    // 500-sample blocks against a 2 400-sample step, so a bar line falls *inside* a block rather than on its edge — the case
    // the delayed start exists for.
    let mut out = vec![0.0; 500];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }
    let at_join = position(&rack, 0);
    assert!(at_join > 1, "the reference should be mid-bar: {at_join}");

    rack.add(DeviceKind::Acid).expect("a second");

    let mut landed = None;
    // Two bars' worth of blocks: a sixteenth at 300 BPM is 2 400 samples, so a bar is 77 blocks of 500.
    for block in 0..160 {
        mixer.render_mono(&mut out);
        let reference = position(&rack, 0);
        let joined = position(&rack, 1);
        if landed.is_none() {
            // Waiting: the joiner's engine has never run, so its step is still the one it was built on.
            assert_eq!(
                joined, 0,
                "block {block}: the joiner started before the bar line"
            );
            if reference == 0 {
                landed = Some(block);
                assert_eq!(
                    joined, 0,
                    "block {block}: the joiner begins on the bar line"
                );
            }
            continue;
        }
        assert_eq!(
            joined, reference,
            "block {block}: the rack fell out of step after the join"
        );
    }
    let landed = landed.expect("the bar must turn within a bar's worth of blocks");
    assert!(
        landed > 1,
        "the joiner waited for the bar, not for one block: {landed}"
    );
}

/// Waiting is silence, not a device that is there and muted: the mix of a rack with a device waiting to join is the mix of
/// the same rack without it.
#[test]
fn a_device_waiting_for_the_bar_adds_nothing_to_the_mix() {
    let mut alone = rack_of(1);
    let mut rack = rack_of(1);
    for r in [&mut alone, &mut rack] {
        r.set_transport(acid_app::device::Transport::Playing);
        r.set_tempo_bpm(300.0);
    }
    let mut thin = Mixer::new(48_000.0, MAX_BLOCK, alone.bus());
    let mut full = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut thin_out = vec![0.0; 1_024];
    let mut full_out = vec![0.0; 1_024];
    // The premise, checked rather than assumed: the two racks are byte-identical while they are the same rack.
    for _ in 0..4 {
        thin.render_mono(&mut thin_out);
        full.render_mono(&mut full_out);
        assert_eq!(
            full_out, thin_out,
            "the two racks must be in lockstep before the add"
        );
    }

    rack.add(DeviceKind::Acid).expect("a second");
    thin.render_mono(&mut thin_out);
    full.render_mono(&mut full_out);
    assert_eq!(
        position(&rack, 1),
        0,
        "a waiting device has not started at all — this is a skip, not a muted note"
    );
    assert_eq!(
        full_out, thin_out,
        "a device waiting for the bar line must not be heard before it"
    );
}

/// A paused rack has no bar line to wait for, so a device that joins one still takes the step the resume will use — the rule
/// this mixer kept from the review that found a paused rack leaking a running device.
#[test]
fn a_device_added_to_a_paused_rack_joins_the_step_it_is_holding() {
    let mut rack = rack_of(1);
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.set_tempo_bpm(300.0);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 480];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }
    rack.set_transport(acid_app::device::Transport::Paused);
    mixer.render_mono(&mut out);
    let held = position(&rack, 0);
    assert!(
        held > 1,
        "the pause should keep a step that is not the bar's: {held}"
    );

    rack.add(DeviceKind::Acid).expect("a second");
    mixer.render_mono(&mut out);
    assert_eq!(
        position(&rack, 1),
        held,
        "the joiner takes the step the resume will use"
    );
}

/// A device that is still waiting when the rack stops and plays again is let in on the *new* downbeat: a start on step 0 is
/// a bar line, so the restart releases it rather than leaving it to wait a whole bar of the new run.
#[test]
fn a_waiting_device_joins_the_restart_on_the_downbeat() {
    let mut rack = rack_of(1);
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.set_tempo_bpm(300.0);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 500];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }
    rack.add(DeviceKind::Acid).expect("a second");
    mixer.render_mono(&mut out);
    assert_eq!(position(&rack, 1), 0, "it is still waiting");

    rack.set_transport(acid_app::device::Transport::Stopped);
    mixer.render_mono(&mut out);
    rack.set_transport(acid_app::device::Transport::Playing);
    mixer.render_mono(&mut out);

    assert_eq!(
        position(&rack, 1),
        position(&rack, 0),
        "the restart is a bar line: the waiting device must begin on it"
    );
}

/// A release that arrives while the device's own transport says stopped is not thrown away: the slot goes back to waiting and
/// joins on the next bar line. Dropped, the device would start at step 0 the moment its mirror played again, under a rack that
/// was mid-bar — the out-of-phase bug the whole waiting path exists to prevent.
#[test]
fn a_release_that_arrives_while_the_device_is_stopped_keeps_it_waiting() {
    let mut rack = rack_of(1);
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.set_tempo_bpm(300.0);
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 500];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }

    rack.add(DeviceKind::Acid).expect("a second");
    // The race, made deterministic: the device's own mirror says stopped while the rack plays on, so its release arrives for a
    // device that is not playing.
    rack.devices()[1]
        .state
        .set_transport(acid_app::device::Transport::Stopped);
    for _ in 0..160 {
        mixer.render_mono(&mut out);
    }
    assert_eq!(
        position(&rack, 1),
        0,
        "a stopped device cannot be let in, however many bar lines pass"
    );

    // Its mirror plays again. A dropped release would start it at once, mid-bar; a kept one holds it for the next bar line.
    rack.devices()[1]
        .state
        .set_transport(acid_app::device::Transport::Playing);
    let mut joined_on_the_bar = false;
    for block in 0..160 {
        mixer.render_mono(&mut out);
        let reference = position(&rack, 0);
        let joined = position(&rack, 1);
        if !joined_on_the_bar {
            if reference != 0 {
                assert_eq!(joined, 0, "block {block}: it started before the bar line");
                continue;
            }
            joined_on_the_bar = true;
            assert_eq!(joined, 0, "block {block}: it begins on the bar line");
            continue;
        }
        assert_eq!(
            joined, reference,
            "block {block}: the rack fell out of step after the join"
        );
    }
    assert!(joined_on_the_bar, "it must join, and on a bar line");
}

/// A drum device is a machine of its own, not the 303 wearing a grid: what the renderer produces for it is what
/// `acid_core::drums::Drums` produces for the same kit and the same pattern, sample for sample, and the mixer passes that on
/// unchanged. In a rack beside the 303 its bar is the rack's: it joins on the reference device's bar line like every other
/// device and then stays on the reference's step.
///
/// Sample for sample is the half that bites: a renderer still holding an `Engine` for every kind would answer with a bass
/// line, and a bass line is not this kit.
#[test]
fn a_drum_device_sounds_through_the_mixer() {
    // A strike on every step, so the shuffle's longer first step moves step 1's onset: one block of step 0 alone would be
    // the same samples with the swing and without it.
    let mut every_step = [Step::rest(); STEPS_PER_PATTERN];
    for step in &mut every_step {
        step.hit(Drum::Kick, false);
    }

    // The drum machine on its own — the mixer, a bare renderer built from the same state, and `Drums` itself, one block each.
    let mut solo = rack_with_drums(stub_kit(0.1, 0.3), every_step);
    solo.remove(0).expect("a rack of two may give one up");
    let state = Arc::clone(&solo.devices()[0].state);
    // Unity gain, so the renderer's master gain is a multiplication by one and the three blocks compare exactly.
    state.volume.set(1.0);
    // A transpose and a shuffle the device asked for: the machine the renderer owns has to hear each of them exactly as the
    // one built beside it does, or the two blocks would differ.
    state.set_transpose(5);
    state.swing.set(0.4);

    // Two steps' worth at 130 BPM, which is past the first step boundary: a swung bar's step 1 begins later than a straight
    // one's, so a machine that never heard the shuffle shows up inside this window.
    let mut bare = AudioRenderer::new(RATE, 1, Arc::clone(&state), MAX_BLOCK);
    let mut alone = vec![0.0; 8_000];
    bare.render_mono(&mut alone);

    let mut mixer = Mixer::new(RATE, MAX_BLOCK, solo.bus());
    let mut mixed = vec![0.0; 8_000];
    mixer.render_mono(&mut mixed);

    let mut machine = Drums::new(RATE, state.kit());
    machine.set_pattern(state.pattern());
    machine.set_mute_mask(state.drum_mute.load(Ordering::Relaxed) as u16);
    machine.set_transpose(state.transpose());
    machine.set_swing(state.swing.get());
    machine.start();
    let mut theirs = vec![0.0; 8_000];
    machine.render(&mut theirs);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let heard = peak(&theirs);
    assert!(heard > 0.05, "the stub kit must be audible at all: {heard}");
    for (at, (rendered, wanted)) in alone.iter().zip(&theirs).enumerate() {
        assert!(
            (rendered - wanted).abs() < 1e-6,
            "sample {at}: the renderer gave {rendered}, the drum machine {wanted}"
        );
    }
    for (at, (mixed, rendered)) in mixed.iter().zip(&alone).enumerate() {
        assert!(
            (mixed - rendered).abs() < 1e-6,
            "sample {at}: the mixer gave {mixed}, the device {rendered}"
        );
    }
    assert_eq!(
        state.position.load(Ordering::Relaxed),
        machine.position(),
        "the step the panel chases is the drum machine's"
    );

    // The same machine beside a 303, joining a rack that is already playing. It waits for the rack's bar line — silence, not a
    // device that is there and muted — and afterwards the two are on the same step: a drum bar and a bass bar begun on the
    // same frame are the same bar.
    //
    // The two racks are one 303 each, and the drum machine is added to one of them *after a bar has gone by*: added before the
    // mixer was built, it would be released on the first block's own downbeat (the reference's first step announces the bar
    // line), and neither the wait nor the join would be exercised at all.
    let mut shared = rack_of(1);
    let thin = rack_of(1);
    for rack in [&shared, &thin] {
        rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so a bar turns inside a few blocks of 500
        rack.set_transport(Transport::Playing);
    }
    let mut with_drums = Mixer::new(RATE, MAX_BLOCK, shared.bus());
    let mut bass_only = Mixer::new(RATE, MAX_BLOCK, thin.bus());
    let mut full = vec![0.0; 500];
    let mut bass = vec![0.0; 500];
    for _ in 0..20 {
        with_drums.render_mono(&mut full);
        bass_only.render_mono(&mut bass);
        assert_eq!(
            full, bass,
            "the two racks are the same 303 until the drum machine is added"
        );
    }
    assert!(
        position(&shared, 0) > 1,
        "the rack should be mid-bar before the drum machine joins: {}",
        position(&shared, 0)
    );
    let index = shared
        .add(DeviceKind::Drums)
        .expect("room for a drum machine");
    dress_drums(&shared, index, stub_kit(0.3, 0.3), kick_bar());

    let mut difference = 0.0f32;
    // The published step has to *move*: every equality below is satisfied by two devices stuck on zero, which is what a
    // renderer that never stores its machine's position would give.
    let mut furthest = 0usize;
    // How many blocks the drum machine was seen waiting: the branch below is worth nothing if it never runs, which is exactly
    // how the first version of this test passed without exercising the wait.
    let mut waited = 0usize;
    let mut joined = false;
    for block in 0..160 {
        with_drums.render_mono(&mut full);
        bass_only.render_mono(&mut bass);
        let drum = position(&shared, 1);
        let rack = position(&shared, 0);
        if drum == 0 && rack != 0 {
            assert_eq!(
                full, bass,
                "block {block}: a drum device waiting for the bar must not be heard"
            );
            waited += 1;
            continue;
        }
        joined = true;
        furthest = furthest.max(rack);
        assert_eq!(
            drum, rack,
            "block {block}: the drum machine's bar line is the rack's"
        );
        difference = difference.max(
            full.iter()
                .zip(&bass)
                .fold(0.0f32, |most, (a, b)| most.max((a - b).abs())),
        );
    }
    assert!(
        joined,
        "the drum device must join within a bar's worth of blocks"
    );
    assert!(
        waited > 0,
        "the drum machine joined a playing rack, so it had to wait for the bar first"
    );
    assert!(
        furthest > 0,
        "the published step must move: two devices stuck on 0 pass every equality above"
    );
    assert!(
        difference > 0.02,
        "a bar of the stub kit must be heard through the mixer: {difference}"
    );
}

/// Review Focus 5's second half: sixteen voices struck on one step, each clamped to ±1 and none of them limited, sum to about
/// sixteen. `Drums` clamps each voice and does not limit the sum on purpose — the spec puts the ceiling in the mixer's
/// limiter — so this asserts both halves: the sum leaves the device *above* the ceiling, and the mixer holds it there.
#[test]
fn all_sixteen_voices_struck_at_once_do_not_exceed_the_ceiling() {
    let ceiling = acid_core::limiter::CEILING;
    let rack = rack_with_drums(sustained(stub_kit(0.4, 1.0)), every_voice_bar());
    for device in rack.devices() {
        device.state.volume.set(1.0);
    }
    let state = Arc::clone(&rack.devices()[1].state);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));

    let mut bare = AudioRenderer::new(RATE, 1, Arc::clone(&state), MAX_BLOCK);
    let mut alone = vec![0.0; 4_096];
    bare.render_mono(&mut alone);
    let sum = peak(&alone);
    assert!(
        sum > ceiling,
        "sixteen voices at once must reach past the ceiling before it is applied: {sum}"
    );

    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut mixed = vec![0.0; 4_096];
    mixer.render_mono(&mut mixed);
    let held = peak(&mixed);
    assert!(
        held <= ceiling + 1e-6,
        "the mixer's ceiling must hold the sum: {held}"
    );
    assert!(held > 0.1, "and it must not be silence either: {held}");
}

/// Review Focus 5's first half: sixteen voices muted is silence with the steps still on. The mask silences the machine; it
/// does not edit the pattern, and it is read every block, so clearing it is heard at the next one.
#[test]
fn every_voice_muted_is_silence() {
    let mut rack = rack_with_drums(stub_kit(0.4, 0.6), every_voice_bar());
    rack.remove(0).expect("a rack of two may give one up");
    let state = Arc::clone(&rack.devices()[0].state);
    mute_every_voice(&state);
    let pattern = state.pattern();
    for drum in Drum::ALL {
        assert!(
            pattern.steps[0].is_hit(drum),
            "{drum:?} is still struck: a mute is not an edit"
        );
    }

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 1_024];
    // Past the attack and into the sample, or "silence" would only prove the block was too short to hear anything.
    let mut muted = 0.0f32;
    for _ in 0..8 {
        mixer.render_mono(&mut out);
        muted = muted.max(peak(&out));
    }
    assert!(
        muted < 1e-6,
        "every voice muted is silence with the steps still on: {muted}"
    );

    // The control, and the proof that the mask is read per block rather than once at open.
    state.drum_mute.store(0, Ordering::Relaxed);
    mixer.render_mono(&mut out);
    let audible = peak(&out);
    assert!(
        audible > 0.05,
        "the same rack with the mask cleared must be loud: {audible}"
    );
}

/// A kit swapped in while a voice is sounding is handed over on the audio thread; the block that is still playing keeps the
/// `Arc` it started with, and nothing panics.
///
/// A kit with no samples makes "the renderer took it" visible: the voice is mid-tail on a 0.4 s sample, and the block after
/// the swap is silent. A kit that never reached the machine would keep the old sample playing to its end.
#[test]
fn a_kit_swapped_while_a_voice_sounds_keeps_the_renderer_legal() {
    let mut rack = rack_with_drums(stub_kit(0.4, 0.6), kick_bar());
    rack.remove(0).expect("a rack of two may give one up");
    let state = Arc::clone(&rack.devices()[0].state);
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    mixer.render_mono(&mut out);
    let sounding = out.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    assert!(
        sounding > 0.05,
        "the hit must be sounding before the swap: {sounding}"
    );

    state.adopt_kit(default_kit());
    mixer.render_mono(&mut out);
    assert_eq!(
        state.kit().name,
        DEFAULT_KIT_NAME,
        "the panel's kit is what the state now holds"
    );
    let after = out.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    assert!(
        after < 1e-6,
        "the swapped-in kit must be what plays mid-tail: {after}"
    );
}

/// The `HIT` key's request reaches the sound, with no transport under it: the panel writes `request_audition`, the renderer
/// drains it once a block, and the voice is heard. Down a request is silence, and a polled request would still be sitting in
/// the slot after the block that served it.
#[test]
fn a_hit_is_heard_with_the_transport_stopped() {
    let mut rack = rack_with_drums(stub_kit(0.4, 0.6), kick_bar());
    rack.remove(0).expect("a rack of two may give one up");
    rack.set_transport(Transport::Stopped);
    let state = Arc::clone(&rack.devices()[0].state);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    mixer.render_mono(&mut out);
    assert_eq!(
        peak(&out),
        0.0,
        "a stopped transport with nothing asked for is silence"
    );

    state.request_audition(Drum::Kick);
    mixer.render_mono(&mut out);
    assert!(
        peak(&out) > 0.05,
        "HIT must be heard with the transport stopped: {}",
        peak(&out)
    );
    assert_eq!(
        state.take_audition(),
        None,
        "the renderer drains the request; it does not poll it"
    );
}

/// The paused half of the joining rule, for the second kind: a paused rack has no bar line to wait for, so a drum device that
/// joins one takes the step the resume will use — the same `seek` the 303 takes, because `Machine` is the only thing that
/// differs between the two.
#[test]
fn a_drum_device_added_to_a_paused_rack_joins_the_step_it_is_holding() {
    let mut rack = rack_of(1);
    rack.set_transport(Transport::Playing);
    rack.set_tempo_bpm(300.0);
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 480];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }
    rack.set_transport(Transport::Paused);
    mixer.render_mono(&mut out);
    let held = position(&rack, 0);
    assert!(
        held > 1,
        "the pause should keep a step that is not the bar's: {held}"
    );

    rack.add(DeviceKind::Drums).expect("a drum machine");
    mixer.render_mono(&mut out);
    assert_eq!(
        position(&rack, 1),
        held,
        "the drum machine takes the step the resume will use"
    );
}

/// Slot 0 is the reference the mixer releases every waiting device on, and a player can remove the 303 that started there — so
/// a drum machine has to answer with its own bar line, or the device that joins it never starts at all.
#[test]
fn a_303_joining_a_drum_machine_finds_its_bar_line() {
    let mut rack = rack_with_drums(stub_kit(0.3, 0.3), kick_bar());
    rack.remove(0).expect("a rack of two may give one up");
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so a bar turns inside a few blocks of 500
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 500];
    for _ in 0..20 {
        mixer.render_mono(&mut out);
    }
    assert!(
        position(&rack, 0) > 1,
        "the drum machine should be mid-bar: {}",
        position(&rack, 0)
    );

    rack.add(DeviceKind::Acid).expect("a 303");
    let mut joined = false;
    for block in 0..160 {
        mixer.render_mono(&mut out);
        let reference = position(&rack, 0);
        let joiner = position(&rack, 1);
        if !joined {
            if reference != 0 {
                assert_eq!(
                    joiner, 0,
                    "block {block}: the 303 started before the bar line"
                );
                continue;
            }
            joined = true;
            assert_eq!(joiner, 0, "block {block}: it begins on the bar line");
            continue;
        }
        assert_eq!(
            joiner, reference,
            "block {block}: the rack fell out of step after the join"
        );
    }
    assert!(joined, "the 303 must join on the drum machine's bar line");
}

/// The grid's edits reach the machine one block at a time: a cell clicked, a pattern switched, and the bar the drum device is
/// playing is the state's next `Arc`. `Machine::of` hands the pattern over at open, so this is the test for the *other* path —
/// the one a hand on the grid uses — and without it a drum device could draw a bar it never plays.
#[test]
fn a_pattern_published_to_a_playing_drum_device_is_heard() {
    let mut rack = rack_with_drums(stub_kit(0.4, 0.6), [Step::rest(); STEPS_PER_PATTERN]);
    rack.remove(0).expect("a rack of two may give one up");
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so a boundary falls inside six blocks of 512
    let state = Arc::clone(&rack.devices()[0].state);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    let mut empty = 0.0f32;
    for _ in 0..4 {
        mixer.render_mono(&mut out);
        empty = empty.max(peak(&out));
    }
    assert_eq!(empty, 0.0, "a bar with no hits on it is silence");

    // A bar struck on every step: within a step of the publish, the next boundary strikes one.
    let mut struck = [Step::rest(); STEPS_PER_PATTERN];
    for step in &mut struck {
        step.hit(Drum::Kick, false);
    }
    state.set_pattern(Arc::new(Pattern::from_steps(struck)));

    let mut heard = 0.0f32;
    for _ in 0..6 {
        mixer.render_mono(&mut out);
        heard = heard.max(peak(&out));
    }
    assert!(
        heard > 0.05,
        "the machine plays the pattern the panel published while it was running: {heard}"
    );
}

/// The 303's master VOLUME is drawn in the top strip of **every** face, the drum machine's included, so a drum device has to
/// hear it: the engine holds the 303's gain and a drum machine has no engine, so a drum block's gain is the renderer's.
#[test]
fn the_volume_knob_sets_a_drum_devices_level() {
    // A held envelope, so the level measured in a block is the gain the renderer applies and not a sample decaying out from
    // under it.
    let mut rack = rack_with_drums(sustained(stub_kit(0.4, 0.6)), kick_bar());
    rack.remove(0).expect("a rack of two may give one up");
    let state = Arc::clone(&rack.devices()[0].state);
    state.volume.set(0.0);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    mixer.render_mono(&mut out);
    assert_eq!(
        peak(&out),
        0.0,
        "volume 0 is a mute, so it is exact silence and not an asymptote"
    );

    // The knob comes up. 512 samples is 10.7 ms against a 10 ms time constant, so the block it lands in holds a gain still
    // climbing (about two thirds of the way) and the blocks after it hold the level the knob asked for: a knob that stepped
    // instead of ramping would show it here, and would click in the player's ears.
    state.volume.set(1.0);
    mixer.render_mono(&mut out);
    let rising = peak(&out);
    for _ in 0..3 {
        mixer.render_mono(&mut out);
    }
    let settled = peak(&out);
    assert!(
        settled > 0.05,
        "a drum device must hear the VOLUME knob: {settled}"
    );
    assert!(
        rising < 0.9 * settled,
        "the gain ramps over its 10 ms rather than stepping: {rising} against {settled}"
    );
    // And back to zero while the hit is still sounding: a mute is exact silence, not a decay towards it.
    state.volume.set(0.0);
    mixer.render_mono(&mut out);
    assert_eq!(
        peak(&out),
        0.0,
        "volume 0 mid-hit is exact silence, not an asymptote"
    );

    // And a device *opened* with the knob down starts at that level instead of ramping down to it: the same rack at half the
    // knob is a quarter as loud (`param::master_volume` is a square taper) from its very first block, and a gain seeded at 1.0
    // would show up as a first block four times too loud.
    let at = |knob: f32| {
        let mut rack = rack_with_drums(sustained(stub_kit(0.4, 0.6)), kick_bar());
        rack.remove(0).expect("a rack of two may give one up");
        rack.devices()[0].state.volume.set(knob);
        let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
        let mut out = vec![0.0; 512];
        mixer.render_mono(&mut out);
        peak(&out)
    };
    let full = at(1.0);
    let half = at(0.5);
    assert!(full > 0.05, "the control has to sound: {full}");
    assert!(
        (half / full - 0.25).abs() < 0.02,
        "a device opens at the level its knob is at: {half} against {full}"
    );
}

/// A key pressed on a drum device's channel sounds nothing this phase — the gate is the 303's, and a drum machine's voices
/// belong to the grid — while the same key on a 303 beside it sounds. Both devices are given the same gate and the same pitch,
/// so the silence is the drum machine's decision and not a message that went missing.
#[test]
fn a_note_on_a_drum_device_is_silent_and_on_a_303_is_not() {
    let mut drums = rack_with_drums(stub_kit(0.4, 0.6), [Step::rest(); STEPS_PER_PATTERN]);
    drums.remove(0).expect("a rack of two may give one up");
    // Both transports stopped, so the only sound either machine can make in this test is the key.
    drums.set_transport(Transport::Stopped);
    let drum = Arc::clone(&drums.devices()[0].state);
    drum.midi_hz.set(220.0);

    let bass = rack_of(1);
    bass.set_transport(Transport::Stopped);
    let acid = Arc::clone(&bass.devices()[0].state);
    acid.midi_hz.set(220.0);

    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    let mut drum_mixer = Mixer::new(RATE, MAX_BLOCK, drums.bus());
    let mut acid_mixer = Mixer::new(RATE, MAX_BLOCK, bass.bus());
    let mut out = vec![0.0; 1_024];

    drum.midi_gate.store(true, Ordering::Release);
    let mut heard = 0.0f32;
    for _ in 0..4 {
        drum_mixer.render_mono(&mut out);
        heard = heard.max(peak(&out));
    }
    assert_eq!(
        heard, 0.0,
        "a key on a drum device sounds nothing this phase: {heard}"
    );

    acid.midi_gate.store(true, Ordering::Release);
    for _ in 0..4 {
        acid_mixer.render_mono(&mut out);
    }
    let sounding = peak(&out);
    assert!(
        sounding > 0.01,
        "the same key on a 303 sounds — the machinery works: {sounding}"
    );
}

/// A drum device follows the rack's transport through every one of the three states, and the three are not the same call:
/// a pause keeps the step for the resume, a stop silences now and plays the bar again next time. `settle_transport` exists
/// because the machine cannot tell those apart, so this is the test that the drum arm of it is the drum machine's rule and not
/// a copy of a neighbour's.
#[test]
fn a_paused_drum_device_keeps_its_step_and_a_stopped_one_plays_the_bar_again() {
    let mut rack = rack_with_drums(stub_kit(0.4, 0.3), kick_bar());
    rack.remove(0).expect("a rack of two may give one up");
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so a 512-sample block is a fifth of a step
    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    let peak = |samples: &[f32]| samples.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    for _ in 0..8 {
        mixer.render_mono(&mut out);
    }
    assert!(
        position(&rack, 0) > 0,
        "the drum machine should be past the downbeat: {}",
        position(&rack, 0)
    );

    // Pause: the tail folds away inside its release, and the step it was on is kept. Measured over a whole bar, so a machine
    // that kept playing — and struck again on the next step it reached — is heard, rather than merely not heard in the block
    // the pause landed in.
    rack.set_transport(Transport::Paused);
    for _ in 0..3 {
        mixer.render_mono(&mut out);
    }
    let mut paused = 0.0f32;
    for _ in 0..80 {
        mixer.render_mono(&mut out);
        paused = paused.max(peak(&out));
    }
    let held = position(&rack, 0);
    assert!(
        paused < 1e-4,
        "a paused drum device is silent, not merely quiet: {paused}"
    );
    assert_eq!(position(&rack, 0), held, "a pause keeps the step it was on");

    // Resume: it carries on from that step rather than from the bar.
    rack.set_transport(Transport::Playing);
    mixer.render_mono(&mut out);
    assert_eq!(
        position(&rack, 0),
        held,
        "a resume carries on from the step the pause kept"
    );
    for _ in 0..4 {
        mixer.render_mono(&mut out);
    }
    assert_ne!(position(&rack, 0), held, "and then the bar goes on");

    // Stop: silent again — over a bar, as the pause was — and the next play is the bar: the same silence, a different step.
    rack.set_transport(Transport::Stopped);
    for _ in 0..3 {
        mixer.render_mono(&mut out);
    }
    let mut stopped = 0.0f32;
    for _ in 0..80 {
        mixer.render_mono(&mut out);
        stopped = stopped.max(peak(&out));
    }
    assert!(stopped < 1e-4, "a stopped drum device is silent: {stopped}");
    rack.set_transport(Transport::Playing);
    mixer.render_mono(&mut out);
    assert_eq!(
        position(&rack, 0),
        0,
        "a play out of a stop is the bar again"
    );
}

/// REVERSE is the rack's, and it has to reach a drum machine's clock: a hit written on step 15 is what step 0 sounds when the
/// line runs backwards. Asked for while the rack is stopped, the direction is taken at once — and then the first play is the
/// last step of the bar, which is why this is a sound and not a position.
#[test]
fn a_drum_device_plays_the_bar_backwards_when_the_rack_asks() {
    let mut steps = [Step::rest(); STEPS_PER_PATTERN];
    steps[15].hit(Drum::Kick, false);
    let mut rack = rack_with_drums(stub_kit(0.4, 0.6), steps);
    rack.remove(0).expect("a rack of two may give one up");
    let state = Arc::clone(&rack.devices()[0].state);
    rack.set_transport(Transport::Stopped);
    state.set_reversed(true);

    let mut mixer = Mixer::new(RATE, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 512];
    // A block with the rack stopped, so the direction reaches the machine before it starts: a running clock takes a
    // direction at the bar line, which is a bar away.
    mixer.render_mono(&mut out);
    assert_eq!(
        out.iter().fold(0.0f32, |most, s| most.max(s.abs())),
        0.0,
        "nothing sounds before the transport starts"
    );

    rack.set_transport(Transport::Playing);
    mixer.render_mono(&mut out);
    let heard = out.iter().fold(0.0f32, |most, s| most.max(s.abs()));
    assert!(
        heard > 0.05,
        "step 15 is the first step of a backwards bar: {heard}"
    );
}
