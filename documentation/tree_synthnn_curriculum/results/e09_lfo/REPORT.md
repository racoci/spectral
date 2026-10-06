# E09 — LFO residual identification

Trainable parameters: 820
Best validation loss: 8.89927e-05
Epochs: 46

| split | baseline fMAE | residual fMAE | baseline depth median | residual depth median | baseline phase MAE | residual phase MAE |
|---|---:|---:|---:|---:|---:|---:|
| IID | 0.0014 Hz | 0.0069 Hz | 0.1292% | 0.3351% | 0.527° | 0.531° |
| OOD | 0.0031 Hz | 0.0120 Hz | 0.1696% | 0.3199% | 1.167° | 1.180° |
| HARD | 0.0029 Hz | 0.0078 Hz | 0.2853% | 0.3997% | 1.120° | 1.129° |

## Training boundary

The analytic estimator remains frozen. The neural module learns only corrections to frequency, depth, and the phase vector.

## Validation

IID, parameter-OOD and hard ridge-perturbation sets are kept disjoint from training.