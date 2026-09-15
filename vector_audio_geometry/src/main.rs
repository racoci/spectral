use rand::{rngs::StdRng, Rng, SeedableRng};
use std::time::Instant;
use vector_audio_geometry::multi_track::{track_multiple, TrackingConfig};
use vector_audio_geometry::reassignment::reassigned_grid;

fn random_track(rng: &mut impl Rng, frames: usize, lo_hz: f32, hi_hz: f32) -> Vec<f32> {
    let mut u = rng.gen_range(lo_hz.log2()..hi_hz.log2());
    let mut v = 0.0f32;
    let mut a = 0.0f32;
    let mut f = vec![0.0; frames];
    for m in 0..frames {
        if m > 0 {
            a = 0.985 * a + 0.015 * rng.gen_range(-1.0..1.0) * 4.0;
            v = (v + a * 0.005) * 0.997;
            u += v * 0.005;
            u = u.clamp(lo_hz.log2(), hi_hz.log2());
        }
        f[m] = 2.0f32.powf(u);
    }
    f
}

fn synthesize_tracks(
    rng: &mut impl Rng,
    fs: f32,
    seconds: f32,
    hop: usize,
) -> (Vec<f32>, Vec<Vec<f32>>) {
    let n = (fs * seconds) as usize;
    let frames = (n + hop - 1) / hop;
    let truth = vec![
        random_track(rng, frames, 60.0, 180.0),
        random_track(rng, frames, 300.0, 1200.0),
        random_track(rng, frames, 3500.0, 12000.0),
    ];

    let mut x = vec![0.0f32; n];
    let mut phases = vec![0.0f32; truth.len()];
    for i in 0..n {
        let m = (i / hop).min(frames - 1);
        for j in 0..truth.len() {
            let f = truth[j][m];
            phases[j] += std::f32::consts::TAU * f / fs;
            x[i] += (1.0 / truth.len() as f32) * phases[j].cos();
        }
        x[i] += 0.002 * rng.gen_range(-1.0..1.0);
    }
    (x, truth)
}

fn main() {
    const FS: f32 = 48_000.0;
    const HOP: usize = 256;
    const DUR: f32 = 0.8;

    let seed = rand::random::<u64>();
    let mut rng = StdRng::seed_from_u64(seed);
    let start = Instant::now();
    let cfg = TrackingConfig::default();

    println!("multi-track vector-audio demo; seed={seed}");

    for case_id in 1u64..=100 {
        let (x, truth) = synthesize_tracks(&mut rng, FS, DUR, HOP);
        let (_channels, grads) = reassigned_grid(
            &x, FS, HOP, 20.0, 20_000.0, 12, 2.0,
        );
        let result = track_multiple(&grads, truth.len(), DUR, &cfg);

        if case_id % 10 == 0 {
            println!(
                "case={case_id:03} tracks={} frames={} elapsed={:.1}s",
                result.tracks.len(),
                result.frames,
                start.elapsed().as_secs_f32()
            );
        }
    }
}
