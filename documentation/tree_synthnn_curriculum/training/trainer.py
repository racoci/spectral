from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Iterable

import torch
import torch.nn as nn

from curriculum.registry import set_trainable_groups, trainable_summary
from curriculum.spec import StageSpec


@dataclass
class StageRunConfig:
    grad_clip: float = 1.0
    log_every: int = 50
    max_steps: int = 2000


def build_optimizer(model: nn.Module, stage: StageSpec):
    params = [p for p in model.parameters() if p.requires_grad]
    if not params:
        return None
    if stage.optimizer == "AdamW":
        return torch.optim.AdamW(params, lr=stage.lr, weight_decay=stage.weight_decay)
    if stage.optimizer == "Adam":
        return torch.optim.Adam(params, lr=stage.lr, weight_decay=stage.weight_decay)
    raise ValueError(stage.optimizer)


def configure_stage(model: nn.Module, stage: StageSpec) -> Dict[str, int]:
    counts = set_trainable_groups(model, stage.trainable_groups)
    summary = trainable_summary(model)
    print(f"[{stage.id}] {stage.name}: trainable={summary}")
    return counts


def check_no_unexpected_gradients(model: nn.Module, allowed_groups: Iterable[str], classifier) -> None:
    allowed = set(allowed_groups)
    for name, p in model.named_parameters():
        if p.grad is None:
            continue
        group = classifier(name)
        if group not in allowed:
            raise RuntimeError(f"Unexpected gradient in frozen group {group}: {name}")
