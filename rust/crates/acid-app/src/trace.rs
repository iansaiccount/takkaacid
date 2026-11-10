//! Opt-in input tracing: `TAKKA_TRACE_INPUT=1` prints what MIDI delivers and what the panel places, to stderr.
//!
//! This exists because two reports ("one press writes two steps", "the stored pitch is always C1") could not be
//! reproduced headless: a test drives one event per frame, and a real device may deliver more. When the flag is unset
//! nothing is formatted and nothing is printed — the closures are only called when the trace is on.

use std::sync::OnceLock;

/// Whether tracing is on. Read once, so the check costs a load.
fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| {
        std::env::var_os(TRACE_ENV).is_some_and(|value| !value.is_empty() && value != "0")
    })
}

/// The variable that turns tracing on.
pub const TRACE_ENV: &str = "TAKKA_TRACE_INPUT";

/// Print one traced line, if tracing is on.
///
/// The message is a closure, so a traced-out run pays nothing for the formatting.
pub fn note(source: &str, message: impl FnOnce() -> String) {
    if enabled() {
        eprintln!("[{TRACE_ENV}] {source}: {}", message());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_trace_prints_nothing_unless_it_is_asked_to() {
        // Whoever runs the tests may have the flag set; the invariant under test is only that a disabled trace never
        // builds its message, so the closure must not run.
        let mut built = false;
        if !enabled() {
            note("test", || {
                built = true;
                "should not be built".to_string()
            });
            assert!(!built, "a disabled trace must not format its message");
        }
    }
}
