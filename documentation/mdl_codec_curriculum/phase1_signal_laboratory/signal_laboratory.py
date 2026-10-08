import numpy as np
from typing import Tuple, List, Dict
import math

class SignalLaboratory:
    """
    API do Laboratório de Sinais (Fase 1).
    Gera sinais controlados com ground truth conhecido para avaliar exatidão
    de representações e métodos de extração (Fase 2).
    O PCM retornado é sempre convertido para uma matriz temporal e um dicionário
    contendo os tensores de ground truth exatos (f(t), phi(t), A(t), etc).
    """
    def __init__(self, sr: float = 12000.0):
        self.sr = sr

    def e06_pure_tone(self, f0: float, A: float, phi0: float, dur: float) -> Tuple[np.ndarray, dict]:
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        phase = 2 * math.pi * f0 * t + phi0
        x = A * np.cos(phase)
        truth = {
            'A': np.full(N, A),
            'f': np.full(N, f0),
            'phi': phase
        }
        return x, truth

    def e07_am_tone(self, f0: float, phi0: float, A_fn: callable, dur: float) -> Tuple[np.ndarray, dict]:
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        A = A_fn(t)
        phase = 2 * math.pi * f0 * t + phi0
        x = A * np.cos(phase)
        truth = {
            'A': A,
            'f': np.full(N, f0),
            'phi': phase
        }
        return x, truth

    def e08_linear_chirp(self, f0: float, alpha: float, A: float, phi0: float, dur: float) -> Tuple[np.ndarray, dict]:
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        f_inst = f0 + alpha * t
        # Integral de f_inst = f0*t + 0.5*alpha*t^2
        phase = 2 * math.pi * (f0 * t + 0.5 * alpha * t**2) + phi0
        x = A * np.cos(phase)
        truth = {
            'A': np.full(N, A),
            'f': f_inst,
            'df': np.full(N, alpha),
            'phi': phase
        }
        return x, truth

    def e09_polynomial_chirp(self, coeffs: List[float], A: float, phi0: float, dur: float) -> Tuple[np.ndarray, dict]:
        """ coeffs = [f0, a1, a2, a3...] para f(t) = f0 + a1*t + a2*t^2 + ... """
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        
        f_inst = np.zeros(N)
        phase_integral = np.zeros(N)
        
        for k, a_k in enumerate(coeffs):
            f_inst += a_k * (t ** k)
            phase_integral += a_k * (t ** (k + 1)) / (k + 1)
            
        phase = 2 * math.pi * phase_integral + phi0
        x = A * np.cos(phase)
        
        # d/dt analítico para J_2
        df_inst = np.zeros(N)
        for k in range(1, len(coeffs)):
            df_inst += coeffs[k] * k * (t ** (k - 1))
            
        truth = {
            'A': np.full(N, A),
            'f': f_inst,
            'df': df_inst,
            'phi': phase
        }
        return x, truth

    def e10_damped_exponential(self, f0: float, gamma: float, A: float, phi0: float, dur: float) -> Tuple[np.ndarray, dict]:
        """ Pólos: x(t) = A * exp(-gamma * t) * cos(w*t + phi) """
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        env = A * np.exp(-gamma * t)
        phase = 2 * math.pi * f0 * t + phi0
        x = env * np.cos(phase)
        truth = {
            'A': env,
            'f': np.full(N, f0),
            'gamma': np.full(N, gamma),
            'phi': phase
        }
        return x, truth

    def e11_multiple_components(self, components: List[dict], dur: float) -> Tuple[np.ndarray, List[dict]]:
        """ Soma de componentes. components é uma lista de configs (tipo E06-E10). """
        # Para simplificar na API central, vamos manter genérico,
        # O usuário gera individualmente e soma, ou passamos uma factory.
        pass

    def e12_crossing_ridges(self, f_start1: float, f_end1: float, f_start2: float, f_end2: float, dur: float) -> Tuple[np.ndarray, dict]:
        """ Gera dois chirps que cruzam no tempo. """
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        
        alpha1 = (f_end1 - f_start1) / dur
        alpha2 = (f_end2 - f_start2) / dur
        
        phase1 = 2 * math.pi * (f_start1 * t + 0.5 * alpha1 * t**2)
        phase2 = 2 * math.pi * (f_start2 * t + 0.5 * alpha2 * t**2)
        
        x1 = np.cos(phase1)
        x2 = np.cos(phase2)
        
        x = x1 + x2
        truth = {
            'comp1': {'f': f_start1 + alpha1 * t, 'phi': phase1, 'A': np.ones(N)},
            'comp2': {'f': f_start2 + alpha2 * t, 'phi': phase2, 'A': np.ones(N)}
        }
        return x, truth
        
    def e13_birth_death(self, f0: float, t_on: float, t_off: float, dur: float) -> Tuple[np.ndarray, dict]:
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        
        mask = (t >= t_on) & (t <= t_off)
        A = np.zeros(N)
        A[mask] = 1.0
        
        # Smooth window edges
        fade_len = int(0.02 * self.sr)
        idx_on = int(t_on * self.sr)
        idx_off = int(t_off * self.sr)
        if idx_on + fade_len < N:
            A[idx_on:idx_on+fade_len] = np.linspace(0, 1, fade_len)
        if idx_off - fade_len > 0:
            A[idx_off-fade_len:idx_off] = np.linspace(1, 0, fade_len)
            
        phase = 2 * math.pi * f0 * t
        x = A * np.cos(phase)
        
        truth = {
            'A': A,
            'f': np.full(N, f0),
            'state': mask.astype(int) # 0: dead, 1: alive
        }
        return x, truth

    def e14_transient(self, t_event: float, dur: float, kind='click') -> Tuple[np.ndarray, dict]:
        """ Gera uma singularidade, click isolado, ruído impulsivo ou decaimento sub-cclico. """
        N = int(dur * self.sr)
        t = np.arange(N) / self.sr
        x = np.zeros(N)
        
        idx_evt = int(t_event * self.sr)
        if kind == 'click':
            # Impulse puro (Dirac delta relaxado por filtro passa-baixa)
            imp = np.zeros(N)
            imp[idx_evt] = 1.0
            import scipy.signal
            b, a = scipy.signal.butter(4, 0.4) # limitando bw para não alias
            x = scipy.signal.filtfilt(b, a, imp)
            x /= np.max(np.abs(x))
        elif kind == 'noise_burst':
            # Burst de ruído exponencial
            rng = np.random.default_rng(42)
            env = np.zeros(N)
            env[idx_evt:] = np.exp(-150.0 * (t[idx_evt:] - t_event))
            x = env * rng.normal(0, 1.0, N)
            
        truth = {
            't_event': t_event,
            'kind': kind,
            'env': np.abs(scipy.signal.hilbert(x)) if kind == 'click' else env
        }
        return x, truth

    def add_noise(self, x: np.ndarray, snr_db: float) -> np.ndarray:
        if snr_db == float('inf'):
            return x
        sig_pow = np.mean(x**2)
        req_snr = 10 ** (snr_db / 10.0)
        noise_pow = sig_pow / req_snr
        rng = np.random.default_rng(42)
        n = rng.normal(0, math.sqrt(max(0, noise_pow)), size=len(x))
        return x + n
