from typing import Tuple, Dict
import numpy as np
import scipy.linalg
from competition_framework import StructuralRepresentation

class M4_HankelSSA(StructuralRepresentation):
    """
    M4 (E22): Hankel / Singular Spectrum Analysis (SSA).
    Mapeia a série temporal 1D em uma matriz de trajetória de Hankel, realiza SVD truncado
    para capturar as variedades invariantes (modos oscilatórios, tendências e componentes de rank baixo),
    e reconstrói o sinal via média anti-diagonal (diagonal averaging / Hankelization).
    """
    def __init__(self, sr: float = 12000.0, window_len: int = 128, rank: int = 8):
        super().__init__("M4_HankelSSA", sr)
        self.L = window_len
        self.rank = rank

    def _diagonal_averaging(self, X_hat: np.ndarray, L: int, K: int, N: int) -> np.ndarray:
        """
        Executa a média anti-diagonal (Hankelização) de X_hat (L x K) para obter vetor 1D de tamanho N.
        """
        x_rec = np.zeros(N)
        counts = np.zeros(N)
        
        # Vetorizado eficiente para cada anti-diagonal
        for i in range(L):
            x_rec[i:i+K] += X_hat[i, :]
            counts[i:i+K] += 1.0
            
        x_rec /= np.maximum(counts, 1.0)
        return x_rec

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        L = min(self.L, N // 2)
        K = N - L + 1
        
        # 1. Construção da Matriz de Trajetória de Hankel (L x K)
        # X[i, j] = x[i + j]
        X = np.empty((L, K), dtype=float)
        for i in range(L):
            X[i, :] = x[i:i+K]
            
        # 2. Decomposição em Valores Singulares (SVD)
        # X = U @ diag(S) @ Vh
        U, S, Vh = scipy.linalg.svd(X, full_matrices=False)
        
        # Seleção do Rank efetivo
        r = min(self.rank, len(S))
        U_r = U[:, :r]
        S_r = S[:r]
        Vh_r = Vh[:r, :]
        
        # 3. Reconstrução da matriz de trajetória de baixo rank
        X_rec_mat = (U_r * S_r) @ Vh_r
        
        # 4. Hankelização / Média Anti-diagonal
        x_hat = self._diagonal_averaging(X_rec_mat, L, K, N)
        
        # Razão de variância explicada
        total_var = np.sum(S**2)
        explained_var = np.sum(S_r**2) / max(total_var, 1e-12)
        
        theta = {
            'singular_values': S_r.tolist(),
            'U_basis': U_r.tolist(), # Base espacial dos modos
            'rank': int(r),
            'window_L': int(L),
            'explained_variance_ratio': float(explained_var)
        }
        
        return theta, x_hat
