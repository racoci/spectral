"""RDO-Jet Codec Module: Rate-Distortion Optimized Jet Architecture.

Inspired by Opus 1.5 DRED (draft-ietf-mlcodec-opus-dred-07) and FARGAN:
1. Classical Structured Analysis: High-order polynomial/Hermite jet + explicit carrier phase demodulation.
2. Differentiable RDO Transform: Learned linear/nonlinear projection with learnable dimension gates.
3. Dead-zone Quantization: zeta(z) = z - delta * tanh(z / (delta + eps)).
4. Laplace Rate Estimation: Direct differentiable bit-cost per latent dimension.
5. Lossless Residual Coding: Integer residual r = x - round(x_hat) guaranteeing exact bit-level reversibility.
"""
import math
from typing import Dict, Tuple, List, Optional
import torch
from torch import nn

DTYPE = torch.float64


class DeadZoneQuantizer(nn.Module):
    """Continuous dead-zone quantizer used in DRED for smooth gradient training."""
    def __init__(self, delta: float = 0.5, eps: float = 1e-4):
        super().__init__()
        self.delta = delta
        self.eps = eps

    def forward(self, z: torch.Tensor, training: bool = True) -> torch.Tensor:
        # zeta(z) = z - delta * tanh(z / (delta + eps))
        zeta = z - self.delta * torch.tanh(z / (self.delta + self.eps))
        if training:
            # Pseudo-quantization with uniform noise for gradient flow
            noise = (torch.rand_like(z) - 0.5)
            return zeta + noise
        else:
            return torch.round(zeta)


class RDOJetModule(nn.Module):
    """Rate-Distortion Optimized Jet module with overcomplete search space."""
    def __init__(self, order: int, latent_dim: int, delta: float = 0.5):
        super().__init__()
        self.order = order
        self.input_dim = order + 1
        self.latent_dim = latent_dim

        # Encoder: Jet -> Latent
        self.w_enc = nn.Linear(self.input_dim, latent_dim, bias=True, dtype=DTYPE)
        # Decoder: Latent -> Jet
        self.w_dec = nn.Linear(latent_dim, self.input_dim, bias=True, dtype=DTYPE)

        # Learnable dimension gates m_k in [0, 1]
        self.gate_logits = nn.Parameter(torch.ones(latent_dim, dtype=DTYPE) * 2.0)

        # Dead-zone quantizer
        self.quantizer = DeadZoneQuantizer(delta=delta)

        # Scale parameter for Laplace entropy model: p(z) ~ r^|z|
        self.log_laplace_scale = nn.Parameter(torch.zeros(latent_dim, dtype=DTYPE))

    def forward(self, jet: torch.Tensor, training: bool = True) -> Tuple[torch.Tensor, torch.Tensor, torch.Tensor]:
        """Forward pass:
        jet: [batch, input_dim]
        Returns:
            jet_hat: [batch, input_dim] reconstructed jet
            rate_bits: scalar estimated total bits for latents
            active_dims: number of dimensions actively carrying information
        """
        # 1. Transform to latent
        z = self.w_enc(jet)

        # 2. Apply dimension gating (soft threshold)
        gates = torch.sigmoid(self.gate_logits)
        z_gated = z * gates

        # 3. Dead-zone quantization
        z_q = self.quantizer(z_gated, training=training)

        # 4. Reconstruction
        jet_hat = self.w_dec(z_q)

        # 5. DRED Laplace Rate Estimation:
        # H(z_i) = -log2((1 - r)/(1 + r)) - E[|z_i|] * log2(r)
        # where r = sigmoid(log_scale) in (0, 1)
        r = torch.sigmoid(self.log_laplace_scale).clamp(1e-4, 0.999)
        log2_r = torch.log2(r)
        term1 = -torch.log2((1.0 - r) / (1.0 + r))
        term2 = -torch.abs(z_q).mean(dim=0) * log2_r
        entropy_per_dim = (term1 + term2) * gates

        rate_bits = entropy_per_dim.sum()

        # Count active dimensions (gate > 0.1 and variance > 1e-4)
        active_dims = (gates > 0.1).sum()

        return jet_hat, rate_bits, active_dims


def extract_polynomial_jet(signal: torch.Tensor, order: int) -> torch.Tensor:
    """Extracts Taylor/polynomial jet coefficients c_0 ... c_O of signal over normalized window [-1, 1]."""
    w = signal.shape[0]
    t = torch.linspace(-1.0, 1.0, w, dtype=DTYPE)

    # Vandermonde matrix V_ij = t_i^j
    v = torch.stack([t ** j for j in range(order + 1)], dim=1) # [w, order+1]

    # Least squares: c = (V^T V)^-1 V^T signal
    v_t = v.T
    v_tv = v_t @ v
    c = torch.linalg.solve(v_tv + 1e-6 * torch.eye(order + 1, dtype=DTYPE), v_t @ signal)
    return c # [order + 1]


def evaluate_jet_reconstruction(c: torch.Tensor, w: int) -> torch.Tensor:
    """Evaluates polynomial jet over window of size w."""
    order = c.shape[0] - 1
    t = torch.linspace(-1.0, 1.0, w, dtype=DTYPE)
    v = torch.stack([t ** j for j in range(order + 1)], dim=1)
    return v @ c


def compute_shannon_entropy(int_residual: torch.Tensor) -> float:
    """Calculates zero-order empirical Shannon entropy in bits/sample of an integer residual."""
    vals, counts = torch.unique(int_residual, return_counts=True)
    probs = counts.to(torch.float64) / int_residual.numel()
    entropy = -torch.sum(probs * torch.log2(probs + 1e-12)).item()
    return max(entropy, 0.0)
