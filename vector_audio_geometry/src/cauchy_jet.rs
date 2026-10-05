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

/// Tipo de Escala Perceptual para Embutimento Holomorfo Universal:
/// eta(y) = q / (f(y) + lambda)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PerceptualScaleType {
    Cqt,       // lambda = 0
    Mel,       // lambda = 700
    Bark,      // lambda = 1960
}

#[derive(Debug, Clone)]
pub struct ShiftedScaleHolomorphicField {
    pub scale_type: PerceptualScaleType,
    pub q: f64,
    pub lambda: f64,
    pub f0: f64,
}

impl ShiftedScaleHolomorphicField {
    pub fn new(scale_type: PerceptualScaleType, q: f64, f0: f64) -> Self {
        let lambda = match scale_type {
            PerceptualScaleType::Cqt => 0.0,
            PerceptualScaleType::Mel => 700.0,
            PerceptualScaleType::Bark => 1960.0,
        };
        Self { scale_type, q, lambda, f0 }
    }

    /// Mapeamento da escala perceptual y -> frequência f(y) em Hz
    pub fn f_from_y(&self, y: f64) -> f64 {
        match self.scale_type {
            PerceptualScaleType::Cqt => self.f0 * 2.0f64.powf(y),
            PerceptualScaleType::Mel => 700.0 * (2.0f64.powf(y / 2595.0) - 1.0),
            PerceptualScaleType::Bark => 1960.0 * (y + 0.53) / (26.28 - y),
        }
    }

    /// Mapeamento inverso f -> y(f)
    pub fn y_from_f(&self, f: f64) -> f64 {
        match self.scale_type {
            PerceptualScaleType::Cqt => (f / self.f0).log2(),
            PerceptualScaleType::Mel => 2595.0 * (1.0 + f / 700.0).log2(),
            PerceptualScaleType::Bark => 26.81 * (f / (1960.0 + f)) - 0.53,
        }
    }

    /// Curva de embutimento no semiplano: eta(y) = q / (f(y) + lambda)
    pub fn eta(&self, y: f64) -> f64 {
        let f = self.f_from_y(y);
        self.q / (f + self.lambda)
    }

    /// Derivada analítica d_eta / dy:
    pub fn d_eta_dy(&self, y: f64) -> f64 {
        let dy = 1e-6;
        (self.eta(y + dy) - self.eta(y - dy)) / (2.0 * dy)
    }

    /// Potencial espectral Phi(f) = 2*pi * q * ln((f + lambda) / (f0 + lambda))
    pub fn phi(&self, f: f64) -> f64 {
        2.0 * PI * self.q * ((f + self.lambda) / (self.f0 + self.lambda)).ln()
    }

    /// Avalia a representação geral E(t, y) = F_lambda(t + i*eta(y))
    pub fn eval_e(&self, spectrum: &[(f64, Complex64)], t: f64, y: f64) -> Complex64 {
        let eta_val = self.eta(y);
        let mut sum = Complex64::new(0.0, 0.0);

        for &(f_k, x_k) in spectrum {
            if f_k <= 0.0 {
                continue;
            }
            let phi_val = self.phi(f_k);
            let exponent_re = phi_val - 2.0 * PI * f_k * eta_val;
            let exponent_im = 2.0 * PI * f_k * t;

            let filter = Complex64::new(exponent_re, exponent_im).exp();
            sum = sum + x_k * filter;
        }

        sum
    }

    /// Verifica a EDP de colapso vertical: dE/dy = i * eta'(y) * dE/dt
    pub fn check_vertical_collapse_pde(
        &self,
        spectrum: &[(f64, Complex64)],
        t: f64,
        y: f64,
        dt: f64,
        dy: f64,
    ) -> (f64, f64) {
        let e_right = self.eval_e(spectrum, t + dt, y);
        let e_left  = self.eval_e(spectrum, t - dt, y);
        let e_up    = self.eval_e(spectrum, t, y + dy);
        let e_down  = self.eval_e(spectrum, t, y - dy);

        let de_dt = (e_right - e_left) / (2.0 * dt);
        let de_dy = (e_up - e_down) / (2.0 * dy);

        let eta_prime = self.d_eta_dy(y);
        // target = i * eta'(y) * (dE_dt.re + i * dE_dt.im) = eta'(y) * (-dE_dt.im + i * dE_dt.re)
        let target_de_dy = Complex64::new(-eta_prime * de_dt.im, eta_prime * de_dt.re);

        let err_re = (de_dy.re - target_de_dy.re).abs() / (de_dy.re.abs() + target_de_dy.re.abs() + 1e-12);
        let err_im = (de_dy.im - target_de_dy.im).abs() / (de_dy.im.abs() + target_de_dy.im.abs() + 1e-12);

        (err_re, err_im)
    }

    /// Calcula a energia L^2 da janela bruta G_y(f):
    /// I_2(y) = integral_0^oo |G_y(f)|^2 df
    pub fn eval_l2_energy(&self, y: f64, fs: f64, n_fft: usize) -> f64 {
        let df = fs / n_fft as f64;
        let eta_val = self.eta(y);
        let mut sum = 0.0;

        for k in 1..(n_fft / 2) {
            let f_k = k as f64 * df;
            let phi_val = self.phi(f_k);
            let exponent = 2.0 * (phi_val - 2.0 * PI * f_k * eta_val);
            if exponent < -40.0 {
                continue;
            }
            sum += exponent.exp() * df;
        }

        sum
    }

    /// Fator de normalização unitária L^2:
    /// N_2(y) = ( integral |G_y(f)|^2 df )^(-1/2)
    pub fn l2_normalization_factor(&self, y: f64, fs: f64, n_fft: usize) -> f64 {
        let energy = self.eval_l2_energy(y, fs, n_fft);
        if energy > 1e-30 {
            1.0 / energy.sqrt()
        } else {
            1.0
        }
    }

    /// Avalia a janela L^2-normalizada G_tilde_y[k]:
    /// Utiliza a técnica de centralização logarítmica para estabilidade numérica:
    /// log_G(f) - log_G(f(y)) para evitar overflow/underflow em exponenciais brutas.
    pub fn normalized_window(&self, y: f64, fs: f64, n_fft: usize) -> Vec<f64> {
        let df = fs / n_fft as f64;
        let eta_val = self.eta(y);
        let f_center = self.f_from_y(y);
        let half = n_fft / 2;
        
        let mut window = vec![0.0f64; half];
        let mut energy_sum = 0.0;
        
        let phi_center = self.phi(f_center);
        let log_g_center = phi_center - 2.0 * PI * f_center * eta_val;

        for k in 1..half {
            let f_k = k as f64 * df;
            let phi_val = self.phi(f_k);
            let exponent = phi_val - 2.0 * PI * f_k * eta_val;
            
            // Centralização logarítmica em relação ao pico da janela
            let centered_exponent = exponent - log_g_center;
            
            // Corta vazamentos abaixo de -745 no expoente (aproximadamente zero em float64)
            if centered_exponent >= -700.0 {
                let w = centered_exponent.exp();
                window[k] = w;
                energy_sum += w * w * df;
            }
        }
        
        let n2 = if energy_sum > 1e-300 {
            1.0 / energy_sum.sqrt()
        } else {
            1.0
        };
        
        // Aplica a normalização L^2
        for k in 1..half {
            window[k] *= n2;
        }

        window
    }

    /// Densidade analítica de frame tight estabilizada numericamente:
    /// rho(y) propto [ eta(y)^(4*pi*q - 1) * exp(-4*pi*lambda*eta(y)) * |eta'(y)| ] / N_2(y)^2
    pub fn analytical_tight_frame_density(&self, y: f64, fs: f64, n_fft: usize) -> f64 {
        let eta_val = self.eta(y);
        let eta_prime = self.d_eta_dy(y).abs();
        let n2 = self.l2_normalization_factor(y, fs, n_fft);

        let power = 4.0 * PI * self.q - 1.0;
        // Evita subfluxo em eta^power calculando no domínio logarítmico
        let log_w = power * eta_val.ln() - 4.0 * PI * self.lambda * eta_val;
        let log_rho = log_w + eta_prime.ln() - 2.0 * n2.ln();

        // Normalização de escala proporcional
        if log_rho.is_finite() {
            log_rho
        } else {
            0.0
        }
    }

    /// Avalia o operador de frame espectral:
    /// H[k] = sum_j rho_j * |G_tilde_j[k]|^2
    pub fn eval_frame_operator(
        windows: &[Vec<f64>],
        weights: &[f64],
        half_n: usize,
    ) -> Vec<f64> {
        let mut h_k = vec![0.0f64; half_n];
        for (w, &rho) in windows.iter().zip(weights.iter()) {
            for k in 0..half_n {
                h_k[k] += rho * w[k] * w[k];
            }
        }
        h_k
    }

    /// Reconstrução de frame tight por mínimos quadrados:
    /// X_hat[k] = ( sum_j rho_j * G_tilde_j[k] * Y_j[k] ) / H[k]
    pub fn reconstruct_spectrum_ls(
        channels: &[Vec<Complex64>],
        windows: &[Vec<f64>],
        weights: &[f64],
        half_n: usize,
    ) -> Vec<Complex64> {
        let h_k = Self::eval_frame_operator(windows, weights, half_n);
        let mut x_hat = vec![Complex64::new(0.0, 0.0); half_n];

        for k in 0..half_n {
            let denom = h_k[k];
            if denom > 1e-30 {
                let mut num = Complex64::new(0.0, 0.0);
                for (j, y_j) in channels.iter().enumerate() {
                    let rho = weights[j];
                    let g_val = windows[j][k];
                    num = num + y_j[k] * (rho * g_val);
                }
                x_hat[k] = num / denom;
            }
        }

        x_hat
    }

    /// Otimização de Densidade de Frame via Gradiente Descendente Projetado (NNLS aproximado)
    /// Min || A * rho - 1 ||_2^2 sujeito a rho >= 0
    /// Onde A_{k, j} = |G_tilde_j[k]|^2
    pub fn optimize_frame_density_nnls(
        windows: &[Vec<f64>],
        k_min: usize,
        k_max: usize,
        iterations: usize,
        learning_rate: f64,
    ) -> Vec<f64> {
        let num_channels = windows.len();
        // Inicialização uniforme ou com a densidade analítica
        let mut rho = vec![1.0; num_channels];
        
        for _ in 0..iterations {
            let mut gradient = vec![0.0; num_channels];
            
            // Avalia o frame operator atual H[k] = sum_j rho_j * A_{k,j}
            for k in k_min..=k_max {
                let mut h_k = 0.0;
                for j in 0..num_channels {
                    let w = windows[j][k];
                    h_k += rho[j] * w * w;
                }
                
                let err = h_k - 1.0; // Resíduo A * rho - 1
                
                // Acumula o gradiente: 2 * A^T * err
                for j in 0..num_channels {
                    let w = windows[j][k];
                    gradient[j] += 2.0 * err * w * w;
                }
            }
            
            // Atualiza rho via gradiente descendente com projeção não-negativa
            for j in 0..num_channels {
                rho[j] = (rho[j] - learning_rate * gradient[j]).max(0.0);
            }
        }
        
        rho
    }
}
