use super::bezier::{BezierCubic2, ScalarBezier};

#[derive(Clone, Debug)]
pub struct Spline2 {
    /// A sequence of contiguous cubic Bezier segments designed to satisfy C2 continuity.
    pub segments: Vec<BezierCubic2>,
}

#[derive(Clone, Debug)]
pub struct ScalarSpline {
    /// A sequence of contiguous scalar cubic Bezier segments (e.g. for amplitude or relative freq tracking).
    pub segments: Vec<ScalarBezier>,
}
