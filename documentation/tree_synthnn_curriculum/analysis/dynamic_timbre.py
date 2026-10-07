"""Módulo Analítico E17.1: Timbre Dinâmico H_k(t) em Base Temporal Congelada."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class DynamicHarmonicSolver:
    """
    Inversor Analítico de Timbre Dinâmico.
    Extrai as trajetórias log-amplitude de cada harmônico e projeta na base temporal congelada.
    """
    def __init__(self, sr: float = 12000.0, num_basis: int = 4):
        self.sr = sr
        self.num_basis = num_basis

    def generate_basis(self, N: int) -> np.ndarray:
        """
        Gera a base temporal congelada B_j(t) usando funções Cosseno Ortogonais.
        Retorna matriz (N, J).
        """
        t_norm = np.linspace(0.0, 1.0, N)
        basis = np.zeros((N, self.num_basis))
        for j in range(self.num_basis):
            # Base 0: constante, Base 1: 1/2 ciclo cos, etc.
            basis[:, j] = np.cos(math.pi * (j + 1) * t_norm)
        return basis

    def project_trajectory(self, log_H_k_t: np.ndarray) -> tuple[float, np.ndarray]:
        """
        Dada a trajetória observada log(H_k(t)), extrai:
        - log H_{k,0} (média temporal / termo constante)
        - Coeficientes c_{kj} da base
        """
        N = len(log_H_k_t)
        basis = self.generate_basis(N)
        
        # log H_{k,0} = media
        log_H_0 = float(np.mean(log_H_k_t))
        residual = log_H_k_t - log_H_0
        
        # Projeta na base: c = (B^T B)^-1 B^T residual
        # Como as bases DCT sao quase ortogonais, OLS direto:
        c, _, _, _ = np.linalg.lstsq(basis, residual, rcond=None)
        
        return log_H_0, c

    def reconstruct_trajectory(self, log_H_0: float, c: np.ndarray, N: int) -> np.ndarray:
        basis = self.generate_basis(N)
        return log_H_0 + basis @ c

