# Reproducibility

## Environment

- Python/PyTorch implementation is self-contained in the package.
- Sample rate: 12 kHz for E02-E05.
- E02/E03 frontend: FFT 16384, Hann window 8192, hop 120 samples, 60 bins/octave, 64 frames.
- E04/E05 frontend: FFT 1024, Hann window 1024, hop 360 samples, 60 bins/octave, 64 frames.

## Commands

The quick path is `python experiments/run_early_smoke.py` followed by `python experiments/validate_early.py`. The latter consumes the smoke checkpoints and, when present, uses `results/e05_2048/E05_model.pt` for the promoted E05 validation.

## Interpretation

E02 and E03 use analytic FFT-grid parabolic ridge estimation and learn only residual corrections. E04 uses closed-form least squares because the scalar amplitude relation is affine. E05 is the first temporally learned inverse head. SynthNN is never updated by E02-E05.
