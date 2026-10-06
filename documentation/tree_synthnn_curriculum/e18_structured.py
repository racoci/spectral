from __future__ import annotations
import copy, json, sys
from pathlib import Path
import numpy as np
import torch
import torch.nn.functional as F
from torch.utils.data import DataLoader
sys.path.insert(0,str(Path(__file__).resolve().parent))
import e18_hierarchical as base


def fixed_root_logZ(e, root, mask):
    n=e.shape[0]
    keep=torch.tensor([j for j in range(n) if j!=root],device=e.device,dtype=torch.long)
    eye=torch.eye(n,device=e.device,dtype=torch.bool)
    colmax=e.masked_fill(eye,-1e9).max(dim=0).values
    s=e-colmax.unsqueeze(0)
    valid=(~eye).to(e.dtype)
    root_col=torch.ones(n,device=e.device,dtype=e.dtype); root_col[root]=0
    W=torch.exp(s)*valid*root_col.unsqueeze(0)
    incoming=W.sum(dim=0)
    L=torch.diag(incoming)-W.T
    M=L.index_select(0,keep).index_select(1,keep)
    sign,ld=torch.linalg.slogdet(M)
    if not torch.isfinite(ld):
        return e.new_tensor(50.0), colmax
    return ld, colmax


def structured_loss(e, r, target, mask):
    B,N,_=e.shape
    losses=[]; root_losses=[]
    for b in range(B):
        n=int(mask[b].sum().item())
        eb=e[b,:n,:n]; rb=r[b,:n]; tb=target[b,:n]
        true_root=int((tb<0).nonzero(as_tuple=False)[0].item())
        true_score=torch.tensor(0.,dtype=e.dtype)
        for c in range(n):
            p=int(tb[c].item())
            if p>=0: true_score=true_score+eb[p,c]
        logz,colmax=fixed_root_logZ(eb,true_root,mask[b,:n])
        shift=colmax.sum()-colmax[true_root]
        losses.append(logz-(true_score-shift))
        root_losses.append(-F.log_softmax(rb,dim=0)[true_root])
    return torch.stack(losses).mean()+0.5*torch.stack(root_losses).mean()


def eval_raw(model,loader):
    return base.evaluate(model,loader)


def train_struct(model,train_ds,epochs=12,lr=3e-4):
    tr=DataLoader(train_ds,32,True,collate_fn=base.collate)
    opt=torch.optim.AdamW(model.parameters(),lr=lr,weight_decay=1e-4)
    for ep in range(1,epochs+1):
        model.train(); vals=[]
        for x,p,m,t,bt in tr:
            e,r,_=model(x,p,m); loss=structured_loss(e,r,t,m)
            opt.zero_grad();loss.backward();torch.nn.utils.clip_grad_norm_(model.parameters(),1.0);opt.step();vals.append(float(loss))
    return {'epochs':epochs,'train_loss':float(np.mean(vals))}


def main(out='/mnt/data/tree_synth_curriculum/results/e18_structured'):
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    m=base.HierarchicalTreeNN(hidden=16,rounds=2)
    prev=Path('/mnt/data/tree_synth_curriculum/results/e18_hierarchical/E18H2_variable_tree.pt')
    m.load_state_dict(torch.load(prev,map_location='cpu'))
    tr=base.TreeDataset(n=1000,nmin=2,nmax=8,noise=.012,seed=1820)
    clean=base.TreeDataset(n=500,nmin=2,nmax=8,noise=.012,seed=1821)
    hard=base.TreeDataset(n=500,nmin=3,nmax=8,noise=.035,close_freq=True,weak_edge_prob=.35,seed=1822)
    val=DataLoader(clean,64,False,collate_fn=base.collate); hd=DataLoader(hard,64,False,collate_fn=base.collate)
    before={'clean':eval_raw(m,val),'hard':eval_raw(m,hd)}
    trrep=train_struct(m,tr,epochs=12,lr=3e-4)
    after={'clean':eval_raw(m,val),'hard':eval_raw(m,hd)}
    torch.save(m.state_dict(),out/'structured.pt')
    rep={'trainable_parameters':sum(p.numel() for p in m.parameters()),'before':before,'training':trrep,'after':after}
    (out/'report.json').write_text(json.dumps(rep,indent=2));(out/'REPORT.md').write_text('# E18 Structured TreeNN\n\n'+json.dumps(rep,indent=2))
    print(json.dumps(rep,indent=2))
if __name__=='__main__':main()
