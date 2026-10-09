# Specification: Expansion and Benchmarking of Phase 2 MDL Mathematical Competitors

## 1. Overview
The Phase 2 of the MDL (Minimum Description Length) Structural Codec Curriculum (`documentation/planning/v14-mdl-structural-codec-curriculum.md`) requires a rigorous empirical competition among 12 foundational mathematical representation families.
Prior work established:
- **M1 (E16):** CQT Ridge Baseline
- **M1b (E17):** CQT Jet-Ridge via Hilbert Analytic Signal
- **M3 (E19-E21):** Matrix Pencil Method
- **M5 (E23-E24):** Sparse Matching Pursuit

This track implements and benchmarks the remaining eight (8) mathematical families:
- **M2 (E18):** High-order chirplet ridge ($\dot{f}, \ddot{f}$ explicit tracking)
- **M4 (E22):** Hankel / SSA (Singular Spectrum Analysis)
- **M6 (E25):** Wavelet maxima (Multiscale singularity tracking for E14 transients)
- **M7 (E26):** Adaptive Fourier Decomposition (AFD / Takenaka-Malmquist rational basis)
- **M8 (E28):** DMD / Koopman (Dynamic Mode Decomposition)
- **M9 (E27):** VMD / EMD (Variational Mode Decomposition)
- **M11 (E29-E30):** State-Space / GP-SDE (Resonator bank with Kalman & RTS smoother)
- **M12 (E31):** Invertible Neural Baseline (PyTorch Normalizing Flow)

## 2. Mathematical Competitor Requirements

### 2.1 M2 (E18): High-Order Chirplet Ridge (`m2_chirplet_ridge.py`)
- **Mathematical Form:** $x(t) \approx \sum_k A_k(t) \cos(2\pi(f_0 t + \frac{1}{2}\dot{f} t^2 + \frac{1}{6}\ddot{f} t^3) + \phi_0)$.
- **Parameters:** Frame-wise $\{f_0, \dot{f}, \ddot{f}, A, \phi\}$.
- **Target Invariant:** Captures fast polynomial chirps (E08, E09) without spectral smear or sub-bin quantization.

### 2.2 M4 (E22): Hankel / SSA (`m4_hankel_ssa.py`)
- **Mathematical Form:** Trajectory Hankel matrix $X_{i,j} = x_{i+j}$, SVD $X = U \Sigma V^T$, rank-$r$ truncation, anti-diagonal averaging.
- **Parameters:** Singular values $\sigma_i$, singular vectors $U_{:, i}$, effective rank $r$.
- **Target Invariant:** Non-parametric low-rank modal decomposition and noise isolation.

### 2.3 M6 (E25): Wavelet Maxima (`m6_wavelet_maxima.py`)
- **Mathematical Form:** Continuous/dyadic wavelet transform $Wf(s, t)$, modulus maxima ridge chains across scales $s \to 0$, reconstruction via dual synthesis frame.
- **Parameters:** Maxima coordinates $(s_k, t_k)$, amplitudes, Lipschitz exponents.
- **Target Invariant:** Multiscale isolation of singularities and clicks (E14) with minimal coefficients.

### 2.4 M7 (E26): Adaptive Fourier Decomposition (`m7_afd.py`)
- **Mathematical Form:** Analytic signal representation in Hardy space $H^2(\mathbb{D})$ decomposed via Takenaka-Malmquist rational orthogonal bases $B_k(z) = \frac{\sqrt{1 - |a_k|^2}}{1 - \bar{a}_k z} \prod_{j=1}^{k-1} \frac{z - a_j}{1 - \bar{a}_j z}$ with adaptive poles $a_k \in \mathbb{D}$.
- **Parameters:** Selected poles $\{a_k\}_{k=1}^K \subset \mathbb{D}$ and projection coefficients $\{c_k\}$.
- **Target Invariant:** Monotonic energy reduction for transient and dispersive waveforms.

### 2.5 M8 (E28): DMD / Koopman (`m8_dmd.py`)
- **Mathematical Form:** Time-delay Hankel snapshots $X = [x_1, \dots, x_{m-1}], Y = [x_2, \dots, x_m]$; Koopman approximation $Y \approx A X$ via truncated SVD; modes $\Phi$ and continuous eigenvalues $\omega_k$.
- **Parameters:** Eigenvalues $\omega_k$, mode projections $b_k$, spatial eigenvectors $\Phi$.
- **Target Invariant:** Global dynamic modal growth/decay and crossing frequencies (E12).

### 2.6 M9 (E27): Variational Mode Decomposition (`m9_vmd.py`)
- **Mathematical Form:** Decomposes signal into $K$ quasi-orthogonal band-limited intrinsic mode functions (IMFs) $u_k(t)$ with center frequencies $\omega_k$ using ADMM in frequency domain.
- **Parameters:** Center frequencies $\omega_k$, IMF envelopes and phases.
- **Target Invariant:** Clean non-stationary separation without mode mixing.

### 2.7 M11 (E29-E30): State-Space / GP-SDE (`m11_state_space_gp.py`)
- **Mathematical Form:** Continuous-discrete state-space model representing multi-harmonic/damped oscillator priors $dx(t) = F x(t) dt + L dw(t), y_k = H x_k + v_k$. Inference via Kalman Filter and RTS Smoother.
- **Parameters:** System matrices $(F, H, Q, R)$, initial state $x_0$, posterior state trajectories.
- **Target Invariant:** Optimal probabilistic estimation with explicit posterior variance.

### 2.8 M12 (E31): Invertible Neural Baseline (`m12_invertible_flow.py`)
- **Mathematical Form:** Normalizing Flow with bijective affine coupling layers $x = [x_1, x_2]$, $y_1 = x_1$, $y_2 = x_2 \odot \exp(s(x_1)) + t(x_1)$ in PyTorch. Exact inverse $x = f^{-1}(z)$.
- **Parameters:** Latent tensor $z \in \mathbb{R}^N$ and network weights $\theta$.
- **Target Invariant:** Exact mathematical bijectivity ($x = f^{-1}(f(x))$), representing the non-interpretable neural compression ceiling.

## 3. Architecture and Integration
- All competitors inherit from `StructuralRepresentation(name, sr)` in `competition_framework.py`.
- Signature: `fit_and_reconstruct(x: np.ndarray) -> Tuple[dict, np.ndarray]`.
- Output: `(theta, x_hat)` where `theta` is a serializable dictionary of parameters and `x_hat` is the 1D NumPy float array matching `len(x)`.
- `run_competition_full.py` evaluates all 12 competitors across standard synthetic benchmarks:
  - `E06_PureTone`: Monochromatic sinusoid.
  - `E08_Chirp`: Linear frequency modulation.
  - `E09_PolynomialChirp`: Quadratic/cubic frequency modulation.
  - `E10_Polo`: Damped resonance.
  - `E12_Crossing`: Dual-chirp crossing trajectories.
  - `E14_Transient_Click`: Singular impulse / high-frequency click.
- Automated parallel evaluation using `ThreadPoolExecutor` or `ProcessPoolExecutor` with safe timeouts.

## 4. Acceptance Criteria
1. All 8 new competitors implemented and cleanly imported in `phase2_competition/`.
2. Unit test suite `tests/test_mdl_phase2_competitors.py` (or within `phase2_competition/test_competitors.py`) passes 100% of tests.
3. `run_competition_full.py` successfully runs and outputs `competition_results_full.json` containing metrics for all 12 models.
4. SI-SDR is finite and positive for all models on their primary intended synthetic domains.
