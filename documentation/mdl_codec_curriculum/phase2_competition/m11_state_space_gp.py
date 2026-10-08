from typing import Tuple, Dict, List
import numpy as np
import scipy.signal
import scipy.linalg
from competition_framework import StructuralRepresentation

class M11_StateSpaceGPSDE(StructuralRepresentation):
    """
    M11 (E29-E30): State-Space / GP-SDE Representation.
    Mapeia processos gaussianos quasi-periódicos com covariância harmônica/Matérn
    em equações diferenciais estocásticas lineares (SDEs contínuas-discretas).
    Utiliza banco de ressonadores acoplados sob inferência exata O(N) via Filtro de Kalman
    e Suavizador Rauch-Tung-Striebel (RTS Smoother), fornecendo estimativa ótima MMSE
    e variância a posteriori explícita.
    """
    def __init__(self, sr: float = 12000.0, num_resonators: int = 2, damping: float = 2.0, Q_var: float = 1e-6, R_var: float = 1e-4):
        super().__init__("M11_StateSpaceGP", sr)
        self.num_resonators = num_resonators
        self.damping = damping
        self.Q_var = Q_var
        self.R_var = R_var

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        dt = 1.0 / self.sr
        
        # 1. Estimação preliminar das frequências dos ressonadores via FFT
        X_fft = np.fft.rfft(x)
        mag_X = np.abs(X_fft)
        freqs_axis = np.fft.rfftfreq(N, d=dt)
        
        peaks, _ = scipy.signal.find_peaks(mag_X, distance=max(2, len(mag_X)//32), height=np.max(mag_X)*0.02)
        if len(peaks) >= self.num_resonators:
            top_peaks = sorted(peaks, key=lambda p: mag_X[p], reverse=True)[:self.num_resonators]
            res_freqs = [float(freqs_axis[p]) for p in sorted(top_peaks)]
        elif len(peaks) > 0:
            res_freqs = [float(freqs_axis[p]) for p in peaks]
            while len(res_freqs) < self.num_resonators:
                res_freqs.append(res_freqs[0] * (len(res_freqs) + 1))
        else:
            res_freqs = [float(440.0 * (k + 1)) for k in range(self.num_resonators)]
            
        M = len(res_freqs)
        dim = 2 * M
        
        # 2. Construção das matrizes de transição A e processo Q (bloco diagonal)
        A = np.zeros((dim, dim), dtype=float)
        Q = np.zeros((dim, dim), dtype=float)
        H = np.zeros((1, dim), dtype=float)
        
        for m, f0 in enumerate(res_freqs):
            w0 = max(2.0 * np.pi * f0, 1e-3)
            phi = w0 * dt
            decay = np.exp(-self.damping * dt)
            
            # Bloco 2x2 do oscilador amortecido
            A_m = decay * np.array([
                [np.cos(phi), np.sin(phi) / w0],
                [-w0 * np.sin(phi), np.cos(phi)]
            ])
            A[2*m : 2*m+2, 2*m : 2*m+2] = A_m
            
            # Matriz de covariância de ruído de processo Q
            Q[2*m, 2*m] = self.Q_var
            Q[2*m+1, 2*m+1] = self.Q_var * (w0**2)
            
            # Matriz de medição (soma as posições dos ressonadores)
            H[0, 2*m] = 1.0
            
        R = self.R_var
        
        # 3. Filtro de Kalman (Passo Direto)
        x_filt = np.zeros((N, dim))
        P_filt = np.zeros((N, dim, dim))
        x_pred = np.zeros((N, dim))
        P_pred = np.zeros((N, dim, dim))
        
        x_curr = np.zeros(dim)
        x_curr[0] = x[0] # inicialização razoável
        P_curr = np.eye(dim) * 1.0
        
        I = np.eye(dim)
        
        for k in range(N):
            if k > 0:
                x_curr = A @ x_curr
                P_curr = A @ P_curr @ A.T + Q
                # Garantir simetria numérica
                P_curr = 0.5 * (P_curr + P_curr.T)
                
            x_pred[k] = x_curr
            P_pred[k] = P_curr
            
            # Atualização de medição
            y = x[k]
            y_pred = float(H @ x_curr)
            e = y - y_pred
            S = float(H @ P_curr @ H.T + R)
            
            K_gain = (P_curr @ H.T) / max(S, 1e-12)
            x_curr = x_curr + (K_gain * e).flatten()
            
            # Forma de Joseph estabilizada
            IKH = I - K_gain @ H
            P_curr = IKH @ P_curr @ IKH.T + K_gain * R @ K_gain.T
            P_curr = 0.5 * (P_curr + P_curr.T)
            
            x_filt[k] = x_curr
            P_filt[k] = P_curr
            
        # 4. Suavizador RTS (Rauch-Tung-Striebel Backward Pass)
        x_smooth = np.zeros((N, dim))
        P_smooth = np.zeros((N, dim, dim))
        x_smooth[-1] = x_filt[-1]
        P_smooth[-1] = P_filt[-1]
        
        for k in range(N - 2, -1, -1):
            G = P_filt[k] @ A.T @ np.linalg.pinv(P_pred[k+1])
            x_smooth[k] = x_filt[k] + G @ (x_smooth[k+1] - x_pred[k+1])
            P_smooth[k] = P_filt[k] + G @ (P_smooth[k+1] - P_pred[k+1]) @ G.T
            P_smooth[k] = 0.5 * (P_smooth[k] + P_smooth[k].T)
            
        # Sinal reconstruído suavizado
        x_hat = np.zeros(N)
        post_var = np.zeros(N)
        for k in range(N):
            x_hat[k] = float(H @ x_smooth[k])
            post_var[k] = float(H @ P_smooth[k] @ H.T)
            
        theta = {
            'resonator_frequencies_hz': res_freqs,
            'damping': self.damping,
            'mean_posterior_variance': float(np.mean(post_var)),
            'Q_var': self.Q_var,
            'R_var': self.R_var,
            'initial_state': x_smooth[0].tolist()
        }
        
        return theta, x_hat
