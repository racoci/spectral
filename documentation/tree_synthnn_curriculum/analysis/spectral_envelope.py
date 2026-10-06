"""Módulo de Estimador de Envelope Espectral com Base Contínua sob Gauge (Estágio E07)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class GaugeFixedSpectralBasis:
    """
    Base paramétrica contínua para o envelope espectral S(u) em escala logarítmica u = log2(f / 440 Hz).
    Garante estritamente por construção as condições de gauge:
        S(0) = 0 dB           (E(440 Hz) = 0 dB)
        dS/du(0) = 0 dB/oct   (sem inclinação afim local em 440 Hz)
    """
    def __init__(self, num_basis: int = 4):
        self.num_basis = num_basis

    def evaluate_basis(self, u: np.ndarray) -> np.ndarray:
        u = np.asarray(u, dtype=float)
        u2 = u ** 2
        u3 = u ** 3
        phi_0 = u2
        phi_1 = u3
        phi_2 = u2 / (1.0 + 0.5 * u2)
        phi_3 = u3 / (1.0 + 0.5 * u2)
        return np.column_stack([phi_0, phi_1, phi_2, phi_3])

    def evaluate_curve(self, u: np.ndarray, weights: np.ndarray) -> np.ndarray:
        B_mat = self.evaluate_basis(u)
        return np.dot(B_mat, weights)

class AnalyticSpectralBasisProjector:
    """
    Projetor analítico conjunto em forma fechada sobre a base de formantes sob gauge.
    Resolve simultaneamente as amplitudes escalares de cada nota a_n e os pesos w_j:
        y_{n, k} = a_n + S(u_{n, k})
    eliminando completamente o viés de nível entre notas distintas.
    """
    def __init__(self, basis: GaugeFixedSpectralBasis, reg_lambda: float = 1e-4):
        self.basis = basis
        self.reg_lambda = reg_lambda

    def project_multinote(self, note_indices: list[int], u_points: np.ndarray, y_points: np.ndarray, num_notes: int) -> tuple[np.ndarray, np.ndarray]:
        N = len(u_points)
        B_mat = self.basis.evaluate_basis(u_points)
        num_basis = B_mat.shape[1]

        M = np.zeros((N, num_notes + num_basis), dtype=float)
        for i in range(N):
            M[i, note_indices[i]] = 1.0
            M[i, num_notes:] = B_mat[i]

        MT = M.T
        reg = self.reg_lambda * np.eye(M.shape[1])
        reg[:num_notes, :num_notes] = 0.0

        theta = np.linalg.solve(MT @ M + reg, MT @ y_points)
        a_notes = theta[:num_notes]
        w_basis = theta[num_notes:]
        return a_notes, w_basis

class SpectralEnvelopeHead(nn.Module):
    def __init__(self, in_features: int = 8, num_basis: int = 4):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, 16),
            nn.Tanh(),
            nn.Linear(16, num_basis)
        )

    def forward(self, z_spec: torch.Tensor) -> torch.Tensor:
        return self.net(z_spec)
