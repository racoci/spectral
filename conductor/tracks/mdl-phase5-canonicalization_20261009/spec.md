# Specification - Phase 5: Canonicalization and Gauges (E39-E41)

## 1. Executive Summary & Vision

In the Minimum Description Length (MDL) structural codec curriculum (`documentation/planning/v14-mdl-structural-codec-curriculum.md`), Phases 0 to 4 established:
1. The discrete integer digital contract and exact reversibility (Phase 0).
2. The rigorous synthetic ground-truth signal laboratory (Phase 1).
3. The multi-model mathematical competition M1–M12 (Phase 2).
4. The exact evaluation of residual description length $L(R)$ and total MDL cost $L(M) + L(R)$ (Phase 3).
5. The block-wise exhaustive structural routing (Phase 4).

**Phase 5** addresses a fundamental identifiability challenge in parametric and structural audio modeling: **Identifiability and Gauge Invariance**. When different parameter vectors or distinct structural operator combinations map to the exact same acoustic observation:
$$D(\theta_1) = D(\theta_2) \quad \text{for } \theta_1 \ne \theta_2$$
an unconstrained parameter estimator faces singular Jacobians, flat optimization valleys ($\ker J \ne \{0\}$), and redundant bit expenditure $L(\theta)$.

Phase 5 divides this challenge into two rigorous categories:
1. **Mathematical Gauge Symmetries (E39, E40):** The equivalence is exact and holds identically across all time and observations. Here, the codec must define canonical gauge projections $\mathcal{G}: \Theta \to \Theta / \sim$ that collapse redundant parameter orbits into a single unique representative, removing redundant bits.
2. **True Physical Ambiguities (E41):** Two distinct physical realities (e.g. an exponentially damped pole vs a narrowband filtered noise burst; or harmonic collisions where $2f_1 = f_2$) generate indistinguishable or near-identical acoustic signals over a finite observation window. Here, symmetry is not a mathematical artifact but a fundamental informational bound. The codec must represent this as an uncertainty distribution $p(\theta | x)$ whose entropy $H(\theta | x)$ quantifies non-identifiability until causal lookahead future observations resolve the ambiguity.

---

## 2. Mathematical Architecture & Diagrams

```text
+---------------------------------------------------------------------------------------------------------+
|                                    PHASE 5: CANONICALIZATION & GAUGES                                   |
+---------------------------------------------------------------------------------------------------------+
|                                                                                                         |
|       +------------------------------------+             +------------------------------------+         |
|       |     UNCONSTRAINED PARAMETER        |             |        PHYSICAL AMBIGUITY          |         |
|       |             SPACE                  |             |          OBSERVATION               |         |
|       |        Theta in R^d                |             |         D(theta_1) ~ D(theta_2)    |         |
|       |                                    |             |    (Finite Window / Heisenberg)    |         |
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
|           - Bit savings: L(G(theta)) < L(theta)                            - Residual cost collapses    |
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

## 3. Detailed Technical Requirements

### 3.1 E39: Exact Equivalence Classes ($D(\theta) = D(g(\theta))$)

The system must define, simulate, and mathematically verify the exact equivalence classes that preserve the observed acoustic signal:

1. **Class 1: Time Shift vs Phase Shift (Sinusoidal/Harmonic Components)**
   $$x(t) = A \cos(2\pi f (t - \Delta t) + \phi_0) \equiv A \cos(2\pi f t + \phi')$$
   where $\phi' = (\phi_0 - 2\pi f \Delta t) \pmod{2\pi}$.
   - For isolated components, time delay $\Delta t$ and phase shift $\Delta \phi$ form a 1D continuous gauge orbit.
   - Requirement: Verify $\max |x_{\Delta t}(t) - x_{\Delta \phi}(t)| < 10^{-12}$.

2. **Class 2: Frequency Modulation (FM) vs Phase Modulation (PM)**
   For a single sinusoidal carrier and modulator:
   - Phase Modulation: $\phi_{\text{PM}}(t) = 2\pi f_c t + I \sin(2\pi f_m t + \psi)$
   - Frequency Modulation: $f_{\text{inst}}(t) = f_c + \Delta f \cos(2\pi f_m t + \psi)$, leading to $\phi_{\text{FM}}(t) = 2\pi f_c t + \frac{\Delta f}{f_m} \sin(2\pi f_m t + \psi)$
   - Under the substitution $\beta = \frac{\Delta f}{f_m} = I$, the instantaneous phase trajectories and synthesized waveforms are mathematically identical:
     $$\max_{t} |x_{\text{FM}}(t) - x_{\text{PM}}(t)| = 0.0$$
   - Jacobian nullity: In a model allowing both FM and PM parameters concurrently, $\operatorname{rank}(J) < \dim(\theta)$, producing a zero singular value along the directional derivative $(\partial / \partial \beta - \partial / \partial I)$.

3. **Class 3: LTI Filter vs Spectral Harmonic Envelope**
   For harmonic excitation $e(t) = \sum_{k=1}^K A_k \cos(2\pi k f_0 t + \phi_k)$, applying an LTI filter with frequency response $H(\omega)$ produces:
   $$y(t) = \sum_{k=1}^K A_k |H(2\pi k f_0)| \cos(2\pi k f_0 t + \phi_k + \arg H(2\pi k f_0))$$
   Modulating the partial amplitudes directly by an envelope vector $E_k = A_k |H(2\pi k f_0)|$ with phase offset $\theta_k = \phi_k + \arg H(2\pi k f_0)$ produces the exact same signal.
   - Requirement: Verify exact numerical identity $\max |y_{\text{filt}}(t) - y_{\text{env}}(t)| < 10^{-12}$.

4. **Class 4: Gain Sign and Polar Phase Offset**
   $$-A \cos(\omega t + \phi) \equiv A \cos(\omega t + \phi + \pi)$$
   A negative amplitude is completely absorbable into a $\pi$ phase rotation.

---

### 3.2 E40: Canonical Gauge Projections ($\mathcal{G}: \Theta \to \Theta / \sim$)

The system must construct canonical projection operators $\mathcal{G}$ that eliminate gauge redundancy:

1. **Global Time/Phase Gauge:**
   - Enforce canonical start time $t_0 \equiv 0$ within each analysis block, mapping all delay information into the initial phase $\phi_0 \in [-\pi, \pi)$.
   - Enforce strictly non-negative amplitude $A \ge 0$; if $A < 0$, map $A \leftarrow -A$ and $\phi_0 \leftarrow (\phi_0 + \pi) \pmod{2\pi} - \pi$.

2. **Angle Modulation Quotient Gauge:**
   - Eliminate redundant parameterization choices (FM vs PM).
   - Define a single canonical representation: **AngleModulation**, parameterized by carrier frequency $f_c$, modulation index $\beta \ge 0$, modulation frequency $f_m > 0$, and modulation phase $\psi \in [-\pi, \pi)$.
   - Prohibit encoding separate FM and PM modes, saving selector bits ($L_{\text{mode}} = 0$).

3. **Minimum-MDL Filter-Envelope Gauge:**
   - Given a timbral shaping requirement, evaluate the description length of an all-pole / rational filter $L(\theta_{\text{filter}})$ (e.g. $2P$ coefficients for $P$ second-order sections) versus the description length of discrete harmonic amplitudes $L(\theta_{\text{env}})$ ($K$ partial weights).
   - The canonical gauge selects the representation that strictly minimizes $L(\theta)$. When $2P < K$, canonicalize to filter poles; when $K \le 2P$, canonicalize to partial amplitudes.

4. **Bit-Saving Verification:**
   - Measure description length before and after canonicalization:
     $$\Delta L = L(\theta) - L(\mathcal{G}(\theta)) > 0$$
   - Ensure acoustic reconstruction error is identically zero:
     $$\|D(\theta) - D(\mathcal{G}(\theta))\|_\infty = 0.0$$

---

### 3.3 E41: True Physical Ambiguity & Uncertainty Distribution ($p(\theta | x)$)

For scenarios where the ambiguity is physical (finite observation window, bandwidth limits, or overlapping causal sources):

1. **Harmonic Collision & Octave Ambiguity:**
   - When two oscillators have frequencies in integer ratio ($f_2 = 2 f_1$), the spectral energy at $2 f_1$ can be generated by partial 2 of voice 1, fundamental of voice 2, or any linear combination $A_1^{(2)} + A_2^{(1)} = A_{\text{obs}}$.
   - Over a short window, the decomposition is physically non-identifiable without independent envelope variations.
   - The system must model this non-identifiability as a continuous posterior distribution $p(A_1, A_2 | x)$, computing the covariance $\Sigma$ and differential entropy $H(p)$.

2. **Damped Resonator vs Narrowband Noise / Transient:**
   - An impulse response of a high-Q resonator $e^{-\alpha t} \cos(\omega_0 t)$ over a short window can be approximated by a transient chirplet or filtered noise grain.
   - Quantify posterior likelihood across competing physical hypotheses:
     $$p(\text{Resonator} | x) \quad \text{vs} \quad p(\text{NoiseGrain} | x)$$

3. **Causal Horizon Disambiguation (Lookahead):**
   - Demonstrate that while $p(\theta | x_{0:T})$ has high entropy at observation length $T$, extending the observation to $T + \tau$ breaks the symmetry (because the resonator continues decaying deterministically while noise grains decorrelate, or because voice 1 and voice 2 apply distinct vibrato/envelope trajectories).
   - Verify that ambiguity entropy $H(p)$ decreases monotonically with lookahead duration:
     $$\frac{\partial H(p)}{\partial \tau} \le 0$$

---

## 4. Acceptance Criteria

1. **Exact Equivalence Verification (E39):**
   - Maximum absolute error for Time-Shift vs Phase-Shift $< 10^{-12}$.
   - Maximum absolute error for FM vs PM $= 0.0$.
   - Maximum absolute error for Filter vs Spectral Envelope $< 10^{-12}$.
   - Jacobian nullity verified numerically via SVD ($\sigma_{\min} / \sigma_{\max} < 10^{-14}$ along the gauge orbit).

2. **Canonical Gauge Integrity (E40):**
   - Canonical projection $\mathcal{G}(\theta)$ is idempotent: $\mathcal{G}(\mathcal{G}(\theta)) = \mathcal{G}(\theta)$.
   - Waveform identity preserved: $\|D(\theta) - D(\mathcal{G}(\theta))\|_\infty < 10^{-12}$.
   - Description length reduction: $L(\mathcal{G}(\theta)) < L(\theta)$ by eliminating redundant selector bits and orbit dimensions.

3. **Uncertainty Quantification (E41):**
   - Posterior distribution $p(\theta | x)$ returns valid probability density, covariance, and entropy $H(\theta | x)$.
   - Causal lookahead demonstrates variance collapse $\sigma^2(T + \tau) < \sigma^2(T)$ on harmonic collisions.

4. **Testing & Code Quality:**
   - 100% test pass rate in `test_canonicalization.py`.
   - Comprehensive execution script `run_phase5.py` producing `phase5_results.json`.
   - Technical documentation report `PHASE5_CANONICALIZATION_REPORT.md`.
