"""Módulo Analítico E17.2: Campo de Transiente de Ataque (Chiff/Pluck)."""
from __future__ import annotations
import math
import numpy as np

class AttackTransientSolver:
    """
    Extrator Analítico de Transiente.
    Modela o residual dos primeiros milissegundos como um envelope de ruído
    A_a(t) projetado numa base B_j(t) e um espectro estático N_a(f).
    """
    def __init__(self, sr: float = 12000.0, num_basis: int = 4, transient_dur: float = 0.1):
        self.sr = sr
        self.num_basis = num_basis
        self.transient_dur = transient_dur
        self.transient_samples = int(self.sr * self.transient_dur)

    def generate_basis(self) -> np.ndarray:
        t_norm = np.linspace(0.0, 1.0, self.transient_samples)
        # Funções B-spline simples (ou polinomiais) decrescentes
        basis = np.zeros((self.transient_samples, self.num_basis))
        for j in range(self.num_basis):
            # Base polinomial amortecida: t^j * exp(-a t)
            basis[:, j] = (t_norm ** j) * np.exp(-5.0 * t_norm)
        return basis

    def project_transient_envelope(self, residual_audio: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """
        Dada a forma de onda do residual, calcula o envelope de energia local
        e projeta na base de ataque.
        Retorna (coeficientes c, densidade espectral PSD).
        """
        # Extrair segmento transiente
        x_trans = residual_audio[:self.transient_samples]
        if len(x_trans) < self.transient_samples:
            x_trans = np.pad(x_trans, (0, self.transient_samples - len(x_trans)))
            
        # Envelope de energia (RMS local)
        env = np.convolve(x_trans**2, np.ones(int(0.005*self.sr))/(0.005*self.sr), mode='same')
        env = np.sqrt(np.maximum(env, 1e-12))
        
        basis = self.generate_basis()
        c, _, _, _ = np.linalg.lstsq(basis, env, rcond=None)
        c = np.maximum(c, 0.0) # Envelope deve ser positivo
        
        # PSD (Power Spectral Density) do transiente
        pad_N = 1024
        psd = np.abs(np.fft.rfft(x_trans, n=pad_N))
        # Normalizar
        psd = psd / np.maximum(np.max(psd), 1e-12)
        
        return c, psd
