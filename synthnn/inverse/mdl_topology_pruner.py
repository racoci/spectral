"""
Módulo de Poda Estrutural por Mínimo Comprimento de Descrição (MDL) para TreeNN (E62).
"""

import numpy as np
from synthnn.core.tree_nn_graph import TreeNNCausalGraph

class MdlTopologyPruner:
    """Algoritmo de poda de arestas de matriz densa baseado no compromisso Taxa-Distorção MDL."""
    def __init__(self, lambda_g: float = 0.015, threshold_tau: float = 0.12):
        self.lambda_g = float(lambda_g)
        self.threshold_tau = float(threshold_tau)

    def compute_distortion(self, graph: TreeNNCausalGraph, target_audio: np.ndarray, duration: float) -> float:
        synth = graph.synthesize(duration=duration)
        return float(np.mean((target_audio - synth) ** 2))

    def prune(self, graph: TreeNNCausalGraph, target_audio: np.ndarray, duration: float = 0.3) -> np.ndarray:
        current_W = np.copy(graph.W)
        N = graph.num_ops

        # 1. Poda por limiar de energia fraca
        for i in range(N):
            for j in range(N):
                if abs(current_W[i, j]) < self.threshold_tau:
                    current_W[i, j] = 0.0

        # 2. Avaliação de poda gulosa para arestas intermediárias
        # Custo de cada aresta em bits
        edge_bits_cost = np.log2(256.0)  # ~8 bits por coeficiente quantizado

        test_graph = TreeNNCausalGraph(ops=graph.ops, W=current_W, output_weights=graph.output_weights, sample_rate=graph.sample_rate)
        base_dist = self.compute_distortion(test_graph, target_audio, duration)

        # Ordena arestas ativas por magnitude crescente
        active_edges = []
        for i in range(N):
            for j in range(N):
                if abs(current_W[i, j]) > 0.0:
                    active_edges.append((abs(current_W[i, j]), i, j))
        active_edges.sort(key=lambda x: x[0])

        for mag, i, j in active_edges:
            # Salva valor
            old_val = current_W[i, j]
            current_W[i, j] = 0.0

            test_graph.W = current_W
            new_dist = self.compute_distortion(test_graph, target_audio, duration)

            delta_dist = new_dist - base_dist
            # Critério MDL: aceita poda se delta_dist <= lambda_g
            if delta_dist <= self.lambda_g:
                # Poda aceita! Atualiza base_dist
                base_dist = new_dist
            else:
                # Rejeita poda: restaura aresta
                current_W[i, j] = old_val

        return current_W
