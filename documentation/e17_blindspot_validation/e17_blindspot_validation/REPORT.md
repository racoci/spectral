# E17.1–E17.8 Blind-Spot Validation
This suite is deliberately diagnostic: it tests representational coverage, identifiability, confounding, and forecast sensitivity before promotion to E18.
## Results
- E17.1 dynamic timbre: median per-partial local-projection error = 0.056 dB; p95 = 0.127 dB.
- E17.2 attack/transient: residual energy in first 80 ms = median 0.291; after 120 ms = median 0.001. The transient is materially outside the current steady harmonic model.
- E17.3 harmonic collision: median condition number = 1.04e+00; exact f2=2*f1 with both voices carrying 6 harmonics gives rank 18 for 24 columns, nullity 6.
- E17.4 relative harmonic phase: independent partial phases give median residual 1.101 rad under the strict phase law, while strict-phase samples give 1.264e-03 rad.
- E17.5 realistic RIR: a 5-parameter Schroeder-like approximation leaves median relative RIR error 0.959; early 80 ms mismatch median 0.932.
- E17.6 horizon 0.02 s: mean phase error from the sampled frequency-estimation noise = 0.003 rad.
- E17.6 horizon 0.05 s: mean phase error from the sampled frequency-estimation noise = 0.007 rad.
- E17.6 horizon 0.10 s: mean phase error from the sampled frequency-estimation noise = 0.014 rad.
- E17.6 horizon 0.20 s: mean phase error from the sampled frequency-estimation noise = 0.029 rad.
- E17.6 horizon 0.50 s: mean phase error from the sampled frequency-estimation noise = 0.072 rad.
- E17.6 horizon 1.00 s: mean phase error from the sampled frequency-estimation noise = 0.144 rad.
- E17.7 equivalence: PM/FM max waveform difference = 0.000e+00; time-shift/phase-gauge max difference = 8.972e-13; filter-vs-envelope construction is exact to 0.000e+00.
- E17.8 adversarial Jacobian: worst observed normalized condition number = 8.11e+07; worst normalized sigma_min = 2.384e-08.

## Interpretation
1. E17.1 is representable, but the old E06/E07 tests do not establish dynamic H_k(t) recovery. A dedicated temporal timbre state is required.
2. E17.2 exposes an omitted excitation/residual field at attacks. A symbolic harmonic model alone is insufficient for pluck/chiff/breath-like transients.
3. E17.3 contains a true identifiability failure: exact harmonic collisions can create a non-zero null space even with noiseless observations. This is not a TreeNN capacity problem.
4. E17.4 shows that strict relative harmonic phase is an additional model assumption, not a consequence of additive synthesis. It needs an explicit gauge/class.
5. E17.5 shows that a simple Schroeder parameterization cannot be assumed to represent arbitrary room responses; the mismatch should enter a residual/effect-field component.
6. E17.6 quantifies why small instantaneous frequency errors become large phase errors with prediction horizon. Jet quality must therefore be judged by downstream forecast error, not only local RMSE.
7. E17.7 should be formalized before graph recovery/model selection: PM/FM, time-shift/phase, and filter/envelope can belong to the same observable equivalence class.
8. E17.8 should become an adversarial data generator: sample parameter configurations near low singular values instead of relying only on IID and hand-built hard cases.

## Promotion decision
E18 should remain blocked until E17.1–E17.7 are represented as explicit benchmark families and E17.8 is integrated into the hard-set generator. The evidence indicates that the main remaining risks are model-class omission and identifiability geometry, not insufficient TreeNN width.
