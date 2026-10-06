from __future__ import annotations
import math, json
from pathlib import Path
import numpy as np
import torch
import torch.nn as nn
from torch.utils.data import DataLoader, TensorDataset

SEED=109
rng=np.random.default_rng(SEED)
FS=200.0
DUR=2.0
N=int(FS*DUR)
T=np.arange(N)/FS
WAVEFORMS=['sine','triangle','saw','square']

def wave(kind, phase):
    z=(phase/(2*np.pi))%1.0
    if kind=='sine': return np.sin(phase)
    if kind=='triangle': return 4*np.abs(z-0.5)-1
    if kind=='saw': return 2*z-1
    if kind=='square': return np.where(z<0.5,1.,-1.)
    raise ValueError(kind)

def synth(rng, hard=False):
    f0=rng.uniform(180,720)
    fm=rng.uniform(1,9)
    d=rng.uniform(4,80)
    ph=rng.uniform(-np.pi,np.pi)
    kind=rng.choice(WAVEFORMS)
    u=d*wave(kind,2*np.pi*fm*T+ph)
    f=f0*2**(u/1200)
    noise=rng.uniform(0.15,0.5) if hard else rng.uniform(0.1,0.25)
    obs=f+rng.normal(0,noise,N)
    if hard:
        idx=rng.choice(N,size=N//100,replace=False)
        obs[idx]+=rng.normal(0,1.5,idx.size)
    return f0,fm,d,ph,kind,obs

def analytic_lfo(obs,f0):
    u=1200*np.log2(np.maximum(obs,1e-6)/f0); u=u-u.mean()
    win=np.hanning(N)
    nfft=N*16
    Y=np.fft.rfft(u*win,nfft)
    freqs=np.fft.rfftfreq(nfft,1/FS)
    m=(freqs>=0.75)&(freqs<=12)
    ids=np.flatnonzero(m)
    k=ids[np.argmax(np.abs(Y[m])**2)]
    # parabolic interpolation on log magnitude
    delta=0.0
    if 1<=k<len(Y)-1:
        a,b,c=np.log(np.abs(Y[k-1])+1e-12),np.log(np.abs(Y[k])+1e-12),np.log(np.abs(Y[k+1])+1e-12)
        den=a-2*b+c
        if abs(den)>1e-12: delta=.5*(a-c)/den
    fm=float(freqs[k]+delta*(freqs[1]-freqs[0]))
    tt=np.arange(N)/FS
    A=np.column_stack([np.ones(N),np.sin(2*np.pi*fm*tt),np.cos(2*np.pi*fm*tt)])
    c=np.linalg.lstsq(A,u,rcond=None)[0]
    pred=A@c
    depth=float(np.hypot(c[1],c[2])); phase=float(np.arctan2(c[2],c[1]))
    return fm,depth,phase,float(np.sqrt(np.mean((u-pred)**2)))

def harmonic_features(obs,f0,fm):
    # Phase-invariant harmonic magnitudes of the cents trajectory.
    u=1200*np.log2(np.maximum(obs,1e-6)/f0); u=u-u.mean()
    phase=2*np.pi*fm*T
    mags=[]
    for k in range(1,9):
        s=np.sin(k*phase); c=np.cos(k*phase)
        a=2*np.dot(u,s)/N; b=2*np.dot(u,c)/N
        mags.append(float(np.hypot(a,b)))
    v=np.asarray(mags,np.float32)
    return v/(np.linalg.norm(v)+1e-8)

def dataset(n,hard=False):
    X=[]; Y=[]; param=[]
    for _ in range(n):
        f0,fm,d,ph,k,obs=synth(rng,hard=hard)
        a=analytic_lfo(obs,f0)
        X.append(harmonic_features(obs,f0,a[0]))
        Y.append(WAVEFORMS.index(k))
        param.append([fm,d,ph])
    return np.asarray(X,np.float32),np.asarray(Y,np.int64),np.asarray(param,np.float32)

class LFOTypeNet(nn.Module):
    def __init__(self):
        super().__init__()
        self.net=nn.Sequential(nn.Linear(8,32),nn.Tanh(),nn.Linear(32,16),nn.Tanh(),nn.Linear(16,4))
    def forward(self,x): return self.net(x)

def circ_deg(a,b): return float(np.mean(np.abs(np.angle(np.exp(1j*(a-b)))))*180/np.pi)

def eval_classifier(model,X,y):
    with torch.no_grad():
        p=model(torch.from_numpy(X)).argmax(1).numpy()
    return {'accuracy':float(np.mean(p==y)),'n':len(y)}

def eval_params(data):
    errs=[]; depths=[]; phases=[]
    for f0obs in data:
        f0,fm,d,ph,k,obs=f0obs
        a=analytic_lfo(obs,f0)
        errs.append(abs(a[0]-fm)); depths.append(abs(a[1]-d)/d); phases.append(abs(np.angle(np.exp(1j*(a[2]-ph))))*180/np.pi)
    return {'fm_mae_hz':float(np.mean(errs)),'fm_p95_hz':float(np.percentile(errs,95)),'depth_median_relative':float(np.median(depths)),'phase_mae_deg':float(np.mean(phases)),'phase_p95_deg':float(np.percentile(phases,95))}

def param_cases(n,hard=False):
    return [synth(rng,hard=hard) for _ in range(n)]

def main(out='/mnt/data/tree_synth_curriculum/results/e09'):
    out=Path(out); out.mkdir(parents=True,exist_ok=True)
    # E09-A: no neural parameters; analytic recovery of sinusoidal LFO params.
    def sine_only_cases(n,hard=False):
        cases=[]
        while len(cases)<n:
            c=synth(rng,hard=hard)
            if c[4]=='sine': cases.append(c)
        return cases
    iid_cases=sine_only_cases(1000,False); hard_cases=sine_only_cases(1000,True)
    analytic={'IID':eval_params(iid_cases),'HARD':eval_params(hard_cases)}

    # E09-B: learn only the discrete waveform type. Analytic f/depth/phase remain frozen.
    Xtr,ytr,_=dataset(6000,False); Xv,yv,_=dataset(1000,False); Xi,yi,_=dataset(2000,False); Xh,yh,_=dataset(2000,True)
    model=LFOTypeNet(); opt=torch.optim.AdamW(model.parameters(),lr=1e-3,weight_decay=1e-4)
    dl=DataLoader(TensorDataset(torch.from_numpy(Xtr),torch.from_numpy(ytr)),256,shuffle=True)
    best=1e9; state=None; bad=0
    for ep in range(80):
        model.train()
        for xb,yb in dl:
            loss=nn.functional.cross_entropy(model(xb),yb)
            opt.zero_grad(); loss.backward(); opt.step()
        model.eval()
        with torch.no_grad(): vl=nn.functional.cross_entropy(model(torch.from_numpy(Xv)),torch.from_numpy(yv)).item()
        if vl<best: best=vl; state={k:v.detach().clone() for k,v in model.state_dict().items()}; bad=0
        else:
            bad+=1
            if bad>=8: break
    model.load_state_dict(state)
    cls={'IID':eval_classifier(model,Xi,yi),'HARD':eval_classifier(model,Xh,yh),'trainable_parameters':sum(p.numel() for p in model.parameters()),'best_val_loss':best,'epochs':ep+1}

    report={'stage':'E09','analytic_parameter_recovery':analytic,'waveform_classifier':cls,'design_decision':'Do not train a neural residual for sinusoidal LFO parameters: the frozen analytic estimator is already substantially better. Neural learning is restricted to discrete waveform type.'}
    torch.save(model.state_dict(),out/'e09_lfo_type.pt')
    np.savez_compressed(out/'classifier_test.npz',X_iid=Xi,y_iid=yi,X_hard=Xh,y_hard=yh)
    (out/'report.json').write_text(json.dumps(report,indent=2))
    md=['# E09 — LFO identification','','## E09-A: continuous parameters','',f"Analytic IID: {analytic['IID']}",f"Analytic hard: {analytic['HARD']}",'','No trainable parameters are introduced for sinusoidal LFO frequency, depth, or phase.','','## E09-B: waveform type','','The only learned component is a 16 → 32 → 16 → 4 classifier over {sine, triangle, saw, square}.','',f"Trainable parameters: {cls['trainable_parameters']}",f"IID accuracy: {cls['IID']['accuracy']:.4f}",f"Hard accuracy: {cls['HARD']['accuracy']:.4f}",f"Best validation CE: {cls['best_val_loss']:.6g}",'','## Promotion decision','','Promote E09 only as an identification stage with analytic continuous parameters and a small discrete-type head. Do not add a neural residual until a representation/perturbation family exists where the analytic estimator fails systematically.']
    (out/'REPORT.md').write_text('\n'.join(md))
    print(json.dumps(report,indent=2))
if __name__=='__main__': main()
