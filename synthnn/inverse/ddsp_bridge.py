"""Módulo Analítico para Ponte DDSP: Expressividade, Ruído Transiente e Fase Relativa."""
from __future__ import annotations
import math
import numpy as np
from scipy.signal import butter, filtfilt
import torch
from torch import nn
from synthnn.analysis.cqt import GaussianCQT

class DDSPBridgeSolver:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr

    def extract_expressive_residual(self, A_meas: np.ndarray, A_adsr: np.ndarray) -> np.ndarray:
        R = A_meas / np.maximum(A_adsr, 1e-4) - 1.0
        b, a = butter(2, 50.0 / (self.sr / 2.0), btype='low')
        R_smooth = filtfilt(b, a, R)
        return R_smooth

    def extract_transient_noise(self, x_noise: np.ndarray, A_adsr: np.ndarray, t_on: float, t_off: float) -> tuple[float, float]:
        N = len(x_noise)
        t = np.arange(N) / self.sr
        dt = 1.0 / self.sr
        dA = np.maximum(0.0, np.gradient(A_adsr, dt))

        t_att_end = t_on + 0.050
        idx_att = np.where((t >= t_on + 0.010) & (t <= t_att_end))[0]
        idx_sus = np.where((t >= t_on + 0.100) & (t <= t_off - 0.050))[0]

        if len(idx_sus) == 0 or len(idx_att) == 0:
            return 0.0, 0.0

        E_att = float(np.mean(x_noise[idx_att]**2))
        E_sus = float(np.mean(x_noise[idx_sus]**2))
        dA_att = float(np.mean(dA[idx_att]**2))

        sigma_base = math.sqrt(max(0.0, E_sus))
        c_trans = max(0.0, (E_att - E_sus) / max(dA_att, 1e-6))

        return sigma_base, c_trans

    def extract_phase_dispersion(self, x: np.ndarray, f0: float, t_onset: float, K: int, cqt: GaussianCQT) -> np.ndarray:
        N = len(x)
        idx_start = int(t_onset * self.sr)
        win_len = int(0.040 * self.sr)
        idx_end = min(N, idx_start + win_len)

        if idx_start >= N or idx_end - idx_start < 10:
            return np.zeros(K)

        seg = x[idx_start:idx_end]
        pad_N = int(self.sr)
        fft_seg = np.fft.rfft(seg, n=pad_N)

        phases_est = []
        for k in range(1, K + 1):
            fk = k * f0
            b = int(round(fk * pad_N / self.sr))
            if b < len(fft_seg):
                C = fft_seg[b]
                phi_est = float(np.angle(C) + math.pi / 2.0)
                phi_0_est = phi_est - 2.0 * math.pi * fk * t_onset
                phases_est.append(phi_0_est)
            else:
                phases_est.append(0.0)

        phi_1 = phases_est[0]
        dispersion = []
        for k in range(1, K + 1):
            raw_dp = phases_est[k - 1] - k * phi_1
            wrapped = (raw_dp + math.pi) % (2 * math.pi) - math.pi
            dispersion.append(wrapped)

        return np.array(dispersion)
