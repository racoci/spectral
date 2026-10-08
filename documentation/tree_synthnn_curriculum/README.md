# Hierarchical SynthNN + TreeNN Curriculum

This package turns the staged training strategy into an executable research scaffold around the existing `SynthNN` implementation.

The core policy is strict block-coordinate learning: in each stage only the smallest semantic parameter groups required by the experiment are trainable; all other SynthNN parameters are frozen.

## Structure

- `synthnn.py`: frozen generator implementation used as the synthesis oracle.
- `curriculum/spec.py`: E00-E21 experimental protocol, exact dataset budgets, model sizes, optimizer settings, losses and promotion criteria.
- `curriculum/registry.py`: maps the current SynthNN parameter names to semantic groups and implements freeze/unfreeze.
- `datasets/generator.py`: deterministic synthetic configuration generation for the early/mid curriculum.
- `training/trainer.py`: stage setup and optimizer construction.
- `experiments/e00_identifiability.py`: initial numerical rank/conditioning screen.
- `experiments/smoke.py`: verifies parameter grouping and rendering.
- `docs/CURRICULUM.md`: operational protocol and rationale.

## Key invariants

1. The SynthNN forward model is frozen during identification stages.
2. No stage learns a parameter family before previous families have passed IID, compositional, OOD and hard-case validation.
3. All phase losses use circular representations rather than a raw `[0, 2π)` regression target.
4. Positive parameters use log-domain errors; frequency uses cents.
5. Voice permutation is evaluated with set matching.
6. Graphs are evaluated both edgewise and as exact topology.
7. Analysis-by-synthesis is introduced only after the individual parameter heads and TreeNN have stabilized.

## Run the scaffold

```bash
python experiments/smoke.py
python experiments/e00_identifiability.py
python experiments/verify_early_frontend.py
python experiments/run_early_smoke.py
python experiments/validate_early.py
```

For a larger single-stage training run, for example:

```bash
python -m training.early_train --stage E05 --train-count 2048 --val-count 256 --epochs 120 --batch-size 256 --lr 1e-3 --out-dir results/e05_2048
```

The package intentionally does not fabricate large training corpora automatically. The stage generator produces deterministic ground-truth configurations; audio/CQT/jet export can be attached to each stage without changing the curriculum definition.

## Current smoke validation

The package has been checked with Python unit tests and a rendering smoke test. The initial E00 smoke test on five scalar SynthNN controls produced a 94x5 finite-difference waveform Jacobian with numerical rank 5 and condition number about 9.34 for that particular configuration. This is only an implementation sanity check; it is not the full E00 identifiability benchmark.

## Implemented E02-E05 validation

The current runnable implementation uses a fixed STFT/log-frequency frontend and deliberately tiny inverse heads. A 256-example smoke run plus an independent validation pass produced:

| Stage | Trainable scalars | Primary result | Status |
|---|---:|---:|---|
| E02 ridge | 369 | IID residual RMSE 0.016 cents; off-grid OOD median RMSE 0.012 cents | PASS |
| E03 pitch | 25 | IID RMSE 0.016 cents; OOD RMSE < 3 cents | PASS |
| E04 amplitude | 2 | IID/OOD RMSE about 2e-6 dB | PASS |
| E05 ADSR | 2212 | IID median relative error 2.6%; boundary envelope RMSE 0.025 | PASS |

E04 is fitted by closed-form least squares because the clean scalar-amplitude relation is exactly affine in the chosen log-magnitude coordinate. E03 retains the analytic ridge and learns only a residual correction; E05 is the first genuinely temporal neural head. The full validation report is in `results/early_validation/REPORT.md` and `report.json`. The E05 promoted smoke checkpoint was trained on 2,048 synthetic examples; the 256-example smoke checkpoint remains for reproducibility.


## E06-E08 status

E06-E08 are now implemented as zero-parameter analytic promotion gates. E06 recovers harmonic amplitude ratios from exact FFT bins; E07 recovers a gauge-fixed quadratic spectral envelope coefficient by least squares; E08 estimates inharmonicity B using sub-bin harmonic frequencies while keeping f0 frozen from E03.

Validation: all E06-E08 IID/OOD gates pass.
