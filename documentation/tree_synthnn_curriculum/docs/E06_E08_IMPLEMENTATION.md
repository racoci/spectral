# E06-E08 implementation

These stages deliberately introduce **zero trainable neural parameters**.
The generator is restricted to exact FFT-bin harmonics and a gauge-fixed
quadratic spectral envelope, so the relevant inverse problems have closed-form
estimators. The curriculum therefore uses these stages to create high-quality
supervision for later learned modules instead of spending network capacity on
linear algebra.

## E06

Model:

`x(t) = sum_k H_k sin(2 pi k f0 t)`

with `H_1 = 1`, `E(f)=1`, and `B=0`.

Because `f0` and all harmonic frequencies are on FFT bins, the amplitude ratio
is recovered directly from the FFT. The stage has zero learned global weights.

## E07

Model:

`S(u) = c2 u^2`, `u = log2(f/440)`.

The gauge `S(0)=0` and `S'(0)=0` removes the constant and linear terms. After
removing the known harmonic amplitudes, `c2` is estimated by one-dimensional
least squares.

## E08

Model:

`f_k = k f0 sqrt(1 + B k^2)`.

Using

`(f_k/k)^2 = f0^2 + (B f0^2) k^2`

reduces the problem to a 2-parameter linear least-squares regression, followed
by `B = slope/intercept`.

The three stages are promotion gates. If these estimators fail, the next
learned network should not be trained yet; the representation or gauge must be
fixed first.
