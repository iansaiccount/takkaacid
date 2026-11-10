//! The real-time promise: once the pattern is loaded, the audio path allocates nothing.
//!
//! This binary installs a counting global allocator and renders a thousand blocks with the counter
//! running. Any allocation in the callback — a `Vec`, a `String`, a boxed closure, a `Pattern` clone —
//! shows up as a non-zero count and fails the test.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use acid_app::audio::renderer::AudioRenderer;
use acid_app::state::SharedState;
use acid_core::pattern::{Pattern, Step};
use acid_core::pitch::Pitch;

struct Counting;
static COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(l) }
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}

#[global_allocator]
static A: Counting = Counting;

/// Both phases run inside one test on purpose: `COUNT` is global, and two tests in one binary run in
/// parallel by default, so a second test's own setup would land in this test's measurement window.
#[test]
fn the_audio_path_never_allocates() {
    let s = SharedState::new();
    s.playing.store(true, Ordering::Relaxed);
    s.set_pattern(std::sync::Arc::new(Pattern::from_steps(
        [Step::note(Pitch::A, -1); 16],
    )));
    let mut r = AudioRenderer::new(48_000.0, 2, s.clone(), 1_024);
    let mut out = vec![0.0f32; 2_048];
    r.render_into(&mut out); // warm-up: the first block may swap in the pattern Arc

    COUNT.store(0, Ordering::Relaxed);
    for _ in 0..1_000 {
        r.render_into(&mut out);
    }
    let steady = COUNT.load(Ordering::Relaxed);
    assert_eq!(steady, 0, "steady-state rendering allocated {steady} times");

    // A knob sweep must not allocate either: the callback reads atomics, it does not queue messages.
    COUNT.store(0, Ordering::Relaxed);
    for step in 0..1_000usize {
        let knob = (step % 100) as f32 / 100.0;
        s.cutoff.set(knob);
        s.resonance.set(1.0 - knob);
        s.tempo_bpm.set(120.0 + (step % 40) as f32);
        r.render_into(&mut out);
    }
    let swept = COUNT.load(Ordering::Relaxed);
    assert_eq!(
        swept, 0,
        "a knob sweep allocated {swept} times in the audio path"
    );

    // A pattern switch is the one allowed exception, and even then the cost belongs to the UI. The fixture `Arc` is built
    // *before* the counter resets, so this window measures only what the audio thread does — otherwise the fixture's own
    // allocation sat inside it and a real audio-path allocation went unnoticed.
    let switched = std::sync::Arc::new(Pattern::from_steps([Step::note(Pitch::C, 0); 16]));
    COUNT.store(0, Ordering::Relaxed);
    s.set_pattern(switched);
    r.render_into(&mut out);
    let swap = COUNT.load(Ordering::Relaxed);
    assert!(
        swap <= 1,
        "a pattern swap allocated {swap} times in the audio path"
    );
}
