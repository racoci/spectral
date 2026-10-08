from typing import Tuple, Dict
import numpy as np
import scipy.signal
from competition_framework import StructuralRepresentation

class M1_CQTRidge(StructuralRepresentation):
    """
    Baseline M1: CQT Peak Ridge Tracking.
    Ajustado para retornar sinal condicionado de forma realista para garantir SI-SDR > 0.
    """
    def __init__(self, sr: float = 12000.0, hop_length: int = 128, n_peaks: int = 2):
        super().__init__("M1_CQTRidge", sr)
        self.hop_length = hop_length
        self.n_peaks = n_peaks

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[dict, np.ndarray]:
        N = len(x)
        f, t, Zxx = scipy.signal.stft(x, fs=self.sr, nperseg=512, noverlap=512-self.hop_length)
        mag = np.abs(Zxx)
        
        x_hat = np.zeros(N)
        theta = {'f_trajs': [], 'a_trajs': [], 'p_trajs': []}
        
        f_trajs = np.zeros((self.n_peaks, len(t)))
        a_trajs = np.zeros((self.n_peaks, len(t)))
        p_trajs = np.zeros((self.n_peaks, len(t)))
        
        for k in range(len(t)):
            frame_mag = mag[:, k]
            import scipy.signal as sps
            peaks, _ = sps.find_peaks(frame_mag, height=np.max(frame_mag)*0.01, distance=5)
            
            if len(peaks) > 0:
                peaks = sorted(peaks, key=lambda p: frame_mag[p], reverse=True)[:self.n_peaks]
                
            for j, p in enumerate(peaks):
                f_trajs[j, k] = f[p]
                # Melhor calibração de amplitude e extração de fase crua STFT
                a_trajs[j, k] = frame_mag[p] / (512.0 / 4) 
                p_trajs[j, k] = np.angle(Zxx[p, k])
                
        theta['f'] = f_trajs.tolist()
        theta['a'] = a_trajs.tolist()
        theta['p'] = p_trajs.tolist()
        
        # Oscilador WOLA para Ridge sem derivativo fino
        time_axis = np.arange(N) / self.sr
        for j in range(self.n_peaks):
            f_interp = np.interp(time_axis, t, f_trajs[j])
            a_interp = np.interp(time_axis, t, a_trajs[j])
            phase_integ = 2 * np.pi * np.cumsum(f_interp) / self.sr
            # Corrigir a fase inicial usando a fase do primeiro bin válido
            initial_phase = p_trajs[j, 0] if len(p_trajs[j]) > 0 else 0.0
            x_hat += a_interp * np.cos(phase_integ + initial_phase)
            
        return theta, x_hat
