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
