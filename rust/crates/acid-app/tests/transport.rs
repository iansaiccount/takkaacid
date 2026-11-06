use acid_core::engine::Engine;

#[test]
fn stopping_silences_the_engine_within_twenty_ms() {
    let mut e = Engine::new(48_000.0);
    e.start();
    let mut warm = vec![0.0; 9_600];
    e.render(&mut warm);
    assert!(acid_core::analysis::rms(&warm) > 0.02);

    e.stop();
    let mut tail = vec![0.0; 1_920]; // 40 ms
    e.render(&mut tail);
    let last_20ms = &tail[960..];
    assert!(acid_core::analysis::rms(last_20ms) < 0.001);
    assert!(!e.is_playing());
}

#[test]
fn shared_state_is_send_and_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<acid_app::state::SharedState>();
}

#[test]
fn atomic_f32_round_trips_negative_and_fractional_values() {
    let a = acid_app::state::AtomicF32::new(0.0);
    for v in [-1.0, -0.25, 0.0, 0.333, 1.0] {
        a.set(v);
        assert!((a.get() - v).abs() < 1e-6);
    }
}
