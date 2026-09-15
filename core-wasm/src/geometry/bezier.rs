use std::ops::{Add, Sub, Mul, Div};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 {
    pub t: f32, // Time coordinate (s)
    pub u: f32, // Log-frequency coordinate: u = log2(f / f_ref)
}

impl Add for Point2 {
    type Output = Self;
    #[inline]
    fn add(self, b: Self) -> Self {
        Self { t: self.t + b.t, u: self.u + b.u }
    }
}

impl Sub for Point2 {
    type Output = Self;
    #[inline]
    fn sub(self, b: Self) -> Self {
        Self { t: self.t - b.t, u: self.u - b.u }
    }
}

impl Mul<f32> for Point2 {
    type Output = Self;
    #[inline]
    fn mul(self, s: f32) -> Self {
        Self { t: self.t * s, u: self.u * s }
    }
}

impl Div<f32> for Point2 {
    type Output = Self;
    #[inline]
    fn div(self, s: f32) -> Self {
        Self { t: self.t / s, u: self.u / s }
    }
}

#[derive(Clone, Debug, PartialEq)]
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

        self.p0 * b0 + self.p1 * b1 + self.p2 * b2 + self.p3 * b3
    }

    /// Evaluates the first derivative of the cubic Bezier curve at parameter s in [0, 1].
    #[inline]
    pub fn d1(&self, s: f32) -> Point2 {
        let q = 1.0 - s;
        (self.p1 - self.p0) * (3.0 * q * q)
            + (self.p2 - self.p1) * (6.0 * q * s)
            + (self.p3 - self.p2) * (3.0 * s * s)
    }

    /// Evaluates the second derivative of the cubic Bezier curve at parameter s in [0, 1].
    #[inline]
    pub fn d2(&self, s: f32) -> Point2 {
        let q = 1.0 - s;
        let a = self.p2 - self.p1 * 2.0 + self.p0;
        let b = self.p3 - self.p2 * 2.0 + self.p1;
        a * (6.0 * q) + b * (6.0 * s)
    }
}

pub fn hermite_to_bezier(p0: Point2, p1: Point2, d0: Point2, d1: Point2, dt: f32) -> BezierCubic2 {
    BezierCubic2 {
        p0,
        p1: p0 + d0 * (dt / 3.0),
        p2: p1 - d1 * (dt / 3.0),
        p3: p1,
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
