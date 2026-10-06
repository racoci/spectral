"""Módulo de Estimadores de Modulação Analógica: FM, PM e AM (Estágios E10, E11, E12)."""
from __future__ import annotations
import math
import numpy as np
from scipy.special import jv
from scipy.signal import hilbert, butter, filtfilt
import torch
from torch import nn

class AnalyticFmPmSolver:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.grid_beta = np.linspace(0.05, 5.5, 546)
        bessel_matrix = np.column_stack([
            np.abs(jv(0, self.grid_beta)),
            np.abs(jv(1, self.grid_beta)),
            np.abs(jv(2, self.grid_beta)),
            np.abs(jv(3, self.grid_beta))
        ])
        self.bessel_profiles = bessel_matrix / np.sum(bessel_matrix, axis=1, keepdims=True)

    def extract_spectrum_peaks(self, x: np.ndarray, fc_approx: float, fm_approx: float) -> tuple[float, float, float, float, float]:
        N = len(x)
        w = np.hanning(N)
        fft_x = np.fft.rfft(x * w)
        freqs = np.fft.rfftfreq(N, d=1.0 / self.sr)
        mags = np.abs(fft_x)
        phases = np.angle(fft_x)

        def peak_info(target_f):
            if target_f <= 0 or target_f >= freqs[-1]:
                return 0.0, target_f, 0.0
            idx = int(np.argmin(np.abs(freqs - target_f)))
            i_min = max(0, idx - 4)
            i_max = min(len(mags) - 1, idx + 4)
            best_i = i_min + np.argmax(mags[i_min : i_max + 1])
            return mags[best_i], freqs[best_i], phases[best_i]

        A0, fc_est, phi0 = peak_info(fc_approx)
        A1_p, f1_p, phi1_p = peak_info(fc_approx + fm_approx)
        A1_m, f1_m, phi1_m = peak_info(fc_approx - fm_approx)
        A2_p, f2_p, _ = peak_info(fc_approx + 2.0 * fm_approx)
        A3_p, f3_p, _ = peak_info(fc_approx + 3.0 * fm_approx)

        fm_est = 0.5 * ((f1_p - fc_est) + (fc_est - f1_m))
        A1 = 0.5 * (A1_p + A1_m)
        A2 = A2_p
        A3 = A3_p

        obs_profile = np.array([max(A0, 1e-4), max(A1, 1e-4), max(A2, 1e-4), max(A3, 1e-4)])
        obs_norm = obs_profile / np.sum(obs_profile)

        dists = np.sum((self.bessel_profiles - obs_norm)**2, axis=1)
        best_idx = int(np.argmin(dists))

        if 0 < best_idx < len(self.grid_beta) - 1:
            d_prev, d_curr, d_next = dists[best_idx - 1], dists[best_idx], dists[best_idx + 1]
            denom = d_prev - 2.0 * d_curr + d_next
            if denom > 1e-12:
                sub_shift = 0.5 * (d_prev - d_next) / denom
                sub_shift = max(-0.5, min(0.5, sub_shift))
                d_beta = self.grid_beta[1] - self.grid_beta[0]
                beta_est = float(self.grid_beta[best_idx] + sub_shift * d_beta)
            else:
                beta_est = float(self.grid_beta[best_idx])
        else:
            beta_est = float(self.grid_beta[best_idx])

        phi_est = float(phi1_p - phi0)
        return float(fc_est), float(fm_est), float(beta_est), phi_est, float(A0)

    def classify_fm_vs_pm(self, beta1: float, beta2: float, fm1: float, fm2: float) -> str:
        ratio_beta = beta2 / max(beta1, 1e-6)
        expected_fm_ratio = fm1 / max(fm2, 1e-6)
        dist_pm = abs(ratio_beta - 1.0)
        dist_fm = abs(ratio_beta - expected_fm_ratio)
        return "FM" if dist_fm < dist_pm else "PM"

class AnalyticAmSolver:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.grid_fam = np.arange(1.0, 32.0, 0.02)
        self.b_lp, self.a_lp = butter(4, 80.0 / (sr / 2.0), btype='low')

    def solve(self, x: np.ndarray, fc_approx: float, fam_approx: float = 5.0) -> tuple[float, float, float, float]:
        N = len(x)
        t = np.arange(N) / self.sr

        fft_x = np.abs(np.fft.rfft(x * np.hanning(N)))
        freqs = np.fft.rfftfreq(N, d=1.0 / self.sr)
        idx = int(np.argmin(np.abs(freqs - fc_approx)))
        i_min, i_max = max(0, idx - 8), min(len(fft_x) - 1, idx + 8)
        fc_est = float(freqs[i_min + np.argmax(fft_x[i_min : i_max + 1])])

        env = np.abs(hilbert(x))
        env_smooth = filtfilt(self.b_lp, self.a_lp, env)

        decim = 40
        t_sub = t[::decim]
        env_sub = env_smooth[::decim]

        valid = (t_sub >= 0.15) & (t_sub <= 0.85)
        t_v = t_sub[valid]
        env_v = env_sub[valid]

        dc = float(np.mean(env_v))
        norm_ac = (env_v - dc) / max(dc, 1e-5)

        best_res = 1e9
        best_fam = 5.0
        best_c = np.array([0.0, 0.2])

        for fam in self.grid_fam:
            M = np.column_stack([np.cos(2.0 * math.pi * fam * t_v), np.sin(2.0 * math.pi * fam * t_v)])
            c, _, _, _ = np.linalg.lstsq(M, norm_ac, rcond=None)
            fit_err = np.sum((norm_ac - M @ c)**2)
            if fit_err < best_res:
                best_res = fit_err
                best_fam = float(fam)
                best_c = c

        m_est = min(1.0, max(0.01, float(np.sqrt(best_c[0]**2 + best_c[1]**2))))
        phi_am = float(np.arctan2(best_c[0], best_c[1]))

        return fc_est, best_fam, m_est, phi_am

class FmPmResidualHead(nn.Module):
    def __init__(self, in_features: int = 96):
        super().__init__()
        self.backbone = nn.Sequential(
            nn.Linear(in_features, 48),
            nn.Tanh(),
            nn.Linear(48, 24),
            nn.Tanh()
        )
        self.param_head = nn.Linear(24, 4)
        self.classifier_head = nn.Linear(24, 2)

        with torch.no_grad():
            self.param_head.weight.fill_(0.0)
            self.param_head.bias.fill_(0.0)

    def forward(self, feat: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        h = self.backbone(feat)
        deltas = 0.05 * torch.tanh(self.param_head(h))
        logits = self.classifier_head(h)
        return deltas, logits

class AmResidualHead(nn.Module):
    def __init__(self, in_features: int = 64):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, 16),
            nn.Tanh(),
            nn.Linear(16, 4)
        )
        with torch.no_grad():
            self.net[4].weight.fill_(0.0)
            self.net[4].bias.fill_(0.0)

    def forward(self, feat: torch.Tensor) -> torch.Tensor:
        return 0.05 * torch.tanh(self.net(feat))
