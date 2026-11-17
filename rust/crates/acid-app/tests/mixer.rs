//! The mixer: every device rendered through one stream, summed, and held under one ceiling.

use acid_app::audio::MAX_BLOCK;
use acid_app::audio::mixer::Mixer;
use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{BusState, DeviceKind, Rack};
use acid_app::midi::route::InputFilter;
use acid_app::persist;
use acid_app::state::SharedState;

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
