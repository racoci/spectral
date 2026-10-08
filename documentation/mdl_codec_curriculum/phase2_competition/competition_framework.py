from __future__ import annotations
from typing import Tuple, Dict
import numpy as np
import time

class StructuralRepresentation:
    """ Interface Base para todos os competidores. """
    def __init__(self, name: str, sr: float):
        self.name = name
        self.sr = sr
        
    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        """ Retorna (parâmetros_estruturais, sinal_reconstruído) """
        raise NotImplementedError

class CompetitionFramework:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.competitors = []

    def register_competitor(self, comp: StructuralRepresentation):
        self.competitors.append(comp)

    def evaluate_signal(self, name: str, x: np.ndarray, truth: dict) -> dict:
        results = {}
        for comp in self.competitors:
            t0 = time.time()
            try:
                theta, x_hat = comp.fit_and_reconstruct(x)
                success = True
            except Exception as e:
                theta, x_hat = {}, np.zeros_like(x)
                success = False
                print(f"[{comp.name}] Falhou: {e}")
                
            elapsed = time.time() - t0
            
            # Residual 
            R = x - x_hat
            mse = np.mean(R**2)
            # Sinal total 
            sig_e = np.mean(x**2)
            # Evitar log 0
            sisdr = 10 * np.log10(max(sig_e, 1e-12) / max(mse, 1e-12))
            
            results[comp.name] = {
                'success': success,
                'sisdr_db': float(sisdr),
                'compute_time_s': elapsed,
                'params': theta
            }
            
        return results
