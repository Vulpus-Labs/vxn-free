//! Nothing here may allocate once constructed: the engines call it on the
//! audio thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use vxn_free_dsp::{Crossover, DelayLine, SideBuffer};

struct Counting;

static COUNT: AtomicUsize = AtomicUsize::new(0);

// SAFETY: forwards to the system allocator, counting calls.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as the caller's contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: as the caller's contract.
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNT.fetch_add(1, Ordering::Relaxed);
        // SAFETY: as the caller's contract.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

#[test]
fn delay_lines_side_buffers_and_crossovers_never_allocate() {
    let mut line = DelayLine::new(4800);
    let mut side = SideBuffer::new(4800, 400);
    let mut xover = Crossover::new(150.0, 48_000.0);
    let before = COUNT.load(Ordering::Relaxed);
    let mut acc = 0.0f32;
    let mut d = 400.0f32;
    for n in 0..100_000 {
        let x = (n as f32 * 0.01).sin();
        let (lo, hi) = xover.split(x);
        line.push(hi);
        side.push(1.0);
        if n % 64 == 0 {
            side.set_past(300, 0.5);
            xover.set_frequency(100.0 + (n % 1000) as f32);
        }
        acc += lo + line.read(d) * side.read(d);
        d = if d > 4000.0 { 400.0 } else { d + 0.017 };
    }
    line.clear();
    side.fill(1.0);
    xover.reset();
    std::hint::black_box(acc);
    assert_eq!(COUNT.load(Ordering::Relaxed), before);
}
