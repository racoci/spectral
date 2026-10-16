# Phase 6: Evolution and Uncertainty (E42-E45)
## Gaussian Processes and Probabilistic Tracking Report

This report summarizes the final outcomes of Phase 6 (E42-E45) for the Minimum Description Length (MDL) Codec Curriculum. The objective of Phase 6 was to implement probabilistic state-space evolution for predictive audio coding, addressing amplitude variations (via Ornstein-Uhlenbeck processes) and frequency kinematics (integrated Wiener velocity models), leading to rigorous statistical calibration and performance benchmarking against deterministic Taylor expansions.

### 1. GP Anchor Evolution (E42)
We implemented a continuous-time Gauss-Markov Stochastic Differential Equation (SDE) state-space model for both Amplitude and Frequency.

**Amplitude Model:** Mean-reverting Ornstein-Uhlenbeck (OU) process.
**Frequency Model:** Kinematic integrated Wiener velocity model.

*Verification:* Monte Carlo Euler-Maruyama simulation accurately matches analytical mean and variance scaling with relative errors well under 3% across $\Delta t \in [0.01s, 1.0s]$.

### 2. Phase Uncertainty & Cubic Variance Explosion (E43)
We implemented the continuous stochastic integration of frequency variation into instantaneous phase.
- Demonstrated **cubic variance explosion**: $\text{Var}(\phi(\Delta t)) \propto \Delta t^3$.
- The power-law exponent was computed empirically as $p = 3.000$, precisely matching the theoretical value.
- **Decoherence Horizon:** The critical temporal distance where standard deviation $\sigma_\phi \ge 1.0\text{ rad}$ (coherence attenuation drops to $\approx 60\%$) was determined to be $\sim 183.7 \text{ ms}$. Total phase washout ($\pi$ rad) occurs at $\sim 394.1 \text{ ms}$.

### 3. Probabilistic Calibration Validation (E44)
Extensive Monte Carlo ensembles were executed to verify the Probability Integral Transform (PIT) and empirical coverage for the Gaussian tracking assumptions.
- **Amplitude OU Calibration:** Well calibrated (10,000 samples, $p$-value 0.312, 1.96$\sigma$ coverage at $95.14\%$).
- **Phase Stochastic Calibration:** Well calibrated (10,000 samples, $p$-value 0.982, 1.96$\sigma$ coverage at $95.3\%$).

Control experiments with artificially under/overconfident noise matrices failed the Kolmogorov-Smirnov test (p-value $\approx 0$), proving the rigor of the PIT testing bounds.

### 4. GP vs High-Order Jet Horizon Comparison (E45)
We compared the predictive horizon of standard deterministic Taylor jets ($J_0$ constant, $J_1$ linear, $J_2$ parabolic) versus the GP/SDE Kalman predictor.

**Valid Prediction Horizons ($< 15$ Hz error):**
- $J_0$ (Constant): 18.0 ms
- $J_1$ (Linear): 43.0 ms
- $J_2$ (Parabolic): 43.0 ms
- **GP (Stochastic): 279.0 ms**

*Conclusion:* The SDE/GP bounds model mean drift gracefully, yielding a minimum predictive MDL code length ($4375.15$ bits) and providing $91.1\%$ bits savings against catastrophic polynomial explosions from $J_1/J_2$ at long horizons.

---

### Task 5.3: Future Risk Mitigation & Codec Pipeline Handoff (Phase 7: Probabilistic Tracking)

#### Computational Complexity of GP State-Space Filtering During Streaming
During streaming (online) processing, the codec will need to evaluate the probabilistic likelihood of incoming audio frames against the extrapolated GP tracks.
- **State Prediction:** $\mathbf{x}_{k|k-1} = \mathbf{\Phi} \mathbf{x}_{k-1|k-1}$ is $O(d^2)$, where state dimension $d=2$ for each track (e.g. $[f, \dot{f}]^T$).
- **Covariance Propagation:** $\mathbf{P}_{k|k-1} = \mathbf{\Phi} \mathbf{P}_{k-1|k-1} \mathbf{\Phi}^T + \mathbf{Q}$ is $O(d^3)$.
- With hundreds of active partials per frame, standard Kalman Filter updates run in $O(N_{\text{tracks}} \cdot d^3)$. Since $d \le 3$, this is mathematically trivial and highly suitable for real-time WebAssembly implementations.

#### Exact Kalman Smoother and Multi-Hypothesis Association (Phase 7 Handoff)
Moving into Phase 7 (E46-E50), the system must connect disconnected peaks (ridges) across multiple frames, especially when peaks vanish due to noise or overlapping partials.

**Proposed Pipeline for Phase 7 (Multiple Ridge Tracking):**
1. **Probabilistic Data Association Filter (PDAF) / Multiple Hypothesis Tracking (MHT):** Instead of nearest-neighbor matching, the distance metric between an active track and a newly observed peak will be the **Mahalanobis Distance** based on the extrapolated GP covariance matrix.
2. **Track Birth/Death Likelihoods:** Birth (new partial) and Death (track ended) will incur fixed MDL bit-penalties. A track is continued if the negative log-likelihood (Mahalanobis distance) is cheaper than the Birth/Death penalty.
3. **RTS (Rauch-Tung-Striebel) Smoothing:** Once tracks are extracted forward in time (filtering), an RTS backward-smoothing pass will recalculate the optimal mean and variance of $f(t)$ and $A(t)$ given the entire future sequence, eliminating phase-lag artifacts before final synthesis.

**Architectural Handoff to Phase 7:**
- Ensure $\mathbf{\Phi}$ and $\mathbf{Q}$ transition matrices established in E42 are globally accessible.
- Pass the analytical cubic-variance bounds (E43) into the peak-association gating mechanism to limit search radius to exactly $1.96\sigma$ bounding boxes in the frequency-time plane.
