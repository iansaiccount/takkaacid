//! The real-time promise: once the pattern is loaded, the audio path allocates nothing.
//!
//! This binary installs a counting global allocator and renders a thousand blocks with the counter
//! running. Any allocation in the callback — a `Vec`, a `String`, a boxed closure, a `Pattern` clone —
//! shows up as a non-zero count and fails the test.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use acid_app::audio::mixer::Mixer;
use acid_app::audio::renderer::AudioRenderer;
use acid_app::device::{DeviceKind, Rack};
use acid_app::persist;
use acid_app::state::SharedState;
use acid_core::drums::{Drum, Kit, default_kit};
use acid_core::pattern::{PATTERN_SLOTS, Pattern, Step};
use acid_core::pitch::Pitch;

/// The rate the renderers are built at, so a kit's samples are the rate the machine reads them at.
const RATE: f32 = 48_000.0;

/// A kit whose every voice is a short sine: sixteen sample voices, which is what a drum block costs.
///
/// Built by the test and not by the callback: a kit is a load, and the whole point of this test is that the *block* is not.
fn stub_kit(amplitude: f32) -> Kit {
    let mut kit = default_kit();
    kit.name = "Stub".to_string();
    let len = (RATE * 0.4) as usize;
    let samples: Vec<f32> = (0..len)
        .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / RATE).sin() * amplitude)
        .collect();
    let sample: Arc<[f32]> = Arc::from(samples);
    for drum in Drum::ALL {
        kit.voices[drum.index()].sample = Arc::clone(&sample);
    }
    kit
}

/// A bar with all sixteen voices struck on the downbeat and on the backbeat, so the window is not spent on silence.
fn busy_bar() -> [Step; PATTERN_SLOTS] {
    let mut steps = [Step::rest(); PATTERN_SLOTS];
    for step in [0, 8] {
        for drum in Drum::ALL {
            steps[step].hit(drum, false);
        }
    }
    steps
}

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
    s.set_transport(acid_app::device::Transport::Playing);
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

    // The mix path is the second real-time path, and it has one allocation in it: `reconcile` rebuilds the slot list when
    // the published rack changes. A rack that did not change must not — its fast path is what this window proves, and a
    // mutation that drops it allocates a `Vec` per block while `Mixer::rebuilds` still counts nothing.
    let mut rack = Rack::from_saved(&persist::SavedState::default());
    rack.add(DeviceKind::Acid)
        .expect("room for a second device");
    rack.set_transport(acid_app::device::Transport::Playing);
    rack.publish();
    let mut mixer = Mixer::new(48_000.0, 2, rack.bus());
    let mut mix = vec![0.0f32; 2_048];
    mixer.render_mono(&mut mix); // warm-up: the first block builds a slot per device
    let built = mixer.rebuilds();

    COUNT.store(0, Ordering::Relaxed);
    for _ in 0..1_000 {
        mixer.render_mono(&mut mix);
    }
    let mixed = COUNT.load(Ordering::Relaxed);
    assert_eq!(
        mixed, 0,
        "a steady rack allocated {mixed} times in the mix path"
    );
    assert_eq!(
        mixer.rebuilds(),
        built,
        "and it still reused its two machines while doing it"
    );

    // The second kind of machine on the same path. A drum slot renders sixteen sample voices and reads its kit, its mask and
    // any `HIT` waiting — all of it per block, none of it allocating.
    let index = rack
        .add(DeviceKind::Drums)
        .expect("room for a drum machine");
    let drum = Arc::clone(&rack.devices()[index].state);
    drum.adopt_kit(stub_kit(0.5));
    drum.set_pattern(Arc::new(Pattern::from_steps(busy_bar())));
    mixer.render_mono(&mut mix); // warm-up: builds the new slot and hands it the pattern
    let rebuilt = mixer.rebuilds();
    assert_eq!(
        rebuilt,
        built + 1,
        "the drum machine is one more machine, not a rebuilt rack"
    );

    COUNT.store(0, Ordering::Relaxed);
    for step in 0..1_000usize {
        // The panel's `HIT`, every block: the drain is on the audio path, so a request is written here and served there.
        drum.request_audition(Drum::ALL[step % Drum::ALL.len()]);
        mixer.render_mono(&mut mix);
    }
    let drummed = COUNT.load(Ordering::Relaxed);
    assert_eq!(
        drummed, 0,
        "a drum machine in the rack allocated {drummed} times in the mix path"
    );

    // A kit swapped while voices are sounding. The kit is built here, on this side of the counter, as the fixture `Arc`s
    // above are; what the window measures is the swap itself, which is one refcount and a pointer — not a decode.
    let swapped = Arc::new(stub_kit(0.2));
    drum.set_kit(Arc::clone(&swapped));
    COUNT.store(0, Ordering::Relaxed);
    for _ in 0..1_000 {
        mixer.render_mono(&mut mix);
    }
    let reswapped = COUNT.load(Ordering::Relaxed);
    assert_eq!(
        reswapped, 0,
        "a kit swapped mid-tail allocated {reswapped} times in the mix path"
    );
}
