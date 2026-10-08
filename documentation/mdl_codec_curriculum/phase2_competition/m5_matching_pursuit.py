from typing import Tuple, Dict
import numpy as np
from competition_framework import StructuralRepresentation

class M5_MatchingPursuit(StructuralRepresentation):
    """
    M5: Sparse Matching Pursuit (Gabor Atoms).
    Iterativamente subtrai a melhor correlação do dicionário, resultando em:
    x = sum c_k g_k + R
    A reconstrução é matematicamente limpa por design, e o residual diminui monotonicamente.
    """
    def __init__(self, sr: float = 12000.0, max_iter: int = 10, dict_size: int = 500):
        super().__init__("M5_MatchingPursuit", sr)
        self.max_iter = max_iter
        self.dict_size = dict_size

    def _generate_dictionary(self, N: int) -> np.ndarray:
        t = np.arange(N) / self.sr
        D = np.zeros((N, self.dict_size))
        
        # Gerar átomos Gabor (senoides envelopadas por Gaussianas)
        rng = np.random.default_rng(42)
        for i in range(self.dict_size):
            f0 = rng.uniform(50.0, 2000.0)
            t0 = rng.uniform(0.0, N / self.sr)
            sigma = rng.uniform(0.01, 0.1)
            
            env = np.exp(-0.5 * ((t - t0) / sigma)**2)
            atom = env * np.cos(2 * np.pi * f0 * t + rng.uniform(-np.pi, np.pi))
            
            norm = np.linalg.norm(atom)
            D[:, i] = atom / (norm + 1e-12)
            
        return D

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        D = self._generate_dictionary(N)
        
        R = x.copy()
        x_hat = np.zeros(N)
        theta = {'atoms': []}
        
        for i in range(self.max_iter):
            # Produto interno <R, g_k>
            corrs = np.abs(D.T @ R)
            best_idx = np.argmax(corrs)
            
            c = D[:, best_idx] @ R
            
            # Adiciona ao reconstruído e subtrai do residual
            x_hat += c * D[:, best_idx]
            R -= c * D[:, best_idx]
            
            theta['atoms'].append({'idx': int(best_idx), 'c': float(c)})
            
        return theta, x_hat
