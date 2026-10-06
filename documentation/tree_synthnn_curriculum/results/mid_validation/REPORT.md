# E06-E08 validation results

All promotion gates passed.

| Stage | Global trainable scalars | IID | OOD | Status |
|---|---:|---|---|---|
| E06 | 0 | p95 logH RMSE = 9.829e-09 | p95 logH RMSE = 7.332e-09 | PASS |
| E07 | 0 | p95 abs(c2) = 4.496e-08 | p95 abs(c2) = 7.132e-08 | PASS |
| E08 | 0 | p95 relative B = 0.364% | p95 relative B = 0.010% | PASS |

E08 keeps f0 fixed at the E03 estimate. Therefore it cannot hide an f0 error by jointly refitting f0 and B.

These are controlled synthetic promotion tests. They validate the inverse operators under the restricted E06-E08 model, not the behavior of the full real-audio pipeline.
