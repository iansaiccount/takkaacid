pub mod analysis;
pub mod engine;
pub mod envelope;
pub mod filter;
pub mod oscillator;
pub mod param;
pub mod pattern;
pub mod pitch;
pub mod timing;
pub mod voice;

pub fn crate_name() -> &'static str {
    "acid-core"
}
