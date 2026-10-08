# M11: State-Space / GP-SDE Representation (E29-E30)

## 1. Mathematical Formulation
Gaussian Process (GP) regression with harmonic and quasi-periodic covariance kernels can be converted exactly into continuous-discrete linear state-space models governed by Stochastic Differential Equations (SDEs) (Särkkä, Solin, & Hartikainen):
$$d\mathbf{x}(t) = \mathbf{F} \mathbf{x}(t) dt + \mathbf{L} d\mathbf{w}(t)$$
$$y_k = \mathbf{H} \mathbf{x}(t_k) + v_k, \quad v_k \sim \mathcal{N}(0, R)$$

For an ensemble of $M$ harmonic oscillators with damping $\gamma_m$ and angular frequency $\omega_m$:
$$\mathbf{F}_m = \begin{bmatrix} 0 & 1 \\ -\omega_m^2 & -2\gamma_m \end{bmatrix}, \quad \mathbf{L}_m = \begin{bmatrix} 0 \\ 1 \end{bmatrix}$$

## 2. Discrete Transition & O(N) Inference
At discrete sampling steps $\Delta t = 1 / f_s$, the matrix exponential yields:
$$\mathbf{A}_m = e^{-\gamma_m \Delta t} \begin{bmatrix} \cos(\omega_m \Delta t) & \frac{\sin(\omega_m \Delta t)}{\omega_m} \\ -\omega_m \sin(\omega_m \Delta t) & \cos(\omega_m \Delta t) \end{bmatrix}$$

1. **Kalman Filter (Forward):** Recursively computes the predictive state $\mathbf{x}_{k|k-1}, \mathbf{P}_{k|k-1}$ and updated state $\mathbf{x}_{k|k}, \mathbf{P}_{k|k}$ in $O(N)$ operations, avoiding the $O(N^3)$ inversion of the dense GP Gram matrix.
2. **Rauch-Tung-Striebel (RTS) Smoother (Backward):** Computes the true global posterior mean $\mathbf{x}_{k|N}$ and marginal variance $\mathbf{P}_{k|N}$.
3. **Reconstruction:** $\widehat{x}(t_k) = \mathbf{H} \mathbf{x}_{k|N}$.

## 3. Structural Information and MDL Relevance
- **Explicit Uncertainty:** Produces posterior variance $\sigma^2_k = \mathbf{H} \mathbf{P}_{k|N} \mathbf{H}^T$ per sample.
- **Continuous Interpolation:** Natural predictor for missing samples, dropouts, and packet loss concealment.
- **State Parameters:** System matrices $(\mathbf{F}, \mathbf{H}, \mathbf{Q}, R)$ and state trajectory.
