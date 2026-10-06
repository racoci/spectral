//! Estruturas de Números Complexos de Alta Performance
//!
//! Implementação com operações estritamente `#[inline(always)]` para garantir
//! ausência total de sobrecarga de abstração (zero-cost abstraction) no compilador LLVM.

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ComplexNumber32 {
    pub real_part: f32,
    pub imaginary_part: f32,
}

impl ComplexNumber32 {
    #[inline(always)]
    pub const fn new(real_part: f32, imaginary_part: f32) -> Self {
        Self { real_part, imaginary_part }
    }

    #[inline(always)]
    pub const fn zero() -> Self {
        Self { real_part: 0.0, imaginary_part: 0.0 }
    }

    #[inline(always)]
    pub fn conjugate(self) -> Self {
        Self::new(self.real_part, -self.imaginary_part)
    }

    #[inline(always)]
    pub fn norm_squared(self) -> f32 {
        self.real_part * self.real_part + self.imaginary_part * self.imaginary_part
    }

    #[inline(always)]
    pub fn absolute_value(self) -> f32 {
        self.norm_squared().sqrt()
    }

    #[inline(always)]
    pub fn phase_angle(self) -> f32 {
        self.imaginary_part.atan2(self.real_part)
    }

    #[inline(always)]
    pub fn exponential(self) -> Self {
        let r = self.real_part.exp();
        Self::new(r * self.imaginary_part.cos(), r * self.imaginary_part.sin())
    }
}

impl std::ops::Add for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.real_part + rhs.real_part, self.imaginary_part + rhs.imaginary_part)
    }
}

impl std::ops::Sub for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.real_part - rhs.real_part, self.imaginary_part - rhs.imaginary_part)
    }
}

impl std::ops::Mul for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.real_part * rhs.real_part - self.imaginary_part * rhs.imaginary_part,
            self.real_part * rhs.imaginary_part + self.imaginary_part * rhs.real_part,
        )
    }
}

impl std::ops::Mul<f32> for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.real_part * rhs, self.imaginary_part * rhs)
    }
}

impl std::ops::Div for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        let den = rhs.norm_squared();
        Self::new(
            (self.real_part * rhs.real_part + self.imaginary_part * rhs.imaginary_part) / den,
            (self.imaginary_part * rhs.real_part - self.real_part * rhs.imaginary_part) / den,
        )
    }
}

impl std::ops::Div<f32> for ComplexNumber32 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f32) -> Self {
        Self::new(self.real_part / rhs, self.imaginary_part / rhs)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct ComplexNumber64 {
    pub real_part: f64,
    pub imaginary_part: f64,
}

impl ComplexNumber64 {
    #[inline(always)]
    pub const fn new(real_part: f64, imaginary_part: f64) -> Self {
        Self { real_part, imaginary_part }
    }

    #[inline(always)]
    pub fn conjugate(self) -> Self {
        Self::new(self.real_part, -self.imaginary_part)
    }

    #[inline(always)]
    pub fn norm_squared(self) -> f64 {
        self.real_part * self.real_part + self.imaginary_part * self.imaginary_part
    }

    #[inline(always)]
    pub fn absolute_value(self) -> f64 {
        self.norm_squared().sqrt()
    }

    #[inline(always)]
    pub fn phase_angle(self) -> f64 {
        self.imaginary_part.atan2(self.real_part)
    }

    #[inline(always)]
    pub fn exponential(self) -> Self {
        let r = self.real_part.exp();
        Self::new(r * self.imaginary_part.cos(), r * self.imaginary_part.sin())
    }
}

impl std::ops::Add for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Self) -> Self {
        Self::new(self.real_part + rhs.real_part, self.imaginary_part + rhs.imaginary_part)
    }
}

impl std::ops::Sub for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.real_part - rhs.real_part, self.imaginary_part - rhs.imaginary_part)
    }
}

impl std::ops::Mul for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: Self) -> Self {
        Self::new(
            self.real_part * rhs.real_part - self.imaginary_part * rhs.imaginary_part,
            self.real_part * rhs.imaginary_part + self.imaginary_part * rhs.real_part,
        )
    }
}

impl std::ops::Mul<f64> for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f64) -> Self {
        Self::new(self.real_part * rhs, self.imaginary_part * rhs)
    }
}

impl std::ops::Div for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: Self) -> Self {
        let den = rhs.norm_squared();
        Self::new(
            (self.real_part * rhs.real_part + self.imaginary_part * rhs.imaginary_part) / den,
            (self.imaginary_part * rhs.real_part - self.real_part * rhs.imaginary_part) / den,
        )
    }
}

impl std::ops::Div<f64> for ComplexNumber64 {
    type Output = Self;
    #[inline(always)]
    fn div(self, rhs: f64) -> Self {
        Self::new(self.real_part / rhs, self.imaginary_part / rhs)
    }
}
