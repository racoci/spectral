from __future__ import annotations

import json
import sys
from pathlib import Path

import torch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from synthnn import SynthNN, example_config
from curriculum.registry import ensure_runtime_parameters


def finite_difference_jacobian(model: SynthNN, names, eps_rel=1e-4):
    params = dict(model.named_parameters())
    rows = []
    base_state = {n: p.detach().clone() for n, p in params.items()}
    for name in names:
        p = params[name]
        if p.numel() != 1:
            continue
        x0 = float(p.detach())
        eps = eps_rel * max(abs(x0), 1.0)
        with torch.no_grad():
            p.copy_(torch.tensor(x0 + eps, dtype=p.dtype))
        yp = model.render(duration=0.25, normalize=False)[0][::32].detach()
        with torch.no_grad():
            p.copy_(torch.tensor(x0 - eps, dtype=p.dtype))
        ym = model.render(duration=0.25, normalize=False)[0][::32].detach()
        with torch.no_grad():
            p.copy_(torch.tensor(x0, dtype=p.dtype))
        rows.append(((yp - ym) / (2 * eps)).reshape(-1))
    with torch.no_grad():
        for n, value in base_state.items():
            params[n].copy_(value)
    J = torch.stack(rows, dim=1) if rows else torch.empty((0, 0))
    return J


def main():
    model = SynthNN(example_config())
    ensure_runtime_parameters(model)
    names = [
        "master_gain",
        "master_drive",
        "voices.0.timbre.fm_index",
        "voices.0.timbre.inharmonicity",
        "global_reverb_mix",
    ]
    J = finite_difference_jacobian(model, names)
    s = torch.linalg.svdvals(J) if J.numel() else torch.empty(0)
    tol = max(float(s.max()) * 1e-8, 1e-12) if s.numel() else 0.0
    report = {
        "selected": names,
        "shape": list(J.shape),
        "singular_values": s.tolist(),
        "rank_numeric_tol": tol,
        "rank": int((s > tol).sum().item()) if s.numel() else 0,
        "condition": float((s.max() / s.min()).item()) if s.numel() and torch.all(s > tol) else float("inf"),
        "method": "central finite-difference waveform Jacobian",
    }
    out = ROOT / "experiments" / "e00_identifiability.json"
    out.write_text(json.dumps(report, indent=2), encoding="utf-8")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
