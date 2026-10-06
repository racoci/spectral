# E09 — LFO identification

## E09-A: continuous parameters

Analytic IID: {'fm_mae_hz': 0.0009111464751051284, 'fm_p95_hz': 0.0034917776446140012, 'depth_median_relative': 0.0008775858917699397, 'phase_mae_deg': 0.3474429232136417, 'phase_p95_deg': 1.2994554310401596}
Analytic hard: {'fm_mae_hz': 0.0014756139306818061, 'fm_p95_hz': 0.0050651739307201816, 'depth_median_relative': 0.0016532719770432605, 'phase_mae_deg': 0.5560904439374904, 'phase_p95_deg': 1.8280241944574822}

No trainable parameters are introduced for sinusoidal LFO frequency, depth, or phase.

## E09-B: waveform type

The only learned component is a 16 → 32 → 16 → 4 classifier over {sine, triangle, saw, square}.

Trainable parameters: 884
IID accuracy: 0.9975
Hard accuracy: 0.9935
Best validation CE: 0.0132877

## Promotion decision

Promote E09 only as an identification stage with analytic continuous parameters and a small discrete-type head. Do not add a neural residual until a representation/perturbation family exists where the analytic estimator fails systematically.