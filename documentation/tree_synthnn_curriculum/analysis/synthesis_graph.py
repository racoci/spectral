"""Módulo SynthNN: Topologia Variável, Seleção de Módulo Gumbel-Softmax e Análise por Síntese (E18, E19, E20)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn
import torch.nn.functional as F

class GumbelModuleSelector(nn.Module):
    """
    Roteador diferenciável de módulos (E19).
    Seleciona o tipo de operador do nó (0: FM, 1: PM, 2: AM, 3: LFO) garantindo retropropagação de gradientes.
    """
    def __init__(self, in_features: int, num_modules: int = 4, initial_tau: float = 1.0):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, num_modules)
        )
        self.tau = initial_tau

    def forward(self, x: torch.Tensor, hard: bool = True) -> tuple[torch.Tensor, torch.Tensor]:
        logits = self.net(x)
        # Gumbel-Softmax trick
        # Em teste, hard=True gera one-hot exato mas mantém o gradiente de soft.
        y = F.gumbel_softmax(logits, tau=self.tau, hard=hard)
        return y, logits

class VariableTopologyTracker:
    """
    Traçador topológico autorregressivo BFS (E18).
    Reconstrói a profundidade e conectividade do grafo a partir da matriz de adjacência emparelhada inferida,
    organizando a ordem de execução do grafo (computação do sinal).
    """
    def __init__(self, max_nodes: int = 12):
        self.max_nodes = max_nodes

    def compile_execution_order(self, adj_matrix: np.ndarray, root_indices: list[int]) -> list[list[int]]:
        """
        Retorna a ordem de avaliação bottom-up (das folhas para as raízes)
        para compilação diferenciável.
        """
        levels = {0: root_indices}
        visited = set(root_indices)
        queue = list(root_indices)
        current_level = 1
        
        while queue:
            next_q = []
            for u in queue:
                for v in range(len(adj_matrix)):
                    if adj_matrix[u, v] > 0 and v not in visited:
                        next_q.append(v)
                        visited.add(v)
            if next_q:
                levels[current_level] = next_q
            queue = next_q
            current_level += 1
            if current_level > 10: # Quebra de ciclo redundante
                break

        # Inverte a ordem para avaliação (as folhas devem ser computadas antes dos pais)
        execution_order = []
        for lvl in sorted(levels.keys(), reverse=True):
            execution_order.append(levels[lvl])

        return execution_order

class AnalysisBySynthesisLoss(nn.Module):
    """
    Função de perda unificada para o pipeline E20 (Análise por Síntese Conjunta).
    Compara o áudio gerado pelo SynthNN contra o sinal original nas representações:
    - Domínio temporal L1 (alinhado por fase)
    - Espectro de Magnitude (L1 log-spectral)
    """
    def __init__(self, sr: float = 12000.0, n_fft: int = 1024):
        super().__init__()
        self.sr = sr
        self.n_fft = n_fft

    def forward(self, x_synth: torch.Tensor, x_target: torch.Tensor) -> dict[str, torch.Tensor]:
        # 1. Perda temporal L1 (necessita alinhamento fino)
        loss_time = F.l1_loss(x_synth, x_target)
        
        # 2. Perda Espectral (Multiscale-STFT simplificada)
        window = torch.hann_window(self.n_fft).to(x_synth.device)
        S_synth = torch.stft(x_synth, n_fft=self.n_fft, hop_length=self.n_fft//4, window=window, return_complex=True)
        S_target = torch.stft(x_target, n_fft=self.n_fft, hop_length=self.n_fft//4, window=window, return_complex=True)
        
        mag_synth = torch.abs(S_synth) + 1e-5
        mag_target = torch.abs(S_target) + 1e-5
        
        # Spectral Convergence Loss
        loss_sc = torch.norm(mag_target - mag_synth, p="fro") / torch.norm(mag_target, p="fro")
        
        # Log Magnitude Loss
        loss_log_mag = F.l1_loss(torch.log(mag_synth), torch.log(mag_target))
        
        total_loss = loss_time + loss_sc + 0.1 * loss_log_mag
        
        return {
            'loss_total': total_loss,
            'loss_time': loss_time,
            'loss_sc': loss_sc,
            'loss_log_mag': loss_log_mag
        }
