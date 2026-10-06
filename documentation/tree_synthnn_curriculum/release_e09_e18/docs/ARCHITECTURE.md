# Final architecture and freeze schedule

The final system has five logical layers:

\[
x\rightarrow F\rightarrow A\rightarrow I\rightarrow S(G,\Theta)
\]

where `F` is the fixed CQT/reassignment/jet frontend, `A` is ridge/event organization, `I` is the learned inverse, and `S` is the differentiable SynthNN generator.

The learned inverse is factorized:

\[
I=(E,P,G),
\]

with an encoder `E`, specialized parameter heads `P`, and TreeNN graph inference `G`.

A staged freeze schedule is:

```text
E00-E01  no neural learning
E02      ridge head only
E03      pitch head only
E04-E05 envelope/ADSR head only
E06      harmonic head only
E07      spectral-envelope head only
E08      inharmonicity residual only
E09      LFO head only
E10     FM head only
E11     PM head only
E12     AM head only
E13     noise head only
E14     one effect head at a time
E15     event head only
E16     TreeNN parameter edges; topology supplied
E17     TreeNN edge classifier only
E18     TreeNN topology decoder
E19     TreeNN module-type decoder
E20     analysis-by-synthesis residual + structural refinement
E21     causal predictor + structural refinement
```

During E20 and E21 the SynthNN remains a fixed differentiable generator. Only its *example-specific latent parameters* are optimized by the offline oracle; its generator weights are not retrained.
