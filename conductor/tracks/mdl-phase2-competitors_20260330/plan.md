# Implementation Plan: Expansion and Benchmarking of Phase 2 MDL Mathematical Competitors

## Phase 1: Framework Preparation & Verification
- [x] Task 1.1: Verify and Refactor `competition_framework.py`
  - Enhance `CompetitionFramework` to support both serial and parallel evaluation (`evaluate_signal` with thread/process pool option), explicit exception tracking, and uniform parameter counting heuristics.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestFramework`
  - Documentation: Dedicated docstrings in `competition_framework.py` and section in `phase2_competition/README.md`.
- [x] Task 1.2: Establish Unit Test Harness `test_competitors.py`
  - Scaffold unit test suite verifying `StructuralRepresentation` contracts, input/output dimensions, finite SI-SDR, and parameter dictionaries across all competitor classes.
  - Automated Testing: `pytest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py` (or `python3 -m unittest ...`)
  - Documentation: Test harness notes in `test_competitors.py`.

## Phase 2: Implementation of Continuous & Discrete Competitors (M2, M4, M6, M7)
- [x] Task 2.1: Implement M2 (E18) - High-Order Chirplet Ridge (`m2_chirplet_ridge.py`)
  - Continuous differential jet tracking $\dot{f}, \ddot{f}$ via local quadratic phase unwrapping or analytic dechirp.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM2Chirplet`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M2_CHIRPLET.md`.
- [x] Task 2.2: Implement M4 (E22) - Hankel / SSA (`m4_hankel_ssa.py`)
  - Singular Spectrum Analysis embedding 1D audio into a Hankel trajectory matrix, computing SVD, and anti-diagonal averaging.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM4HankelSSA`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M4_HANKEL_SSA.md`.
- [x] Task 2.3: Implement M6 (E25) - Wavelet Maxima (`m6_wavelet_maxima.py`)
  - Multiscale wavelet transform with modulus maxima tracking across scales for transient detection and dual synthesis reconstruction.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM6WaveletMaxima`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M6_WAVELET_MAXIMA.md`.
- [x] Task 2.4: Implement M7 (E26) - Adaptive Fourier Decomposition (`m7_afd.py`)
  - Rational approximation in Hardy space $H^2(\mathbb{D})$ using Takenaka-Malmquist orthogonal system and maximal energy projection.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM7AFD`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M7_AFD.md`.

## Phase 3: Implementation of Dynamic, Modal & State-Space Competitors (M8, M9, M11, M12)
- [ ] Task 3.1: Implement M8 (E28) - DMD / Koopman (`m8_dmd.py`)
  - Dynamic Mode Decomposition over delay-coordinate Hankel snapshot pairs $(X, Y)$ to identify Koopman modes and eigenvalues.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM8DMD`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M8_DMD.md`.
- [ ] Task 3.2: Implement M9 (E27) - Variational Mode Decomposition (`m9_vmd.py`)
  - Non-recursive variational mode decomposition into band-limited IMFs using ADMM in spectral domain.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM9VMD`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M9_VMD.md`.
- [ ] Task 3.3: Implement M11 (E29-E30) - State-Space / GP-SDE (`m11_state_space_gp.py`)
  - Resonator bank state-space model with Kalman filter and Rauch-Tung-Striebel (RTS) smoother for optimal continuous trajectory estimation.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM11StateSpace`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M11_STATE_SPACE.md`.
- [ ] Task 3.4: Implement M12 (E31) - Invertible Neural Baseline (`m12_invertible_flow.py`)
  - Bijective Normalizing Flow in PyTorch with RealNVP affine coupling layers, achieving bit-exact or near-exact inverse reconstruction.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase2_competition/test_competitors.py -k TestM12InvertibleFlow`
  - Documentation: Architectural notes in `documentation/mdl_codec_curriculum/phase2_competition/M12_INVERTIBLE_FLOW.md`.

## Phase 4: Full Multi-Model Parallel Benchmark & Synthesis
- [ ] Task 4.1: Integrate All Competitors into `run_competition_full.py`
  - Update `run_competition_full.py` to evaluate all 12 competitors (M1, M1b, M2, M3, M4, M5, M6, M7, M8, M9, M11, M12) across curriculum test signals in parallel.
  - Compute and log SI-SDR, execution time (ms), and parameter counts.
  - Output results to `competition_results_full.json`.
  - Automated Testing: `python3 documentation/mdl_codec_curriculum/phase2_competition/run_competition_full.py`
  - Documentation: Summary report in `documentation/mdl_codec_curriculum/phase2_competition/PHASE2_COMPETITION_REPORT.md`.
- [ ] Task 4.2: Comprehensive Unit and Integration Test Pass
  - Run full test suite for all competitors.
  - Automated Testing: `python3 -m unittest discover -s documentation/mdl_codec_curriculum/phase2_competition -p "test_*.py"`
  - Documentation: Test report in `phase2_competition/README.md`.
- [ ] Task 4.3: Risk Mitigation & Computational Complexity Analysis
  - Document trade-offs, O(N) vs O(N^3) bottlenecks (e.g. SVD in Hankel/SSA and DMD vs O(N) Kalman smoother), and future mitigation strategies for Phase 3 (hybrid residual codec).
  - Documentation: Dedicated section in `PHASE2_COMPETITION_REPORT.md`.
