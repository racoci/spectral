# Stage execution checklist

For every E0x stage, produce these artifacts before promotion:

`manifest.json` — parameter ranges, random seeds, counts, architecture, frozen groups, loss weights.

`metrics.json` — train/validation/IID/compositional/structural-OOD/hard metrics.

`interventions.json` — one-parameter counterfactual isolation measurements.

`predictions.npz` — compact predictions/targets for the test suite.

`checkpoint.pt` — only the trainable inverse head or residual module, never a newly trained copy of the frozen SynthNN.

`report.md` — automatically rendered pass/fail decision.

Promotion rule:

\[
PASS_k =
PASS_{IID}\land PASS_{comp}\land PASS_{OOD}
\land PASS_{hard}\land PASS_{intervention}
\land PASS_{freeze}.
\]

A failed stage blocks the next complexity increment. The repair workflow is deliberately narrow:

1. Re-check normalization and gauge constraints.
2. Re-check the synthetic distribution and renderer against the declared ground truth.
3. Increase model capacity by one small increment.
4. Repeat the same validation suite.

Do not solve a failure by unfreezing unrelated modules.
