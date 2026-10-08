from __future__ import annotations
from typing import Tuple, Dict, Any, List, Optional
import numpy as np
import time
from concurrent.futures import ThreadPoolExecutor, as_completed

class StructuralRepresentation:
    """
    Interface Base para todos os competidores da Fase 2 (M1 a M12).
    Cada competidor deve herdar desta classe e implementar:
    `fit_and_reconstruct(x: np.ndarray) -> Tuple[dict, np.ndarray]`
    onde:
      - `theta` (dict): Parâmetros estruturais compactos do modelo.
      - `x_hat` (np.ndarray): Sinal 1D reconstruído com a mesma dimensão de x.
    """
    def __init__(self, name: str, sr: float):
        self.name = name
        self.sr = sr
        
    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        """ Retorna (parâmetros_estruturais, sinal_reconstruído) """
        raise NotImplementedError


def count_structural_parameters(params: Any) -> int:
    """
    Função utilitária recursiva para quantificar o número total de escalares
    (floats/ints/pesos) contidos no dicionário de parâmetros estruturais.
    Serve como proxy inicial para o termo de complexidade L(M) no cálculo do MDL.
    """
    if isinstance(params, (int, float, bool, np.number)):
        return 1
    elif isinstance(params, (list, tuple)):
        return sum(count_structural_parameters(item) for item in params)
    elif isinstance(params, np.ndarray):
        return int(params.size)
    elif isinstance(params, dict):
        return sum(count_structural_parameters(v) for v in params.values())
    return 0


class CompetitionFramework:
    """
    Framework de Competição da Fase 2 para benchmarking objetivo de
    representações de áudio baseadas em MDL.
    Avalia SI-SDR (dB), tempo computacional (ms) e contagem de parâmetros estruturais.
    """
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.competitors: List[StructuralRepresentation] = []

    def register_competitor(self, comp: StructuralRepresentation):
        self.competitors.append(comp)

    def _evaluate_single_competitor(self, comp: StructuralRepresentation, x: np.ndarray) -> Tuple[str, dict]:
        t0 = time.time()
        try:
            theta, x_hat = comp.fit_and_reconstruct(x)
            success = True
            # Garantir formato correto e dimensionalidade
            if not isinstance(x_hat, np.ndarray):
                x_hat = np.asarray(x_hat, dtype=float)
            if len(x_hat) != len(x):
                # Ajustar truncamento ou preenchimento de segurança
                if len(x_hat) > len(x):
                    x_hat = x_hat[:len(x)]
                else:
                    x_hat = np.pad(x_hat, (0, len(x) - len(x_hat)))
            # Substituir NaNs ou Infs por zeros para segurança numérica
            if not np.all(np.isfinite(x_hat)):
                x_hat = np.nan_to_num(x_hat, nan=0.0, posinf=0.0, neginf=0.0)
        except Exception as e:
            theta, x_hat = {'error': str(e)}, np.zeros_like(x)
            success = False
            print(f"[{comp.name}] Falhou com exceção: {e}")
            
        elapsed = time.time() - t0
        
        # Residual 
        R = x - x_hat
        mse = float(np.mean(R**2))
        sig_e = float(np.mean(x**2))
        
        # Scale-Invariant / Signal-to-Distortion Ratio em dB
        sisdr = float(10.0 * np.log10(max(sig_e, 1e-12) / max(mse, 1e-12)))
        num_params = count_structural_parameters(theta)
        
        result_payload = {
            'success': success,
            'sisdr_db': sisdr,
            'compute_time_s': elapsed,
            'params': theta,
            'num_params': num_params
        }
        return comp.name, result_payload

    def evaluate_signal(self, name: str, x: np.ndarray, truth: dict, parallel: bool = False, max_workers: Optional[int] = None) -> dict:
        results = {}
        if parallel and len(self.competitors) > 1:
            with ThreadPoolExecutor(max_workers=max_workers) as executor:
                futures = {executor.submit(self._evaluate_single_competitor, comp, x): comp for comp in self.competitors}
                for fut in as_completed(futures):
                    comp_name, res = fut.result()
                    results[comp_name] = res
        else:
            for comp in self.competitors:
                comp_name, res = self._evaluate_single_competitor(comp, x)
                results[comp_name] = res
                
        return results
