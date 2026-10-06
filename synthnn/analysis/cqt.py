"""Módulo de Transformada de Fourier de Resolução Constante (Gaussian CQT 60 bins/octave)."""
from __future__ import annotations
import math
import numpy as np

class GaussianCQT:
    """Banco de filtros CQT Gaussiano estritamente calibrado em banda-base."""
    def __init__(self, sr: float = 12000.0, bins_per_octave: int = 60, fmin: float = 40.0, fmax: float = 5500.0):
        self.sr = sr
        self.B = bins_per_octave
        self.fmin = fmin
        self.fmax = fmax
        self.sigma_oct = 0.95 / self.B
        self.Q = 1.0 / (2.0 ** (1.0 / self.B) - 1.0) # ~86.06

        num_octaves = math.log2(self.fmax / self.fmin)
        self.num_channels = int(round(self.B * num_octaves))
        self.yc = np.arange(self.num_channels) / self.B
        self.freqs = self.fmin * (2.0 ** self.yc)

    def find_nearest_bin(self, f: float) -> int:
        idx = int(np.argmin(np.abs(self.freqs - f)))
        return max(0, min(self.num_channels - 1, idx))

    def evaluate_coefficient_and_gain(self, x: np.ndarray, t0: float, fc: float) -> tuple[complex, float]:
        """
        Avalia o coeficiente C(t0, fc) e retorna (C_val, window_gain).
        Para uma senoide pura de amplitude A exatamente em fc, |C_val| / window_gain == A.
        """
        n0 = int(round(t0 * self.sr))
        sigma_t = self.Q / (2.0 * math.pi * fc)
        half_w = int(round(3.5 * sigma_t * self.sr))

        start = max(0, n0 - half_w)
        end = min(len(x), n0 + half_w + 1)
        seg = x[start:end]

        t_seg = (np.arange(start, end) - n0) / self.sr
        g = np.exp(-0.5 * (t_seg / sigma_t)**2)

        window_gain = 0.5 * float(np.sum(g))

        carrier = np.exp(-2j * math.pi * fc * t_seg)
        C_val = complex(np.sum(seg * g * carrier))

        return C_val, window_gain

    def extract_instantaneous_amplitude_and_freq(self, x: np.ndarray, t0: float, fc: float) -> tuple[float, float]:
        """
        Extrai (A_calibrado, f_inst) no instante t0 na vizinhança de fc.
        Aplica a correção analítica de off-bin exata da transformada de Fourier da Gaussiana:
        G(f - fc) = exp(-0.5 * (2*pi*(f_inst - fc)*sigma_t)^2).
        """
        dt = 1.0 / self.sr
        C_0, gain_0 = self.evaluate_coefficient_and_gain(x, t0, fc)
        C_p, _ = self.evaluate_coefficient_and_gain(x, t0 + dt, fc)
        
        d_phase = math.atan2((C_p * C_0.conjugate()).imag, (C_p * C_0.conjugate()).real) / dt
        f_inst = d_phase / (2.0 * math.pi)

        # Magnitude normalizada
        mag = abs(C_0) / max(gain_0, 1e-12)

        # Correção analítica de atenuação de Fourier da Gaussiana
        sigma_t = self.Q / (2.0 * math.pi * fc)
        exact_attenuation = math.exp(-0.5 * (2.0 * math.pi * (f_inst - fc) * sigma_t)**2)
        a_calibrated = mag / max(exact_attenuation, 0.01)

        return a_calibrated, f_inst
