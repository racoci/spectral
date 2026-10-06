# E18 — Structural bottleneck resolution

## Main finding

The failure of the original E18 pairwise edge head is not primarily a lack of MLP capacity. The correct abstraction is a rooted tree with one parent per non-root node and a global structural decoder.

The current TreeNN therefore has four components:

1. compact node encoder;
2. relational edge scorer;
3. two rounds of soft child-to-parent message passing;
4. global rooted-tree decoder.

The trainable budget remains only 2,771 scalar parameters.

## Mathematical model

For node embeddings `h_i`, candidate directed edge scores are

` s_ij = F_e(h_i, h_j, r_ij) `

where `r_ij` contains relational jet information.

Soft parent probabilities are normalized jointly with a root class:

` p(i -> j) = exp(s_ij) / (exp(r_j) + sum_k exp(s_kj)) `

and

` p(root=j) = exp(r_j) / (exp(r_j) + sum_k exp(s_kj)) `.

Child evidence is aggregated back into candidate parents and the node embeddings are updated before the second edge-scoring pass.

## Global decoding

Independent argmax predictions can contain several roots or inconsistent local choices. For a selected root `r`, the final tree is decoded as a maximum-weight rooted arborescence.

For training, the structured partition function for a fixed root is computed by the directed Matrix-Tree Theorem. If `w_ij = exp(s_ij)`, the directed Laplacian is formed from incoming edge weights and the root row/column is removed. Then

` log Z_r = log det(L_r) `

and the structured negative log likelihood is

` L_struct = log Z_r - S(G_true) `.

A separate root cross-entropy term identifies the root.

The Matrix-Tree implementation was exhaustively verified for `N = 2, 3, 4`; relative numerical error was below `4e-7`.

## Results

### E18H1 — fixed four-node trees

2,771 trainable parameters.

Parent F1: 0.9900.

Exact tree accuracy: 0.9600.

### E18H2 — variable 2–8 nodes

2,771 trainable parameters.

Parent F1: 0.9666.

Raw independent parent prediction exact-tree rate: 0.0900.

With global rooted-arborescence decoding, exact-tree rate: 0.8800 on a clean 2–8-node test set.

### E18H3 — noisy / close-frequency / weak-edge regime

Zero-shot raw parent F1: 0.8227.

After progressive fine-tuning: parent F1 0.8583.

With global rooted-arborescence decoding after fine-tuning: exact-tree rate 0.524 on 500 hard examples.

### E18H4 — 8–10 nodes, hard regime

Global constrained decoding reaches 0.05 exact-tree accuracy and parent F1 0.7225. This is not promoted.

## Identifiability ceiling

An oracle analysis of the causal relation score showed that the hard regime itself contains ambiguous examples. Under the current 128-frame context and noise/weak-edge distribution, roughly one fifth to one quarter of true edges do not have a positive margin over the best false candidate.

Increasing temporal context from 128 to 512 samples improved the fraction of positive oracle margins from roughly 0.78 to 0.83 in the pilot, but did not remove the ambiguity.

Therefore the correct next objective is not to force the TreeNN to hallucinate an exact graph. We should expose an explicit uncertainty/residual path and distinguish:

`identified structure`

from

`structurally unresolved interaction`.

## Promotion decision

Promote E18H1/E18H2 as the structural core.

Do not promote E18H4.

Do not start E19 module-type selection yet.

The next experiment should combine multiscale temporal context, causal edge statistics, and an explicit abstention/residual mechanism. Only after this passes should the variable module-type problem be introduced.
