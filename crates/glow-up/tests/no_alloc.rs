//! The engine never allocates once built: it runs on the audio thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use glow_up::engine::GlowUpEngine;
use glow_up::{A_MULT, B_DIV, FOCUS, GRID, MODE, RESONANCE, THRESHOLD, WIDTH};
use silkscreen::bridge::Engine;

struct Counting;

static COUNT: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Whether this thread's allocations are counted: tests run on
    /// several threads, and only the one under test should count.
    static WATCHED: Cell<bool> = const { Cell::new(false) };
}

fn count() {
    if WATCHED.with(Cell::get) {
        COUNT.fetch_add(1, Ordering::Relaxed);
    }
}

/// Allocations `f` makes on this thread.
fn allocations(f: impl FnOnce()) -> usize {
    let before = COUNT.load(Ordering::Relaxed);
    WATCHED.with(|w| w.set(true));
    f();
    WATCHED.with(|w| w.set(false));
    COUNT.load(Ordering::Relaxed) - before
}

// SAFETY: forwards to the system allocator, counting calls.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: as the caller's contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as the caller's contract.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: as the caller's contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn processing_and_every_setting_never_allocate() {
    let mut engine = GlowUpEngine::new(48_000.0);
    engine.set_param(RESONANCE, 100.0);
    engine.reset();
    let x: Vec<f32> = (0..48_000)
        .map(|n| {
            let t = n as f32 / 48_000.0;
            0.3 * (std::f32::consts::TAU * (220.0 + 60.0 * t) * t).sin()
        })
        .collect();
    let mut l = x.clone();
    let mut r = x.clone();
    let mut words = 0;
    let n = allocations(|| {
        for pass in 0..6u8 {
            let p = f32::from(pass);
            engine.set_param(GRID, (p + 1.0) % 3.0);
            engine.set_param(MODE, p % 2.0);
            engine.set_param(FOCUS, 20.0 * p);
            engine.set_param(A_MULT, p);
            engine.set_param(B_DIV, 7.0 - p);
            engine.set_param(WIDTH, 10.0 + 30.0 * p);
            engine.set_param(THRESHOLD, p);
            l.copy_from_slice(&x);
            r.copy_from_slice(&x);
            for (a, b) in l.chunks_mut(173).zip(r.chunks_mut(173)) {
                engine.process(&mut [a, b]);
                engine.frame(&mut |f| words += f.len());
            }
        }
        engine.reset();
    });
    std::hint::black_box(&l);
    assert!(words > 0);
    assert_eq!(n, 0);
}
