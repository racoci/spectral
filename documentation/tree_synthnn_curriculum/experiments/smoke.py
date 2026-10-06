from __future__ import annotations

import json
import sys
from pathlib import Path

import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from synthnn import SynthNN, example_config
from curriculum.spec import STAGES
from curriculum.registry import parameter_groups, set_trainable_groups, trainable_summary
from datasets.generator import render_example


def smoke():
    model = SynthNN(example_config())
    groups = parameter_groups(model)
    print("PARAMETER GROUP COUNTS")
    print(json.dumps({k: sum(dict(model.named_parameters())[n].numel() for n in v) for k,v in groups.items()}, indent=2))

    for stage in STAGES[:5]:
        set_trainable_groups(model, stage.trainable_groups)
        print(stage.id, stage.name, trainable_summary(model))

    y, meta, cfg = render_example("E06", seed=123)
    assert y.ndim == 2 and y.shape[0] == 2
    assert torch.isfinite(y).all()
    print("render ok", tuple(y.shape), meta)


if __name__ == "__main__":
    smoke()
