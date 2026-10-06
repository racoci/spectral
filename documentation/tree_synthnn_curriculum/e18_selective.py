from __future__ import annotations
import json,sys
from pathlib import Path
import numpy as np, torch
from torch.utils.data import DataLoader
sys.path.insert(0,str(Path(__file__).resolve().parent))
import e18_hierarchical as base


def evaluate_selective(model, loader, coverages=(1.0,.9,.75,.5,.25)):
    rows=[]
    with torch.no_grad():
      samples=[]
      for node,pair,mask,target,bt in loader:
        e,r,_=model(node,pair,mask); B,N,_=e.shape
        probs=torch.softmax(torch.cat([e,r[:,None,:]],1),dim=1)
        pred=torch.argmax(probs,dim=1)
        for b in range(B):
          n=int(mask[b].sum()); confs=[]; correct=[]; true=target[b,:n].numpy(); pr=pred[b,:n].numpy()
          for j in range(n):
            pp=probs[b,:,j].numpy(); order=np.argsort(pp)[::-1]; c=pp[order[0]]-pp[order[1]]; t=n if true[j]<0 else true[j]; confs.append(float(c)); correct.append(int(pr[j]==t))
          tree_conf=float(min(confs)); tree_ok=int(all(correct)); samples.append((tree_conf,tree_ok,confs,correct))
    samples.sort(key=lambda z:z[0],reverse=True); total=len(samples)
    for cov in coverages:
      k=max(1,int(round(total*cov))); sel=samples[:k]; exact=float(np.mean([x[1] for x in sel])); edge_acc=float(np.mean([np.mean(x[3]) for x in sel]));
      rows.append({'coverage':cov,'accepted':k,'tree_exact':exact,'mean_edge_accuracy':edge_acc,'threshold_min_margin':float(sel[-1][0])})
    return rows

def main(out='/mnt/data/tree_synth_curriculum/results/e18_selective'):
  out=Path(out);out.mkdir(parents=True,exist_ok=True)
  m=base.HierarchicalTreeNN(hidden=16,rounds=2);m.load_state_dict(torch.load('/mnt/data/tree_synth_curriculum/results/e18_hierarchical/E18H3_hard_tree.pt',map_location='cpu'))
  hard=DataLoader(base.TreeDataset(n=1000,nmin=3,nmax=8,noise=.035,close_freq=True,weak_edge_prob=.35,seed=1823),64,False,collate_fn=base.collate)
  rows=evaluate_selective(m,hard)
  rep={'model_trainable_parameters':sum(p.numel() for p in m.parameters()),'selective':rows}
  (out/'report.json').write_text(json.dumps(rep,indent=2));(out/'REPORT.md').write_text('# E18 Selective TreeNN\n\n'+json.dumps(rep,indent=2));print(json.dumps(rep,indent=2))
if __name__=='__main__':main()
