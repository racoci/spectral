use std::ops::{Add, Sub, Mul, Div};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point2 { pub t: f32, pub u: f32 }

impl Add for Point2 { type Output = Self; fn add(self, b: Self) -> Self { Self { t: self.t+b.t, u:self.u+b.u } } }
impl Sub for Point2 { type Output = Self; fn sub(self, b: Self) -> Self { Self { t: self.t-b.t, u:self.u-b.u } } }
impl Mul<f32> for Point2 { type Output = Self; fn mul(self, s:f32) -> Self { Self { t:self.t*s, u:self.u*s } } }
impl Div<f32> for Point2 { type Output = Self; fn div(self, s:f32) -> Self { Self { t:self.t/s, u:self.u/s } } }

#[derive(Clone, Copy, Debug)]
pub struct BezierCubic2 { pub p0:Point2, pub p1:Point2, pub p2:Point2, pub p3:Point2 }

impl BezierCubic2 {
    #[inline] pub fn eval(&self, s:f32) -> Point2 {
        let q=1.0-s;
        let b0=q*q*q; let b1=3.0*q*q*s; let b2=3.0*q*s*s; let b3=s*s*s;
        self.p0*b0 + self.p1*b1 + self.p2*b2 + self.p3*b3
    }
    #[inline] pub fn d1(&self, s:f32) -> Point2 {
        let q=1.0-s;
        (self.p1-self.p0)*(3.0*q*q)
            +(self.p2-self.p1)*(6.0*q*s)
            +(self.p3-self.p2)*(3.0*s*s)
    }
    #[inline] pub fn d2(&self, s:f32) -> Point2 {
        let q=1.0-s;
        let a=self.p2-self.p1*2.0+self.p0;
        let b=self.p3-self.p2*2.0+self.p1;
        a*(6.0*q)+b*(6.0*s)
    }
}

#[derive(Clone, Debug)]
pub struct Spline2 { pub segments: Vec<BezierCubic2> }

impl Spline2 {
    pub fn eval(&self, t:f32) -> Point2 {
        assert!(!self.segments.is_empty());
        if self.segments.len()==1 { return self.segments[0].eval(t.clamp(0.0,1.0)); }
        let n=self.segments.len() as f32;
        let x=(t.clamp(0.0,1.0)*n).min(n-1e-6);
        let i=x.floor() as usize;
        self.segments[i].eval(x-i as f32)
    }
    pub fn start(&self)->Point2 { self.segments.first().unwrap().p0 }
    pub fn end(&self)->Point2 { self.segments.last().unwrap().p3 }
}

#[derive(Clone, Debug)]
pub struct ScalarBezier { pub p0:f32,pub p1:f32,pub p2:f32,pub p3:f32 }
impl ScalarBezier {
    pub fn eval(&self,s:f32)->f32 { let q=1.0-s; q*q*q*self.p0+3.0*q*q*s*self.p1+3.0*q*s*s*self.p2+s*s*s*self.p3 }
}

pub fn hermite_to_bezier(p0:Point2,p1:Point2,d0:Point2,d1:Point2,dt:f32)->BezierCubic2 {
    BezierCubic2 {
        p0,
        p1: p0 + d0*(dt/3.0),
        p2: p1 - d1*(dt/3.0),
        p3: p1,
    }
}
