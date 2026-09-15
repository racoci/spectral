use crate::geometry::bezier::Point2;

/// A lightweight representation of a reassigned time-frequency coefficient for tracking.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TrackBin {
    /// Reassigned frequency in Hz.
    pub freq_hz: f32,
    /// Reassigned time in seconds.
    pub time_s: f32,
    /// Energy (squared magnitude) of the coefficient.
    pub energy: f32,
}

/// Tracks a trajectory through reassigned coefficients by locating the strongest channel
/// in each frame and computing an energy-weighted centroid of log-frequency log2(f / 20.0)
/// in a small neighborhood around the peak channel (e.g., j-1 to j+1).
///
/// `bins` is a 2D matrix of shape [channels][frames].
/// `radius` defines the neighborhood size around the peak channel (e.g., 1 for j-1 to j+1).
pub fn track_centroid(bins: &[Vec<TrackBin>], radius: usize) -> Vec<Point2> {
    assert!(!bins.is_empty(), "Tracker requires at least one channel.");
    let channels = bins.len();
    let frames = bins[0].len();
    let mut out = Vec::with_capacity(frames);

    for m in 0..frames {
        let mut best_j = 0;
        let mut max_energy = -1.0f32;

        for j in 0..channels {
            let energy = bins[j][m].energy;
            if energy > max_energy {
                max_energy = energy;
                best_j = j;
            }
        }

        let best_bin = bins[best_j][m];
        let center_u = if best_bin.freq_hz > 0.0 {
            (best_bin.freq_hz / 20.0).max(1e-9).log2()
        } else {
            0.0
        };

        let mut sum_w = 0.0f32;
        let mut sum_wu = 0.0f32;

        let start_j = best_j.saturating_sub(radius);
        let end_j = (best_j + radius).min(channels - 1);

        for j in start_j..=end_j {
            let b = bins[j][m];
            if b.freq_hz > 0.0 && b.freq_hz.is_finite() {
                let u = (b.freq_hz / 20.0).max(1e-9).log2();
                let w = b.energy;
                sum_w += w;
                sum_wu += w * u;
            }
        }

        let u = if sum_w > 0.0 {
            sum_wu / sum_w
        } else {
            center_u
        };

        let t = best_bin.time_s;
        out.push(Point2 { t, u });
    }
    out
}

/// Decimates a dense sequence of tracked points down to a specific target count of
/// evenly spaced control points.
pub fn subsample_points(points: &[Point2], target_count: usize) -> Vec<Point2> {
    if points.len() <= target_count {
        return points.to_vec();
    }
    let grid_t: Vec<f32> = points.iter().map(|p| p.t).collect();
    let grid_u: Vec<f32> = points.iter().map(|p| p.u).collect();
    crate::analysis::spline_fit::sample_uniform(&grid_t, &grid_u, target_count)
}
