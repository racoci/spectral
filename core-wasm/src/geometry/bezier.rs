#[derive(Clone, Copy, Debug)]
pub struct Point2 {
    pub t: f32, // Time coordinate (s)
    pub u: f32, // Log-frequency coordinate: u = log2(f / f_ref)
}

#[derive(Clone, Debug)]
pub struct BezierCubic2 {
    pub p0: Point2,
    pub p1: Point2,
    pub p2: Point2,
    pub p3: Point2,
}

impl BezierCubic2 {
    /// Evaluates the cubic Bezier curve at parameter s in [0, 1].
    #[inline]
    pub fn eval(&self, s: f32) -> Point2 {
        let q = 1.0 - s;
        let b0 = q * q * q;
        let b1 = 3.0 * q * q * s;
        let b2 = 3.0 * q * s * s;
        let b3 = s * s * s;

        Point2 {
            t: b0 * self.p0.t + b1 * self.p1.t + b2 * self.p2.t + b3 * self.p3.t,
            u: b0 * self.p0.u + b1 * self.p1.u + b2 * self.p2.u + b3 * self.p3.u,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ScalarBezier {
    pub p0: f32,
    pub p1: f32,
    pub p2: f32,
    pub p3: f32,
}

impl ScalarBezier {
    /// Evaluates the scalar cubic Bezier curve at parameter s in [0, 1].
    #[inline]
    pub fn eval(&self, s: f32) -> f32 {
        let q = 1.0 - s;
        let b0 = q * q * q;
        let b1 = 3.0 * q * q * s;
        let b2 = 3.0 * q * s * s;
        let b3 = s * s * s;
        
        b0 * self.p0 + b1 * self.p1 + b2 * self.p2 + b3 * self.p3
    }
}
