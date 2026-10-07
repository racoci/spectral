"""Cabeça de Transiente de Ataque (E17.2) no espaço SynthNN."""
from __future__ import annotations
import math
import torch
from torch import nn

class AttackTransientHead(nn.Module):
    def __init__(self, in_features: int, num_basis: int = 4, psd_bins: int = 64):
        super().__init__()
        self.num_basis = num_basis
        self.psd_bins = psd_bins
        
        self.net = nn.Sequential(
            nn.Linear(in_features, 64),
            nn.Tanh()
        )
        self.c_out = nn.Linear(64, num_basis)
        self.psd_out = nn.Linear(64, psd_bins)
        
        with torch.no_grad():
            self.c_out.weight.fill_(0.0)
            self.c_out.bias.fill_(0.0)
            self.psd_out.weight.fill_(0.0)
            self.psd_out.bias.fill_(-2.0) # log pequeno

    def forward(self, z: torch.Tensor) -> tuple[torch.Tensor, torch.Tensor]:
        """
        z: (batch, in_features)
        Retorna:
            c: (batch, num_basis) coeficientes positivos
            psd: (batch, psd_bins) espectro normalizado
        """
        feat = self.net(z)
        c = torch.relu(self.c_out(feat)) # coeficientes positivos
        psd_logits = self.psd_out(feat)
        psd = torch.sigmoid(psd_logits) # espectro em [0, 1]
        
        return c, psd
