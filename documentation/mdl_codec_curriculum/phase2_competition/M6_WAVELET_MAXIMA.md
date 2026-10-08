# M6: Wavelet Maxima (WTMM) (E25)

## 1. Mathematical Formulation
Wavelet Transform Modulus Maxima (WTMM), introduced by Stéphane Mallat and Wen-Liang Hwang (1992), characterizes isolated singularities, edges, and transient impulses (such as clicks, attack transients, and step onsets E14).

Given a continuous/dyadic wavelet family $\psi_{s}(t) = \frac{1}{\sqrt{s}} \psi\left(\frac{t}{s}\right)$:
$$W f(s, t) = (f * \psi_s)(t)$$
A point $(s_0, t_0)$ is a **Modulus Maximum** if:
$$\frac{\partial |W f(s_0, t)|}{\partial t}\Big|_{t=t_0} = 0$$
and $|W f(s_0, t)|$ is locally convex.

## 2. Singularity Exponents and Transients
For an isolated singularity at $t = \tau$ with Lipschitz regularity $\alpha$:
$$|W f(s, t)| \le C \cdot s^{\alpha + 1/2}$$
- **Dirac impulse / click ($\alpha = 0$):** Modulus maxima propagate across all fine scales without decay.
- **Discontinuity in derivative ($\alpha = 1$):** Modulus decays proportionally to $s^{3/2}$.

## 3. Dual Frame Synthesis
To guarantee numerical stability and exact invertibility, we employ a continuous-frequency partition of unity filterbank:
$$\sum_{j=1}^J |\Psi_j(\omega)|^2 + |\Phi_0(\omega)|^2 = A(\omega) > 0$$
The dual synthesis filters are:
$$\widetilde{\Psi}_j(\omega) = \frac{\Psi_j(\omega)}{A(\omega)}$$
The reconstructed signal is synthesized as:
$$\widehat{x}(t) = \sum_{j=1}^J \left( W_j * \widetilde{\psi}_j \right)(t) + \left( c_0 * \widetilde{\phi}_0 \right)(t)$$
Extracting only dominant modulus maxima lines yields an ultra-sparse description of transient acoustic events.
