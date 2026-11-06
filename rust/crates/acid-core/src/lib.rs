pub mod analysis;
pub mod bank;
pub mod clock;
pub mod drive;
pub mod engine;
pub mod envelope;
pub mod factory;
pub mod filter;
pub mod oscillator;
pub mod param;
pub mod pattern;
pub mod pitch;
pub mod sequencer;
pub mod timing;
pub mod voice;

pub fn crate_name() -> &'static str {
    "acid-core"
}
