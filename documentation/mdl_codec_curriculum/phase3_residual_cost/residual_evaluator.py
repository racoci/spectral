from __future__ import annotations
import numpy as np
from collections import Counter
import math

class ResidualEvaluator:
    """
    Avalia o custo do resíduo (Fase 3).
    Garante o Contrato Digital (E00): o sinal alvo x e a reconstrução x_hat 
    são quantizados no domínio PCM 16-bit. O resíduo R = x - x_hat 
    é armazenado como inteiro. Calcula a Entropia de Shannon para
    estimar o custo L(R) em bits sob compressão ótima (Lossless Entropy Coding - E56).
    """
    def __init__(self, bit_depth: int = 16):
        self.bit_depth = bit_depth
        # Fator de escala para mapear float [-1, 1] para int
        self.scale = (1 << (self.bit_depth - 1)) - 1

    def to_pcm(self, x_float: np.ndarray) -> np.ndarray:
        """ Converte sinal em float point para PCM Inteiro saturado. """
        x_scaled = np.round(x_float * self.scale)
        x_clipped = np.clip(x_scaled, -self.scale - 1, self.scale)
        return x_clipped.astype(np.int32)

    def compute_residual(self, x_true_float: np.ndarray, x_hat_float: np.ndarray) -> np.ndarray:
        """ Retorna o resíduo exato R[n] no domínio inteiro (E33). """
        x_true_int = self.to_pcm(x_true_float)
        x_hat_int = self.to_pcm(x_hat_float)
        return x_true_int - x_hat_int

    def shannon_entropy(self, data: np.ndarray) -> float:
        """
        Calcula a entropia de ordem zero H(R) em bits por amostra (E34).
        """
        if len(data) == 0:
            return 0.0
        
        counts = Counter(data)
        N = len(data)
        entropy = 0.0
        
        for count in counts.values():
            p_x = count / N
            entropy -= p_x * math.log2(p_x)
            
        return entropy

    def param_cost_estimate(self, num_params: int, bits_per_param: float = 32.0) -> float:
        """
        Estimativa do custo L(theta) para armazenar os parâmetros.
        Assume por padrão que cada parâmetro custa 32 bits (float normal), 
        que pode ser refinado com quantização customizada futuramente.
        """
        return float(num_params * bits_per_param)

    def evaluate(self, x_true: np.ndarray, x_hat: np.ndarray, num_params: int) -> dict:
        """
        Retorna o MDL completo: L_total = L(theta) + L(R)
        Tudo calculado em Bits totais do arquivo/bloco.
        """
        N = len(x_true)
        R_int = self.compute_residual(x_true, x_hat)
        
        h_r = self.shannon_entropy(R_int)
        
        # Custo do resíduo L(R) em bits = Entropia * Número de amostras
        L_R = h_r * N
        # Custo dos Parâmetros
        L_theta = self.param_cost_estimate(num_params)
        
        L_total = L_theta + L_R
        
        # Baseline do arquivo bruto
        x_true_int = self.to_pcm(x_true)
        h_x = self.shannon_entropy(x_true_int)
        L_raw = h_x * N
        
        # Compression Ratio (quanto menor, mais comprimido. < 1.0 é compressão efetiva)
        compression_ratio = L_total / max(L_raw, 1.0)
        
        return {
            'L_theta_bits': float(L_theta),
            'L_residual_bits': float(L_R),
            'L_total_bits': float(L_total),
            'L_raw_baseline_bits': float(L_raw),
            'residual_entropy_bps': float(h_r),
            'raw_entropy_bps': float(h_x),
            'compression_ratio': float(compression_ratio)
        }
