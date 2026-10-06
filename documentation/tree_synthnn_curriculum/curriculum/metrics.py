from __future__ import annotations

import math
from typing import Dict

import torch


def cents_error(pred: torch.Tensor, true: torch.Tensor) -> torch.Tensor:
    return 1200.0 * torch.abs(torch.log2(torch.clamp(pred, min=1e-8) / torch.clamp(true, min=1e-8)))


def circular_error(pred_phase: torch.Tensor, true_phase: torch.Tensor) -> torch.Tensor:
    return torch.angle(torch.exp(1j * (pred_phase - true_phase))).abs()


def relative_error(pred: torch.Tensor, true: torch.Tensor, eps: float = 1e-8) -> torch.Tensor:
    return torch.abs(pred - true) / torch.clamp(torch.abs(true), min=eps)


def normalized_graph_cost(n_nodes: int, n_edges: int, n_params: int, *, a=.5, b=1., c=.1) -> float:
    return a * n_nodes + b * n_edges + c * n_params


def stage_score(metrics: Dict[str, float], thresholds: Dict[str, float]) -> bool:
    for key, lim in thresholds.items():
        value = metrics[key]
        if key.endswith("_max"):
            if value > lim:
                return False
        else:
            if value < lim:
                return False
    return True
