"""Módulo de Estimadores de Ruído Estocástico (E13) e Efeitos Não-Locais (E14)."""
from __future__ import annotations
import math
import numpy as np
from scipy.signal import welch, medfilt
import torch
from torch import nn

class AnalyticNoiseEstimator:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.grid_fk = np.geomspace(400.0, 3000.0, 80)

    def estimate(self, x: np.ndarray) -> tuple[float, float, float, float, np.ndarray, np.ndarray]:
        f_welch, pxx = welch(x, fs=self.sr, nperseg=2048, noverlap=1024)
        log_p = 10.0 * np.log10(np.maximum(pxx, 1e-14))

        bg_p = medfilt(log_p, 25)
        diff = log_p - bg_p
        non_peak = diff < 2.5

        mask = non_peak & (f_welch >= 120.0) & (f_welch <= self.sr * 0.45)
        f_sub = f_welch[mask]
        p_clean = log_p[mask]

        best_res = 1e9
        best_alpha = 1.0
        best_fk = 1000.0
        best_c0 = -60.0

        for fk in self.grid_fk:
            u = np.log10(1.0 + (f_sub / fk)**2)
            X = np.column_stack([np.ones(len(u)), -5.0 * u])
            c, res, _, _ = np.linalg.lstsq(X, p_clean, rcond=None)
            ss_res = np.sum((p_clean - X @ c)**2)
            if ss_res < best_res:
                best_res = ss_res
                best_c0 = float(c[0])
                best_alpha = max(0.0, min(2.5, float(c[1])))
                best_fk = float(fk)

        rmse_db = float(np.sqrt(best_res / len(p_clean)))
        return best_alpha, best_fk, best_c0, rmse_db, f_welch, log_p

class NoiseParametricHead(nn.Module):
    def __init__(self, in_features: int = 64):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, 16),
            nn.Tanh(),
            nn.Linear(16, 6)
        )
        with torch.no_grad():
            self.net[4].weight.fill_(0.0)
            self.net[4].bias.fill_(0.0)

    def forward(self, feat: torch.Tensor) -> torch.Tensor:
        return 0.05 * torch.tanh(self.net(feat))

class AnalyticEffectsEstimator:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr

    def detect_delay(self, x: np.ndarray, y: np.ndarray) -> tuple[bool, float, float]:
        """
        Detecta presença de delay/eco via resposta ao impulso de deconvolução espectral Y(f) / X(f):
            H(f) = 1 + g * e^(-i * 2*pi * f * tau)
        Retorna: (is_active, tau_ms, gain)
        """
        N = len(x)
        fft_x = np.fft.rfft(x)
        fft_y = np.fft.rfft(y)

        H_est = fft_y / (fft_x + 1e-4)
        h_time = np.abs(np.fft.irfft(H_est))

        min_idx = int(0.015 * self.sr)
        max_idx = int(0.240 * self.sr)
        search_region = h_time[min_idx : max_idx]

        peak_idx_rel = np.argmax(search_region)
        peak_val = search_region[peak_idx_rel]
        peak_idx = min_idx + peak_idx_rel

        is_active = peak_val > 0.08
        tau_ms = (peak_idx / self.sr) * 1000.0
        gain = float(peak_val) if is_active else 0.0

        return is_active, float(tau_ms), float(gain)

    def detect_filter(self, x: np.ndarray, y: np.ndarray, f0: float = 180.0) -> tuple[bool, float]:
        """
        Detecta presença de filtro passa-baixas e estima frequência de corte a -3 dB
        avaliando a função de transferência estritamente nos harmônicos excitados.
        Retorna: (is_active, fc_est)
        """
        N = len(x)
        fft_x = np.abs(np.fft.rfft(x))
        fft_y = np.abs(np.fft.rfft(y))

        h_gains, h_freqs = [], []
        for k in range(1, 30):
            fk = k * f0
            if fk >= self.sr * 0.45:
                break
            idx = int(round(fk * N / self.sr))
            if idx < len(fft_x):
                gain = fft_y[idx] / max(fft_x[idx], 1e-4)
                h_gains.append(gain)
                h_freqs.append(fk)

        h_gains = np.array(h_gains)
        h_freqs = np.array(h_freqs)

        idx = np.where(h_gains < 0.707)[0]
        if len(idx) > 0 and idx[0] > 0:
            i = idx[0]
            g0, g1 = h_gains[i - 1], h_gains[i]
            f0_pt, f1_pt = h_freqs[i - 1], h_freqs[i]
            fc_est = f0_pt + (0.707 - g0) / (g1 - g0 + 1e-8) * (f1_pt - f0_pt)
            is_active = fc_est < self.sr * 0.38
            return is_active, float(fc_est)
        else:
            return False, float(self.sr * 0.45)

    def detect_reverb(self, y: np.ndarray) -> tuple[bool, float, float]:
        N = len(y)
        t = np.arange(N) / self.sr

        # Avaliação da cauda pós-release em t in [0.55, 1.3] s
        mask_tail = (t >= 0.55) & (t <= 1.3)
        if np.sum(mask_tail) < 100:
            return False, 0.0, 0.0

        t_t = t[mask_tail] - 0.55
        tail = y[mask_tail]

        energy_tail_db = 10.0 * np.log10(np.mean(tail**2) + 1e-12)

        # Integração reversa de Schroeder
        schroeder = np.cumsum((tail**2)[::-1])[::-1]
        log_schroeder = 10.0 * np.log10(np.maximum(schroeder, 1e-12))
        slope, _ = np.polyfit(t_t, log_schroeder, 1)

        is_active = (energy_tail_db > -32.0) and (slope < -12.0) and (slope > -150.0)
        t60 = float(min(4.0, max(0.2, -60.0 / slope))) if is_active else 0.0
        wet = float(min(0.9, max(0.05, 1.0 - math.exp(-t60)))) if is_active else 0.0

        return is_active, t60, wet
