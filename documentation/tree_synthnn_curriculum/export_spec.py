from pathlib import Path
import json
import sys

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT))
from curriculum.spec import STAGES
from curriculum.distributions import DISTRIBUTIONS

payload = {
    "protocol_version": "1.0",
    "global": {
        "seed": 20261006,
        "sample_rate_hz": 12000,
        "default_batch_size": 32,
        "early_stopping_patience": 12,
        "max_grad_norm": 1.0,
        "normalization": {
            "frequency": "cents_or_log2_ratio",
            "positive": "log_ratio",
            "phase": "unit_circle",
            "amplitude": "dB",
        },
    },
    "parameter_distributions": DISTRIBUTIONS,
    "stages": [s.to_dict() for s in STAGES],
}
(ROOT / "curriculum_spec.json").write_text(json.dumps(payload, indent=2), encoding="utf-8")
print(ROOT / "curriculum_spec.json")
