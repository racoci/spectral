"""
Classificador Relacional Bilinear de Arestas Causal para Síntese Modular (E64).
"""

from typing import List
import numpy as np

class RelationalEdgeClassifier:
    """Classificador neural bilinear para prever a probabilidade de acoplamento entre operadores."""
    def __init__(self, feature_dim: int = 8):
        self.feature_dim = feature_dim
        # Vetor de pesos calibrado para pares de embeddings acústicos
        self.weights = np.array([
            0.496,  0.355,  0.301,  0.468,  1.147, -1.072,  0.018,  0.482,
           -0.996,  0.439, -0.262, -0.709,  0.376,  0.047,  0.192, -0.853,
            0.000,  0.000,  0.118,  0.110, -0.346, -0.107,  0.004,  0.062,
           -0.500,  0.795, -0.433, -0.517,  0.065, -0.787,  0.188, -0.509
        ], dtype=np.float64)
        self.bias = 0.0

    def predict_pair(self, hi: np.ndarray, hj: np.ndarray) -> float:
        """Calcula probabilidade de o nó j modular o nó i: p_ij."""
        z = np.concatenate([hi, hj, hi * hj, np.abs(hi - hj)])
        score = float(np.dot(self.weights, z) + self.bias)
        # Sigmoide
        prob = 1.0 / (1.0 + np.exp(-score))
        return prob

    def predict_topology(self, embeddings: List[np.ndarray]) -> np.ndarray:
        """Gera matriz de probabilidades P in [0, 1]^(N x N)."""
        N = len(embeddings)
        P = np.zeros((N, N), dtype=np.float64)
        for i in range(N):
            for j in range(N):
                if i == j:
                    P[i, j] = 0.0
                else:
                    P[i, j] = self.predict_pair(embeddings[i], embeddings[j])
        return P
