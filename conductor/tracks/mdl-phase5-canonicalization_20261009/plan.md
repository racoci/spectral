# Implementation Plan - Phase 5: Canonicalization and Gauges (E39-E41)

## Phase 1: Mathematical Foundations & Equivalence Classes (E39)
- [x] Task 1.1: Design and Implement Exact Equivalence Transforms (`equivalence_classes.py`)
  - Implement mathematical transformation operators and exact waveform verifiers for:
    1. Time-shift $\Delta t$ vs Phase-gauge $\Delta \phi = -2\pi f \Delta t \pmod{2\pi}$.
    2. Angle Modulation equivalence: Frequency Modulation (FM) with $\Delta f$ vs Phase Modulation (PM) with $I = \Delta f / f_m$.
    3. Filter LTI frequency response $H(\omega)$ vs Harmonic Spectral Envelope vector $E_k$.
    4. Sign-polarity and polar phase offset ($A < 0 \iff \phi + \pi$).
  - Implement numerical Jacobian rank analysis and SVD nullity checks proving zero singular values along the gauge orbits.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase5_canonicalization/test_canonicalization.py -k TestEquivalenceClasses`
  - Documentation: Dedicated docstrings and mathematical specifications in `equivalence_classes.py` and section in `PHASE5_CANONICALIZATION_REPORT.md`.

## Phase 2: Canonical Gauge Projections & Bit Savings (E40)
- [x] Task 2.1: Implement Canonical Gauge Operators (`canonical_gauge.py`)
  - Implement the idempotent projection $\mathcal{G}: \Theta \to \Theta / \sim$:
    1. Time/Phase Gauge: Force $t_0 \equiv 0$ per block, $A \ge 0$, and $\phi_0 \in [-\pi, \pi)$.
    2. Angle Modulation Quotient: Eliminate separate FM/PM model indicators, collapsing to canonical `AngleModulation(f_c, beta, f_m, psi)`.
    3. Minimum-MDL Filter vs Envelope Selection: Dynamically compute parametric complexity $L(\theta_{\text{filter}})$ vs $L(\theta_{\text{env}})$ and canonicalize to the representation with strictly minimal description length.
  - Prove invariant reconstruction: $\|D(\theta) - D(\mathcal{G}(\theta))\|_\infty < 10^{-12}$.
  - Compute parameter bit savings $\Delta L = L(\theta) - L(\mathcal{G}(\theta))$.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase5_canonicalization/test_canonicalization.py -k TestCanonicalGauge`
  - Documentation: Architectural notes and API documentation in `canonical_gauge.py` and `PHASE5_CANONICALIZATION_REPORT.md`.

## Phase 3: True Physical Ambiguity & Uncertainty Distribution (E41)
- [x] Task 3.1: Implement Physical Ambiguity & Posterior Distribution (`ambiguity_distribution.py`)
  - Model physical non-identifiabilities under finite-length and band-limited observations:
    1. Harmonic collisions and octave superposition ($2 f_1 = f_2$).
    2. Damped resonator pole vs narrowband filtered noise transient.
  - Implement Gaussian/GMM posterior estimator $p(\theta | x)$ computing covariance $\Sigma_\theta$ and differential ambiguity entropy $H(\theta | x)$.
  - Implement causal horizon lookahead simulation showing monotonic entropy decay $\partial H / \partial \tau \le 0$ as temporal horizon expands.
  - Automated Testing: `python3 -m unittest documentation/mdl_codec_curriculum/phase5_canonicalization/test_canonicalization.py -k TestAmbiguityDistribution`
  - Documentation: Statistical formulations in `ambiguity_distribution.py` and `PHASE5_CANONICALIZATION_REPORT.md`.

## Phase 4: Integration, Benchmarking & Curriculum Validation
- [x] Task 4.1: End-to-End Curriculum Benchmark Runner (`run_phase5.py`)
  - Orchestrate automated execution across synthetic ground truth signals (pure tones, FM/PM, filtered harmonics, collisions, resonators).
  - Export structured metrics to `phase5_results.json`.
  - Automated Testing: `python3 documentation/mdl_codec_curriculum/phase5_canonicalization/run_phase5.py`
  - Documentation: Results summary in `phase5_results.json` and `PHASE5_CANONICALIZATION_REPORT.md`.
- [x] Task 4.2: Full Test Suite Execution & Coverage Verification
  - Run all unit and integration tests across Phase 5 with strict coverage checks.
  - Automated Testing: `python3 -m unittest discover -s documentation/mdl_codec_curriculum/phase5_canonicalization -p "test_*.py"`
  - Documentation: Test report logged in `PHASE5_CANONICALIZATION_REPORT.md`.
- [x] Task 4.3: Future Risk Mitigation & Codec Pipeline Handoff
  - Analyze computational complexity and numerical sensitivity of gauge transformations during online streaming.
  - Formulate mitigation strategy for Phase 6 (Gaussian Process / SDE state trajectory prediction on canonical quotient variables).
  - Documentation: Comprehensive report in `documentation/mdl_codec_curriculum/phase5_canonicalization/PHASE5_CANONICALIZATION_REPORT.md`.
