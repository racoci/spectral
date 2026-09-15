use super::bezier::{BezierCubic2, ScalarBezier, Point2};

#[derive(Clone, Debug, PartialEq)]
pub struct Spline2 {
    /// A sequence of contiguous cubic Bezier segments designed to satisfy C2 continuity.
    pub segments: Vec<BezierCubic2>,
}

impl Spline2 {
    /// Evaluates the spline at a normalized parameter t in [0, 1].
    pub fn eval(&self, t: f32) -> Point2 {
        assert!(!self.segments.is_empty());
        if self.segments.len() == 1 {
            return self.segments[0].eval(t.clamp(0.0, 1.0));
        }
        let n = self.segments.len() as f32;
        let x = (t.clamp(0.0, 1.0) * n).min(n - 1e-6);
        let i = x.floor() as usize;
        self.segments[i].eval(x - i as f32)
    }

    /// Returns the start point of the spline.
    pub fn start(&self) -> Point2 {
        self.segments.first().unwrap().p0
    }

    /// Returns the end point of the spline.
    pub fn end(&self) -> Point2 {
        self.segments.last().unwrap().p3
    }
}

#[derive(Clone, Debug)]
pub struct ScalarSpline {
    /// A sequence of contiguous scalar cubic Bezier segments (e.g. for amplitude or relative freq tracking).
    pub segments: Vec<ScalarBezier>,
}
