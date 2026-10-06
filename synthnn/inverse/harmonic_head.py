"""Módulo da Cabeça Preditiva Compartilhada de Harmônicos (Estágio E06)."""
from __future__ import annotations
import math
import numpy as np
import torch
from torch import nn

class SharedHarmonicDecoder(nn.Module):
    def __init__(self, latent_dim: int = 8, hidden_dim: int = 32):
        super().__init__()
        self.residual_net = nn.Sequential(
            nn.Linear(latent_dim + 1, hidden_dim),
            nn.Tanh(),
            nn.Linear(hidden_dim, 1)
        )
        with torch.no_grad():
            self.residual_net[2].weight.fill_(0.0)
            self.residual_net[2].bias.fill_(0.0)

    def forward(self, z: torch.Tensor, k_indices: torch.Tensor) -> torch.Tensor:
        batch_size = z.shape[0]
        num_k = k_indices.shape[0]

        ln_k_obs = torch.tensor([math.log(2.0), math.log(3.0), math.log(4.0)], dtype=z.dtype, device=z.device)
        denom = torch.sum(ln_k_obs**2)
        alpha = -torch.sum(z[:, 1:4] * ln_k_obs.unsqueeze(0), dim=-1, keepdim=True) / denom

        ln_k_all = torch.log(k_indices.float()).view(1, num_k).expand(batch_size, num_k)
        base_log_h = -alpha * ln_k_all

        z_expanded = z.unsqueeze(1).expand(batch_size, num_k, z.shape[-1])
        feat = torch.cat([z_expanded, ln_k_all.unsqueeze(-1)], dim=-1)
        delta_h = self.residual_net(feat).squeeze(-1)
        delta_h = delta_h - delta_h[:, 0:1]

        total_log_h = base_log_h + delta_h

        for idx, k in enumerate(k_indices):
            k_val = int(k.item())
            if 1 < k_val <= 4:
                total_log_h[:, idx] = z[:, k_val - 1]

        return torch.exp(torch.clamp(total_log_h, min=-12.0, max=0.0))

class HarmonicLatentEncoder(nn.Module):
    def __init__(self, in_features: int = 4, latent_dim: int = 8):
        super().__init__()
        self.encoder = nn.Sequential(
            nn.Linear(in_features, 16),
            nn.Tanh(),
            nn.Linear(16, latent_dim - in_features)
        )

    def forward(self, x_spec: torch.Tensor) -> torch.Tensor:
        latent_features = self.encoder(x_spec)
        return torch.cat([x_spec, latent_features], dim=-1)
