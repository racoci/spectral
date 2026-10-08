# M7: Adaptive Fourier Decomposition (AFD) (E26)

## 1. Mathematical Formulation
Adaptive Fourier Decomposition (AFD), pioneered by Tao Qian and collaborators, provides an adaptive rational orthogonal decomposition of signals in the Hardy space $H^2(\mathbb{D})$ over the complex unit disk $\mathbb{D} = \{z \in \mathbb{C} : |z| < 1\}$.

Unlike classical Fourier series where basis frequencies are fixed integers, AFD selects adaptive poles $\{a_k\}_{k=1}^K \subset \mathbb{D}$ inside the unit disk. The decomposition uses the **Takenaka-Malmquist (TM) orthonormal system**:
$$B_k(z) = e_{a_k}(z) \prod_{j=1}^{k-1} \frac{z - a_j}{1 - \bar{a}_j z}$$
where $e_a(z)$ is the normalized Szegö reproducing kernel:
$$e_a(z) = \frac{\sqrt{1 - |a|^2}}{1 - \bar{a} z}$$

## 2. Maximal Energy Principle (MEP)
At iteration $k$, given the reduced remainder $f_k \in H^2(\mathbb{D})$:
1. Select pole $a_k \in \mathbb{D}$ maximizing the projection:
   $$a_k = \arg\max_{a \in \mathbb{D}} |\langle f_k, e_a \rangle|$$
2. Compute the expansion coefficient:
   $$c_k = \langle f_k, e_{a_k} \rangle$$
3. Update the recursive remainder via the inverse Blaschke step:
   $$f_{k+1}(z) = \frac{f_k(z) - c_k e_{a_k}(z)}{\frac{z - a_k}{1 - \bar{a}_k z}}$$

## 3. Key Properties in Audio Modeling
- **Monotonic Energy Decay:** $\|f_{k+1}\| < \|f_k\|$ strictly holds for every non-zero remainder.
- **Natural Analytic Alignment:** Modulates both instantaneous frequency and instantaneous damping via the polar coordinates $(r_k, \theta_k)$ of pole $a_k$.
- **Compactness:** Represents damped exponentials (E10) and non-stationary bursts with fewer rational coefficients than polynomial Fourier harmonics.
