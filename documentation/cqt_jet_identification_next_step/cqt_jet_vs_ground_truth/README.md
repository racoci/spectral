# CQT jet identification experiments

Three progressively refined experiments were run:

1. `cqt_jet_vs_ground_truth` — magnitude-ridge implicit derivatives from a 60-bin/octave librosa CQT.
2. `cqt_complex_jet_benchmark` — temporal phase jet of the complex CQT; this exposed phase-sampling aliasing when the carrier is reconstructed into the coefficient phase.
3. `gaussian_cqt_jet_benchmark` — custom Gaussian CQT with 60 bins/octave, kept in baseband. This is the current preferred formulation for phase/frequency tests.
4. `analytic_parameter_recovery` — parameter fitting from the recovered ridge trajectories.
5. `gauge_fixed_factorization` — explicit gauge analysis for harmonic structure vs spectral envelope.

The current experiments are controlled one-note signals with known ground truth.
