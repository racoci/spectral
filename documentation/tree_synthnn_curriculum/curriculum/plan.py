from __future__ import annotations

from typing import Dict
from .spec import STAGES
from .distributions import DISTRIBUTIONS


def stage_manifest(stage_id: str) -> Dict:
    stage = next(s for s in STAGES if s.id == stage_id)
    return {
        "stage": stage.to_dict(),
        "distributions": DISTRIBUTIONS,
        "protocol": {
            "batch_size": 32,
            "max_steps": 2000 if stage.train_examples < 50000 else 4000,
            "early_stopping_patience": 12,
            "gradient_clip": 1.0,
            "fixed_validation_seeds": [100000 + i for i in range(stage.val_examples // max(1, min(stage.val_examples, 64)))],
            "required_test_suites": ["iid", "compositional", "structural_ood", "hard", "counterfactual", "freeze_integrity"],
        },
    }
