# CQT 60-bin/octave vs analytic synthesizer jet

Controlled single-note signal, SR=12000, CQT 60 bins/octave, hop=256. The signal contains a smooth pitch sweep + vibrato, Gaussian-smoothed ADSR, tremolo, 8 harmonics, inharmonicity, and an absolute-frequency spectral envelope.

The CQT ridge is found from log magnitude. Ridge velocity and acceleration are obtained from the implicit-jet equations

xi' = - M_tx / M_xx

xi'' = -(M_ttx + 2 M_txx xi' + M_xxx xi'^2) / M_xx

then converted to frequency derivatives. Amplitude is locally integrated and calibrated against pure-tone CQT response.

## Per-harmonic measurements

 k  freq_rmse_hz  freq_rel_rmse  df_rmse_hz_s  d2f_rmse_hz_s2  logamp_rmse  median_freq_error_cents
 1      3.050967       0.011682    103.997076     3162.812385     0.680504                16.369319
 2      3.635263       0.006958    154.493828     5863.978890     0.609782                 7.930974
 3      4.832295       0.006163    194.205919     9170.450114     0.614365                 6.894910
 4      6.150380       0.005879    243.229130    11766.674346     0.590644                 6.676937

## F0 / parameter recovery

{
  "pitch_rmse_hz": 3.050008120452842,
  "pitch_rmse_cents": 20.531053348883464,
  "pitch_derivative_rmse_hz_s": 103.9835432595049,
  "vibrato_residual_rmse_cents": 21.483932319755915,
  "vibrato_true_residual_rmse_cents": 31.59184849536569
}

Important: this compares local CQT ridge geometry with the analytic generator, not raw CQT coefficient phase. Finite-Q response, hop sampling, overlap between harmonics, and amplitude calibration contribute to the residual error.
