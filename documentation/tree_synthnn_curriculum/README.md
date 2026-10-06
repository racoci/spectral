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
```

The package intentionally does not fabricate large training corpora automatically. The stage generator produces deterministic ground-truth configurations; audio/CQT/jet export can be attached to each stage without changing the curriculum definition.

## Current smoke validation

The package has been checked with Python unit tests and a rendering smoke test. The initial E00 smoke test on five scalar SynthNN controls produced a 94x5 finite-difference waveform Jacobian with numerical rank 5 and condition number about 9.34 for that particular configuration. This is only an implementation sanity check; it is not the full E00 identifiability benchmark.
