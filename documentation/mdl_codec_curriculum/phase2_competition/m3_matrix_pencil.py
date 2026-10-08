from typing import Tuple, Dict
import numpy as np
from competition_framework import StructuralRepresentation

class M3_MatrixPencil(StructuralRepresentation):
    """
    M3: Matrix Pencil Method.
    Extrator de pólos locais (r*e^{jw}) via SVD da matriz Hankel, 
    ideal para cruzamentos e componentes transientes densos, não sofre com
    a grade de tempo-frequência.
    """
    def __init__(self, sr: float = 12000.0, hop_length: int = 128, K: int = 2):
        super().__init__("M3_MatrixPencil", sr)
        self.hop_length = hop_length
        self.K = K # Num polos (componentes exponenciais) por bloco. Como trabalhamos em reais, K = 2 * n_senoides.

    def _matrix_pencil_block(self, x_block: np.ndarray, L: int) -> Tuple[np.ndarray, np.ndarray]:
        """ Retorna (z, amplitudes_complexas) """
        N = len(x_block)
        if N <= L: return np.zeros(self.K, dtype=complex), np.zeros(self.K, dtype=complex)
        
        # 1. Matriz Hankel
        Y = np.zeros((L + 1, N - L))
        for i in range(L + 1):
            Y[i, :] = x_block[i:i+N-L]
            
        # 2. SVD truncado
        U, S, Vh = np.linalg.svd(Y, full_matrices=False)
        U_s = U[:, :self.K]
        
        # 3. Y1 e Y2
        Y1 = U_s[:-1, :]
        Y2 = U_s[1:, :]
        
        # 4. Encontrar polos z (autovalores de Y1^+ Y2)
        # Y1^+ Y2 v = z v
        Y1_pinv = np.linalg.pinv(Y1)
        z = np.linalg.eigvals(Y1_pinv @ Y2)
        
        # 5. Encontrar amplitudes (Least squares local)
        # x_n = sum_k c_k z_k^n
        n_vec = np.arange(N)
        Z_mat = np.power.outer(z, n_vec).T # (N, K)
        
        c, _, _, _ = np.linalg.lstsq(Z_mat, x_block, rcond=None)
        
        return z, c

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        block_size = self.hop_length * 2
        L = block_size // 2 # Pencil parameter L, typically N/3 to N/2
        
        num_blocks = N // self.hop_length
        
        x_hat = np.zeros(N)
        weight = np.zeros(N)
        
        theta = {'blocks': []}
        
        for i in range(num_blocks - 1):
            start = i * self.hop_length
            end = start + block_size
            if end > N: break
            
            xb = x[start:end]
            z, c = self._matrix_pencil_block(xb, L)
            
            # Reconstrói bloco local
            n_vec = np.arange(len(xb))
            Z_mat = np.power.outer(z, n_vec).T
            xb_hat = np.real(Z_mat @ c)
            
            # Cross-fade overlap-add (Hann window)
            w = np.hanning(len(xb))
            x_hat[start:end] += xb_hat * w
            weight[start:end] += w
            
            # Coletar parâmetros
            freqs = np.angle(z) * self.sr / (2 * np.pi)
            damp = -np.log(np.abs(z)) * self.sr
            amps = np.abs(c)
            phases = np.angle(c)
            
            theta['blocks'].append({
                'f': freqs.tolist(),
                'gamma': damp.tolist(),
                'a': amps.tolist(),
                'p': phases.tolist()
            })
            
        # Normalizar WOLA
        mask = weight > 1e-6
        x_hat[mask] /= weight[mask]
        
        return theta, x_hat
