import math, json, random
import numpy as np

BPO=12
CHANNELS=120
FRAMES=60
CFG=dict(max_jump_oct=.30, lv=.50, la=3.0, lr=.25, sr=1/12, local=.10, step=1/48, passes=8)

def synth_truth(rng):
    tracks=[]
    ranges=[(2.5,3.5),(4.0,6.5),(7.5,9.5)]
    for lo,hi in ranges:
        u=rng.uniform(lo,hi,size=FRAMES)
        # lowpass random walk
        v=0.0
        out=np.empty(FRAMES)
        out[0]=u[0]
        a=0.0
        for m in range(1,FRAMES):
            a=.985*a+.015*rng.uniform(-1,1)*0.8
            v=(v+a*.005)*.997
            out[m]=out[m-1]+v*.005
        tracks.append(out)
    return np.array(tracks)

def make_energy(truth,rng):
    e=np.exp(rng.normal(-7.0,.35,size=(FRAMES,CHANNELS)))
    for tr in truth:
        for m,u in enumerate(tr):
            center=u*BPO
            bins=np.arange(CHANNELS)
            e[m]+=2.5*np.exp(-.5*((bins-center)/(BPO/30))**2)
    return e

def dp_init(e, K):
    E=e.copy(); paths=[]; jump=int(math.ceil(CFG['max_jump_oct']*BPO))
    for _ in range(K):
        dp=np.full((FRAMES,CHANNELS),-np.inf); prev=np.zeros((FRAMES,CHANNELS),np.int32)
        dp[0]=np.log1p(E[0])
        for m in range(1,FRAMES):
            for k in range(CHANNELS):
                lo=max(0,k-jump); hi=min(CHANNELS-1,k+jump)
                ps=np.arange(lo,hi+1)
                du=(k-ps)/BPO
                vals=dp[m-1,ps]+np.log1p(E[m,k])-CFG['lv']*du*du
                q=int(np.argmax(vals)); dp[m,k]=vals[q]; prev[m,k]=ps[q]
        k=int(np.argmax(dp[-1])); path=np.empty(FRAMES,np.int32); path[-1]=k
        for m in range(FRAMES-1,0,-1): path[m-1]=prev[m,path[m]]
        u=path/BPO; paths.append(u)
        bins=np.arange(CHANNELS)/BPO
        for m in range(FRAMES):
            d=(bins-u[m])/(1/18)
            E[m]*=np.maximum(0,1-np.exp(-.5*d*d))
    return paths

def interp(e,m,u):
    x=np.clip(u*BPO,0,CHANNELS-1)
    a=int(np.floor(x)); b=min(a+1,CHANNELS-1); q=x-a
    return e[m,a]*(1-q)+e[m,b]*q

def obj(paths,e):
    val=0.0
    for p in paths:
        for m in range(FRAMES):
            val-=math.log1p(float(interp(e,m,p[m])))
            if m: val += CFG['lv']*(p[m]-p[m-1])**2
            if m>1: val += CFG['la']*(p[m]-2*p[m-1]+p[m-2])**2
    for i in range(len(paths)):
        for j in range(i+1,len(paths)):
            d=paths[i]-paths[j]
            val += CFG['lr']*float(np.exp(-.5*(d/CFG['sr'])**2).sum())
    return val

def refine(paths,e):
    paths=[p.copy() for p in paths]
    step=CFG['step']
    for _ in range(CFG['passes']):
        for j in range(len(paths)):
            for m in range(FRAMES):
                cur=float(paths[j][m]); best=cur; bestv=obj(paths,e)
                radius=int(round(CFG['local']/step))
                for r in range(-radius,radius+1):
                    cand=np.clip(cur+r*step,0,(CHANNELS-1)/BPO)
                    paths[j][m]=cand
                    v=obj(paths,e)
                    if v<bestv:
                        bestv=v; best=cand
                paths[j][m]=best
        step*=.5
    return paths

def match_error(found,truth):
    K=len(found); used=set(); errs=[]
    for i,f in enumerate(found):
        vals=[]
        for j,t in enumerate(truth):
            if j in used: continue
            vals.append((np.sqrt(np.mean((f-t)**2)),j))
        d,j=min(vals)
        used.add(j); errs.append(d)
    return errs

rng=np.random.default_rng(20260915)
allerr=[]
for case in range(3):
    truth=synth_truth(rng); e=make_energy(truth,rng); init=dp_init(e,3); out=refine(init,e)
    er=match_error(out,truth); allerr += er
print(json.dumps({
    'cases':3,
    'tracks':9,
    'mean_log2_error':float(np.mean(allerr)),
    'median_log2_error':float(np.median(allerr)),
    'max_log2_error':float(np.max(allerr)),
    'mean_cents':float(1200*np.mean(allerr)),
    'max_cents':float(1200*np.max(allerr))
}, indent=2))
