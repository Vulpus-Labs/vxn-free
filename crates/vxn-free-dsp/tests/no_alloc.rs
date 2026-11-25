//! Nothing here may allocate once constructed: the engines call it on the
//! audio thread.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use vxn_free_dsp::{Crossover, DelayLine, Peak, SideBuffer, Stft, ThreeBand, peaks};

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
fn delay_lines_side_buffers_and_crossovers_never_allocate() {
    let mut line = DelayLine::new(4800);
    let mut side = SideBuffer::new(4800, 400);
    let mut xover = Crossover::new(150.0, 48_000.0);
    let mut acc = 0.0f32;
    let mut d = 400.0f32;
    let n = allocations(|| {
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
    });
    std::hint::black_box(acc);
    assert_eq!(n, 0);
}

#[test]
fn a_three_band_split_never_allocates() {
    let mut split = ThreeBand::new(200.0, 2500.0, 48_000.0);
    let mut acc = 0.0f32;
    let n = allocations(|| {
        for n in 0..100_000 {
            if n % 32 == 0 {
                split.set_frequencies(100.0 + (n % 900) as f32, 2000.0 + (n % 5000) as f32);
            }
            acc += split.split((n as f32 * 0.01).sin()).iter().sum::<f32>();
        }
        split.reset();
    });
    std::hint::black_box(acc);
    assert_eq!(n, 0);
}

#[test]
fn the_stft_and_peak_picking_never_allocate() {
    let mut stft = Stft::new(2048, 8, 256, 64);
    let mut found = [Peak::default(); 8];
    let x: Vec<f32> = (0..4096).map(|n| (n as f32 * 0.07).sin()).collect();
    let mut total = 0;
    let n = allocations(|| {
        for (window, hop) in [(256, 64), (2048, 256), (128, 128)] {
            stft.configure(window, hop);
            for chunk in x.chunks(37) {
                stft.push(chunk, |f| total += peaks(&f, 48_000.0, 10.0, &mut found));
            }
            if let Some(f) = stft.latest() {
                total += peaks(&f, 48_000.0, 10.0, &mut found);
            }
        }
    });
    std::hint::black_box(total);
    assert_eq!(n, 0);
}
