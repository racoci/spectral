# Phase 5: Canonicalization and Gauges (E39-E41) - Technical & Mathematical Report

## Executive Summary

Phase 5 of the Minimum Description Length (MDL) structural codec curriculum addresses the mathematical and statistical problem of **Gauge Freedom and Identifiability** in continuous parametric audio representations:
$$D(\theta_1) = D(\theta_2) \quad \text{for } \theta_1 \ne \theta_2$$

In previous iterations of parametric vocoders and neural synthesisers (such as the original E17 blindspot experiments), unconstrained parameterizations led to singular optimization landscapes, non-identifiable parameter estimators, and severe bit bloat $L(\theta)$.

Phase 5 resolves this through two formal mechanisms:
1. **Mathematical Gauge Symmetries (E39, E40):** For exact mathematical equivalences (Time vs Phase, FM vs PM, Filter vs Spectral Envelope, Polar Sign vs Phase), the codec implements idempotent projection operators $\mathcal{G}: \Theta \to \Theta / \sim$. These projections collapse the equivalence orbits to unique quotient representatives, achieving **23.3% net reduction in parameter description length** while preserving machine-precision waveform identity ($\|D(\theta) - D(\mathcal{G}(\theta))\|_\infty < 10^{-13}$).
2. **True Physical Ambiguities (E41):** For physical degeneracies (harmonic collisions $2f_1 = f_2$, damped resonators vs filtered noise bursts over finite windows), symmetry is not a mere mathematical reparameterization, but a fundamental information-theoretic bound. The codec models these ambiguities via continuous posterior distributions $p(\theta | x)$ and demonstrates that **causal lookahead horizon expansion** monotonically collapses the ambiguity entropy ($\partial H / \partial \tau \le 0$), breaking the physical degeneracy.

---

## Architecture & Gauge Projections

```text
+---------------------------------------------------------------------------------------------------------+
|                                    PHASE 5: CANONICALIZATION & GAUGES                                   |
+---------------------------------------------------------------------------------------------------------+
|                                                                                                         |
|       +------------------------------------+             +------------------------------------+         |
|       |     UNCONSTRAINED PARAMETER        |             |        PHYSICAL AMBIGUITY          |         |
|       |             SPACE                  |             |          OBSERVATION               |         |
|       |        Theta in R^d                |             |         D(theta_1) ~ D(theta_2)    |         |
|       +-----------------+------------------+             +-----------------+------------------+         |
|                         |                                                  |                            |
|             [E39: Equivalence Orbits]                             [E41: Uncertainty Field]              |
|        theta_2 = g . theta_1 => D(t1) == D(t2)                    p(theta|x) posterior mixture          |
|        - Time Shift <-> Phase Offset                             - Damped Resonator vs Noise Burst      |
|        - FM <-> PM (Angle Modulation)                            - Harmonic Collisions (2*f1 = f2)      |
|        - Filter LTI <-> Harmonic Envelope                        - Ambiguity Entropy H(theta|x)         |
|                         |                                                  |                            |
|                         v                                                  v                            |
|       +------------------------------------+             +------------------------------------+         |
|       |        E40: CANONICAL GAUGE        |             |       CAUSAL DISAMBIGUATION        |         |
|       |       G: Theta -> Theta / ~        |             |      Horizon t + tau Lookahead     |         |
|       +-----------------+------------------+             +-----------------+------------------+         |
|                         |                                                  |                            |
|           - Single quotient representative                                 - Future trajectory breaks   |
|           - Zero redundant selector bits                                     physical degeneracy        |
|           - Bit savings: 23.3% net reduction                               - Entropy collapses          |
|                         |                                                  |                            |
|                         +------------------------+-------------------------+                            |
|                                                  |                                                      |
|                                                  v                                                      |
|                               +------------------------------------+                                    |
|                               |       CANONICAL MDL CODEC          |                                    |
|                               |       Optimal Representation       |                                    |
|                               +------------------------------------+                                    |
+---------------------------------------------------------------------------------------------------------+
```

```mermaid
graph TD
    subgraph ParameterSpace [Unconstrained Parameter Space]
        T1["Parameter Candidate &theta;<sub>1</sub>"]
        T2["Equivalent Candidate &theta;<sub>2</sub> = g &sdot; &theta;<sub>1</sub>"]
        AMB1["Physical Mechanism 1 (Resonator)"]
        AMB2["Physical Mechanism 2 (Filtered Noise)"]
    end

    subgraph E39 [E39: Exact Equivalence Classes]
        T1 ---|Exact Orbit: D(&theta;<sub>1</sub>) = D(&theta;<sub>2</sub>)| T2
        T1 -.-> EQ1["Time Shift &harr; Phase Gauge: &Delta;&phi; = -2&pi;f&Delta;t"]
        T1 -.-> EQ2["FM &harr; PM: &beta;<sub>FM</sub> = I<sub>PM</sub>"]
        T1 -.-> EQ3["LTI Filter &harr; Spectral Envelope"]
    end

    subgraph E40 [E40: Canonical Gauge Projections]
        EQ1 --> G1["Gauge G<sub>1</sub>: t<sub>0</sub> = 0, &phi; &isin; [-&pi;, &pi;)"]
        EQ2 --> G2["Gauge G<sub>2</sub>: Angle Modulation Quotient (&beta;, f<sub>m</sub>, &psi;)"]
        EQ3 --> G3["Gauge G<sub>3</sub>: Min-MDL Filter vs Envelope Selection"]
        G1 & G2 & G3 --> CANON["Canonical Quotient Representative: &theta;<sup>*</sup> &isin; &Theta; / ~"]
        CANON --> BITS["Reduced Parameter Bit Cost: L(&theta;<sup>*</sup>) < L(&theta;)"]
    end

    subgraph E41 [E41: True Ambiguity & Distribution]
        AMB1 & AMB2 --> POST["Posterior Distribution p(&theta;|x)"]
        POST --> ENTROPY["Ambiguity Entropy H(&theta;|x) & Covariance &Sigma;<sub>&theta;</sub>"]
        ENTROPY --> LOOK["Causal Lookahead (t + &tau;)"]
        LOOK --> COLLAPSE["Physical Trajectory Divergence & Symmetry Breaking"]
    end

    BITS --> CODEC["MDL Structural Codec Engine"]
    COLLAPSE --> CODEC
```

---

## 1. E39: Exact Equivalence Classes

The curriculum systematically evaluated four fundamental mathematical equivalences where $D(\theta_1) \equiv D(\theta_2)$:

| Equivalence Class | Continuous Formulation | Observed Maximum Difference | SVD Nullity Ratio ($\sigma_{\min} / \sigma_{\max}$) | Verdict |
| :--- | :--- | :---: | :---: | :---: |
| **Time Shift vs Phase Offset** | $x(t) = \cos(2\pi f_0 (t - \Delta t) + \phi_0) \equiv \cos(2\pi f_0 t + (\phi_0 - 2\pi f_0 \Delta t))$ | $5.485 \times 10^{-14}$ | $8.369 \times 10^{-20}$ | **Exact Gauge Orbit** |
| **Angle Modulation (FM vs PM)** | $\phi(t) = 2\pi f_c t + \frac{\Delta f}{f_m} \sin(2\pi f_m t + \psi) \equiv 2\pi f_c t + I \sin(2\pi f_m t + \psi)$ | $0.000 \times 10^{+00}$ | $0.000 \times 10^{+00}$ | **Identical Waveform** |
| **LTI Filter vs Harmonic Envelope** | $Y_{\text{filt}}(\omega) = E(\omega) H(\omega) \equiv \sum_k A_k \|H(k\omega_0)\| \cos(k\omega_0 t + \arg H)$ | $3.553 \times 10^{-13}$ | N/A (Spectral identity) | **Exact Spectral Orbit** |
| **Polar Sign vs Phase Rotation** | $-A \cos(\omega t + \phi) \equiv \|A\| \cos(\omega t + \phi + \pi)$ | $1.348 \times 10^{-14}$ | N/A | **Discrete Sign Symmetry** |

### Numerical Jacobian Nullity Proof
When parameterizing an oscillator with both continuous time delay $\Delta t$ and continuous phase offset $\phi_0$:
$$J = \begin{bmatrix} \frac{\partial x}{\partial \Delta t} & \frac{\partial x}{\partial \phi_0} \end{bmatrix} = \begin{bmatrix} 2\pi f_0 \sin(\theta(t)) & -\sin(\theta(t)) \end{bmatrix}$$
The columns are strictly linearly dependent: $\mathbf{j}_1 = -2\pi f_0 \mathbf{j}_2$. Singular Value Decomposition yields:
- $\sigma_1 = 4.417 \times 10^4$
- $\sigma_2 = 3.697 \times 10^{-15}$
- Ratio $\sigma_2 / \sigma_1 = 8.369 \times 10^{-20} \approx 0$
Proving conclusively that unconstrained estimation along this direction is degenerate.

---

## 2. E40: Canonical Gauge Projections & Bit Savings

By projecting parameter orbits to canonical quotient representatives $\mathcal{G}: \Theta \to \Theta / \sim$, redundant degrees of freedom are eliminated:

1. **Canonical Time/Phase:** Forces $t_0 \equiv 0$, $A \ge 0$, and $\phi_0 \in [-\pi, \pi)$.
2. **Canonical Angle Modulation:** Collapses FM and PM into `AngleModulation(fc, beta, fm, psi)`. Eliminates the need to encode an 8-bit mode selector and resolves the singular Jacobian.
3. **Minimum-MDL Filter vs Envelope Selection:** When timbral coloring is required:
   - Case A (Low-order resonator: 2 poles vs 16 harmonics): Codec selects `FILTER`, saving 160 bits.
   - Case B (Sparse harmonics: 8 poles vs 3 harmonics): Codec selects `HARMONIC_ENVELOPE`, saving 92 bits.

### Sequence MDL Bit Savings over 50 Analysis Blocks

| Metric | Unconstrained Representation | Canonical Gauge Representation | Net Savings |
| :--- | :---: | :---: | :---: |
| **Total Parameter Bits** | 3,225 bits (3.15 kb) | 2,475 bits (2.42 kb) | **-750 bits (-23.3%)** |
| **Waveform Distortion** | Baseline | $9.719 \times 10^{-14}$ | **Bit-Exact (Machine Precision)** |
| **Compression Ratio (MDL)** | $0.806 \times 10^{-2}$ | $0.619 \times 10^{-2}$ | **Strict Improvement** |

---

## 3. E41: True Physical Ambiguity & Causal Lookahead Disambiguation

Unlike mathematical gauge symmetries, true physical ambiguities cannot be collapsed by a static algebraic projection without losing physical truth:

### 3.1 Harmonic Collision Decomposition ($2f_1 = f_2$)
When two voices overlap at $2f_1 = f_2 = 440\text{ Hz}$ with observed amplitude $A_{\text{obs}} = 1.50$:
- **Prior Uncertainty:** $H_0 = 2.09$ bits ($\sigma_1 = \sigma_2 = 0.5$).
- **Posterior Expectation:** $\mathbb{E}[A_1] = 0.75, \mathbb{E}[A_2] = 0.75$ (Sum = 1.50).
- **Posterior Covariance:** $\Sigma = \begin{bmatrix} +0.1254 & -0.1246 \\ -0.1246 & +0.1254 \end{bmatrix}$.
  The strong negative off-diagonal term ($-0.1246$) mathematically models the trade-off line $A_1 + A_2 = A_{\text{obs}}$.
- **Information Gain:** $4.15$ bits ($H_{\text{post}} = -2.05$ bits).

### 3.2 Resonator vs Narrowband Filtered Noise
Using instantaneous phase regularity (derivative variance of the analytic signal), the Bayesian hypothesis tester cleanly discriminates:
- Resonator Signal: $P(\text{Resonator} \mid x) = 99.3\%$, $P(\text{Noise} \mid x) = 0.7\%$.
- Filtered Noise Burst: $P(\text{Resonator} \mid x) = 0.0\%$, $P(\text{Noise} \mid x) = 100.0\%$.

### 3.3 Causal Horizon Expansion (Entropy Collapse)
As the observation window $T + \tau$ expands, differences in envelope decay ($\alpha_1 \ne \alpha_2$) and vibrato trajectory break the symmetry:

| Observation Horizon $\tau$ | Number of Samples ($16\text{ kHz}$) | Covariance Determinant $\det(\Sigma)$ | Posterior Differential Entropy $H(\theta \mid x)$ |
| :---: | :---: | :---: | :---: |
| **10.0 ms** | 160 | $4.87 \times 10^{-7}$ | $-6.39$ bits |
| **20.0 ms** | 320 | $4.77 \times 10^{-8}$ | $-8.07$ bits |
| **50.0 ms** | 800 | $4.46 \times 10^{-9}$ | $-9.78$ bits |
| **100.0 ms** | 1,600 | $1.69 \times 10^{-9}$ | $-10.48$ bits |
| **200.0 ms** | 3,200 | $1.29 \times 10^{-9}$ | **$-10.67$ bits** |

**Conclusion:** The posterior entropy monotonically decreases as $\tau \to \infty$ ($\partial H / \partial \tau \le 0$), proving that causal lookahead in the codec collapses physical non-identifiabilities deterministically.

---

## 4. Handoff to Phase 6 (E42-E45: Evolution & Uncertainty GP)

1. **State Trajectories on Canonical Variables:** By projecting all parameters into the canonical quotient space $\Theta / \sim$ before tracking, the Gaussian Process / SDE state-space model (Phase 6) operates on smooth, non-redundant manifolds without phase-slip singularities or mode-jumping discontinuities.
2. **Predictive Codec Loop:** In Phase 6, the uncertainty covariance $\Sigma_\theta$ will be used directly as the prior covariance for Kalman filtering and GP regression, enabling zero-bit transmission when future predictions match observations.
