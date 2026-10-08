# M8: Dynamic Mode Decomposition (DMD) / Koopman Operator (E28)

## 1. Mathematical Formulation
Dynamic Mode Decomposition (DMD), introduced by Peter Schmid (2010) and extended by Tu et al. (2014), connects dynamical systems theory with spatial modal decomposition. In the context of audio, a 1D scalar signal $x(t)$ is lifted to a delay-coordinate embedding:
$$X_1 = [x_0, x_1, \dots, x_{M-1}], \quad X_2 = [x_1, x_2, \dots, x_M]$$
The nonlinear dynamics of the acoustic pressure field are approximated by a linear best-fit Koopman operator $A$:
$$X_2 \approx A X_1$$

## 2. Exact DMD Algorithm
1. **Truncated SVD:** $X_1 = U_r \Sigma_r V_r^*$ with rank $r \ll \min(L, M)$.
2. **Reduced Koopman Matrix:**
   $$\widetilde{A} = U_r^* X_2 V_r \Sigma_r^{-1} \in \mathbb{C}^{r \times r}$$
3. **Eigendecomposition:** $\widetilde{A} W = W \Lambda$, with $\Lambda = \text{diag}(\lambda_1, \dots, \lambda_r)$.
4. **Exact DMD Modes:**
   $$\Phi = X_2 V_r \Sigma_r^{-1} W \in \mathbb{C}^{L \times r}$$
5. **Continuous Eigenvalues:**
   $$\omega_k = \frac{\ln(\lambda_k)}{\Delta t}$$
   where $\text{Im}(\omega_k) / (2\pi)$ yields the continuous oscillation frequency ($Hz$), and $\text{Re}(\omega_k)$ represents the exponential damping/growth rate ($\text{s}^{-1}$).
6. **Modal Reconstruction:**
   $$x_{snap}(t_m) = \sum_{k=1}^r \phi_k b_k e^{\omega_k m \Delta t}$$

## 3. Advantages in Structural Audio Codecs
- Solves crossing frequency ridges (E12) and damped modal resonances (E10) without discrete FFT binning.
- Separates physical modal decay rates from harmonic oscillation frequencies.
- Description length $L(M)$ scales with the dynamic rank $r$ rather than temporal sequence length $N$.
