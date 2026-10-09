"""
E41: True Physical Ambiguity and Uncertainty Distribution.
Models physical non-identifiabilities where observation is finite-length or band-limited,
representing them as posterior distributions p(theta|x) whose entropy collapses
under causal lookahead horizon expansion.
"""

from __future__ import annotations
import numpy as np
from typing import Dict, Any, List


class HarmonicCollisionAmbiguity:
    """
    Models the non-identifiability when two harmonic components coincide in frequency
    (e.g. fundamental of voice 2 coincides with 2nd harmonic of voice 1: f2 = 2*f1).
    Over a short static window, the individual amplitudes A1 and A2 are non-identifiable,
    constrained only by their observed linear combination A1 + A2 = A_obs.
    """
    def __init__(self, sr: float = 16000.0):
        self.sr = sr

    def compute_posterior(
        self,
        f1: float = 220.0,
        f2: float = 440.0,
        observed_amplitude: float = 1.5,
        prior_mean1: float = 0.8,
        prior_mean2: float = 0.8,
        prior_std1: float = 0.5,
        prior_std2: float = 0.5,
        noise_std: float = 0.05
    ) -> Dict[str, Any]:
        """
        Computes Bayesian linear-Gaussian posterior p(A1, A2 | y = A1 + A2 + e).
        """
        # Prior parameters
        m0 = np.array([prior_mean1, prior_mean2], dtype=np.float64)
        Sigma0 = np.diag([prior_std1**2, prior_std2**2])
        C = np.array([[1.0, 1.0]], dtype=np.float64)  # Observation operator
        R = np.array([[noise_std**2]], dtype=np.float64)

        # Innovation covariance S = C * Sigma0 * C^T + R
        S = C @ Sigma0 @ C.T + R
        inv_S = np.linalg.inv(S)

        # Kalman / Bayesian Gain K = Sigma0 * C^T * S^-1
        K = Sigma0 @ C.T @ inv_S

        # Posterior mean m = m0 + K * (y - C * m0)
        y = np.array([[observed_amplitude]])
        innovation = y - C @ m0.reshape(2, 1)
        m_post = m0.reshape(2, 1) + K @ innovation

        # Posterior covariance Sigma = (I - K * C) * Sigma0
        I = np.eye(2)
        Sigma_post = (I - K @ C) @ Sigma0

        # Prior differential entropy in bits
        det_sigma0 = np.linalg.det(Sigma0)
        prior_entropy_bits = 0.5 * np.log2(np.maximum(det_sigma0, 1e-18) * ((2.0 * np.pi * np.e)**2))

        # Differential entropy of 2D Gaussian in bits:
        # H(p) = 0.5 * log2( det(2 * pi * e * Sigma) )
        det_sigma = np.linalg.det(Sigma_post)
        det_scaled = np.maximum(det_sigma, 1e-18) * ((2.0 * np.pi * np.e)**2)
        entropy_bits = 0.5 * np.log2(det_scaled)

        eigvals = np.linalg.eigvalsh(Sigma_post)

        return {
            'mean_A1': float(m_post[0, 0]),
            'mean_A2': float(m_post[1, 0]),
            'covariance': Sigma_post.tolist(),
            'prior_entropy_bits': float(prior_entropy_bits),
            'entropy_bits': float(entropy_bits),
            'information_gain_bits': float(prior_entropy_bits - entropy_bits),
            'eigenvalues': [float(eigvals[0]), float(eigvals[1])]
        }


class ResonatorVsNoiseAmbiguity:
    """
    Models the physical ambiguity between an exponentially decaying resonator pole
    and a narrowband filtered noise grain over short observation windows.
    """
    def __init__(self, sr: float = 16000.0):
        self.sr = sr

    def generate_resonator(self, f0: float = 500.0, decay_rate: float = 25.0, duration: float = 0.1) -> np.ndarray:
        N = int(self.sr * duration)
        t = np.linspace(0, duration, N, endpoint=False)
        return np.exp(-decay_rate * t) * np.cos(2.0 * np.pi * f0 * t)

    def generate_noise_grain(self, center_f: float = 500.0, bandwidth: float = 50.0, duration: float = 0.1) -> np.ndarray:
        N = int(self.sr * duration)
        t = np.linspace(0, duration, N, endpoint=False)
        rng = np.random.RandomState(42)
        white = rng.randn(N)

        # Gaussian spectral bandpass around center_f
        freqs = np.fft.rfftfreq(N, 1.0 / self.sr)
        H = np.exp(-0.5 * ((freqs - center_f) / bandwidth)**2)
        filtered = np.fft.irfft(np.fft.rfft(white) * H, n=N)

        # Apply smooth Hann envelope
        window = np.hanning(N)
        grain = filtered * window
        grain_norm = grain / (np.max(np.abs(grain)) + 1e-9)
        return grain_norm

    def evaluate_likelihood(self, x: np.ndarray) -> Dict[str, Any]:
        """
        Evaluates the relative likelihood p(Resonator | x) vs p(Noise | x)
        by inspecting instantaneous phase derivative variance (regularity)
        and autocorrelation envelope decay.
        """
        N = len(x)
        # Analytic signal via Hilbert transform
        X_fft = np.fft.fft(x)
        h = np.zeros(N)
        if N % 2 == 0:
            h[0] = h[N // 2] = 1
            h[1:N // 2] = 2
        else:
            h[0] = 1
            h[1:(N + 1) // 2] = 2
        analytic = np.fft.ifft(X_fft * h)

        unwrapped_phase = np.unwrap(np.angle(analytic))
        inst_freq = np.diff(unwrapped_phase) * self.sr / (2.0 * np.pi)

        # Trim boundary 10% to eliminate edge artifacts of Hilbert transform
        trim = max(int(N * 0.1), 2)
        inst_freq_interior = inst_freq[trim:-trim] if len(inst_freq) > 2 * trim else inst_freq

        # For a resonator, inst_freq is constant (variance near 0)
        # For filtered noise, inst_freq fluctuates randomly
        freq_var = float(np.var(inst_freq_interior))

        # Likelihood under H_resonator (expected var theta_0 = 100) vs H_noise (expected var theta_1 = 30000)
        theta_0 = 200.0
        theta_1 = 30000.0
        
        # Exponential density: p(v | H) = (1/theta) * exp(-v / theta)
        # In log space to prevent numerical overflow/underflow
        log_p_res = -np.log(theta_0) - (freq_var / theta_0)
        log_p_noise = -np.log(theta_1) - (freq_var / theta_1)

        max_log = max(log_p_res, log_p_noise)
        w_res = np.exp(log_p_res - max_log)
        w_noise = np.exp(log_p_noise - max_log)

        p_res = float(w_res / (w_res + w_noise))
        p_noise = float(w_noise / (w_res + w_noise))

        entropy = float(-p_res * np.log2(max(p_res, 1e-12)) - p_noise * np.log2(max(p_noise, 1e-12)))

        return {
            'instantaneous_freq_variance': freq_var,
            'p_resonator': p_res,
            'p_noise': p_noise,
            'ambiguity_entropy': entropy
        }


class CausalLookaheadDisambiguator:
    """
    Demonstrates that while an ambiguity p(theta | x_0:T) exists over a short window T,
    expanding the observation horizon to T + tau breaks the physical symmetry,
    collapsing posterior covariance and decreasing differential entropy monotonically.
    """
    def __init__(self, sr: float = 16000.0):
        self.sr = sr

    def simulate_horizon_expansion(
        self,
        f1: float = 220.0,
        f2: float = 440.0,
        horizons: List[float] = [0.02, 0.05, 0.1, 0.2],
        decay1: float = 15.0,
        decay2: float = 35.0,
        noise_std: float = 0.05
    ) -> List[Dict[str, Any]]:
        """
        Simulates two overlapping sources at f2 = 2*f1 with distinct decay rates.
        Computes the Fisher Information matrix and posterior entropy as horizon expands.
        """
        trace = []

        for h in horizons:
            N = int(self.sr * h)
            t = np.linspace(0, h, N, endpoint=False)

            # Basis signals
            u1 = np.exp(-decay1 * t) * np.cos(2.0 * np.pi * f2 * t)
            u2 = np.exp(-decay2 * t) * np.cos(2.0 * np.pi * f2 * t)

            # Fisher Information Matrix: I_F = (1/sigma^2) * U^T * U
            U = np.stack([u1, u2], axis=1)
            I_F = (U.T @ U) / (noise_std**2)

            # Regularize with prior precision
            prior_var = 1.0
            I_prior = np.eye(2) / prior_var
            I_total = I_F + I_prior

            Sigma_post = np.linalg.inv(I_total)

            det_sigma = np.linalg.det(Sigma_post)
            det_scaled = np.maximum(det_sigma, 1e-18) * ((2.0 * np.pi * np.e)**2)
            entropy_bits = 0.5 * np.log2(det_scaled)

            trace.append({
                'horizon_seconds': float(h),
                'num_samples': N,
                'det_covariance': float(det_sigma),
                'entropy_bits': float(entropy_bits),
                'covariance': Sigma_post.tolist()
            })

        return trace
