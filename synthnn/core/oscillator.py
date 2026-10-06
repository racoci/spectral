"""Módulo de osciladores contínuos com integração analítica exata de fase."""
from __future__ import annotations
import math
import numpy as np

def generate_sine(t: np.ndarray, amp: float, f0: float, phase0: float = 0.0) -> np.ndarray:
    """Gera senoide estacionária pura x(t) = A * sin(2*pi*f0*t + phase0)."""
    return float(amp) * np.sin(2.0 * math.pi * float(f0) * t + float(phase0))

def generate_chirp(t: np.ndarray, amp: float, f0: float, f_dot: float = 0.0, f_ddot: float = 0.0, phase0: float = 0.0) -> np.ndarray:
    """Gera sinal com modulação polinomial de fase: phi(t) = 2*pi*(f0*t + 0.5*f_dot*t^2 + (1/6)*f_ddot*t^3) + phase0."""
    phase = 2.0 * math.pi * (float(f0) * t + 0.5 * float(f_dot) * (t**2) + (1.0 / 6.0) * float(f_ddot) * (t**3)) + float(phase0)
    return float(amp) * np.sin(phase)
