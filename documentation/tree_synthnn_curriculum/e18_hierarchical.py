from __future__ import annotations
import json, math, copy
from pathlib import Path
import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F
from torch.utils.data import DataLoader, Dataset

SEED = 1818
np.random.seed(SEED)
torch.manual_seed(SEED)

T = np.arange(128, dtype=np.float32) / 128.0
MAX_N = 8


def _corr(a, b):
    a = a - a.mean()
    b = b - b.mean()
    den = float(np.linalg.norm(a) * np.linalg.norm(b) + 1e-8)
    return float(np.dot(a, b) / den)


def _xcorr_max(a, b, maxlag=10):
    a = a - a.mean(); b = b - b.mean()
    den = np.linalg.norm(a) * np.linalg.norm(b) + 1e-8
    vals = []
    for lag in range(-maxlag, maxlag + 1):
        if lag < 0:
            aa, bb = a[:lag], b[-lag:]
        elif lag > 0:
            aa, bb = a[lag:], b[:-lag]
        else:
            aa, bb = a, b
        vals.append(float(np.dot(aa, bb) / den))
    vals = np.asarray(vals)
    k = int(np.argmax(np.abs(vals)))
    return float(vals[k]), float((k - maxlag) / len(a))


def generate_tree(nnodes: int, noise: float = 0.01, beta_lo: float = 0.2, beta_hi: float = 1.5,
                  close_freq: bool = False, weak_edge_prob: float = 0.0, rng=None):
    rng = np.random.default_rng() if rng is None else rng
    f0 = rng.uniform(3.0, 12.0, nnodes).astype(np.float32)
    if close_freq and nnodes > 1:
        center = float(rng.uniform(5.0, 9.0))
        f0 = np.clip(center + rng.normal(0, 0.45, nnodes), 2.5, 12.5).astype(np.float32)
    phase0 = rng.uniform(-math.pi, math.pi, nnodes).astype(np.float32)

    # Canonical rooted tree: node 0 is root; every other node has exactly one parent.
    parent = np.full(nnodes, -1, dtype=np.int64)
    beta = np.zeros(nnodes, dtype=np.float32)
    depth = np.zeros(nnodes, dtype=np.int64)
    for child in range(1, nnodes):
        candidates = np.arange(child)
        # Prefer shallow nodes so both shallow and deep structures occur.
        weights = 0.7 ** depth[candidates]
        weights = weights / weights.sum()
        p = int(rng.choice(candidates, p=weights))
        parent[child] = p
        beta[child] = rng.uniform(beta_lo, beta_hi)
        if weak_edge_prob > 0 and rng.random() < weak_edge_prob:
            beta[child] = rng.uniform(0.03, 0.15)
        depth[child] = depth[p] + 1

    # Canonical modulation: parent phase is modulated by its children.
    children = [[] for _ in range(nnodes)]
    for c in range(1, nnodes):
        children[parent[c]].append(c)
    theta = np.zeros((nnodes, len(T)), dtype=np.float32)
    inst_f = np.zeros_like(theta)
    for i in range(nnodes - 1, -1, -1):
        th = 2 * np.pi * f0[i] * T + phase0[i]
        mod = np.zeros_like(T)
        for c in children[i]:
            mod += beta[c] * np.sin(theta[c])
        theta[i] = th + mod
        dmod = np.gradient(mod, T)
        inst_f[i] = f0[i] + dmod / (2 * np.pi) + rng.normal(0, noise, len(T))

    # Randomly permute node identities to prevent exploiting canonical index/order.
    perm = rng.permutation(nnodes)
    inv = np.empty(nnodes, dtype=np.int64)
    inv[perm] = np.arange(nnodes)
    f = f0[perm]
    th = theta[perm]
    ff = inst_f[perm]
    new_parent = np.full(nnodes, -1, dtype=np.int64)
    new_beta = np.zeros(nnodes, dtype=np.float32)
    for c_old in range(nnodes):
        c = inv[c_old]
        p_old = parent[c_old]
        new_parent[c] = -1 if p_old < 0 else inv[p_old]
        new_beta[c] = beta[c_old]

    node_feat = []
    for i in range(nnodes):
        df = ff[i] - np.mean(ff[i])
        ddf = np.gradient(df, T)
        s = np.sin(th[i]); c = np.cos(th[i])
        node_feat.append([
            f[i] / 12.0,
            float(np.std(df)),
            float(np.sqrt(np.mean(df * df))),
            float(np.std(ddf)),
            float(np.mean(s)),
            float(np.mean(c)),
            float(np.std(s)),
            float(np.std(c)),
            float(np.mean(np.abs(df))),
        ])
    node_feat = np.asarray(node_feat, dtype=np.float32)

    # Pair features for candidate edge parent -> child, vectorized across all node pairs.
    df = ff - ff.mean(axis=1, keepdims=True)
    ddf = np.gradient(df, T, axis=1)
    sinp = np.sin(th); cosp = np.cos(th)
    def corrmat(a, b):
        an = a / (np.linalg.norm(a, axis=1, keepdims=True) + 1e-8)
        bn = b / (np.linalg.norm(b, axis=1, keepdims=True) + 1e-8)
        return an @ bn.T
    c_ds = corrmat(df, sinp)
    c_dc = corrmat(df, cosp)
    c_dds = corrmat(ddf, sinp)
    c_ddc = corrmat(ddf, cosp)
    pair = np.zeros((nnodes, nnodes, 10), dtype=np.float32)
    f_norm = f / 12.0
    pair[:,:,0] = f_norm[:,None]
    pair[:,:,1] = f_norm[None,:]
    pair[:,:,2] = np.abs(f_norm[:,None] - f_norm[None,:])
    pair[:,:,3] = c_ds
    pair[:,:,4] = c_dc
    pair[:,:,5] = c_dds
    pair[:,:,6] = c_ddc
    pair[:,:,7] = np.maximum(c_ds, c_dc)
    pair[:,:,8] = np.maximum(np.abs(c_ds), np.abs(c_dc))
    pair[:,:,9] = np.minimum(c_ds, c_dc)
    for i in range(nnodes): pair[i,i,:] = 0

    target = new_parent
    return node_feat, pair, target, new_beta


class TreeDataset(Dataset):
    def __init__(self, n: int, nmin=2, nmax=8, noise=0.01, close_freq=False, weak_edge_prob=0.0, seed=0):
        self.rng = np.random.default_rng(seed)
        self.items = []
        for _ in range(n):
            nnodes = int(self.rng.integers(nmin, nmax + 1))
            self.items.append(generate_tree(nnodes, noise=noise, close_freq=close_freq,
                                            weak_edge_prob=weak_edge_prob, rng=self.rng))

    def __len__(self): return len(self.items)
    def __getitem__(self, i): return self.items[i]


def collate(batch):
    B = len(batch)
    N = max(x[0].shape[0] for x in batch)
    nf = batch[0][0].shape[1]
    pf = batch[0][1].shape[-1]
    node = np.zeros((B, N, nf), np.float32)
    pair = np.zeros((B, N, N, pf), np.float32)
    mask = np.zeros((B, N), np.float32)
    targets = np.full((B, N), -1, np.int64)  # parent index; -1 = invalid padding
    betas = np.zeros((B, N), np.float32)
    for b,(x,p,t,bb) in enumerate(batch):
        n=x.shape[0]
        node[b,:n]=x; pair[b,:n,:n]=p; mask[b,:n]=1; targets[b,:n]=t; betas[b,:n]=bb
    return torch.from_numpy(node), torch.from_numpy(pair), torch.from_numpy(mask), torch.from_numpy(targets), torch.from_numpy(betas)


class HierarchicalTreeNN(nn.Module):
    """Small hierarchical TreeNN: node encoder -> soft parent graph -> message pass -> parent/root decoder."""
    def __init__(self, node_dim=9, pair_dim=10, hidden=16, rounds=2):
        super().__init__()
        self.rounds = rounds
        self.node_enc = nn.Sequential(nn.Linear(node_dim, hidden), nn.Tanh())
        self.msg = nn.Linear(hidden, hidden, bias=False)
        self.update = nn.Sequential(nn.Linear(2*hidden, hidden), nn.Tanh(), nn.Linear(hidden, hidden))
        self.edge = nn.Sequential(nn.Linear(2*hidden + pair_dim, hidden), nn.Tanh(), nn.Linear(hidden, 1))
        self.root = nn.Sequential(nn.Linear(hidden, hidden//2), nn.Tanh(), nn.Linear(hidden//2, 1))
        self.beta_head = nn.Sequential(nn.Linear(2*hidden + pair_dim, hidden), nn.Tanh(), nn.Linear(hidden, 1))

    def edge_logits(self, h, pair):
        B,N,H = h.shape
        hi = h.unsqueeze(2).expand(B,N,N,H)
        hj = h.unsqueeze(1).expand(B,N,N,H)
        q = torch.cat([hi, hj, pair], dim=-1)
        logits = self.edge(q).squeeze(-1)
        eye = torch.eye(N, device=h.device, dtype=torch.bool).unsqueeze(0)
        return logits.masked_fill(eye, -1e4)

    def forward(self, node, pair, mask):
        h = self.node_enc(node)
        B,N,H = h.shape
        valid_pair = mask.unsqueeze(1) * mask.unsqueeze(2)
        final_logits = None
        root_logits = None
        for _ in range(self.rounds):
            e = self.edge_logits(h, pair)
            e = e.masked_fill(valid_pair == 0, -1e4)
            # For each child j, normalize over possible parents i plus explicit root class.
            r = self.root(h).squeeze(-1)
            class_logits = torch.cat([e, r.unsqueeze(1)], dim=1)  # B, N+1, N
            p = torch.softmax(class_logits, dim=1)
            parent_p = p[:, :N, :]
            child_from = self.msg(h).unsqueeze(1) * parent_p.permute(0,2,1).unsqueeze(-1)
            # sum over children for each candidate parent
            msg = child_from.sum(dim=2)
            denom = parent_p.permute(0,2,1).sum(dim=2, keepdim=True) + 1e-6
            msg = msg / denom
            h = h + self.update(torch.cat([h, msg], dim=-1))
            final_logits = e
            root_logits = self.root(h).squeeze(-1)
        beta = self.beta_head(torch.cat([
            h.unsqueeze(2).expand(B,N,N,H),
            h.unsqueeze(1).expand(B,N,N,H),
            pair
        ], dim=-1)).squeeze(-1)
        return final_logits, root_logits, beta


def loss_fn(edge_logits, root_logits, beta_logits, mask, target, beta_true):
    B,N,_ = edge_logits.shape
    classes = torch.cat([edge_logits, root_logits.unsqueeze(1)], dim=1).permute(0,2,1)
    tgt = torch.where(target >= 0, target, torch.full_like(target, N))
    valid = mask.bool()
    lp = F.cross_entropy(classes[valid], tgt[valid])
    # A rooted tree has exactly one root; softly penalize multiple-root solutions.
    p_root = torch.softmax(classes, dim=-1)[..., N]
    root_count = (p_root * mask).sum(dim=1)
    lroot = F.mse_loss(root_count, torch.ones_like(root_count))
    parent_idx = target.clamp_min(0).long()
    batch_idx = torch.arange(B, device=target.device).unsqueeze(1).expand(B,N)
    child_idx = torch.arange(N, device=target.device).unsqueeze(0).expand(B,N)
    pred_b = beta_logits[batch_idx, parent_idx, child_idx]
    pos = (target >= 0) & valid
    if pos.any():
        lb = F.smooth_l1_loss(F.softplus(pred_b[pos]), beta_true[pos])
    else:
        lb = edge_logits.new_zeros(())
    return lp + 0.02*lroot + 0.05*lb, lp.detach(), (lb + lroot).detach()


def evaluate(model, loader, device='cpu'):
    model.eval(); total=0; exact=0; edge_tp=edge_fp=edge_fn=0; root_ok=0; root_total=0
    beta_err=[]
    with torch.no_grad():
        for node,pair,mask,target,bt in loader:
            node,pair,mask,target,bt=[z.to(device) for z in (node,pair,mask,target,bt)]
            e,r,b=model(node,pair,mask)
            B,N,_=e.shape
            class_logits=torch.cat([e,r.unsqueeze(1)],dim=1) # B,N+1,N
            pred=torch.argmax(class_logits,dim=1) # B,N
            for bb in range(B):
                n=int(mask[bb].sum().item()); total+=1
                ok=True
                for c in range(n):
                    t=int(target[bb,c].item()); q=int(pred[bb,c].item())
                    if t==-1: continue
                    if q!=t: ok=False
                    if t>=0:
                        if q==t: edge_tp += 1
                        else: edge_fn += 1
                    if q<n and t>=0 and q!=t: edge_fp += 1
                    if t==-1 and q==n: root_ok+=1
                # exactly one root should be predicted; compare full parent vector
                true_struct = target[bb,:n].clone()
                true_struct[true_struct < 0] = n
                if torch.equal(pred[bb,:n].cpu(), true_struct.cpu()):
                    exact+=1
    precision=edge_tp/(edge_tp+edge_fp+1e-9); recall=edge_tp/(edge_tp+edge_fn+1e-9)
    return {'parent_precision':float(precision),'parent_recall':float(recall),'parent_f1':float(2*precision*recall/(precision+recall+1e-9)),
            'exact_tree':float(exact/max(total,1))}


def train_stage(name, train_kwargs, val_kwargs, epochs=50, batch_size=64, hidden=16, rounds=2, lr=2e-3, out_dir=None):
    train_ds=TreeDataset(**train_kwargs)
    val_ds=TreeDataset(**val_kwargs)
    tr=DataLoader(train_ds,batch_size=batch_size,shuffle=True,collate_fn=collate)
    va=DataLoader(val_ds,batch_size=batch_size,shuffle=False,collate_fn=collate)
    model=HierarchicalTreeNN(hidden=hidden,rounds=rounds)
    opt=torch.optim.AdamW(model.parameters(),lr=lr,weight_decay=1e-4)
    best=float('inf'); best_state=None; bad=0
    for ep in range(1,epochs+1):
        model.train()
        for batch in tr:
            node,pair,mask,target,bt=[z for z in batch]
            e,r,b=model(node,pair,mask)
            loss,_,_=loss_fn(e,r,b,mask,target,bt)
            opt.zero_grad(); loss.backward(); nn.utils.clip_grad_norm_(model.parameters(),1.0); opt.step()
        model.eval(); vals=[]
        with torch.no_grad():
            for batch in va:
                node,pair,mask,target,bt=batch
                e,r,b=model(node,pair,mask)
                vals.append(float(loss_fn(e,r,b,mask,target,bt)[0]))
        vl=float(np.mean(vals))
        if vl<best:
            best=vl; best_state=copy.deepcopy(model.state_dict()); bad=0
        else:
            bad+=1
            if bad>=8: break
    model.load_state_dict(best_state)
    metrics=evaluate(model,va)
    if out_dir:
        out=Path(out_dir); out.mkdir(parents=True,exist_ok=True)
        torch.save(model.state_dict(),out/f'{name}.pt')
    return model, {'best_val_loss':best,'epochs':ep,'trainable_parameters':sum(p.numel() for p in model.parameters()), **metrics}


def main(out='/mnt/data/tree_synth_curriculum/results/e18_hierarchical'):
    out=Path(out); out.mkdir(parents=True,exist_ok=True)
    # H1: fixed 4 nodes, clean; H2: variable 2-8, H3: hard close/weak/noisy.
    stages={}
    m1, s1=train_stage('E18H1_fixed4',
        {'n':900,'nmin':4,'nmax':4,'noise':0.01,'close_freq':False,'weak_edge_prob':0.0,'seed':1801},
        {'n':250,'nmin':4,'nmax':4,'noise':0.01,'close_freq':False,'weak_edge_prob':0.0,'seed':1802},
        epochs=30,hidden=16,rounds=2,out_dir=out/'checkpoints'); stages['E18H1']=s1
    # Reuse weights for variable topology, preserving learned representation and adding no capacity.
    m2=copy.deepcopy(m1)
    tr2=DataLoader(TreeDataset(n=1200,nmin=2,nmax=8,noise=0.012,seed=1803),64,shuffle=True,collate_fn=collate)
    va2=DataLoader(TreeDataset(n=300,nmin=2,nmax=8,noise=0.012,seed=1804),64,shuffle=False,collate_fn=collate)
    opt=torch.optim.AdamW(m2.parameters(),lr=1e-3,weight_decay=1e-4)
    best=float('inf'); state=None; bad=0
    for ep in range(1,31):
        m2.train()
        for batch in tr2:
            node,pair,mask,target,bt=batch; e,r,b=m2(node,pair,mask); loss,_,_=loss_fn(e,r,b,mask,target,bt)
            opt.zero_grad();loss.backward();nn.utils.clip_grad_norm_(m2.parameters(),1);opt.step()
        m2.eval(); vl=[]
        with torch.no_grad():
            for batch in va2:
                node,pair,mask,target,bt=batch; e,r,b=m2(node,pair,mask); vl.append(float(loss_fn(e,r,b,mask,target,bt)[0]))
        cur=float(np.mean(vl))
        if cur<best:best=cur;state=copy.deepcopy(m2.state_dict());bad=0
        else:
            bad+=1
            if bad>=8:break
    m2.load_state_dict(state); s2={'best_val_loss':best,'epochs':ep,'trainable_parameters':sum(p.numel() for p in m2.parameters()),**evaluate(m2,va2)}; stages['E18H2']=s2
    # Hard generalization: first evaluate zero-shot, then progressively fine-tune only the same 2771 weights.
    m3=copy.deepcopy(m2)
    easy_hard=DataLoader(TreeDataset(n=800,nmin=3,nmax=8,noise=0.02,close_freq=True,weak_edge_prob=0.15,seed=1805),64,shuffle=True,collate_fn=collate)
    opt3=torch.optim.AdamW(m3.parameters(),lr=5e-4,weight_decay=1e-4)
    for ep in range(8):
        m3.train()
        for batch in easy_hard:
            node,pair,mask,target,bt=batch; e,r,b=m3(node,pair,mask); loss,_,_=loss_fn(e,r,b,mask,target,bt)
            opt3.zero_grad();loss.backward();nn.utils.clip_grad_norm_(m3.parameters(),1);opt3.step()
    mid=DataLoader(TreeDataset(n=900,nmin=3,nmax=8,noise=0.035,close_freq=True,weak_edge_prob=0.35,seed=1806),64,shuffle=True,collate_fn=collate)
    for ep in range(10):
        m3.train()
        for batch in mid:
            node,pair,mask,target,bt=batch; e,r,b=m3(node,pair,mask); loss,_,_=loss_fn(e,r,b,mask,target,bt)
            opt3.zero_grad();loss.backward();nn.utils.clip_grad_norm_(m3.parameters(),1);opt3.step()
    hard_eval_loader=DataLoader(TreeDataset(n=500,nmin=3,nmax=8,noise=0.035,close_freq=True,weak_edge_prob=0.35,seed=1807),64,shuffle=False,collate_fn=collate)
    unseen_eval_loader=DataLoader(TreeDataset(n=400,nmin=8,nmax=10,noise=0.05,close_freq=True,weak_edge_prob=0.45,seed=1808),64,shuffle=False,collate_fn=collate)
    stages['E18H3_hard_zero_shot']=evaluate(m2,hard_eval_loader)
    stages['E18H3_hard_finetuned']=evaluate(m3,hard_eval_loader)
    stages['E18H4_unseen_10_nodes']=evaluate(m3,unseen_eval_loader)
    torch.save(m2.state_dict(),out/'E18H2_variable_tree.pt')
    torch.save(m3.state_dict(),out/'E18H3_hard_tree.pt')
    (out/'report.json').write_text(json.dumps(stages,indent=2))
    md=['# E18 Hierarchical TreeNN','','## Motivation','The flat pairwise edge head from E17 loses global tree context when topology and node identity vary. E18H uses an explicit parent-vs-root decoder and two rounds of soft child-to-parent message passing.','', '## Results', '```json', json.dumps(stages,indent=2), '```']
    (out/'REPORT.md').write_text('\n'.join(md))
    print(json.dumps(stages,indent=2))

if __name__=='__main__': main()
