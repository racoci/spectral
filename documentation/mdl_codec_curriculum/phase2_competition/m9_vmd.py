from typing import Tuple, Dict, List
import numpy as np
import scipy.signal
from competition_framework import StructuralRepresentation

class M9_VMD(StructuralRepresentation):
    """
    M9 (E27): Variational Mode Decomposition (VMD).
    Decompõe o sinal de forma não-recursiva em K Modos Intrinsecamente Variacionais (IMFs)
    de banda estreita, formulado como um problema de otimização variacional restrito resolvido
    via ADMM (Alternating Direction Method of Multipliers) no domínio da frequência.
    """
    def __init__(self, sr: float = 12000.0, K: int = 4, alpha: float = 1500.0, max_iter: int = 40, tol: float = 1e-6):
        super().__init__("M9_VMD", sr)
        self.K = K
        self.alpha = alpha
        self.max_iter = max_iter
        self.tol = tol

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        f_axis = np.fft.rfftfreq(N, d=1.0/self.sr)
        omega = 2.0 * np.pi * f_axis
        X = np.fft.rfft(x)
        T = len(X)
        
        # 1. Inicialização inteligente das frequências centrais nos picos do espectro
        mag_X = np.abs(X)
        peaks, props = scipy.signal.find_peaks(mag_X, distance=max(2, T // 32), height=np.max(mag_X) * 0.02)
        if len(peaks) >= self.K:
            sorted_peaks = sorted(peaks, key=lambda p: mag_X[p], reverse=True)[:self.K]
            init_bins = sorted(sorted_peaks)
        elif len(peaks) > 0:
            init_bins = list(peaks)
            while len(init_bins) < self.K:
                init_bins.append(int((len(init_bins) + 1) * T / (self.K + 1)))
        else:
            init_bins = [int((k + 1) * T / (self.K + 1)) for k in range(self.K)]
            
        omega_k = np.array([omega[min(b, T - 1)] for b in init_bins], dtype=float)
        
        # 2. Tensores de modos no domínio da frequência
        u_hat = np.zeros((self.K, T), dtype=complex)
        lambda_hat = np.zeros(T, dtype=complex)
        
        # 3. Laço ADMM
        converged_iter = self.max_iter
        for it in range(self.max_iter):
            u_prev = u_hat.copy()
            sum_u = np.sum(u_hat, axis=0)
            
            for k in range(self.K):
                # Subtrai os demais modos
                sum_others = sum_u - u_hat[k, :]
                res = X - sum_others + lambda_hat / 2.0
                
                # Filtro de Wiener centrado em omega_k[k]
                denom = 1.0 + 2.0 * self.alpha * (omega - omega_k[k])**2
                u_hat[k, :] = res / denom
                
                # Atualiza soma acumulada
                sum_u = sum_others + u_hat[k, :]
                
                # Centro de gravidade espectral da potência do modo
                p_spec = np.abs(u_hat[k, :])**2
                sum_p = np.sum(p_spec)
                if sum_p > 1e-12:
                    omega_k[k] = np.sum(omega * p_spec) / sum_p
                    
            # Atualização do multiplicador de Lagrange
            lambda_hat += 0.5 * (X - sum_u)
            
            # Critério de convergência relativa
            norm_u = np.sum(np.abs(u_hat)**2)
            diff = np.sum(np.abs(u_hat - u_prev)**2) / max(norm_u, 1e-12)
            if it > 2 and diff < self.tol:
                converged_iter = it + 1
                break
                
        # 4. Síntese dos modos no tempo
        u_time = np.zeros((self.K, N), dtype=float)
        imf_energies = []
        imf_amps = []
        
        for k in range(self.K):
            u_k = np.fft.irfft(u_hat[k, :], n=N)
            u_time[k, :] = u_k
            imf_energies.append(float(np.sum(u_k**2)))
            imf_amps.append(float(np.sqrt(np.mean(u_k**2))))
            
        x_hat = np.sum(u_time, axis=0)
        
        theta = {
            'K': self.K,
            'alpha': self.alpha,
            'center_freqs_hz': [float(w / (2.0 * np.pi)) for w in omega_k],
            'imf_energies': imf_energies,
            'imf_rms_amps': imf_amps,
            'converged_iter': converged_iter
        }
        
        return theta, x_hat
