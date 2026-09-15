# Multi-track spline tracker

## Objective

The implementation turns the reassigned log-frequency plane into a compact geometric model with several simultaneous trajectories. A trajectory is represented by `u_j[m] = log2(f_j[m]/fmin)` and later compressed into a C2 cubic spline / Bezier path.

## Energy field

For every complex reassigned coefficient `Z[k,m]` with reassigned frequency `fhat`, the algorithm scatters `|Z|^2` linearly onto the neighbouring log-frequency bins. The resulting tensor is

`P[m,k] >= 0`.

This makes the tracker independent of the original filter-bank channel geometry: it works on the reassigned energy field.

## Initial trajectory extraction

Each trajectory is initialized by dynamic programming. For a path `k_m`, the score is

`sum_m log(1 + P[m,k_m]) - lambda_v sum_m (u_m-u_{m-1})^2`.

Transitions are restricted to `max_jump_oct`, which prevents arbitrary jumps between distant frequencies.

After a path is extracted, a Gaussian tube around it is suppressed from the working energy field. This greedy stage provides good separated seeds without solving the full K-track combinatorial problem.

## Joint refinement

The initial paths are then refined by coordinate descent on a shared objective:

`E = -sum_j,m log(1 + P[m,u_j(m)])`

`  + lambda_v sum_j,m (Delta u_j)^2`

`  + lambda_a sum_j,m (Delta^2 u_j)^2`

`  + lambda_r sum_{i<j,m} exp(-0.5 * ((u_i-u_j)/sigma_r)^2)`.

The first term attracts paths to energy. The second penalizes velocity. The third penalizes changes in velocity, i.e. discrete acceleration. The fourth creates soft repulsion between tracks so independent tracks do not collapse onto the same ridge.

Only local contributions affected by the coordinate being changed are recomputed. Therefore a candidate update is `O(K)` rather than reevaluating the entire objective. This is essential for a practical prototype.

The optimizer is coordinate descent, not a proof of global optimality. The result depends on initialization, regularization weights and the number of refinement passes.

## Geometric compression

After refinement, each sampled path is reduced to a configurable number of knots and passed through the existing natural cubic spline fitter. The spline is represented as cubic Bezier segments. The resulting object has explicit `C2` geometric continuity at interior knots.

## Relation to the vector-audio model

The output path provides the fundamental candidate `u_0(t)`. Further partials can be expressed relative to it:

`u_k(t) = u_0(t) + Delta_u_k(t)`.

This creates a pitch/timbre separation: a global pitch translation changes `u_0` while leaving `Delta_u_k` approximately invariant.

## Test strategy

`tests/multi_track.rs` constructs several smooth log-frequency trajectories in a synthetic energy field and checks that the refined paths remain distinct. The existing infinite Monte Carlo driver can be extended to synthesize audio, run phase-gradient reassignment, call `track_multiple`, and compare the recovered paths against the generated truth.

The Rust toolchain was not available in this environment, so compilation was not claimed. The source has been organized as a conventional Rust crate and the algorithmic design is intended to be compiled with stable Rust plus `rand` and `rayon`.
