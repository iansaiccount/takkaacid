//! The mixer: every device rendered through one stream, summed, and held under one ceiling.

use acid_app::audio::MAX_BLOCK;
use acid_app::audio::mixer::Mixer;
use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{BusState, DeviceKind, Rack};
use acid_app::midi::route::InputFilter;
use acid_app::persist;
use acid_app::state::SharedState;
use acid_core::pattern::STEPS_PER_PATTERN as STEPS;

fn rack_of(n: usize) -> Rack {
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    for _ in 1..n {
        rack.add(DeviceKind::Acid).expect("room");
    }
    rack.publish();
    rack
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
fn a_steady_rack_rebuilds_no_engine() {
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

#[test]
fn a_device_added_mid_run_comes_in_on_the_bar_the_others_are_on() {
    let mut rack = rack_of(1);
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.set_tempo_bpm(300.0); // a sixteenth every 2_400 samples, so steps pass quickly
    let mut mixer = Mixer::new(48_000.0, MAX_BLOCK, rack.bus());
    let mut out = vec![0.0; 4_096];
    for _ in 0..16 {
        mixer.render_mono(&mut out);
    }
    let reference = rack.devices()[0]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        reference > 1,
        "the reference device should have moved on: {reference}"
    );

    // `add` fans the rack's transport and tempo out, so the join is the only thing under test here.
    rack.add(DeviceKind::Acid).expect("a second");
    mixer.render_mono(&mut out);

    let joined = rack.devices()[1]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    let reference_now = rack.devices()[0]
        .state
        .position
        .load(std::sync::atomic::Ordering::Relaxed);
    // The seek aligns the *step*, not the sample inside it: the joining device's clock restarted, so it may cross into the
    // next step a few samples apart from the reference. One step of slack is the honest assertion.
    let ahead = (joined + STEPS - reference_now) % STEPS;
    assert!(
        ahead <= 1 || ahead == STEPS - 1,
        "a joining device must land within a step of the rack: joined {joined}, reference {reference_now}"
    );
}
