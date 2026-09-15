use vector_audio_geometry::multi_track::{initialize_paths, refine_paths, TrackingConfig};

fn gaussian_grid(frames: usize, channels: usize, centers: &[(f32, f32)]) -> Vec<f32> {
    let mut e = vec![0.0f32; frames * channels];
    for m in 0..frames {
        let t = m as f32 / (frames.saturating_sub(1).max(1) as f32);
        for &(u0, slope) in centers {
            let u = u0 + slope * (t - 0.5);
            let c = u * 12.0;
            for k in 0..channels {
                let x = (k as f32 - c) / 1.3;
                e[m * channels + k] += 10.0 * (-0.5 * x * x).exp();
            }
        }
    }
    e
}

#[test]
fn multiple_smooth_paths_stay_distinct() {
    let frames = 32;
    let channels = 120;
    let cfg = TrackingConfig {
        refinement_passes: 2,
        local_search_oct: 0.08,
        local_search_step_oct: 1.0 / 24.0,
        ..TrackingConfig::default()
    };

    let energy = gaussian_grid(
        frames,
        channels,
        &[(3.0, 0.10), (4.8, -0.12), (7.2, 0.06)],
    );

    let init = initialize_paths(
        &energy,
        channels,
        frames,
        3,
        &cfg,
    );
    let out = refine_paths(
        init,
        &energy,
        channels,
        frames,
        &cfg,
    );

    for m in 0..frames {
        for i in 0..out.len() {
            for j in (i + 1)..out.len() {
                assert!((out[i][m] - out[j][m]).abs() > 0.05);
            }
        }
    }
}
