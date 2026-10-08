from typing import Tuple, Dict, List
import numpy as np
import scipy.signal
from competition_framework import StructuralRepresentation

class M2_ChirpletRidge(StructuralRepresentation):
    """
    M2 (E18): High-Order Chirplet Ridge Tracking.
    Incorpora explicitamente a frequência instantânea f_0, o chirp rate f_dot e a curvatura f_ddot
    no rastreamento diferencial da fase:
        phi(t) = phi_0 + 2*pi*(f_0*t + 0.5*f_dot*t^2 + (1/6)*f_ddot*t^3)
    Garante representação contínua de alta ordem para chirps lineares, quadráticos e cúbicos (E08, E09)
    sem dispersão espectral de sub-bins.
    """
    def __init__(self, sr: float = 12000.0, hop_length: int = 128, block_size: int = 256):
        super().__init__("M2_ChirpletRidge", sr)
        self.hop_length = hop_length
        self.block_size = block_size

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        dt = 1.0 / self.sr
        num_blocks = max(1, (N - self.block_size) // self.hop_length + 1)
        
        # Analytic signal para desmodulação da fase instantânea
        xa = scipy.signal.hilbert(x)
        
        x_hat = np.zeros(N)
        weight = np.zeros(N)
        
        f0_list = []
        fdot_list = []
        fddot_list = []
        amp_list = []
        phi0_list = []
        
        hann_w = np.hanning(self.block_size)
        
        for b in range(num_blocks):
            start = b * self.hop_length
            end = start + self.block_size
            if end > N:
                break
                
            xb = xa[start:end]
            t_local = np.arange(self.block_size) * dt # tempo local centrado em 0
            
            # Amplitude média/RMS local
            amp = float(np.mean(np.abs(xb)))
            
            # Fase desenrolada
            phase_local = np.unwrap(np.angle(xb))
            
            # Ajuste polinomial cúbico de fase:
            # phase(t) = c3 * t^3 + c2 * t^2 + c1 * t + c0
            # onde:
            # c1 = 2*pi*f0
            # c2 = pi * f_dot  => f_dot = c2 / pi
            # c3 = (1/3)*pi * f_ddot => f_ddot = 3 * c3 / pi
            poly = np.polyfit(t_local, phase_local, 3)
            c3, c2, c1, c0 = poly
            
            f0 = float(c1 / (2.0 * np.pi))
            f_dot = float(c2 / np.pi)
            f_ddot = float(3.0 * c3 / np.pi)
            phi0 = float(c0)
            
            f0_list.append(f0)
            fdot_list.append(f_dot)
            fddot_list.append(f_ddot)
            amp_list.append(amp)
            phi0_list.append(phi0)
            
            # Reconstrução local via modelo polinomial de fase
            phase_model = c3 * (t_local**3) + c2 * (t_local**2) + c1 * t_local + c0
            block_hat = amp * np.cos(phase_model)
            
            x_hat[start:end] += block_hat * hann_w
            weight[start:end] += hann_w
            
        # Tratamento de calda residual se N não for múltiplo exato
        if num_blocks * self.hop_length < N:
            # Global polynomial fallback para extremidades se houver descontinuidade
            phase_global = np.unwrap(np.angle(xa))
            t_global = np.arange(N) * dt
            amp_global = np.abs(xa)
            poly_g = np.polyfit(t_global, phase_global, 3)
            global_hat = amp_global * np.cos(np.polyval(poly_g, t_global))
            
            mask = weight < 1e-4
            x_hat[mask] = global_hat[mask]
            weight[mask] = 1.0

        mask_valid = weight > 1e-6
        x_hat[mask_valid] /= weight[mask_valid]
        
        theta = {
            'f0': f0_list,
            'f_dot': fdot_list,
            'f_ddot': fddot_list,
            'amp': amp_list,
            'phi0': phi0_list,
            'num_frames': len(f0_list)
        }
        
        return theta, x_hat
