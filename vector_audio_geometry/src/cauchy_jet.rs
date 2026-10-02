//! Geometria Holomorfa da Wavelet de Cauchy e STFT Gaussiana (Bargmann-Fock)
//!
//! Este módulo implementa a álgebra exata e a verificação das equações diferenciais:
//! 1. z = t + i * (q * p / (2 * pi)) no semiplano superior H+
//! 2. F_n(z) = C * int_0^oo x_hat(f) * f^(q+n) * e^(2*pi*i*f*z) df
//! 3. Escada: W_n = p^(q+n+1/2) * F_n
//! 4. EDP: d_p F_n = i * (q / 2*pi) * d_t F_n  <=>  d_zbar F_n = 0
//! 5. Equações de Cauchy-Riemann e Laplace
//! 6. Quociente de Reatribuição R = W_1 / W_0 = F_1 / (p * F_0)
//! 7. STFT Gaussiana e Transformada de Bargmann com Fator de Gauge Exato

use std::f64::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex64 {
    pub re: f64,
    pub im: f64,
}

impl Complex64 {
    #[inline]
    pub fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    #[inline]
    pub fn conj(self) -> Self {
        Self { re: self.re, im: -self.im }
    }

    #[inline]
    pub fn norm_sq(self) -> f64 {
        self.re * self.re + self.im * self.im
    }

    #[inline]
    pub fn abs(self) -> f64 {
        self.norm_sq().sqrt()
    }

    #[inline]
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }

    #[inline]
    pub fn exp(self) -> Self {
        let r = self.re.exp();
        Self {
            re: r * self.im.cos(),
            im: r * self.im.sin(),
        }
    }
}

impl std::ops::Add for Complex64 {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self { re: self.re + rhs.re, im: self.im + rhs.im }
    }
}

impl std::ops::Sub for Complex64 {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self { re: self.re - rhs.re, im: self.im - rhs.im }
    }
}

impl std::ops::Mul for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: Self) -> Self {
        Self {
            re: self.re * rhs.re - self.im * rhs.im,
            im: self.re * rhs.im + self.im * rhs.re,
        }
    }
}

impl std::ops::Mul<f64> for Complex64 {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: f64) -> Self {
        Self { re: self.re * rhs, im: self.im * rhs }
    }
}

impl std::ops::Div for Complex64 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: Self) -> Self {
        let den = rhs.norm_sq();
        Self {
            re: (self.re * rhs.re + self.im * rhs.im) / den,
            im: (self.im * rhs.re - self.re * rhs.im) / den,
        }
    }
}

impl std::ops::Div<f64> for Complex64 {
    type Output = Self;
    #[inline]
    fn div(self, rhs: f64) -> Self {
        Self { re: self.re / rhs, im: self.im / rhs }
    }
}

/// Avaliador de Campo Holomorfo de Cauchy no Semiplano Superior H+
#[derive(Debug, Clone)]
pub struct CauchyHolomorphicField {
    pub q: f64,
    pub c_norm: f64,
}

impl CauchyHolomorphicField {
    pub fn new(q: f64) -> Self {
        Self {
            q,
            c_norm: 1.0,
        }
    }

    /// Avalia a função holomorfa normalizada F_n(t, p)
    /// F_n(t, p) = C * sum_k x_hat(f_k) * f_k^(q+n) * exp(-q*p*f_k) * exp(2*pi*i*f_k*t) * df
    pub fn eval_fn(
        &self,
        spectrum: &[(f64, Complex64)], // (frequência f_k, amplitude complexa X(f_k))
        t: f64,
        p: f64,
        n: usize,
    ) -> Complex64 {
        let mut sum = Complex64::new(0.0, 0.0);
        let q_n = self.q + n as f64;

        for &(f_k, x_k) in spectrum {
            if f_k <= 0.0 {
                continue;
            }
            let weight = f_k.powf(q_n) * (-self.q * p * f_k).exp();
            if weight < 1e-18 {
                continue;
            }

            let angle = 2.0 * PI * f_k * t;
            let phase_shifter = Complex64::new(angle.cos(), angle.sin());
            let term = x_k * phase_shifter * weight;
            sum = sum + term;
        }

        sum * self.c_norm
    }

    /// Avalia o coeficiente da transformada de Cauchy CQT:
    /// W_n(t, p) = p^(q+n+1/2) * F_n(t, p)
    pub fn eval_wn(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        p: f64,
        n: usize,
    ) -> Complex64 {
        let fn_val = self.eval_fn(spectrum, t, p, n);
        let scale = p.powf(self.q + n as f64 + 0.5);
        fn_val * scale
    }

    /// Avalia o quociente de reatribuição analítica:
    /// R = W_1 / W_0 = F_1 / (p * F_0)
    /// Retorna (f_hat, t_hat, R)
    pub fn reassignment(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        p: f64,
    ) -> (f64, f64, Complex64) {
        let w0 = self.eval_wn(spectrum, t, p, 0);
        let w1 = self.eval_wn(spectrum, t, p, 1);
        let r = w1 / w0;

        let f_hat = (1.0 / p) * r.re;
        let t_hat = t - (self.q * p / (2.0 * PI)) * r.im;

        (f_hat, t_hat, r)
    }

    /// Verifica numericamente as Equações de Cauchy-Riemann para F_0 = U + i*V:
    /// dU/dt = (2*pi / q) * dV/dp
    /// dV/dt = -(2*pi / q) * dU/dp
    /// Retorna os erros relativos (|res1| / scale, |res2| / scale)
    pub fn check_cauchy_riemann(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        p: f64,
        dt: f64,
        dp: f64,
    ) -> (f64, f64) {
        let _f_center = self.eval_fn(spectrum, t, p, 0);
        let f_right  = self.eval_fn(spectrum, t + dt, p, 0);
        let f_left   = self.eval_fn(spectrum, t - dt, p, 0);
        let f_up     = self.eval_fn(spectrum, t, p + dp, 0);
        let f_down   = self.eval_fn(spectrum, t, p - dp, 0);

        let du_dt = (f_right.re - f_left.re) / (2.0 * dt);
        let dv_dt = (f_right.im - f_left.im) / (2.0 * dt);

        let du_dp = (f_up.re - f_down.re) / (2.0 * dp);
        let dv_dp = (f_up.im - f_down.im) / (2.0 * dp);

        let cr_factor = 2.0 * PI / self.q;
        let target_du_dt = cr_factor * dv_dp;
        let target_dv_dt = -cr_factor * du_dp;

        let err1 = (du_dt - target_du_dt).abs() / (du_dt.abs() + target_du_dt.abs() + 1e-12);
        let err2 = (dv_dt - target_dv_dt).abs() / (dv_dt.abs() + target_dv_dt.abs() + 1e-12);

        (err1, err2)
    }

    /// Verifica a identidade da escada de derivadas puras:
    /// F_(n+1) = (1 / 2*pi*i) * d_t F_n = (1 / 2*pi) * (dV_n/dt - i * dU_n/dt)
    pub fn check_ladder_derivative(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        p: f64,
        n: usize,
        dt: f64,
    ) -> (f64, f64) {
        let fn_plus_1 = self.eval_fn(spectrum, t, p, n + 1);

        let f_right = self.eval_fn(spectrum, t + dt, p, n);
        let f_left  = self.eval_fn(spectrum, t - dt, p, n);

        let du_dt = (f_right.re - f_left.re) / (2.0 * dt);
        let dv_dt = (f_right.im - f_left.im) / (2.0 * dt);

        // (1 / 2*pi*i) * (dU + i*dV) = (dV - i*dU) / (2*pi)
        let ladder_u = dv_dt / (2.0 * PI);
        let ladder_v = -du_dt / (2.0 * PI);

        let err_u = (fn_plus_1.re - ladder_u).abs() / (fn_plus_1.re.abs() + 1e-12);
        let err_v = (fn_plus_1.im - ladder_v).abs() / (fn_plus_1.im.abs() + 1e-12);

        (err_u, err_v)
    }

    /// Verifica a EDP fundamental: d_p F_n = i * (q / 2*pi) * d_t F_n
    pub fn check_pde(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        p: f64,
        n: usize,
        dt: f64,
        dp: f64,
    ) -> (f64, f64) {
        let f_right = self.eval_fn(spectrum, t + dt, p, n);
        let f_left  = self.eval_fn(spectrum, t - dt, p, n);
        let f_up    = self.eval_fn(spectrum, t, p + dp, n);
        let f_down  = self.eval_fn(spectrum, t, p - dp, n);

        let df_dt = (f_right - f_left) / (2.0 * dt);
        let df_dp = (f_up - f_down) / (2.0 * dp);

        // i * (q / 2*pi) * (d_t u + i * d_t v) = (q / 2*pi) * (-d_t v + i * d_t u)
        let factor = self.q / (2.0 * PI);
        let target_dp = Complex64::new(-factor * df_dt.im, factor * df_dt.re);

        let err_re = (df_dp.re - target_dp.re).abs() / (df_dp.re.abs() + target_dp.re.abs() + 1e-12);
        let err_im = (df_dp.im - target_dp.im).abs() / (df_dp.im.abs() + target_dp.im.abs() + 1e-12);

        (err_re, err_im)
    }
}

/// Representação Canônica com absorção de 2*pi:
/// q_antigo = 2*pi * q
/// F(z) = C * sum_k x_hat(f_k) * exp( 2*pi * (q * log(f_k) + i * f_k * z) )
/// com z = t + i*eta, onde eta = q / f_c = q * p
#[derive(Debug, Clone)]
pub struct CauchyCanonicalField {
    pub q: f64,
}

impl CauchyCanonicalField {
    pub fn new(q: f64) -> Self {
        Self { q }
    }

    /// Avalia o potencial complexo Phi_C(f, z) = q * log(f) + i * f * z
    pub fn phi_c(&self, f: f64, z: Complex64) -> Complex64 {
        let log_f = f.ln();
        Complex64::new(
            self.q * log_f - f * z.im,
            f * z.re,
        )
    }

    /// Avalia a função holomorfa canônica F(z):
    /// F(t, eta) = sum_k x_k * exp(2*pi * Phi_C(f_k, z))
    pub fn eval_f(&self, spectrum: &[(f64, Complex64)], t: f64, eta: f64) -> Complex64 {
        let mut sum = Complex64::new(0.0, 0.0);
        let z = Complex64::new(t, eta);

        for &(f_k, x_k) in spectrum {
            if f_k <= 0.0 {
                continue;
            }
            let phi = self.phi_c(f_k, z);
            let exponent = phi * (2.0 * PI);
            let kernel = exponent.exp();
            sum = sum + x_k * kernel;
        }

        sum
    }

    /// Ponto estacionário complexo: f_* = i * q / z
    /// Para t = 0, z = i*eta => f_* = q / eta = f_c exatamente!
    pub fn stationary_frequency(&self, t: f64, eta: f64) -> Complex64 {
        let z = Complex64::new(t, eta);
        let num = Complex64::new(0.0, self.q);
        num / z
    }

    /// Verifica as equações de Cauchy-Riemann puras em (t, eta):
    /// dU/deta = -dV/dt
    /// dV/deta =  dU/dt
    /// SEM NENHUM FATOR DE 2*PI!
    pub fn check_pure_cauchy_riemann(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        eta: f64,
        dt: f64,
        deta: f64,
    ) -> (f64, f64) {
        let f_right = self.eval_f(spectrum, t + dt, eta);
        let f_left  = self.eval_f(spectrum, t - dt, eta);
        let f_up    = self.eval_f(spectrum, t, eta + deta);
        let f_down  = self.eval_f(spectrum, t, eta - deta);

        let du_dt = (f_right.re - f_left.re) / (2.0 * dt);
        let dv_dt = (f_right.im - f_left.im) / (2.0 * dt);

        let du_deta = (f_up.re - f_down.re) / (2.0 * deta);
        let dv_deta = (f_up.im - f_down.im) / (2.0 * deta);

        // Cauchy-Riemann clássica pura:
        // du/deta = -dv/dt  => res1 = |du/deta + dv/dt|
        // dv/deta =  du/dt  => res2 = |dv/deta - du/dt|
        let err1 = (du_deta + dv_dt).abs() / (du_deta.abs() + dv_dt.abs() + 1e-12);
        let err2 = (dv_deta - du_dt).abs() / (dv_deta.abs() + du_dt.abs() + 1e-12);

        (err1, err2)
    }
}

/// Avaliador da STFT Gaussiana e Transformada de Bargmann com Fator de Gauge Exato
pub struct BargmannGaussianSTFT;

impl BargmannGaussianSTFT {
    /// Avalia a STFT Gaussiana com convenção positiva:
    /// V_g^+ x(t, f) = integral_{-oo}^oo x(tau) * exp(-pi * (tau - t)^2) * exp(2*pi*i*f*tau) dtau
    pub fn eval_stft_gaussian<F>(x: F, t: f64, f: f64, tau_range: f64, steps: usize) -> Complex64
    where
        F: Fn(f64) -> f64,
    {
        let dtau = (2.0 * tau_range) / steps as f64;
        let mut sum_re = 0.0;
        let mut sum_im = 0.0;

        for i in 0..=steps {
            let tau = -tau_range + i as f64 * dtau;
            let val = x(tau);
            let win = (-PI * (tau - t).powi(2)).exp();
            let angle = 2.0 * PI * f * tau;

            let weight = val * win;
            sum_re += weight * angle.cos() * dtau;
            sum_im += weight * angle.sin() * dtau;
        }

        Complex64::new(sum_re, sum_im)
    }

    /// Fator de Gauge Exato de Bargmann:
    /// B_x(z) = exp(pi * t^2 - pi * z^2 / 2) * V_g^+ x(t, f)
    /// com z = t + i*f
    pub fn to_bargmann(v_g: Complex64, t: f64, f: f64) -> Complex64 {
        // z = t + i*f
        // z^2 = t^2 - f^2 + 2*i*t*f
        // -pi * z^2 / 2 = -pi*(t^2 - f^2)/2 - i * pi * t * f
        // pi * t^2 - pi * z^2 / 2 = pi * (t^2 + f^2)/2 - i * pi * t * f
        let exponent = Complex64::new(
            PI * (t * t + f * f) * 0.5,
            -PI * t * f,
        );
        let gauge = exponent.exp();
        gauge * v_g
    }

    /// Verifica as equações de Cauchy-Riemann de Bargmann:
    /// d_t B_x + i * d_f B_x = 0  (pois z = t + if)
    pub fn check_bargmann_holomorphy<F>(
        x: F,
        t: f64,
        f: f64,
        dt: f64,
        df: f64,
        tau_range: f64,
        steps: usize,
    ) -> f64
    where
        F: Fn(f64) -> f64,
    {
        let v_right = Self::eval_stft_gaussian(&x, t + dt, f, tau_range, steps);
        let v_left  = Self::eval_stft_gaussian(&x, t - dt, f, tau_range, steps);
        let v_up    = Self::eval_stft_gaussian(&x, t, f + df, tau_range, steps);
        let v_down  = Self::eval_stft_gaussian(&x, t, f - df, tau_range, steps);

        let b_right = Self::to_bargmann(v_right, t + dt, f);
        let b_left  = Self::to_bargmann(v_left, t - dt, f);
        let b_up    = Self::to_bargmann(v_up, t, f + df);
        let b_down  = Self::to_bargmann(v_down, t, f - df);

        let db_dt = (b_right - b_left) / (2.0 * dt);
        let db_df = (b_up - b_down) / (2.0 * df);

        // d_t B + i * d_f B = (db_dt.re - db_df.im) + i * (db_dt.im + db_df.re)
        let cr_re = db_dt.re - db_df.im;
        let cr_im = db_dt.im + db_df.re;

        let res = (cr_re * cr_re + cr_im * cr_im).sqrt();
        let scale = db_dt.abs() + db_df.abs() + 1e-12;
        res / scale
    }
}
