# Generic holomorphic perceptual transform

This directory contains a tested numerical experiment for the generic construction

    E(t,y) = F(t + i eta(y))

with

    Phi'(f(y)) = 2*pi*eta(y)
    sigma_y^2 = -1/(2*pi*eta'(y)*f'(y))

The experiment uses a constant target resolution of 1.15 grid bins in the chosen perceptual coordinate y, then:

1. integrates eta(y);
2. constructs Phi'(f)=2*pi*eta(y(f));
3. builds the spectral filters G_y(f)=exp(Phi(f)-2*pi*f*eta(y));
4. normalizes each filter in L2;
5. solves the non-negative least-squares problem sum_j rho_j |G_j(f_k)|^2 ~= 1;
6. computes the complex coefficients with one input FFT and one IFFT per perceptual channel;
7. renders the coefficients with the requested YCbCr convention.

Scales tested: linear frequency, Cauchy/log frequency, Mel, and Bark (smooth Traunmuller form).

YCbCr display:

- Y is 20 log10(|z|/reference), mapped linearly from -96 dB to 0 dB into 0..255.
- The decoded Y code defines the magnitude already represented by luma.
- The magnitude residual relative to that quantized magnitude controls saturation.
- phase(arg z) controls chroma direction.
- Cb = 128 + 127*saturation*sin(phase)
- Cr = 128 + 127*saturation*cos(phase)
- saturation >= 1/255, so the neutral chroma center is never used.
- RGB conversion is full-range BT.601.

The YCbCr mapping is a visualization mapping, not a claim of lossless 24-bit encoding of arbitrary complex values.

The displayed coefficients are L2-normalized and multiplied by sqrt(rho_j), so they use the numerically fitted tight-frame density. The unnormalized holomorphic field remains the mathematical object before this y-dependent analysis normalization.
