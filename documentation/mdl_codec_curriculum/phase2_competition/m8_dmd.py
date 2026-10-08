from typing import Tuple, Dict
import numpy as np
import scipy.linalg
from competition_framework import StructuralRepresentation

class M8_DMD(StructuralRepresentation):
    """
    M8 (E28): Dynamic Mode Decomposition (DMD) / Koopman Operator.
    Aproxima o operador linear de transição de estados de dimensão infinita de Koopman
    z_{t+1} = A z_t sobre coordenadas de atraso (snapshots de Hankel).
    Extrai frequências contínuas, taxas de crescimento/decaimento e modos dinâmicos
    espaciais sem amostragem discreta de Fourier.
    """
    def __init__(self, sr: float = 12000.0, window_len: int = 64, rank: int = 8):
        super().__init__("M8_DMD", sr)
        self.L = window_len
        self.rank = rank

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        dt = 1.0 / self.sr
        L = min(self.L, N // 3)
        M = N - L # número de snapshots temporais
        
        # 1. Construção dos snapshots temporais deslocados X1 e X2
        # X1[:, m] = x[m : m + L]
        # X2[:, m] = x[m + 1 : m + 1 + L]
        X1 = np.empty((L, M), dtype=float)
        X2 = np.empty((L, M), dtype=float)
        for i in range(L):
            X1[i, :] = x[i:i+M]
            X2[i, :] = x[i+1:i+1+M]
            
        # 2. SVD truncado de X1
        U, S, Vh = scipy.linalg.svd(X1, full_matrices=False)
        r = min(self.rank, len(S))
        Ur = U[:, :r]
        Sr = S[:r]
        Vr = Vh[:r, :].T.conj()
        
        # 3. Operador de Koopman projetado no subespaço POD
        # Atilde = Ur^* @ X2 @ Vr @ inv(Sr)
        Sr_inv = np.diag(1.0 / np.maximum(Sr, 1e-12))
        Atilde = Ur.T.conj() @ X2 @ Vr @ Sr_inv
        
        # 4. Decomposição espectral de Atilde
        eigvals, W = np.linalg.eig(Atilde)
        
        # 5. Modos DMD exatos (Tu et al. 2014)
        Phi = X2 @ Vr @ Sr_inv @ W
        
        # 6. Frequências contínuas e taxas de crescimento
        # lambda_k = exp(omega_k * dt) => omega_k = ln(lambda_k) / dt
        # Clamp |eigvals| para evitar divergência exponencial na síntese
        eigvals_clamped = eigvals.copy()
        mags = np.abs(eigvals_clamped)
        eigvals_clamped[mags > 1.05] /= (mags[mags > 1.05] / 1.05)
        
        omega = np.log(eigvals_clamped.astype(complex)) / dt
        freqs_hz = np.imag(omega) / (2.0 * np.pi)
        growth_rates = np.real(omega)
        
        # 7. Amplitudes iniciais via mínimos quadrados no snapshot 0
        b, _, _, _ = np.linalg.lstsq(Phi, X1[:, 0], rcond=None)
        
        # 8. Reconstrução da trajetória temporal
        Time_dynamics = np.zeros((r, M), dtype=complex)
        for m in range(M):
            Time_dynamics[:, m] = (eigvals_clamped**m) * b
            
        X_rec = (Phi @ Time_dynamics).real
        
        # Média anti-diagonal
        x_hat = np.zeros(N)
        counts = np.zeros(N)
        for i in range(L):
            x_hat[i:i+M] += X_rec[i, :]
            counts[i:i+M] += 1.0
            
        # Último ponto extrapolado pelo modo
        counts[N-1] = 1.0
        x_hat /= counts
        x_hat[N-1] = (Phi[-1, :] @ ((eigvals_clamped**M) * b)).real
        
        theta = {
            'rank': int(r),
            'frequencies_hz': freqs_hz.tolist(),
            'growth_rates': growth_rates.tolist(),
            'amplitudes': np.abs(b).tolist(),
            'phases': np.angle(b).tolist(),
            'eigenvalues_mag': np.abs(eigvals).tolist()
        }
        
        return theta, x_hat
