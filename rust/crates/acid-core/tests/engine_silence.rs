use acid_core::analysis::{peak, rms};
use acid_core::engine::Engine;

#[test]
fn stopped_engine_renders_silence_at_every_rate_and_block_size() {
    for sr in [44_100.0, 48_000.0, 96_000.0] {
        let mut e = Engine::new(sr);
        for block in [1usize, 7, 64, 511, 4096] {
            let mut buf = vec![0.5f32; block];
            e.render(&mut buf);
            assert_eq!(peak(&buf), 0.0, "sr {sr}, block {block}");
            assert_eq!(rms(&buf), 0.0, "sr {sr}, block {block}");
        }
    }
}
