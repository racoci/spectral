# M4: Hankel / Singular Spectrum Analysis (SSA) (E22)

## 1. Mathematical Formulation
Singular Spectrum Analysis (SSA) is a non-parametric technique combining time series embedding, multivariate geometry, and SVD.
Given a discrete signal $x = [x_0, x_1, \dots, x_{N-1}]^T$:
1. **Embedding:** Map $x$ into an $L \times K$ trajectory Hankel matrix, where $L$ is the window length and $K = N - L + 1$:
   $$X = \begin{bmatrix}
   x_0 & x_1 & \cdots & x_{K-1} \\
   x_1 & x_2 & \cdots & x_K \\
   \vdots & \vdots & \ddots & \vdots \\
   x_{L-1} & x_L & \cdots & x_{N-1}
   \end{bmatrix}$$
2. **SVD Decomposition:**
   $$X = \sum_{i=1}^d \sigma_i u_i v_i^T$$
   Each triplet $(\sigma_i, u_i, v_i)$ generates an elementary matrix $X^{(i)} = \sigma_i u_i v_i^T$.
3. **Low-Rank Grouping:** Select indices $\mathcal{I} = \{1, \dots, r\}$ corresponding to dominant signal oscillations, discarding noise sub-spaces:
   $$\widehat{X} = \sum_{i \in \mathcal{I}} \sigma_i u_i v_i^T$$
4. **Diagonal Averaging (Hankelization):** Map the matrix $\widehat{X}$ back to a 1D reconstructed series $\widehat{x}$ by averaging along cross-diagonals $i + j = n$.

## 2. Properties in MDL Codec
- **Non-parametric:** Captures arbitrary non-linear harmonics and damped exponentials without pre-specifying basis functions.
- **Complexity:** Governed by the low rank $r$ and the singular vector basis $U_r \in \mathbb{R}^{L \times r}$.
- **Noise Robustness:** Low-rank truncation acts as an optimal Wiener-like geometric projection filter.
