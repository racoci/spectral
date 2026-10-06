# Operational Curriculum: E00–E21

## Core mathematical policy

For stage k, partition all model parameters into trainable and frozen sets:

\[
\Omega = \Omega_k \cup \bar\Omega_k,
\qquad
\theta_i = \theta_i^{*} \;\; i \in \bar\Omega_k.
\]

Optimize only:

\[
\theta_{\Omega_k}^{*}
= \arg\min_{\theta_{\Omega_k}} L_k.
\]

Promotion occurs only after the stage satisfies validation thresholds and intervention tests.

## Experimental table

| ID | Objective | Trainable | Examples train/val | Complexity | Model | LR | Main losses | Promotion criterion |
|---|---|---|---|---|---|---:|---|---|
| E00 | Identifiability | none | 4k/500 | scalar subspaces | Jacobian+SVD | — | rank, conditioning | target subspace full-rank; no near-null direction |
| E01 | CQT + jets | none | 10k/1k | 1 component, J≤4 | fixed frontend | — | frequency/derivative error | pitch <1 cent; derivative rel. error <2% |
| E02 | Ridge | analysis residual | 12k/1.5k | 1 ridge | linear→16→1 | 3e-3 | pitch, tracking, smoothness | F1>0.995; hard RMSE<8 cents |
| E03 | f0 | pitch | 16k/2k | clean sine | 32→16→3 | 2e-3 | f, df, d2f | <1 cent IID; <3 cents OOD |
| E04 | amplitude | envelope | 10k/1k | scalar A | linear→16→1 | 2e-3 | log amplitude | <0.1 dB |
| E05 | ADSR | envelope | 24k/3k | 4 params | 64→32→4 | 1e-3 | onset, timing, sustain, env | <5% median parameter error |
| E06 | harmonics | harmonics | 30k/4k | K≤16 | shared k-conditioned MLP | 1e-3 | H, CQT, waveform | H RMSE<0.01 |
| E07 | spectral envelope | spectral envelope | 30k/4k | 2→quadratic→spline basis | low-D basis projection | 8e-4 | envelope, CQT, waveform | <0.75 dB; gauge residual <1e-4 |
| E08 | inharmonicity | B | 16k/2k | K=12 | analytic LS + residual MLP | 5e-4 | B, frequency | <5% relative |
| E09 | LFO | LFO | 24k/3k | single sinusoid | 64→32→4 | 1e-3 | depth, freq, phase | freq <2%; phase <5° |
| E10 | FM | FM | 30k/4k | one modulator | 96→48→8 | 8e-4 | FM params, J, audio | <5% median parameter error |
| E11 | PM | PM | 24k/3k | one modulator | 96→48→8 | 8e-4 | PM params, J, audio | <5% |
| E12 | AM | AM | 24k/3k | one modulator | 64→32→6 | 1e-3 | AM params, envelope, audio | depth RMSE<0.01 |
| E13 | noise | noise | 30k/4k | ρ, α, knee | 64→32→6 | 7e-4 | PSD, α, knee, audio | PSD log RMSE<1 dB |
| E14 | effects | effect group | 50k/6k | one effect at a time | effect-specific heads | 5e-4 | multiscale STFT, CQT, audio | <5% single-effect parameter error |
| E15 | events | events | 50k/6k | ≤1–4 notes | temporal onset/offset heads | 8e-4 | onset, offset, pitch, velocity | onset F1>0.995; <2 ms |
| E16 | known graph | modulation graph | 40k/5k | ≤4 nodes | message passing | 5e-4 | edge params, J, audio | edge RMSE<0.02 |
| E17 | edge inference | graph structure | 60k/8k | ≤6 nodes | shared node+pairwise head | 5e-4 | edge, degree, topology | edge F1>0.99; exact>0.95 |
| E18 | variable topology | graph structure | 100k/12k | ≤10 nodes | autoregressive TreeNN | 3e-4 | graph, node type, edge, cost | exact>0.90; normalized GED<0.05 |
| E19 | module selection | graph structure | 120k/15k | ≤12 nodes | TreeNN + Gumbel | 2e-4 | module type, graph, CQT, audio | type>0.98; exact graph>0.90 |
| E20 | analysis-by-synthesis | graph + residual | 150k/20k | ≤12 nodes | encoder+TreeNN+frozen SynthNN | 1e-4 | waveform, CQT, jets, graph, sparsity | SI-SDR>30 dB; graph>0.90 |
| E21 | predictive codec | causal residual + graph | 200k/25k | ≤8 voices, ≤16 nodes | causal predictor+TreeNN+SynthNN | 1e-4 | future wave/CQT/jet/rate/graph | report error-vs-horizon and beat equal-rate baseline |

## Dataset construction

Every example stores:

\[
(x, G, \Theta, J, \mathcal E).
\]

Do not store only waveforms. The exact generative state is the supervision target.

Generate three variants:

\[
x_{clean},\quad x_{perturbed},\quad x_{hard}.
\]

Also generate counterfactual pairs:

\[
\Theta' = \Theta + \Delta e_i.
\]

The isolation test is:

\[
I_i =
\frac{|\Delta \hat\theta_i|}
{\sum_{j\neq i}|\Delta\hat\theta_j|+\epsilon}.
\]

## Parameter distributions

Use log-domain sampling for positive quantities:

\[
p = \exp(u),\qquad u\sim U(\log p_{min},\log p_{max}).
\]

Use bounded sigmoid parameterization:

\[
p=p_{min}+(p_{max}-p_{min})\sigma(u).
\]

Frequency is evaluated in cents:

\[
d_f(f,\hat f)=1200\left|\log_2\frac{\hat f}{f}\right|.
\]

Phase is evaluated on the unit circle:

\[
d_\phi=|e^{i\hat\phi}-e^{i\phi}|.
\]

## Anti-degeneracy gauges

Keep the previously established factorization gauge:

\[
H_1=1,
\]

\[
E(440\,\mathrm{Hz})=0\ \mathrm{dB},
\]

\[
\left.\frac{dE}{d\log_2 f}\right|_{440}=0.
\]

Without this, the inverse problem has equivalent parameterizations.

## Validation taxonomy

### IID

Same parameter distribution as training.

### Compositional OOD

All individual values are known during training, but the combinations are novel.

### Structural OOD

Train on lower width/depth and test on larger trees.

### Parameter OOD

Test beyond training intervals.

### Hard/degenerate

Explicitly create nearly confounded mechanisms, e.g.

\[
f_m\approx f_0,
\]

or harmonic/envelope combinations with similar spectral effects.

## Promotion protocol

A stage cannot unlock the next family because training loss alone is low. It must satisfy:

1. IID criterion.
2. Compositional criterion.
3. OOD criterion.
4. Hard-case criterion.
5. Counterfactual intervention test.
6. No gradient leakage into frozen groups.

## Final predictive metric

For history ending at n:

\[
(G_n,\Theta_n)=I(x_{1:n}),
\]

then predict without observing the future:

\[
\hat x_{n+1:n+H} = S(G_n,\hat\Theta_{n+1:n+H}).
\]

Use:

\[
E(H)=
\frac{\|x_{n+1:n+H}-\hat x_{n+1:n+H}\|_2}
{\|x_{n+1:n+H}\|_2}.
\]

And measure rate:

\[
R=\frac{\text{transmitted bits}}{\text{predicted samples}}.
\]

The key figure is therefore:

\[
R\mapsto E(H)
\]

or, equivalently, prediction horizon versus additional bits.
