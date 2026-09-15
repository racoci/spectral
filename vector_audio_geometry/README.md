# Vector Audio Geometry — next implementation stage

This crate implements the first concrete audio -> vector-geometry loop.

Pipeline:

1. log-spaced Gaussian filter bank;
2. complex coefficients + temporal/spectral phase gradients;
3. reassigned time/log-frequency points;
4. energy-weighted preliminary trajectory extraction;
5. natural cubic C2 spline fit;
6. vector trajectory error measurement in cents;
7. infinite Monte-Carlo harness with reproducible seed.

The spline fitter uses a natural cubic spline in time/log-frequency and converts each polynomial interval to a cubic Bezier segment. This gives a compact C2 vector representation.

The current tracker is deliberately conservative: it fits one dominant trajectory. Multi-track assignment, crossing resolution, timbre surfaces and texture surfaces are subsequent layers.

`cargo test` runs the geometry invariant. `cargo run --release` runs the continuous Monte-Carlo analysis loop until Ctrl+C.

## Multi-track geometric fitting

The crate now includes `multi_track.rs`. It represents the reassigned spectrogram as a power field on the log-frequency grid, initializes several trajectories with dynamic programming, then jointly refines them by coordinate descent.

The shared objective is

`E = -sum log(1 + P(m,u_j(m))) + lambda_v sum (Delta u)^2 + lambda_a sum (Delta^2 u)^2 + lambda_r sum exp(-0.5 (u_i-u_j)^2/sigma_r^2)`.

The last term is a soft repulsion between tracks. The final sample paths are compressed to C2 cubic splines using the existing natural-cubic-to-Bezier conversion.

This is an explicit prototype of multi-curve fitting, not yet a global optimum solver. Greedy DP initialization plus joint coordinate descent is used to keep the implementation small and deterministic.
