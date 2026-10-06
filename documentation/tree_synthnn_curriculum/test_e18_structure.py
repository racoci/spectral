import itertools, math, sys
from pathlib import Path
import numpy as np
import torch
sys.path.insert(0,str(Path(__file__).resolve().parent))
from e18_structured import fixed_root_logZ

# Exhaustive verification for N<=4: matrix-tree theorem partition == direct enumeration.
def enumerate_trees(n, root, score):
    total=0.0; count=0
    nodes=range(n)
    for parents in itertools.product(nodes, repeat=n-1):
        pmap={j:p for j,p in zip([j for j in nodes if j!=root],parents)}
        if any(pmap[j]==j for j in pmap): continue
        ok=True
        # exactly one root and every node reaches root when following parents
        for start in nodes:
            if start==root: continue
            seen=set(); u=start
            while u!=root:
                if u in seen or u not in pmap: ok=False; break
                seen.add(u); u=pmap[u]
            if not ok: break
        if not ok: continue
        s=0.0
        for j,p in pmap.items(): s+=score[p,j]
        total+=math.exp(s); count+=1
    return total,count

for n in [2,3,4]:
    torch.manual_seed(1800+n)
    score=torch.randn(n,n)*0.7
    score.fill_diagonal_(-1e9)
    for root in range(n):
        brute,count=enumerate_trees(n,root,score.numpy())
        lz,cm=fixed_root_logZ(score.clone(),root,torch.ones(n)); logz=float(lz + cm.sum() - cm[root])
        rel=abs(math.exp(logz)-brute)/(brute+1e-12)
        print({'n':n,'root':root,'trees':count,'relative_error':rel})
        assert rel<1e-6
print('E18 structured theorem tests: PASS')
