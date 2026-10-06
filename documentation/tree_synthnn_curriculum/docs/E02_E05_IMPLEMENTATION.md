# E02-E05 implementation

The first learned blocks are intentionally small and use a fixed frontend. The exact Gaussian-CQT/reassignment implementation remains an E01 validation target; E02-E05 use `FixedLogFrequencyFrontend`, an STFT followed by fixed log-frequency complex interpolation and temporal phase jets.

The early curriculum uses two fixed scales: E02-E03 use an FFT grid with `n_fft=16384`, `win_length=8192`, `hop_length=120` for pitch precision; E04-E05 use `n_fft=1024`, `hop_length=360` for temporal envelope resolution. Both are cropped/padded to exactly 64 frames. The frontend also exposes a fixed frame-activity mask from the spectral floor; inactive frames are excluded from E02-E04 statistics so the network never learns the arbitrary pre-onset ridge.

## E02

Input per frame: a 7-bin local ridge patch. Each bin carries log magnitude, phase-derived frequency velocity, and local log-frequency offset in cents. The model predicts only a residual frequency correction in cents.

Trainable model: `Linear(21,16) -> SiLU -> Linear(16,1)` = 369 scalars. The last layer is zero-initialized so training starts at the analytic ridge.

The analytic argmax ridge is never learned. A closed-form parabolic sub-bin correction is computed directly on the original FFT grid, and the 16-unit head learns only a residual correction to that refined frequency. `track_accuracy_5c` measures how often the final refined ridge is within 5 cents of the synthetic ground truth.

## E03

Input: the analytic sub-bin ridge estimate in `log2(f/440)`. Output: one residual correction in cents. The analytic ridge is taken directly from the original FFT grid; no second parabolic correction is applied. First/second derivative quantities are passed through analytically at E03.

Trainable model: `1 -> 8 -> 1` = 25 scalars.

## E04

Input: mean ridge log-magnitude. Output: amplitude in dB. The relation is affine in this clean scalar-amplitude experiment, so no nonlinear network is justified. The head is a single trainable affine map fitted by closed-form least squares; no iterative optimizer is used.

Trainable model: `1 -> 1` = 2 scalars.

## E05

Input: 64-frame ridge envelope normalized to remove unknown absolute source scale. Output: constrained attack/decay/sustain/release.

Trainable model: `64 -> 32 -> 4` = 2,212 scalars.

This is deliberately the first stage with a temporal encoder. Onset and note-duration heads are not yet learned; they are held as auxiliary ground-truth fields for the subsequent E15 event stage.

For E05, parameter relative error at the extreme bounds is reported as a diagnostic but is not the primary promotion criterion: the ADSR parameterization is only identifiable up to the temporal resolution of the fixed envelope frontend. Boundary promotion therefore uses envelope waveform RMSE in addition to the IID parameter error.

## Synthetic data policy

The generator uses an exact analytic oracle for the sine branch, matching the frequency and smooth ADSR conventions of the current SynthNN implementation. `verify_early_frontend.py` checks the analytic oracle against the actual SynthNN renderer before training.

The default full-stage budgets remain those in `curriculum/spec.py`; the smoke runner deliberately uses tiny datasets. E04 is fitted analytically; E02, E03 and E05 use iterative training. Use the CLI to scale training counts after the smoke tests pass.

## Validation result

An independent validation pass after training checks IID, interior off-grid OOD, amplitude counterfactuals, ADSR boundary behavior and SynthNN freeze integrity. All current promotion gates pass. The exact numerical results are stored in `results/early_validation/report.json`.

The E05 boundary parameter error itself remains higher near extreme short attack/decay values; this is reported diagnostically. The promoted boundary criterion is envelope reconstruction RMSE, because the fixed 30 ms envelope frame spacing does not make every extreme ADSR parameter independently identifiable.
