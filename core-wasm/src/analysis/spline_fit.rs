use crate::geometry::bezier::{Point2, BezierCubic2, hermite_to_bezier};
use crate::geometry::spline::Spline2;

/// Natural cubic spline interpolation of samples y(x), returned as cubic Bezier segments (Spline2).
/// The t coordinates must be strictly increasing.
pub fn fit_c2(points: &[Point2]) -> Spline2 {
    assert!(points.len() >= 2, "Spline fitting requires at least 2 points.");
    for w in points.windows(2) {
        assert!(w[1].t > w[0].t, "t coordinates must be strictly increasing.");
    }
    let n = points.len();
    if n == 2 {
        let d = (points[1] - points[0]) / (points[1].t - points[0].t);
        return Spline2 {
            segments: vec![hermite_to_bezier(points[0], points[1], d, d, points[1].t - points[0].t)]
        };
    }

    let mut h = vec![0.0f32; n - 1];
    for i in 0..n - 1 {
        h[i] = points[i + 1].t - points[i].t;
    }

    // Natural cubic spline second derivatives m[i].
    let mut a = vec![0.0f32; n];
    let mut b = vec![0.0f32; n];
    let mut c = vec![0.0f32; n];
    let mut r = vec![0.0f32; n];
    b[0] = 1.0;
    b[n - 1] = 1.0;
    for i in 1..n - 1 {
        a[i] = h[i - 1];
        b[i] = 2.0 * (h[i - 1] + h[i]);
        c[i] = h[i];
        r[i] = 6.0 * ((points[i + 1].u - points[i].u) / h[i] - (points[i].u - points[i - 1].u) / h[i - 1]);
    }

    // Thomas algorithm.
    for i in 1..n {
        let q = a[i] / b[i - 1];
        b[i] -= q * c[i - 1];
        r[i] -= q * r[i - 1];
    }
    let mut m = vec![0.0f32; n];
    m[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n - 1).rev() {
        m[i] = (r[i] - c[i] * m[i + 1]) / b[i];
    }

    let mut segs = Vec::with_capacity(n - 1);
    for i in 0..n - 1 {
        let hi = h[i];
        let d0 = (points[i + 1].u - points[i].u) / hi - hi * (2.0 * m[i] + m[i + 1]) / 6.0;
        let d1 = (points[i + 1].u - points[i].u) / hi + hi * (m[i] + 2.0 * m[i + 1]) / 6.0;
        let p0 = points[i];
        let p3 = points[i + 1];
        let bcurve = BezierCubic2 {
            p0,
            p1: Point2 {
                t: p0.t + hi / 3.0,
                u: p0.u + d0 * hi / 3.0,
            },
            p2: Point2 {
                t: p3.t - hi / 3.0,
                u: p3.u - d1 * hi / 3.0,
            },
            p3,
        };
        segs.push(bcurve);
    }
    Spline2 { segments: segs }
}

/// Uniformly resamples grid data down to target_points.
pub fn sample_uniform(grid_t: &[f32], grid_u: &[f32], target_points: usize) -> Vec<Point2> {
    assert_eq!(grid_t.len(), grid_u.len());
    assert!(target_points >= 2);
    if target_points == grid_t.len() {
        return grid_t.iter().zip(grid_u).map(|(&t, &u)| Point2 { t, u }).collect();
    }
    let mut out = Vec::with_capacity(target_points);
    for j in 0..target_points {
        let q = j as f32 * (grid_t.len() - 1) as f32 / (target_points - 1) as f32;
        let i = q.floor() as usize;
        let a = q - i as f32;
        if i + 1 >= grid_t.len() {
            out.push(Point2 { t: grid_t[i], u: grid_u[i] });
        } else {
            out.push(Point2 {
                t: grid_t[i] * (1.0 - a) + grid_t[i + 1] * a,
                u: grid_u[i] * (1.0 - a) + grid_u[i + 1] * a,
            });
        }
    }
    out
}

/// Fit a C2 spline to a track after uniform decimation.
pub fn fit_track_c2(raw: &[Point2], control_points: usize) -> Spline2 {
    let p = sample_uniform(
        &raw.iter().map(|x| x.t).collect::<Vec<_>>(),
        &raw.iter().map(|x| x.u).collect::<Vec<_>>(),
        control_points.max(2),
    );
    fit_c2(&p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c2_continuity() {
        let p: Vec<_> = (0..8)
            .map(|i| {
                let t = i as f32 / 7.0;
                Point2 {
                    t,
                    u: (2.0 * t).sin() * 0.2,
                }
            })
            .collect();
        let spline = fit_c2(&p);

        // Check interpolation accuracy at grid points
        for (seg, q) in spline.segments.iter().zip(p.windows(2)) {
            assert!((seg.eval(0.0).t - q[0].t).abs() < 1e-6);
            assert!((seg.eval(1.0).t - q[1].t).abs() < 1e-6);
            assert!((seg.eval(0.0).u - q[0].u).abs() < 1e-6);
            assert!((seg.eval(1.0).u - q[1].u).abs() < 1e-6);
        }

        // Validate C2 continuity numerically: check that B_j'(1) == B_j+1'(0) and B_j''(1) == B_j+1''(0)
        for i in 0..spline.segments.len() - 1 {
            let d1_left = spline.segments[i].d1(1.0);
            let d1_right = spline.segments[i + 1].d1(0.0);
            assert!((d1_left.t - d1_right.t).abs() < 1e-5, "t-derivative mismatch at index {}", i);
            assert!((d1_left.u - d1_right.u).abs() < 1e-5, "u-derivative mismatch at index {}", i);

            let d2_left = spline.segments[i].d2(1.0);
            let d2_right = spline.segments[i + 1].d2(0.0);
            assert!((d2_left.t - d2_right.t).abs() < 1e-5, "t second-derivative mismatch at index {}", i);
            assert!((d2_left.u - d2_right.u).abs() < 1e-5, "u second-derivative mismatch at index {}", i);
        }
    }
}
