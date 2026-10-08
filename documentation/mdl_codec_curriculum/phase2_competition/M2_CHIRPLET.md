# M2: High-Order Chirplet Ridge (E18)

## 1. Mathematical Formulation
A classical sinusoidal ridge model assumes locally constant instantaneous frequency within short analysis frames:
$$s(t) \approx A \cos(2\pi f_0 t + \phi_0)$$
For rapidly modulated sounds (linear chirps E08, quadratic chirps E09, Doppler shifts), this approximation induces severe spectral leakage and staircase artifacts.

The **High-Order Chirplet Ridge** extends the Taylor expansion of the instantaneous phase $\Phi(t)$ up to 3rd order:
$$\Phi(t) = \phi_0 + 2\pi \left( f_0 t + \frac{1}{2} \dot{f} t^2 + \frac{1}{6} \ddot{f} t^3 \right)$$
where:
- $f_0$: Instantaneous frequency at frame center ($Hz$)
- $\dot{f}$: Chirp rate ($\text{Hz/s}$)
- $\ddot{f}$: Frequency acceleration / chirp curvature ($\text{Hz/s}^2$)

## 2. Parameter Extraction & Reconstruction
Within each WOLA analysis window of length $L$ and hop $H$:
1. Demodulate the analytic signal $x_a(t) = x(t) + j \mathcal{H}[x](t)$.
2. Unwrap the local phase $\theta(t) = \text{unwrap}(\angle x_a(t))$.
3. Fit a degree-3 polynomial $\theta(t) = c_3 t^3 + c_2 t^2 + c_1 t + c_0$.
4. Extract structural parameters:
   $$f_0 = \frac{c_1}{2\pi}, \quad \dot{f} = \frac{c_2}{\pi}, \quad \ddot{f} = \frac{3 c_3}{\pi}, \quad \phi_0 = c_0$$
5. Reconstruct windowed blocks with exact instantaneous phase and blend via Hann overlap-add.

## 3. Structural Complexity
For $B$ analysis frames, $M_2$ retains:
$$\theta = \{f_0, \dot{f}, \ddot{f}, A, \phi_0\}_b \in \mathbb{R}^{5 \times B}$$
This yields an exact, compact description of high-curvature FM trajectories with minimal parameters.
