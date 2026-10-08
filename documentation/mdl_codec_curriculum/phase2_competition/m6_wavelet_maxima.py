from typing import Tuple, Dict, List
import numpy as np
import scipy.signal
from competition_framework import StructuralRepresentation

class M6_WaveletMaxima(StructuralRepresentation):
    """
    M6 (E25): Wavelet Transform Modulus Maxima (WTMM).
    Rastreamento multiescala de singularidades e transientes (E14) via transformada
    wavelet contínua/diádica.
    Identifica linhas de máximos locais |W f(s, t)| através das escalas s -> 0,
    quantificando o esqueleto de singularidades e reconstruindo o sinal via frame dual perfeito.
    """
    def __init__(self, sr: float = 12000.0, num_scales: int = 16, threshold_ratio: float = 0.05):
        super().__init__("M6_WaveletMaxima", sr)
        self.num_scales = num_scales
        self.threshold_ratio = threshold_ratio

    def _build_wavelet_filterbank(self, N: int) -> Tuple[np.ndarray, np.ndarray, np.ndarray, np.ndarray, np.ndarray]:
        """
        Gera um filterbank diádico contínuo no domínio da frequência satisfazendo
        a identidade exata do operador de frame: sum_j |Psi_j(w)|^2 + |Phi_0|^2 + |Phi_high|^2 = A(w).
        """
        freqs = np.fft.rfftfreq(N, d=1.0/self.sr)
        K = len(freqs)
        
        # Escalas geometricamente espaçadas de f_min até perto de Nyquist
        f_min = max(20.0, self.sr / (N / 2))
        f_max = self.sr * 0.45
        center_freqs = np.geomspace(f_min, f_max, self.num_scales)
        
        # Filtros Gaussianos em frequência (Morlet / Gabor multiescala)
        filters = np.zeros((self.num_scales, K), dtype=float)
        frame_energy = np.zeros(K, dtype=float)
        
        for j, fc in enumerate(center_freqs):
            sigma_f = fc * 0.2
            H = np.exp(-0.5 * ((freqs - fc) / sigma_f)**2)
            filters[j, :] = H
            frame_energy += H**2
            
        # Filtro passa-baixa residual para DC e infra-graves
        lowpass = np.exp(-0.5 * (freqs / f_min)**2)
        frame_energy += lowpass**2
        
        # Filtro passa-alta residual para frequências no limite de Nyquist
        highpass = np.exp(-0.5 * ((freqs - self.sr * 0.5) / (self.sr * 0.05))**2)
        frame_energy += highpass**2
        
        norm_factor = np.maximum(frame_energy, 1e-12)
        
        return center_freqs, filters, lowpass, highpass, norm_factor

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        center_freqs, filters, lowpass, highpass, norm_factor = self._build_wavelet_filterbank(N)
        
        # 1. Análise espectral multiescala
        X_fft = np.fft.rfft(x)
        
        maxima_chains = []
        rec_fft = np.zeros_like(X_fft, dtype=complex)
        
        for j in range(self.num_scales):
            # Coeficientes na escala j
            W_j_fft = X_fft * filters[j, :]
            # Converte para domínio temporal para localizar os máximos de módulo
            w_j_time = np.fft.irfft(W_j_fft, n=N)
            modulus = np.abs(w_j_time)
            
            thresh = float(np.max(modulus) * self.threshold_ratio) if np.max(modulus) > 0 else 0.0
            min_dist = max(2, int(self.sr / (center_freqs[j] * 2)))
            
            peaks, _ = scipy.signal.find_peaks(modulus, height=thresh, distance=min_dist)
            
            maxima_chains.append({
                'scale_idx': j,
                'center_freq': float(center_freqs[j]),
                'peak_indices': peaks.tolist(),
                'peak_amplitudes': [float(modulus[p]) for p in peaks],
                'num_maxima': len(peaks)
            })
            
            # Síntese via projeção no dual frame
            rec_fft += W_j_fft * (filters[j, :] / norm_factor)
            
        # Adiciona resíduos passa-baixa e passa-alta normalizados
        rec_fft += (X_fft * lowpass) * (lowpass / norm_factor)
        rec_fft += (X_fft * highpass) * (highpass / norm_factor)
        
        # Reconstrução no tempo
        x_hat = np.fft.irfft(rec_fft, n=N)
        
        total_maxima = sum(m['num_maxima'] for m in maxima_chains)
        
        theta = {
            'num_scales': self.num_scales,
            'total_maxima': total_maxima,
            'maxima_by_scale': maxima_chains,
            'center_freqs': center_freqs.tolist()
        }
        
        return theta, x_hat
