"""
Módulo de Codec de Cristas com Representação Aprendida e Surprise Encoding (E51 a E59).
"""

from typing import Tuple, List, Optional
import numpy as np

class LearnedRidgeDictionary:
    """Dicionário ortonormal de bases para representação compacta de cristas temporais."""
    def __init__(self, block_len: int = 16, num_components: int = 4):
        self.block_len = int(block_len)
        self.num_components = int(num_components)
        self.mean = np.zeros(self.block_len, dtype=np.float64)

        # Base canônica inicial ortonormal (DCT-II normalizada)
        self.basis = np.zeros((self.block_len, self.num_components), dtype=np.float64)
        for k in range(self.num_components):
            for n in range(self.block_len):
                self.basis[n, k] = np.cos(np.pi * k * (n + 0.5) / self.block_len)
            norm = np.linalg.norm(self.basis[:, k])
            if norm > 1e-9:
                self.basis[:, k] /= norm

    def fit(self, training_data: np.ndarray):
        """Ajusta as bases do dicionário via decomposição espectral / PCA sobre blocos de cristas."""
        # training_data shape: (N, block_len)
        self.mean = np.mean(training_data, axis=0)
        centered = training_data - self.mean

        # Matriz de covariância
        cov = (centered.T @ centered) / len(training_data)

        # Autovalores e autovetores
        eigvals, eigvecs = np.linalg.eigh(cov)

        # Ordena em ordem decrescente de autovalor
        idx_sorted = np.argsort(eigvals)[::-1]
        top_indices = idx_sorted[:self.num_components]
        self.basis = eigvecs[:, top_indices]

    def transform(self, blocks: np.ndarray) -> np.ndarray:
        """Projeta blocos no espaço latente de coeficientes."""
        centered = blocks - self.mean
        return centered @ self.basis

    def inverse_transform(self, coeffs: np.ndarray) -> np.ndarray:
        """Reconstrói os blocos originais a partir dos coeficientes latentes."""
        return coeffs @ self.basis.T + self.mean


class RidgeInnovationQuantizer:
    """Quantizador uniforme de coeficientes e inovações latentes."""
    def __init__(self, step_size: float = 0.1):
        self.step_size = float(step_size)

    def quantize(self, coeffs: np.ndarray) -> np.ndarray:
        return np.round(coeffs / self.step_size).astype(np.int64)

    def dequantize(self, indices: np.ndarray) -> np.ndarray:
        return indices.astype(np.float64) * self.step_size


class SurpriseRidgeCodec:
    """Codificador e decodificador causal com gating bayesiano de surpresa (E58-E59)."""
    def __init__(self, threshold_lambda: float = 0.08, heartbeat_interval: int = 15, sigma: float = 2.0):
        self.threshold_lambda = float(threshold_lambda)
        self.heartbeat_interval = int(heartbeat_interval)
        self.sigma = float(sigma)

        # Estado do decodificador interno
        self.dec_f = 0.0
        self.dec_slope = 0.0
        self.frames_since_transmission = 0
        self.initialized = False

    def process_frame(self, f_obs: float) -> Tuple[bool, float]:
        f_obs = float(f_obs)

        if not self.initialized:
            self.dec_f = f_obs
            self.dec_slope = 0.0
            self.frames_since_transmission = 0
            self.initialized = True
            return True, f_obs

        # Predição livre do decodificador baseada em dinâmica causal
        f_pred = self.dec_f + self.dec_slope

        # Medida de divergência / surpresa da inovação: delta G
        delta_g = ((f_obs - f_pred) ** 2) / (2.0 * (self.sigma ** 2))

        # Decisão de gating
        must_transmit = (delta_g > self.threshold_lambda) or (self.frames_since_transmission >= self.heartbeat_interval)

        if must_transmit:
            # Transmite atualização de surpresa
            new_slope = f_obs - self.dec_f
            self.dec_slope = 0.7 * self.dec_slope + 0.3 * new_slope
            self.dec_f = f_obs
            self.frames_since_transmission = 0
            return True, self.dec_f
        else:
            # Suprime transmissão; decodificador propaga predição autônoma
            self.dec_f = f_pred
            self.frames_since_transmission += 1
            return False, self.dec_f
