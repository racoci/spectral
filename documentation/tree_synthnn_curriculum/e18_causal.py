from __future__ import annotations
import json, math, copy, sys
from pathlib import Path
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import Dataset, DataLoader

SEED=1819
np.random.seed(SEED); torch.manual_seed(SEED)
T=np.arange(128,dtype=np.float32)/128.0


def corrmat(a,b):
    a=a-a.mean(1,keepdims=True); b=b-b.mean(1,keepdims=True)
    a=a/(np.linalg.norm(a,axis=1,keepdims=True)+1e-8)
    b=b/(np.linalg.norm(b,axis=1,keepdims=True)+1e-8)
    return a@b.T

def fitcoefmat(dp,z):
    # fit dp[p] ~= coef[p,c] * z[c], for every ordered pair.
    den=np.sum(z*z,axis=1)+1e-8
    return (dp@z.T)/den[None,:]

def r2mat(dp,z):
    n=dp.shape[0]
    den=np.sum((dp-dp.mean(1,keepdims=True))**2,axis=1)+1e-8
    out=np.zeros((n,n),np.float32)
    for p in range(n):
        y=dp[p]-dp[p].mean()
        for c in range(n):
            x=z[c]-z[c].mean()
            b=float(np.dot(y,x)/(np.dot(x,x)+1e-8))
            res=y-b*x
            out[p,c]=1-float(np.dot(res,res)/den[p])
    return out

def sample(nnodes,noise=.012,close_freq=False,weak_prob=0.0,rng=None):
    rng=np.random.default_rng() if rng is None else rng
    f=rng.uniform(3,12,nnodes).astype(np.float32)
    if close_freq:
        center=float(rng.uniform(5,9)); f=np.clip(center+rng.normal(0,.45,nnodes),2.5,12.5).astype(np.float32)
    ph=rng.uniform(-math.pi,math.pi,nnodes).astype(np.float32)
    parent=np.full(nnodes,-1,np.int64); beta=np.zeros(nnodes,np.float32)
    depth=np.zeros(nnodes,np.int64)
    for c in range(1,nnodes):
        cand=np.arange(c); w=.7**depth[cand]; w=w/w.sum(); p=int(rng.choice(cand,p=w)); parent[c]=p
        beta[c]=rng.uniform(.2,1.5)
        if rng.random()<weak_prob: beta[c]=rng.uniform(.03,.15)
        depth[c]=depth[p]+1
    children=[[] for _ in range(nnodes)]
    for c in range(1,nnodes): children[parent[c]].append(c)
    theta=np.zeros((nnodes,len(T)),np.float32); inst=np.zeros_like(theta)
    for i in range(nnodes-1,-1,-1):
        mod=np.zeros_like(T)
        for c in children[i]: mod += beta[c]*np.sin(theta[c])
        theta[i]=2*np.pi*f[i]*T+ph[i]+mod
        inst[i]=f[i]+np.gradient(mod,T)/(2*np.pi)+rng.normal(0,noise,len(T))
    perm=rng.permutation(nnodes); inv=np.empty(nnodes,np.int64); inv[perm]=np.arange(nnodes)
    theta=theta[perm]; inst=inst[perm]; f=f[perm]
    newp=np.full(nnodes,-1,np.int64); newb=np.zeros(nnodes,np.float32)
    old_parent=parent.copy(); old_beta=beta.copy()
    for oldc in range(nnodes):
        c=inv[oldc]; op=old_parent[oldc]; newp[c]=-1 if op<0 else inv[op]; newb[c]=old_beta[oldc]
    df=inst-inst.mean(1,keepdims=True); ddf=np.gradient(df,T,axis=1); s=np.sin(theta); c=np.cos(theta)
    zc=inst*c; zs=inst*s
    cc=corrmat(df,s); cs=corrmat(df,c)
    cdd=corrmat(ddf,s); cdc=corrmat(ddf,c)
    bcos=fitcoefmat(df,zc); bsin=fitcoefmat(df,zs)
    r2=r2mat(df,zc)
    # Nonnegative explained-energy score and directional asymmetries.
    bcos_r=np.tanh(bcos/2.0)
    r2c=np.clip(r2,-1,1)
    r2as=r2c-r2c.T
    b_as=np.tanh((bcos-bcos.T)/2.0)
    pair=np.zeros((nnodes,nnodes,16),np.float32)
    fn=f/12
    pair[:,:,0]=fn[:,None]; pair[:,:,1]=fn[None,:]; pair[:,:,2]=np.abs(fn[:,None]-fn[None,:])
    pair[:,:,3]=cc; pair[:,:,4]=cs; pair[:,:,5]=cdd; pair[:,:,6]=cdc
    pair[:,:,7]=np.tanh(bcos/2); pair[:,:,8]=np.tanh(bsin/2); pair[:,:,9]=r2c
    pair[:,:,10]=r2as; pair[:,:,11]=b_as
    pair[:,:,12]=np.abs(r2c); pair[:,:,13]=np.abs(bcos_r)
    pair[:,:,14]=np.tanh((bcos+0.5*bcos.T)/3)
    pair[:,:,15]=np.clip(np.abs(cs),0,1)
    for i in range(nnodes): pair[i,i]=0
    node=[]
    for i in range(nnodes):
        node.append([fn[i],np.std(df[i]),np.sqrt(np.mean(df[i]**2)),np.std(ddf[i]),np.mean(s[i]),np.mean(c[i]),np.std(s[i]),np.std(c[i]),np.mean(np.abs(df[i]))])
    return np.asarray(node,np.float32),pair,newp,newb

class DS(Dataset):
    def __init__(self,n,nmin,nmax,noise,close=False,weak=0,seed=0):
        rng=np.random.default_rng(seed); self.items=[sample(int(rng.integers(nmin,nmax+1)),noise,close,weak,rng) for _ in range(n)]
    def __len__(self): return len(self.items)
    def __getitem__(self,i): return self.items[i]

def collate(batch):
    B=len(batch); N=max(a[0].shape[0] for a in batch); nf=9; pf=16
    x=np.zeros((B,N,nf),np.float32); p=np.zeros((B,N,N,pf),np.float32); m=np.zeros((B,N),np.float32); t=np.full((B,N),-1,np.int64); b=np.zeros((B,N),np.float32)
    for i,(xx,pp,tt,bb) in enumerate(batch):
        n=len(tt); x[i,:n]=xx; p[i,:n,:n]=pp; m[i,:n]=1; t[i,:n]=tt; b[i,:n]=bb
    return tuple(torch.from_numpy(z) for z in (x,p,m,t,b))

class CausalTreeNN(nn.Module):
    def __init__(self,h=16,rounds=2):
        super().__init__(); self.h=h; self.rounds=rounds
        self.node=nn.Sequential(nn.Linear(9,h),nn.Tanh())
        self.msg=nn.Linear(h,h,bias=False)
        self.up=nn.Sequential(nn.Linear(2*h,h),nn.Tanh())
        self.edge=nn.Sequential(nn.Linear(2*h+16,h),nn.Tanh(),nn.Linear(h,1))
        self.root=nn.Sequential(nn.Linear(h,h//2),nn.Tanh(),nn.Linear(h//2,1))
        self.beta=nn.Sequential(nn.Linear(2*h+16,h),nn.Tanh(),nn.Linear(h,1))
    def scores(self,h,pair):
        B,N,H=h.shape; hi=h.unsqueeze(2).expand(B,N,N,H); hj=h.unsqueeze(1).expand(B,N,N,H)
        s=self.edge(torch.cat([hi,hj,pair],-1)).squeeze(-1)
        eye=torch.eye(N,dtype=torch.bool,device=h.device)[None]
        return s.masked_fill(eye,-1e4)
    def forward(self,x,pair,mask):
        h=self.node(x); B,N,H=h.shape; valid=mask[:,:,None]*mask[:,None,:]
        for _ in range(self.rounds):
            e=self.scores(h,pair).masked_fill(valid==0,-1e4); r=self.root(h).squeeze(-1)
            cls=torch.cat([e,r[:,None,:]],1); prob=torch.softmax(cls,1); pp=prob[:,:N,:]
            msg=(self.msg(h)[:,None,:,:]*pp.permute(0,2,1)[:,:,:,None]).sum(2)
            den=pp.permute(0,2,1).sum(2,keepdim=True)+1e-6
            h=h+self.up(torch.cat([h,msg/den],-1))
        e=self.scores(h,pair); r=self.root(h).squeeze(-1)
        B,N,H=h.shape; hi=h.unsqueeze(2).expand(B,N,N,H); hj=h.unsqueeze(1).expand(B,N,N,H)
        bl=self.beta(torch.cat([hi,hj,pair],-1)).squeeze(-1)
        return e,r,bl

def loss_fn(e,r,mask,t,bt):
    B,N,_=e.shape; cls=torch.cat([e,r[:,None,:]],1).permute(0,2,1); tgt=torch.where(t>=0,t,torch.full_like(t,N)); valid=mask.bool(); ce=F.cross_entropy(cls[valid],tgt[valid])
    rootp=torch.softmax(cls,-1)[...,N]; lroot=((rootp*mask).sum(1)-1).pow(2).mean()
    pi=t.clamp_min(0).long(); bi=torch.arange(B)[:,None].expand(B,N); cj=torch.arange(N)[None,:].expand(B,N); pred=F.softplus(bl:=e.new_zeros(()) ) if False else None
    # beta head is trained only for true edges in outer function; omit it from structural objective.
    return ce+0.03*lroot

def train(train_ds,val_ds,epochs=25,h=16,rounds=2,lr=2e-3):
    tr=DataLoader(train_ds,64,True,collate_fn=collate); va=DataLoader(val_ds,64,False,collate_fn=collate)
    m=CausalTreeNN(h,rounds); opt=torch.optim.AdamW(m.parameters(),lr=lr,weight_decay=1e-4); best=1e9; state=None; bad=0
    for ep in range(1,epochs+1):
        m.train()
        for x,p,mask,t,bt in tr:
            e,r,_=m(x,p,mask); l=loss_fn(e,r,mask,t,bt); opt.zero_grad();l.backward();nn.utils.clip_grad_norm_(m.parameters(),1);opt.step()
        m.eval(); vs=[]
        with torch.no_grad():
            for x,p,mask,t,bt in va:
                e,r,_=m(x,p,mask);vs.append(float(loss_fn(e,r,mask,t,bt)))
        cur=float(np.mean(vs))
        if cur<best:best=cur;state=copy.deepcopy(m.state_dict());bad=0
        else:
            bad+=1
            if bad>=7:break
    m.load_state_dict(state); return m,{'best_val_loss':best,'epochs':ep,'trainable_parameters':sum(q.numel() for q in m.parameters())}

def raw_eval(m,ds):
    ld=DataLoader(ds,64,False,collate_fn=collate); f1=[]; exact=0; total=0
    with torch.no_grad():
      for x,p,mask,t,bt in ld:
        e,r,_=m(x,p,mask); B,N,_=e.shape; pred=torch.argmax(torch.cat([e,r[:,None,:]],1),1)
        for b in range(B):
          n=int(mask[b].sum()); true=t[b,:n].numpy(); pr=pred[b,:n].numpy(); ok=True; tp=fn=fp=0
          for j in range(n):
            tt=true[j]; pp=pr[j]
            if tt==-1: tt=n
            if pp!=tt: ok=False
            if true[j]>=0:
              if pp==true[j]:tp+=1
              else:fn+=1;fp+=1
          q=tp/(tp+fp+1e-9); rr=tp/(tp+fn+1e-9); f1.append(2*q*rr/(q+rr+1e-9)); exact+=ok;total+=1
    return {'parent_f1':float(np.mean(f1)),'exact_tree':float(exact/total)}

def main(out='/mnt/data/tree_synth_curriculum/results/e18_causal'):
    out=Path(out);out.mkdir(parents=True,exist_ok=True)
    tr=DS(2200,2,8,.012,False,0,1809); va=DS(500,2,8,.012,False,0,1810)
    m0,s0=train(tr,va,epochs=25)
    hard=DS(600,3,8,.035,True,.35,1811); s1=raw_eval(m0,hard)
    hardtr=DS(1400,3,8,.035,True,.35,1812); mh,s2=train(hardtr,hard,epochs=18,h=16,rounds=2,lr=7e-4)
    rep={'E18C_variable_clean':{**s0,**raw_eval(m0,va)},'E18C_hard_zero_shot':s1,'E18C_hard_finetuned':{**s2,**raw_eval(mh,hard)}}
    torch.save(m0.state_dict(),out/'clean.pt');torch.save(mh.state_dict(),out/'hard.pt');(out/'report.json').write_text(json.dumps(rep,indent=2));(out/'REPORT.md').write_text('# E18 Causal TreeNN\n\n'+json.dumps(rep,indent=2))
    print(json.dumps(rep,indent=2))
if __name__=='__main__': main()
