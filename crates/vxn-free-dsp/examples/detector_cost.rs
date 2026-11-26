//! What the peak detector costs a frame, at each window, on a held chord
//! of four saws (C3, E3, G3, C4): the mean time of `detect`, best of five
//! runs of ten seconds. Run with `--release`.

use std::f64::consts::TAU;
use std::time::{Duration, Instant};

use vxn_free_dsp::detector::PeakDetector;
use vxn_free_dsp::stft::Stft;

const RATE: f32 = 48_000.0;

fn chord(seconds: f64) -> Vec<f32> {
    let n = (seconds * f64::from(RATE)) as usize;
    let mut x = vec![0.0f32; n];
    for root in [130.81, 164.81, 196.0, 261.63] {
        for k in 1..60 {
            let hz = root * f64::from(k);
            if hz >= 16_000.0 {
                break;
            }
            let w = TAU * hz / f64::from(RATE);
            let a = 0.08 / f64::from(k);
            for (i, v) in x.iter_mut().enumerate() {
                *v += (a * (w * i as f64).sin()) as f32;
            }
        }
    }
    x
}

fn main() {
    let x = chord(10.0);
    println!("window  ns a frame  peaks a frame");
    for window in [512usize, 1024, 2048] {
        let mut best = f64::MAX;
        let mut peaks = 0.0;
        for _ in 0..5 {
            let mut stft = Stft::new(2048, 4, window, window / 4);
            let mut d = PeakDetector::new(2048);
            d.set_window(window);
            d.set_threshold_db(6.0);
            let (mut spent, mut frames, mut found) = (Duration::ZERO, 0u32, 0usize);
            stft.push(&x, |f| {
                let t = Instant::now();
                d.detect(&f, RATE);
                spent += t.elapsed();
                frames += 1;
                found += d.peaks().len();
            });
            best = best.min(spent.as_nanos() as f64 / f64::from(frames));
            peaks = found as f64 / f64::from(frames);
        }
        println!("{window:6}  {best:10.0}  {peaks:13.1}");
    }
}
