from typing import Tuple, Dict
import numpy as np
import torch
import torch.nn as nn
from competition_framework import StructuralRepresentation

class AffineCouplingLayer(nn.Module):
    def __init__(self, in_features: int, hidden_dim: int = 16):
        super().__init__()
        # Dividimos ao meio
        self.in_features = in_features
        self.half = in_features // 2
        
        # Rede s (scale) e t (translation)
        self.net = nn.Sequential(
            nn.Linear(self.half, hidden_dim),
            nn.Tanh(),
            nn.Linear(hidden_dim, self.in_features - self.half * 2 + in_features)
        )
        
        with torch.no_grad():
            self.net[-1].weight.fill_(0.0)
            self.net[-1].bias.fill_(0.0)

    def forward(self, x: torch.Tensor, inverse: bool = False) -> torch.Tensor:
        x1, x2 = x[:, :self.half], x[:, self.half:]
        
        st = self.net(x1)
        s, t = st[:, :self.half], st[:, self.half:]
        
        if not inverse:
            y1 = x1
            y2 = x2 * torch.exp(s) + t
        else:
            y1 = x1
            y2 = (x2 - t) * torch.exp(-s)
            
        return torch.cat([y1, y2], dim=1)

class M12_InvertibleFlow(StructuralRepresentation):
    """
    M12: Invertible Neural Baseline (Normalizing Flow / RealNVP).
    Transformação bijetiva que serve como teto de compressibilidade puramente aprendido ("black box").
    A garantia é invertibilidade exata no domínio de precisão flutuante.
    """
    def __init__(self, sr: float = 12000.0, hop_length: int = 128, n_layers: int = 4):
        super().__init__("M12_InvertibleFlow", sr)
        self.hop_length = hop_length
        self.n_layers = n_layers
        
        self.layers = nn.ModuleList([
            AffineCouplingLayer(in_features=hop_length) for _ in range(n_layers)
        ])

    def _process_block(self, xb: np.ndarray, inverse: bool) -> np.ndarray:
        x_t = torch.tensor(xb, dtype=torch.float32).unsqueeze(0)
        
        layers = reversed(self.layers) if inverse else self.layers
        
        for i, layer in enumerate(layers):
            # Alternar as metades a cada camada (simulado com roll ou flip manual)
            if i % 2 == 1:
                x_t = torch.flip(x_t, dims=[1])
                
            x_t = layer(x_t, inverse=inverse)
            
            if i % 2 == 1:
                x_t = torch.flip(x_t, dims=[1])
                
        return x_t.squeeze(0).detach().numpy()

    def fit_and_reconstruct(self, x: np.ndarray) -> Tuple[Dict, np.ndarray]:
        N = len(x)
        x_hat = np.zeros(N)
        theta = {'latent_z': []}
        
        num_blocks = N // self.hop_length
        
        # Como o Flow está cru (pesos não treinados ou treinados em 0), ele apenas mapeia a identidade agora.
        # Numa avaliação real, ele seria treinado sobre um corpus pra descobrir "z" com entropia mínima.
        for i in range(num_blocks):
            start = i * self.hop_length
            end = start + self.hop_length
            xb = x[start:end]
            
            # encode
            zb = self._process_block(xb, inverse=False)
            theta['latent_z'].append(zb.tolist())
            
            # decode exato
            xb_hat = self._process_block(zb, inverse=True)
            x_hat[start:end] = xb_hat
            
        return theta, x_hat
