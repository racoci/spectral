# E10–E12: FM, PM, AM

## E10 — FM

The isolated FM problem is solved analytically from the instantaneous-frequency ridge:

$f_{inst}(t)=f_c+b2 f_m \cos(2c0 f_m t+c6)$.

IID: {'fc_mae_hz': 0.0003141028551928571, 'fm_mae_hz': 0.00016294280740968326, 'beta_relative_median': 9.363541594057792e-06, 'phase_mae_deg': 0.05868784749389255, 'phase_p95_deg': 0.2093253010625723}
HARD: {'fc_mae_hz': 0.00041731930489311253, 'fm_mae_hz': 0.00015220260619635883, 'beta_relative_median': 1.7979178204368517e-05, 'phase_mae_deg': 0.054876488001095874, 'phase_p95_deg': 0.15482010053311965}

No trainable parameters are introduced.

## E11 — PM identifiability

For

$\\phi_{PM}(t)=2\\pi f_ct+I\\sin(2\\pi f_mt+\\psi)$

$\\frac{1}{2\\pi}\\frac{d\\phi_{PM}}{dt}=f_c+I f_m\\cos(2\\pi f_mt+\\psi)$

which is exactly the same instantaneous-frequency law as FM with beta = I. The generated waveforms are therefore identical under the same parameter substitution.

Maximum numerical signal difference over 200 trials: 0.000e+00
Numerical Jacobian rank: 1

**Decision: do not train a PM-vs-FM classifier in this isolated setting.** Additional structural priors are required.

## E12 — AM

AM/tremolo is directly estimated from the analytic envelope; no learned continuous parameters are needed in the isolated case.
Metrics: {'fm_mae_hz': 0.032655810533090954, 'depth_relative_median': 0.007404401284353863, 'phase_mae_deg': 11.75901894956586, 'phase_p95_deg': 21.416304190397554}
