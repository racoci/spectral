"""Módulo de Estimação de Amplitude Estacionária (Estágio E4)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class AnalyticAmplitudeHead:
    """Estimador de amplitude de forma fechada calibrado com correção de off-bin."""
    def __init__(self, cqt_engine):
        self.cqt = cqt_engine

    def predict(self, x: np.ndarray, t0: float, f_target: float) -> tuple[float, float]:
        """
        Retorna (A_hat, f_inst_hat) usando o filtro CQT mais próximo e calibração analítica.
        """
        best_bin = self.cqt.find_nearest_bin(f_target)
        fc = self.cqt.freqs[best_bin]
        a_hat, f_hat = self.cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)
        return a_hat, f_hat

class ResidualAmplitudeHead(nn.Module):
    """Pequena camada residual neural para refinar a amplitude em regimes de alto ruído."""
    def __init__(self):
        super().__init__()
        # Entrada: [log(A_analytic), log(f_inst/fc), d_phase]
        self.net = nn.Sequential(
            nn.Linear(3, 8),
            nn.Tanh(),
            nn.Linear(8, 1) # prediz delta_logA
        )
        # Inicialização com pesos quase nulos para partida neutra (saída delta = 0)
        with torch.no_grad():
            self.net[2].weight.fill_(0.0)
            self.net[2].bias.fill_(0.0)

    def forward(self, z: torch.Tensor, a_analytic: torch.Tensor) -> torch.Tensor:
        delta_log_a = self.net(z).squeeze(-1)
        return a_analytic * torch.exp(delta_log_a)
