from __future__ import annotations
import json, sys
from pathlib import Path
import numpy as np
import torch
import networkx as nx
from torch.utils.data import DataLoader
sys.path.insert(0, str(Path(__file__).resolve().parent))
import e18_hierarchical as base


def decode_tree(edge_logits, root_logits):
    n=edge_logits.shape[0]
    # Select one root, then compute a maximum-weight rooted arborescence.
    root=int(np.argmax(root_logits))
    G=nx.DiGraph(); G.add_nodes_from(range(n))
    for i in range(n):
        for j in range(n):
            if i==j or j==root: continue
            G.add_edge(i,j,weight=float(edge_logits[i,j]))
    # NetworkX returns a maximum spanning arborescence on all nodes when the graph is complete.
    A=nx.maximum_spanning_arborescence(G,attr='weight',preserve_attrs=True)
    parent=np.full(n,-1,dtype=np.int64)
    for i,j in A.edges(): parent[j]=i
    return parent


def eval_constrained(model, loader):
    model.eval(); exact=0; total=0; tp=fp=fn=0
    with torch.no_grad():
        for node,pair,mask,target,bt in loader:
            e,r,b=model(node,pair,mask)
            B,N,_=e.shape
            for bb in range(B):
                n=int(mask[bb].sum())
                pred=decode_tree(e[bb,:n,:n].numpy(), r[bb,:n].numpy())
                true=target[bb,:n].numpy().copy()
                for j in range(n):
                    if true[j]<0: true[j]=-1
                if np.array_equal(pred,true): exact+=1
                total+=1
                for j in range(n):
                    if true[j]>=0:
                        if pred[j]==true[j]: tp+=1
                        else: fn+=1; fp+=1
    p=tp/(tp+fp+1e-9); q=tp/(tp+fn+1e-9)
    return {'parent_precision':p,'parent_recall':q,'parent_f1':2*p*q/(p+q+1e-9),'exact_tree':exact/max(total,1),'n':total}


def main():
    out=Path('/mnt/data/tree_synth_curriculum/results/e18_hierarchical')
    m=base.HierarchicalTreeNN(hidden=16,rounds=2)
    m.load_state_dict(torch.load('/mnt/data/tree_synth_curriculum/results/e18_hierarchical/E18H2_variable_tree.pt',map_location='cpu'))
    hard=DataLoader(base.TreeDataset(n=500,nmin=2,nmax=8,noise=.012,seed=1824),64,False,collate_fn=base.collate)
    unseen=DataLoader(base.TreeDataset(n=200,nmin=3,nmax=8,noise=.035,close_freq=True,weak_edge_prob=.35,seed=1825),64,False,collate_fn=base.collate)
    rep={'E18S_constrained_clean':eval_constrained(m,hard),'E18S_constrained_hard':eval_constrained(m,unseen)}
    (out/'CONSTRAINED_DECODE.json').write_text(json.dumps(rep,indent=2)); print(json.dumps(rep,indent=2))
if __name__=='__main__': main()
