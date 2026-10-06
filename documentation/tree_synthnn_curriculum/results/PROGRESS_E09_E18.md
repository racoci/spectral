# Curriculum progress E09–E18

The curriculum is being promoted conservatively: a stage is only promoted when the current hypothesis is identifiable and the learned component passes the structural validation gate.

## Decisions

- **E09:** Promote. Continuous sinusoidal LFO parameters remain analytic; learn only discrete waveform type.
- **E10:** Promote. Isolated FM is analytic.
- **E11:** Stop/redirect. PM and FM are exactly non-identifiable in the isolated model.
- **E12:** Promote. Isolated AM is analytic.
- **E13:** Promote. Fractal-noise slope/mix are analytic in the controlled residual model.
- **E14:** Provisional promote. Event timing is usable, but the next experiment should add sub-frame correction under CQT/jets.
- **E15:** Do not promote. Raw FFT peak tracking is not robust enough for multi-voice separation.
- **E16:** Promote as an analytic preconditioner.
- **E17:** Promote. A 177-parameter relational edge head reaches F1 0.952 on fixed 3-node trees.
- **E18:** Do not promote. The same head reaches only F1 0.625 on variable 2–5 node topologies.

The next structural experiment should therefore be a hierarchical TreeNN embedding that explicitly aggregates child evidence before edge classification, rather than simply increasing the pairwise MLP.
