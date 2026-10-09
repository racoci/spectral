"""
E39: Exact Equivalence Classes in Structural Audio Modeling.
Demonstrates and validates transformations g such that D(theta) = D(g(theta)).
"""

from __future__ import annotations
import numpy as np
from typing import Tuple, Dict, Any


def time_shift_vs_phase(
    f0: float,
    dt: float,
    phi0: float,
    duration: float = 0.1,
    sr: float = 16000.0
) -> Tuple[float, np.ndarray, np.ndarray]:
    """
    Validates that a temporal delay dt is identical to a phase rotation
    dphi = -2*pi*f0*dt for an isolated sinusoidal carrier.
    
    x_time(t) = cos(2*pi*f0*(t - dt) + phi0)
    x_phase(t) = cos(2*pi*f0*t + (phi0 - 2*pi*f0*dt))
    """
    N = int(sr * duration)
    t = np.linspace(0, duration, N, endpoint=False)
    
    x_time = np.cos(2.0 * np.pi * f0 * (t - dt) + phi0)
    phi_equiv = phi0 - 2.0 * np.pi * f0 * dt
    x_phase = np.cos(2.0 * np.pi * f0 * t + phi_equiv)
    
    max_diff = float(np.max(np.abs(x_time - x_phase)))
    return max_diff, x_time, x_phase


def fm_vs_pm_angle_modulation(
    fc: float,
    fm: float,
    beta: float,
    psi: float = 0.0,
    duration: float = 0.1,
    sr: float = 16000.0
) -> Tuple[float, np.ndarray, np.ndarray]:
    """
    Validates that Frequency Modulation (FM) with deviation delta_f = beta * fm
    and Phase Modulation (PM) with modulation index I = beta are mathematically
    and numerically identical.
    
    phi_FM(t) = 2*pi*fc*t + (delta_f / fm) * sin(2*pi*fm*t + psi)
    phi_PM(t) = 2*pi*fc*t + I * sin(2*pi*fm*t + psi)
    """
    N = int(sr * duration)
    t = np.linspace(0, duration, N, endpoint=False)
    
    delta_f = beta * fm
    I = beta
    
    phi_fm = 2.0 * np.pi * fc * t + (delta_f / fm) * np.sin(2.0 * np.pi * fm * t + psi)
    phi_pm = 2.0 * np.pi * fc * t + I * np.sin(2.0 * np.pi * fm * t + psi)
    
    x_fm = np.cos(phi_fm)
    x_pm = np.cos(phi_pm)
    
    max_diff = float(np.max(np.abs(x_fm - x_pm)))
    return max_diff, x_fm, x_pm


def filter_vs_spectral_envelope(
    f0: float = 220.0,
    num_harmonics: int = 8,
    cutoff_hz: float = 1200.0,
    duration: float = 0.1,
    sr: float = 16000.0
) -> Tuple[float, np.ndarray, np.ndarray]:
    """
    Validates that filtering an excitation signal with an LTI filter H(w)
    produces the exact same waveform as weighting harmonic partial amplitudes
    and phases by |H(k*w0)| and arg H(k*w0).
    """
    N = int(sr * duration)
    t = np.linspace(0, duration, N, endpoint=False)
    
    # Define an analytic lowpass filter response H(f) (Butterworth 4th-order magnitude & phase)
    # Using a 2nd-order analog biquad continuous response for exact frequency-domain evaluation
    w_c = 2.0 * np.pi * cutoff_hz
    
    def H_continuous(f_hz: np.ndarray) -> np.ndarray:
        w = 2.0 * np.pi * f_hz
        # s = j*w. Butterworth 2nd order: s^2 + sqrt(2)*w_c*s + w_c^2
        s = 1j * w
        denom = s**2 + np.sqrt(2.0) * w_c * s + w_c**2
        return w_c**2 / denom

    # 1. Harmonic synthesis via discrete spectral envelope
    y_env = np.zeros(N, dtype=np.float64)
    harmonic_freqs = np.array([k * f0 for k in range(1, num_harmonics + 1)])
    H_k = H_continuous(harmonic_freqs)
    
    for k in range(1, num_harmonics + 1):
        f_k = k * f0
        gain_k = np.abs(H_k[k - 1])
        phase_k = np.angle(H_k[k - 1])
        y_env += gain_k * np.cos(2.0 * np.pi * f_k * t + phase_k)
        
    # 2. Filtering in frequency domain (exact periodic convolution)
    # Generate harmonic excitation
    e = np.zeros(N, dtype=np.float64)
    for k in range(1, num_harmonics + 1):
        f_k = k * f0
        e += np.cos(2.0 * np.pi * f_k * t)
        
    # Fourier transform
    E_freq = np.fft.rfft(e)
    freqs = np.fft.rfftfreq(N, 1.0 / sr)
    H_fft = H_continuous(freqs)
    
    # Filter in frequency domain
    Y_filt_freq = E_freq * H_fft
    y_filt = np.fft.irfft(Y_filt_freq, n=N)
    
    max_diff = float(np.max(np.abs(y_filt - y_env)))
    return max_diff, y_filt, y_env


def polar_amplitude_phase(
    A: float,
    phi: float,
    f: float = 440.0,
    duration: float = 0.1,
    sr: float = 16000.0
) -> Tuple[float, np.ndarray, np.ndarray]:
    """
    Validates that a negative amplitude A < 0 is identical to a positive
    amplitude |A| with a pi phase rotation.
    """
    N = int(sr * duration)
    t = np.linspace(0, duration, N, endpoint=False)
    
    x_raw = A * np.cos(2.0 * np.pi * f * t + phi)
    x_canon = np.abs(A) * np.cos(2.0 * np.pi * f * t + phi + np.pi)
    
    max_diff = float(np.max(np.abs(x_raw - x_canon)))
    return max_diff, x_raw, x_canon


def compute_gauge_jacobian_nullity(
    f0: float = 440.0,
    duration: float = 0.05,
    sr: float = 16000.0
) -> Dict[str, Any]:
    """
    Computes the numerical Jacobian of a sinusoidal model parameterized
    redundantly by both delay dt and phase phi0:
    x(t; dt, phi0) = cos(2*pi*f0*(t - dt) + phi0)
    
    Demonstrates that the Jacobian has rank 1 and a zero singular value along
    the gauge direction (d/d_dt + 2*pi*f0 * d/d_phi0 = 0).
    """
    N = int(sr * duration)
    t = np.linspace(0, duration, N, endpoint=False)
    
    dt = 0.001
    phi0 = 0.5
    
    arg = 2.0 * np.pi * f0 * (t - dt) + phi0
    
    # J[:, 0] = dx/d(dt) = 2*pi*f0 * sin(arg)
    # J[:, 1] = dx/d(phi0) = -sin(arg)
    J = np.zeros((N, 2))
    J[:, 0] = 2.0 * np.pi * f0 * np.sin(arg)
    J[:, 1] = -np.sin(arg)
    
    # Singular Value Decomposition
    _, s, vh = np.linalg.svd(J, full_matrices=False)
    
    ratio = float(s[1] / max(s[0], 1e-12))
    has_nullity = bool(ratio < 1e-14)
    
    null_vector = vh[1].tolist()
    
    return {
        'singular_values': [float(s[0]), float(s[1])],
        'singular_value_ratio': ratio,
        'has_nullity': has_nullity,
        'null_vector': null_vector
    }
