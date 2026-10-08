from typing import Tuple, Dict, List
import numpy as np
import scipy.signal
from competition_framework import StructuralRepresentation

class M7_AdaptiveFourierDecomposition(StructuralRepresentation):
    """
    M7 (E26): Adaptive Fourier Decomposition (AFD).
    Decomposição racional adaptativa em espaço de Hardy H^2(D) via sistema ortogonal/racional
    de Takenaka-Malmquist (TM).
    A cada iteração k, seleciona um polo a_k no disco unitário (|a_k| < 1) que maximiza
    a projeção de energia no núcleo reprodutor de Szegö (Maximal Energy Principle - MEP),
    garantindo decaimento monotônico estrito do residual e modelagem de ressonâncias com amortecimento.
    """
    def __init__(self, sr: float = 12000.0, num_atoms: int = 12, max_radius: float = 0.998):
        super().__init__("M7_AFD", sr)
        self.num_atoms = num_atoms
        self.max_radius = max_radius

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        # Espectro analítico no círculo unitário z_m = exp(j * 2*pi*m / N)
        X = np.fft.fft(x)
        M = len(X)
        half_M = M // 2
        
        theta_grid = 2.0 * np.pi * np.arange(M) / M
        z = np.exp(1j * theta_grid)
        
        R = X.copy()
        atoms = []
        coeffs = []
        poles = []
        
        # Grade de raios concêntricos incluindo polos de alto Q
        radii = [0.85, 0.95, 0.99, 0.998, 0.9995]
        
        for it in range(self.num_atoms):
            mag_R = np.abs(R[:half_M])
            if np.max(mag_R) < 1e-6:
                break
                
            # Seleciona bins de frequência com maior energia residual
            peaks, _ = scipy.signal.find_peaks(mag_R, distance=max(2, half_M // 32), height=np.max(mag_R) * 0.05)
            if len(peaks) == 0:
                top_bins = np.argsort(mag_R)[-8:]
            else:
                top_bins = peaks[np.argsort(mag_R[peaks])[-8:]]
                
            # Expande para incluir vizinhança sub-bin fina
            candidate_bins = []
            for b in top_bins:
                candidate_bins.extend([float(b), float(b) - 0.25, float(b) + 0.25])
                
            best_c = 0.0 + 0.0j
            best_ea = None
            best_a = None
            best_proj = -1.0
            
            for p_val in candidate_bins:
                th = 2.0 * np.pi * p_val / M
                for r in radii:
                    a = r * np.exp(1j * th)
                    # Szegö kernel racional: e_a(z) = sqrt(1 - |a|^2) / (1 - conj(a)*z)
                    ea = np.sqrt(max(1.0 - r**2, 1e-8)) / (1.0 - np.conj(a) * z)
                    ea_norm = np.linalg.norm(ea)
                    
                    c = np.sum(R * np.conj(ea)) / (ea_norm**2)
                    proj = np.abs(c) * ea_norm
                    if proj > best_proj:
                        best_proj = proj
                        best_c = c
                        best_ea = ea
                        best_a = a
                        
            if best_ea is None or best_proj < 1e-8:
                break
                
            # Subtrai projeção do residual
            R = R - best_c * best_ea
            atoms.append(best_ea)
            coeffs.append(best_c)
            poles.append(best_a)
            
        if len(atoms) == 0:
            return {'num_atoms': 0}, np.zeros(N)
            
        # Otimização global por mínimos quadrados sobre a base racional TM selecionada
        A_mat = np.column_stack(atoms)
        c_opt, _, _, _ = np.linalg.lstsq(A_mat, X, rcond=None)
        
        X_hat = A_mat @ c_opt
        # Reconstrução real com fator analítico 2.0
        x_hat = 2.0 * np.fft.ifft(X_hat).real
        
        theta = {
            'num_atoms': len(poles),
            'poles_r': [float(np.abs(p)) for p in poles],
            'poles_theta': [float(np.angle(p)) for p in poles],
            'poles_re': [float(np.real(p)) for p in poles],
            'poles_im': [float(np.imag(p)) for p in poles],
            'coeffs_mag': [float(np.abs(c)) for c in c_opt],
            'coeffs_phase': [float(np.angle(c)) for c in c_opt]
        }
        
        return theta, x_hat
