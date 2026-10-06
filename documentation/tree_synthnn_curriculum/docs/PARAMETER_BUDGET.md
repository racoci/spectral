# Minimum trainable state per stage

The numbers below describe the *new learned state*, not the frozen SynthNN generator weights.

| Stage | New learned quantities | Nominal output dimension |
|---|---|---:|
| E02 | ridge position/confidence/residual correction | 1–3 |
| E03 | f0, df/dt, d2f/dt2 | 3 |
| E04 | log amplitude | 1 |
| E05 | attack, decay, sustain, release | 4 |
| E06 | harmonic coefficient per queried k | 1 per k |
| E07 | low-dimensional spectral envelope coefficients | 3–8 |
| E08 | inharmonicity residual | 1 |
| E09 | LFO depth, rate, cos phase, sin phase | 4 |
| E10 | FM ratio, index, cos phase, sin phase + residual | 4–8 |
| E11 | PM ratio, index, cos phase, sin phase + residual | 4–8 |
| E12 | AM depth, rate, cos phase, sin phase | 4 |
| E13 | noise level, alpha, knee + residual | 3–6 |
| E14 | one effect's active controls | 1–8 |
| E15 | event onset, offset, pitch, velocity | 4 per event |
| E16 | edge strength/type for a known edge | 1–4 per edge |
| E17 | edge probability for a candidate pair | 1 per pair |
| E18 | node/parent/edge decisions | O(nodes + edges) |
| E19 | module type logits | O(number of candidate types) |
| E20 | analysis residual + graph refinement | small residual vector |
| E21 | causal state transition + sparse corrections | small state vector |

The rule is that a high-dimensional tensor should be replaced by a shared function whenever the semantics allow it. For example, harmonic amplitudes use a shared function of harmonic index k instead of an independent network head per harmonic.

For phase-like quantities always predict a two-dimensional unit-circle representation:

\[
(c,s)=(\cos\phi,\sin\phi),
\qquad
\hat\phi=\operatorname{atan2}(\hat s,\hat c).
\]

For positive quantities use a log parameterization. For bounded quantities use a sigmoid parameterization.
