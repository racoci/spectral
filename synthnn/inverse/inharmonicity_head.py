"""Módulo de Estimador de Inarmonicidade B (Estágio E08)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class AnalyticInharmonicitySolver:
    def __init__(self, K: int = 8):
        self.K = K
        self.k_vec = np.arange(1, K + 1, dtype=float)

    def solve(self, partial_freqs: np.ndarray) -> tuple[float, float, np.ndarray]:
        K = len(partial_freqs)
        k_vec = self.k_vec[:K]
        y = partial_freqs / k_vec

        X1 = np.column_stack([np.ones(K), k_vec**2])
        c1 = np.linalg.lstsq(X1, y, rcond=None)[0]
        f0_1 = max(10.0, float(c1[0]))
        B_1 = max(1e-8, float(2.0 * c1[1] / f0_1))

        if B_1 > 1.5e-3 and K >= 4:
            X2 = np.column_stack([np.ones(K), k_vec**2, k_vec**4])
            c2 = np.linalg.lstsq(X2, y, rcond=None)[0]
            f0_est = max(10.0, float(c2[0]))
            B_est = max(1e-8, float(2.0 * c2[1] / f0_est))
            y_fit = X2 @ c2
        else:
            f0_est = f0_1
            B_est = B_1
            y_fit = X1 @ c1

        residual_vec = y - y_fit
        return f0_est, B_est, residual_vec

class InharmonicityResidualHead(nn.Module):
    def __init__(self, in_features: int = 16):
        super().__init__()
        self.mlp = nn.Sequential(
            nn.Linear(in_features, 16),
            nn.Tanh(),
            nn.Linear(16, 8),
            nn.Tanh(),
            nn.Linear(8, 1)
        )
        with torch.no_grad():
            self.mlp[0].weight.normal_(0.0, 0.01)
            self.mlp[2].weight.normal_(0.0, 0.01)
            self.mlp[4].weight.fill_(0.0)
            self.mlp[4].bias.fill_(0.0)

    def forward(self, feat: torch.Tensor, B_analytic: torch.Tensor) -> torch.Tensor:
        delta = 0.05 * torch.tanh(self.mlp(feat).squeeze(-1))
        return B_analytic * torch.exp(delta)
