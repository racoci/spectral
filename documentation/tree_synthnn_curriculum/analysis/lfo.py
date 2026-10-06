"""Módulo de Estimador de Modulação Periódica LFO (Estágio E09)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class AnalyticLfoEstimator:
    """
    Estimador analítico exato de parâmetros de LFO por desmodulação de cruzamento de zero
    e projeção periódica ortogonal (Lomb-Scargle contínuo).
    Imune à atenuação passa-baixas de janelas temporais gaussianas.
    """
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.grid_fm = np.arange(1.5, 12.0, 0.05)

    def estimate(self, x: np.ndarray, f0: float) -> tuple[float, float, float, np.ndarray, np.ndarray]:
        """
        Retorna:
            fm_est: taxa em Hz
            d_est: profundidade em cents
            phi_est: fase em radianos
            t_eval: tempos de amostragem
            y_eval: desvios em cents
        """
        zero_crossings = np.where((x[:-1] < 0) & (x[1:] >= 0))[0]
        if len(zero_crossings) < 10:
            return 5.0, 20.0, 0.0, np.linspace(0.2, 0.8, 32), np.zeros(32)

        # Interpolação linear da fração de cruzamento de zero
        t_cross = []
        for zc in zero_crossings:
            x0, x1 = x[zc], x[zc + 1]
            denom = x1 - x0
            frac = -x0 / denom if abs(denom) > 1e-8 else 0.0
            t_cross.append((zc + frac) / self.sr)

        t_cross = np.array(t_cross)
        periods = np.diff(t_cross)
        t_mid = 0.5 * (t_cross[:-1] + t_cross[1:])
        f_measured = 1.0 / np.maximum(periods, 1e-6)
        cents_measured = 1200.0 * np.log2(np.maximum(10.0, f_measured) / f0)

        # Recortar no intervalo estável [0.25, 0.75]
        mask = (t_mid >= 0.25) & (t_mid <= 0.75)
        if np.sum(mask) < 8:
            mask = np.ones(len(t_mid), dtype=bool)

        t_eval = t_mid[mask]
        y_eval = cents_measured[mask] - np.mean(cents_measured[mask])

        # Projeção periódica ortogonal de máxima verossimilhança
        best_res = 1e9
        best_fm = 5.0
        best_c = np.array([0.0, 20.0])

        for fm in self.grid_fm:
            M = np.column_stack([np.cos(2.0 * math.pi * fm * t_eval), np.sin(2.0 * math.pi * fm * t_eval)])
            c, _, _, _ = np.linalg.lstsq(M, y_eval, rcond=None)
            fit_err = np.sum((y_eval - M @ c)**2)
            if fit_err < best_res:
                best_res = fit_err
                best_fm = float(fm)
                best_c = c

        d_est = max(0.5, float(np.sqrt(best_c[0]**2 + best_c[1]**2)))
        phi_est = float(np.arctan2(best_c[0], best_c[1]))

        return best_fm, d_est, phi_est, t_eval, y_eval

class LfoParametricHead(nn.Module):
    """
    Cabeça neural residual para refinamento dos parâmetros de LFO (fm, depth, phase).
    """
    def __init__(self, in_features: int = 64):
        super().__init__()
        self.backbone = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, 16),
            nn.Tanh()
        )
        self.rate_head = nn.Linear(16, 1)
        self.depth_head = nn.Linear(16, 1)
        self.phase_vector_head = nn.Linear(16, 2)

        with torch.no_grad():
            self.rate_head.weight.fill_(0.0)
            self.rate_head.bias.fill_(0.0)
            self.depth_head.weight.fill_(0.0)
            self.depth_head.bias.fill_(0.0)
            self.phase_vector_head.weight.fill_(0.0)
            self.phase_vector_head.bias.fill_(0.0)

    def forward(
        self,
        x_feat: torch.Tensor,
        fm_analytic: torch.Tensor,
        d_analytic: torch.Tensor,
        vec_analytic: torch.Tensor
    ) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        h = self.backbone(x_feat)

        # Refinamento residual estrito em torno da solução analítica de alta precisão
        delta_fm = 0.05 * torch.tanh(self.rate_head(h).squeeze(-1))
        fm = fm_analytic * torch.exp(delta_fm)

        delta_d = 0.05 * torch.tanh(self.depth_head(h).squeeze(-1))
        depth = d_analytic * torch.exp(delta_d)

        # Correção angular suave no círculo S^1
        delta_vec = 0.05 * torch.tanh(self.phase_vector_head(h))
        vec_tot = vec_analytic + delta_vec
        norm = torch.norm(vec_tot, dim=-1, keepdim=True) + 1e-6
        phase_vec = vec_tot / norm

        return fm, depth, phase_vec
