# M9: Variational Mode Decomposition (VMD) (E27)

## 1. Mathematical Formulation
Variational Mode Decomposition (VMD), introduced by Konstantin Dragomiretskiy and Dominique Zosso (2014), formulates empirical mode extraction as a well-posed constrained variational problem.

Given a signal $x(t)$, VMD seeks $K$ sub-signals (modes) $u_k(t)$, each predominantly compact around a center frequency $\omega_k$:
$$\min_{\{u_k\}, \{\omega_k\}} \sum_{k=1}^K \left\| \partial_t \left[ \left(\delta(t) + \frac{j}{\pi t}\right) * u_k(t) \right] e^{-j\omega_k t} \right\|_2^2 \quad \text{s.t.} \quad \sum_{k=1}^K u_k = x$$

## 2. ADMM Optimization in Frequency Domain
The augmented Lagrangian is minimized using the Alternating Direction Method of Multipliers (ADMM):
1. **Wiener Filtering Mode Update:**
   $$\widehat{u}_k^{n+1}(\omega) = \frac{\widehat{x}(\omega) - \sum_{i \ne k} \widehat{u}_i(\omega) + \frac{\widehat{\lambda}(\omega)}{2}}{1 + 2\alpha(\omega - \omega_k)^2}$$
2. **Center of Gravity Frequency Update:**
   $$\omega_k^{n+1} = \frac{\int_0^\infty \omega |\widehat{u}_k^{n+1}(\omega)|^2 d\omega}{\int_0^\infty |\widehat{u}_k^{n+1}(\omega)|^2 d\omega}$$
3. **Lagrange Multiplier Dual Ascent:**
   $$\widehat{\lambda}^{n+1}(\omega) = \widehat{\lambda}^n(\omega) + \tau \left( \widehat{x}(\omega) - \sum_{k=1}^K \widehat{u}_k^{n+1}(\omega) \right)$$

## 3. Contrast with EMD & Strengths in MDL
- **Non-recursive:** Avoids error accumulation and mode mixing that plague classical Hilbert-Huang sifting.
- **Quasi-Orthogonal:** Decomposes multi-harmonic or overlapping tones into mathematically separated, compact frequency bands.
- **Reconstruction:** $\widehat{x}(t) = \sum_{k=1}^K u_k(t)$ converges monotonically.
