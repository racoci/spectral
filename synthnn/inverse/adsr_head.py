"""Módulo de Cabeça Preditiva para ADSR Paramétrico (Estágio E5)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class ParametricAdsrHead(nn.Module):
    """
    Cabeça preditiva de ADSR que mapeia o perfil de log-magnitude temporal mu_A(t)
    para os parâmetros normalizados (tau_A, tau_D, S, tau_R) com restrições físicas embutidas.
    """
    def __init__(self, in_features: int = 32):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 16),
            nn.ReLU(),
            nn.Linear(16, 16),
            nn.ReLU(),
            nn.Linear(16, 4) # outputs (u_A, u_D, u_S, u_R)
        )

    def forward(self, z_env: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
        """
        Retorna (tau_A, tau_D, S, tau_R) estritamente restritos aos intervalos físicos:
        - tau_A in (0, 0.5)
        - tau_D in (0, 0.5)
        - S in (0, 1)
        - tau_R in (0, 1.0)
        """
        raw = self.net(z_env)
        tau_a = torch.sigmoid(raw[..., 0]) * 0.5
        tau_d = torch.sigmoid(raw[..., 1]) * 0.5
        sustain = torch.sigmoid(raw[..., 2])
        tau_r = torch.sigmoid(raw[..., 3]) * 1.0
        return tau_a, tau_d, sustain, tau_r

def extract_adsr_features(cqt_engine, x: np.ndarray, t_on: float, duration: float, f0: float, n_points: int = 32) -> np.ndarray:
    """
    Extrai o vetor de features z_env a partir da curva de magnitude mu_A(t) = log|C(t, f0)|
    amostrada uniformemente no intervalo de interesse da nota.
    """
    best_bin = cqt_engine.find_nearest_bin(f0)
    fc = cqt_engine.freqs[best_bin]
    
    # Amostrar no intervalo [t_on, t_on + duration * 1.5]
    t_span = duration * 1.6
    t_eval = np.linspace(t_on, t_on + t_span, n_points)
    
    log_mags = []
    for t_i in t_eval:
        C_val, gain = cqt_engine.evaluate_coefficient_and_gain(x, t_i, fc)
        mag = abs(C_val) / max(gain, 1e-12)
        log_mags.append(math.log(max(mag, 1e-5)))
        
    log_mags = np.array(log_mags, dtype=np.float32)
    # Normalização de baseline para independência de ganho master
    max_val = np.max(log_mags)
    norm_profile = log_mags - max_val # perfil normalizado em relação ao pico
    return norm_profile
