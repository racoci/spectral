"""Módulo de envelopes paramétricos contínuos e ADSR."""
from __future__ import annotations
import numpy as np

def adsr_curve(t: np.ndarray, t_on: float, duration: float, t_attack: float, t_decay: float, sustain: float, t_release: float) -> np.ndarray:
    """
    Gera envelope ADSR suave C1 com tempos contínuos e interpolação cúbica nos nós.
    t: vetor de tempo absoluto
    """
    y = np.zeros_like(t, dtype=float)
    t_off = t_on + duration
    
    a = max(float(t_attack), 1e-5)
    d = max(float(t_decay), 1e-5)
    s = float(np.clip(sustain, 0.0, 1.0))
    r = max(float(t_release), 1e-5)

    tau = t - t_on
    
    # Máscaras das 4 fases:
    # 1. Attack: [0, a)
    mask_a = (tau >= 0) & (tau < a)
    ua = tau[mask_a] / a
    y[mask_a] = 3.0 * (ua**2) - 2.0 * (ua**3)
    
    # 2. Decay: [a, a + d)
    mask_d = (tau >= a) & (tau < a + d)
    ud = (tau[mask_d] - a) / d
    decay_curve = 3.0 * (ud**2) - 2.0 * (ud**3)
    y[mask_d] = 1.0 + (s - 1.0) * decay_curve
    
    # 3. Sustain: [a + d, duration)
    mask_s = (tau >= a + d) & (tau < duration)
    y[mask_s] = s
    
    # 4. Release: [duration, duration + r)
    tau_rel = t - t_off
    mask_r = (tau_rel >= 0) & (tau_rel < r)
    ur = tau_rel[mask_r] / r
    rel_curve = 3.0 * (ur**2) - 2.0 * (ur**3)
    y[mask_r] = s * (1.0 - rel_curve)
    
    return y
