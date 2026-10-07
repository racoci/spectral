"""Cabeça de Timbre Dinâmico (E17.1) no espaço SynthNN."""
from __future__ import annotations
import math
import torch
from torch import nn

class DynamicHarmonicHead(nn.Module):
    """
    Prevê os coeficientes c_{kj} da base temporal para modular log H_k(t)
    garantindo que o timbre evolua durante a nota sem explodir a dimensionalidade.
    """
    def __init__(self, in_features: int, num_harmonics: int = 16, num_basis: int = 4):
        super().__init__()
        self.num_harmonics = num_harmonics
        self.num_basis = num_basis
        
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.Tanh(),
            nn.Linear(32, num_harmonics * num_basis)
        )
        
        # Inicializar os coeficientes dinâmicos próximos a 0 (estático por default)
        with torch.no_grad():
            self.net[2].weight.fill_(0.0)
            self.net[2].bias.fill_(0.0)

    def generate_basis_tensor(self, N: int, device: torch.device) -> torch.Tensor:
        t_norm = torch.linspace(0.0, 1.0, N, device=device).unsqueeze(1) # (N, 1)
        j_idx = torch.arange(1, self.num_basis + 1, device=device).float().unsqueeze(0) # (1, J)
        basis = torch.cos(math.pi * j_idx * t_norm) # (N, J)
        return basis

    def forward(self, z: torch.Tensor, log_H0: torch.Tensor, N: int) -> torch.Tensor:
        """
        z: (batch, in_features)
        log_H0: (batch, num_harmonics) amplitudes estáticas médias.
        Retorna H_k(t): (batch, num_harmonics, N)
        """
        batch_size = z.shape[0]
        # Prevê coeficientes C: (batch, num_harmonics, num_basis)
        C = self.net(z).view(batch_size, self.num_harmonics, self.num_basis)
        
        basis = self.generate_basis_tensor(N, z.device) # (N, J)
        
        # Multiplicação matricial batch: C @ B^T
        # C: (batch, K, J), basis: (N, J) -> basis^T: (J, N)
        # resultado: (batch, K, N)
        temporal_modulation = torch.matmul(C, basis.t())
        
        # Soma a média: log H_k(t) = log H_{k,0} + sum c_{kj} B_j(t)
        log_H_t = log_H0.unsqueeze(-1) + temporal_modulation
        
        # clamp para estabilidade e volta pra linear
        return torch.exp(torch.clamp(log_H_t, min=-12.0, max=0.0))
