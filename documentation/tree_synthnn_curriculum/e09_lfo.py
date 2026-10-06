from __future__ import annotations
import math, json
from dataclasses import dataclass
from pathlib import Path
import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import TensorDataset, DataLoader

SEED=9
rng=np.random.default_rng(SEED)
FS=200.0
DUR=2.0
N=int(FS*DUR)
T=np.arange(N)/FS


def synth_case(rng, f0_range=(180.,720.), fm_range=(1.0,9.0), depth_range=(2.,80.), noise_hz=0.25, drift_hz=0.0, hard=False):
    f0=rng.uniform(*f0_range)
    fm=rng.uniform(*fm_range)
    depth=rng.uniform(*depth_range)
    phase=rng.uniform(-math.pi, math.pi)
    # cents-domain vibrato: u(t)=1200 log2(f/f0)
    u=depth*np.sin(2*math.pi*fm*T+phase)
    if drift_hz:
        u += drift_hz*np.sin(2*math.pi*0.35*T+rng.uniform(-math.pi,math.pi))
    f=f0*(2.0**(u/1200.0))
    obs=f + rng.normal(0.0, noise_hz, N)
    if hard:
        # mild outliers emulate ridge perturbations, not signal failure
        idx=rng.choice(N, size=max(2,N//80), replace=False)
        obs[idx] += rng.normal(0, 2.0, idx.size)
    return f0,fm,depth,phase,obs


def fit_sine_grid(obs, f0, fm_min=0.75, fm_max=12.0, pad=16):
    # Fast analytic baseline: periodogram of cents-domain ridge, then local LS refit.
    u=1200*np.log2(np.maximum(obs, 1e-6)/f0)
    u=u-u.mean()
    n=len(u)
    nfft=n*pad
    Y=np.fft.rfft(u*np.hanning(n), nfft)
    freqs=np.fft.rfftfreq(nfft, 1/FS)
    mask=(freqs>=fm_min)&(freqs<=fm_max)
    idx=np.flatnonzero(mask)[np.argmax(np.abs(Y[mask])**2)]
    if 1 <= idx < len(Y)-1:
        ym1=np.log(np.abs(Y[idx-1])+1e-12); y0=np.log(np.abs(Y[idx])+1e-12); yp1=np.log(np.abs(Y[idx+1])+1e-12)
        den=(ym1-2*y0+yp1)
        delta=0.5*(ym1-yp1)/den if abs(den)>1e-12 else 0.0
    else:
        delta=0.0
    fm0=float(freqs[idx]+delta*(freqs[1]-freqs[0]))
    ang=2*np.pi*fm0*np.arange(n)/FS
    A=np.column_stack([np.ones(n),np.sin(ang),np.cos(ang)])
    coef, *_=np.linalg.lstsq(A,u,rcond=None)
    pred=A@coef
    b,c=coef[1],coef[2]
    depth0=float(np.hypot(b,c)); phase0=float(np.arctan2(c,b))
    residual=float(np.sqrt(np.mean((u-pred)**2)))
    return np.array([fm0,depth0,np.cos(phase0),np.sin(phase0),residual],np.float64)


def make_dataset(n, ranges, noise_hz, hard=False, drift_hz=0.0):
    X=[]; Y=[]
    for _ in range(n):
        f0,fm,d,p,obs=synth_case(rng, f0_range=ranges[0], fm_range=ranges[1], depth_range=ranges[2], noise_hz=noise_hz, drift_hz=drift_hz, hard=hard)
        a=fit_sine_grid(obs,f0)
        # normalized baseline features + labels as residuals
        feat=np.array([a[0]/10., a[1]/100., a[2], a[3], a[4]/100., f0/720.], np.float32)
        target=np.array([(fm-a[0])/10., (d-a[1])/100., np.cos(p)-a[2], np.sin(p)-a[3]], np.float32)
        X.append(feat); Y.append(target)
    return np.asarray(X), np.asarray(Y)

class ResidualLFO(nn.Module):
    def __init__(self):
        super().__init__()
        self.net=nn.Sequential(nn.Linear(6,32),nn.Tanh(),nn.Linear(32,16),nn.Tanh(),nn.Linear(16,4))
    def forward(self,x): return self.net(x)


def decode(base, delta):
    fm=base[:,0]*10+delta[:,0]*10
    dep=base[:,1]*100+delta[:,1]*100
    c=base[:,2]+delta[:,2]; s=base[:,3]+delta[:,3]
    n=torch.sqrt(c*c+s*s+1e-9); c=c/n; s=s/n
    phase=torch.atan2(s,c)
    return torch.stack([fm,dep,c,s,phase],1)


def labels_from_batch(X,Y):
    # reconstruct ground-truth from baseline+delta
    fm=X[:,0]*10 + Y[:,0]*10
    dep=X[:,1]*100 + Y[:,1]*100
    c=X[:,2]+Y[:,2]; s=X[:,3]+Y[:,3]
    n=np.sqrt(c*c+s*s)+1e-12
    c/=n; s/=n
    return np.column_stack([fm,dep,c,s,np.arctan2(s,c)])


def evaluate(model, X, Y, name):
    with torch.no_grad():
        xb=torch.from_numpy(X); pred=model(xb)
        out=decode(xb,pred).numpy()
    truth=labels_from_batch(X,Y)
    fm_abs=np.abs(out[:,0]-truth[:,0])
    dep_rel=np.abs(out[:,1]-truth[:,1])/truth[:,1]
    phase=np.abs(np.angle(np.exp(1j*(out[:,4]-truth[:,4]))))*180/np.pi
    return {
        'name':name,'n':len(X),
        'fm_mae_hz':float(np.mean(fm_abs)), 'fm_p95_hz':float(np.percentile(fm_abs,95)),
        'depth_relative_median':float(np.median(dep_rel)), 'depth_relative_p95':float(np.percentile(dep_rel,95)),
        'phase_mae_deg':float(np.mean(phase)), 'phase_p95_deg':float(np.percentile(phase,95))
    }

def baseline_eval(X,Y,name):
    truth=labels_from_batch(X,Y)
    base=np.column_stack([X[:,0]*10,X[:,1]*100,X[:,2],X[:,3]])
    phase=np.arctan2(base[:,3],base[:,2]); fm=np.abs(base[:,0]-truth[:,0]); dep=np.abs(base[:,1]-truth[:,1])/truth[:,1]
    ph=np.abs(np.angle(np.exp(1j*(phase-truth[:,4]))))*180/np.pi
    return {'name':name,'n':len(X),'fm_mae_hz':float(np.mean(fm)),'fm_p95_hz':float(np.percentile(fm,95)),'depth_relative_median':float(np.median(dep)),'depth_relative_p95':float(np.percentile(dep,95)),'phase_mae_deg':float(np.mean(ph)),'phase_p95_deg':float(np.percentile(ph,95))}


def main(outdir):
    out=Path(outdir); out.mkdir(parents=True,exist_ok=True)
    # Stage training: deliberately modest noise and no non-sinusoidal drift.
    Xtr,Ytr=make_dataset(24000,((180,720),(1.0,9.0),(2,80)),0.25,False,0.0)
    Xv,Yv=make_dataset(3000,((180,720),(1.0,9.0),(2,80)),0.25,False,0.0)
    Xi,Yi=make_dataset(5000,((180,720),(1.0,9.0),(2,80)),0.25,False,0.0)
    Xood,Yood=make_dataset(5000,((150,800),(0.6,12),(1,100)),0.35,True,0.0)
    Xhard,Yhard=make_dataset(5000,((180,720),(1.0,9.0),(2,80)),0.5,True,0.25)
    model=ResidualLFO()
    opt=torch.optim.AdamW(model.parameters(),lr=1e-3,weight_decay=1e-4)
    train=DataLoader(TensorDataset(torch.from_numpy(Xtr),torch.from_numpy(Ytr)),batch_size=256,shuffle=True)
    best=float('inf'); best_state=None; patience=0
    for epoch in range(100):
        model.train()
        for xb,yb in train:
            pred=model(xb)
            # residual targets are already scaled; phase vector is Euclidean here
            loss=(pred-yb).pow(2).mean()
            opt.zero_grad(); loss.backward(); opt.step()
        model.eval()
        with torch.no_grad():
            val=((model(torch.from_numpy(Xv))-torch.from_numpy(Yv)).pow(2).mean()).item()
        if val<best:
            best=val; best_state={k:v.detach().clone() for k,v in model.state_dict().items()}; patience=0
        else:
            patience+=1
            if patience>=10: break
    model.load_state_dict(best_state)
    report={'stage':'E09','trainable_parameters':sum(p.numel() for p in model.parameters()),'best_val_loss':best,'epochs':epoch+1,'baseline':{},'residual':{}}
    for name,X,Y in [('IID',Xi,Yi),('OOD',Xood,Yood),('HARD',Xhard,Yhard)]:
        report['baseline'][name]=baseline_eval(X,Y,name)
        report['residual'][name]=evaluate(model,X,Y,name)
    torch.save(model.state_dict(),out/'e09_lfo_residual.pt')
    np.savez_compressed(out/'datasets.npz',X_train=Xtr,Y_train=Ytr,X_val=Xv,Y_val=Yv,X_iid=Xi,Y_iid=Yi,X_ood=Xood,Y_ood=Yood,X_hard=Xhard,Y_hard=Yhard)
    (out/'report.json').write_text(json.dumps(report,indent=2))
    lines=['# E09 — LFO residual identification','',f"Trainable parameters: {report['trainable_parameters']}",f"Best validation loss: {best:.6g}",f"Epochs: {epoch+1}",'','| split | baseline fMAE | residual fMAE | baseline depth median | residual depth median | baseline phase MAE | residual phase MAE |','|---|---:|---:|---:|---:|---:|---:|']
    for name in ['IID','OOD','HARD']:
        b=report['baseline'][name]; r=report['residual'][name]
        lines.append(f"| {name} | {b['fm_mae_hz']:.4f} Hz | {r['fm_mae_hz']:.4f} Hz | {b['depth_relative_median']:.4%} | {r['depth_relative_median']:.4%} | {b['phase_mae_deg']:.3f}° | {r['phase_mae_deg']:.3f}° |")
    lines += ['', '## Training boundary', '', 'The analytic estimator remains frozen. The neural module learns only corrections to frequency, depth, and the phase vector.', '', '## Validation', '', 'IID, parameter-OOD and hard ridge-perturbation sets are kept disjoint from training.']
    (out/'REPORT.md').write_text('\n'.join(lines))
    print(json.dumps(report,indent=2))

if __name__=='__main__':
    main('/mnt/data/tree_synth_curriculum/results/e09_lfo')
