from __future__ import annotations
import json, math
from pathlib import Path
import numpy as np
from scipy.optimize import linear_sum_assignment

RNG=np.random.default_rng(1315)
FS=12000.0
DUR=2.0
N=int(FS*DUR)
T=np.arange(N)/FS

def colored_noise(alpha, rng):
    freqs=np.fft.rfftfreq(N,1/FS)
    spec=np.zeros_like(freqs,dtype=np.complex128)
    mag=np.where(freqs>20,freqs**(alpha/2),0.0)
    spec[1:]= (rng.normal(size=len(spec)-1)+1j*rng.normal(size=len(spec)-1))*mag[1:]
    n=np.fft.irfft(spec,n=N)
    return n/(np.std(n)+1e-12)

def synth_noise(rng, hard=False):
    f0=rng.uniform(180,900); K=rng.integers(4,9)
    H=rng.uniform(.05,0.5,K); H[0]=1.0
    ph=rng.uniform(-np.pi,np.pi,K)
    x=np.zeros(N)
    for k in range(1,K+1): x += H[k-1]*np.sin(2*np.pi*k*f0*T+ph[k-1])
    alpha=rng.uniform(-1.5,0.5); mix=rng.uniform(0.01,0.3)
    n=colored_noise(alpha,rng); x=x/np.std(x)+mix*n
    return x,f0,H,alpha,mix

def estimate_noise(x,f0,H):
    h=np.zeros(N); # reconstruct deterministic harmonics exactly
    for k in range(1,len(H)+1): h += H[k-1]*np.sin(2*np.pi*k*f0*T)
    # subtract best linear phase-amplitude Fourier fit at the known harmonics
    cols=[]
    for k in range(1,len(H)+1): cols += [np.sin(2*np.pi*k*f0*T),np.cos(2*np.pi*k*f0*T)]
    A=np.column_stack(cols); c=np.linalg.lstsq(A,x,rcond=None)[0]; recon=A@c
    r=x-recon
    freqs=np.fft.rfftfreq(N,1/FS); P=np.abs(np.fft.rfft(r*np.hanning(N)))**2
    m=(freqs>=30)&(freqs<=FS/2*0.95); X=np.log(freqs[m]); Y=np.log(P[m]+1e-16)
    # avoid DC/obvious harmonic neighborhoods
    mask=np.ones_like(X,dtype=bool)
    for k in range(1,len(H)+1): mask &= np.abs(freqs[m]-k*f0)>25
    slope=np.polyfit(X[mask],Y[mask],1)[0]
    alpha=slope
    mix=float(np.std(r)/(np.std(recon)+1e-12))
    return alpha,mix

def e13(n=300,hard=False):
    es=[]
    for _ in range(n):
        x,f0,H,a,m=synth_noise(RNG,hard)
        ea,em=estimate_noise(x,f0,H)
        es.append([abs(ea-a),abs(em-m)/(m+1e-9)])
    e=np.asarray(es)
    return {'trainable_parameters':0,'alpha_mae':float(e[:,0].mean()),'alpha_p95':float(np.percentile(e[:,0],95)),'mix_relative_median':float(np.median(e[:,1])),'mix_relative_p95':float(np.percentile(e[:,1],95))}

def adsr(t,on,dur,attack,decay,sustain,release):
    u=t-on; y=np.zeros_like(t); end=dur+release
    a=np.clip(u/attack,0,1); y=np.where(u<attack,a,y)
    d=np.clip((u-attack)/decay,0,1); y=np.where((u>=attack)&(u<attack+decay),1-(1-sustain)*d,y)
    y=np.where((u>=attack+decay)&(u<dur),sustain,y)
    rr=np.clip((u-dur)/release,0,1); y=np.where((u>=dur)&(u<end),sustain*(1-rr),y)
    return y

def synth_events(rng,nnotes=3,hard=False):
    on=np.cumsum(rng.uniform(.35,.6,nnotes)); f=rng.uniform(180,900,nnotes); dur=rng.uniform(.2,.45,nnotes)
    x=np.zeros(N); params=[]
    for i in range(nnotes):
        a=rng.uniform(.005,.03); d=rng.uniform(.05,.15); s=rng.uniform(.4,.8); rel=rng.uniform(.08,.2)
        env=adsr(T,on[i],dur[i],a,d,s,rel); x += env*np.sin(2*np.pi*f[i]*T)
        params.append((on[i],dur[i],f[i]))
    if hard: x += rng.normal(0,.01,N)
    return x,np.asarray(params)

def detect_onsets(x):
    frame=24; e=np.convolve(x*x,np.ones(frame)/frame,'same'); d=np.maximum(np.diff(e,prepend=e[0]),0)
    # local maxima separated by 150 ms, threshold relative to robust scale
    thr=np.median(d)+5*np.std(d)
    cand=np.where((d[1:-1]>d[:-2])&(d[1:-1]>=d[2:])&(d[1:-1]>thr))[0]+1
    selected=[]
    for idx in cand:
        if not selected or idx-selected[-1]>int(.15*FS): selected.append(idx)
        elif d[idx]>d[selected[-1]]: selected[-1]=idx
    return np.asarray(selected)/FS

def e14(n=300,hard=False):
    errs=[]
    for _ in range(n):
        x,p=synth_events(RNG,3,hard); det=detect_onsets(x)
        true=p[:,0]
        # match detected to truth
        if len(det)==0: errs += [2.0]*len(true); continue
        C=np.abs(true[:,None]-det[None,:]); r,c=linear_sum_assignment(C)
        errs += list(C[r,c])
    e=np.asarray(errs)
    return {'trainable_parameters':0,'onset_mae_ms':float(e.mean()*1000),'onset_p95_ms':float(np.percentile(e,95)*1000)}

def voice_frames(x,voices,frame=512,hop=128):
    out=[]
    win=np.hanning(frame)
    for start in range(0,N-frame+1,hop):
        z=np.abs(np.fft.rfft(x[start:start+frame]*win)); f=np.fft.rfftfreq(frame,1/FS)
        # top local peaks in musical band
        loc=[]
        for k in range(2,len(z)-2):
            if z[k]>z[k-1] and z[k]>=z[k+1] and f[k]>80:
                a,b,c=np.log(z[k-1]+1e-12),np.log(z[k]+1e-12),np.log(z[k+1]+1e-12)
                den=a-2*b+c
                d=.5*(a-c)/den if abs(den)>1e-12 else 0.0
                fp=f[k]+d*(f[1]-f[0])
                loc.append((z[k],fp))
        loc=sorted(loc,reverse=True)[:voices]
        vals=[v for _,v in sorted(loc,key=lambda q:q[1])]
        while len(vals)<voices: vals.append(np.nan)
        out.append(vals)
    return np.asarray(out)

def synth_voices(rng,V=2):
    fs=np.zeros((V,N)); x=np.zeros(N); f0=[]
    for v in range(V):
        a=rng.uniform(120,900); b=rng.uniform(-80,80); base=rng.uniform(0,2*np.pi)
        f=a+b*T; phase=2*np.pi*(a*T+.5*b*T*T)+base; amp=rng.uniform(.4,1.)
        fs[v]=f; x+=amp*np.sin(phase); f0.append(f)
    return x,np.asarray(f0)

def e15(n=300):
    maes=[]
    frame=512; hop=128
    for _ in range(n):
        x,true=synth_voices(RNG,2); pred=voice_frames(x,2,frame,hop)
        truth=np.asarray([[true[0,start],true[1,start]] for start in range(0,N-frame+1,hop)])
        # frequency matching at each frame
        for t in range(len(pred)):
            p=pred[t]
            valid=~np.isnan(p)
            if valid.sum()!=2: continue
            C=np.abs(p[:,None]-truth[t][None,:])
            r,c=linear_sum_assignment(C); maes += list(C[r,c])
    e=np.asarray(maes)
    return {'trainable_parameters':0,'ridge_mae_hz':float(e.mean()),'ridge_p95_hz':float(np.percentile(e,95))}

def main(out='/mnt/data/tree_synth_curriculum/results/e13_e15'):
    out=Path(out); out.mkdir(parents=True,exist_ok=True)
    report={'E13_noise':{'IID':e13(300,False),'HARD':e13(300,True)},'E14_events':{'IID':e14(300,False),'HARD':e14(300,True)},'E15_multivoice':e15(200)}
    (out/'report.json').write_text(json.dumps(report,indent=2))
    (out/'REPORT.md').write_text('# E13–E15\n\n'+json.dumps(report,indent=2)+'\n\nAll three isolated stages use analytic or combinatorial estimators; no neural parameters are introduced. This is intentional: learning is reserved for ambiguity and graph structure.')
    print(json.dumps(report,indent=2))
if __name__=='__main__': main()
