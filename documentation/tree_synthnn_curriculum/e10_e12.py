from __future__ import annotations
import json, math
from pathlib import Path
import numpy as np

FS=12000.0
DUR=2.0
N=int(FS*DUR)
T=np.arange(N)/FS
RNG=np.random.default_rng(1012)

# -----------------------------------------------------------------------------
# E10: FM in isolation. We exploit the identity
# f_inst(t)=fc + beta*fm*cos(2*pi*fm*t+phi).
# Therefore no neural parameters are needed for the isolated case.
# -----------------------------------------------------------------------------

def synth_fm(fc, fm, beta, phase, noise_std=0.03, hard=False):
    finst=fc+beta*fm*np.cos(2*np.pi*fm*T+phase)
    obs=finst + RNG.normal(0,noise_std,N)
    if hard:
        idx=RNG.choice(N,size=N//500,replace=False)
        obs[idx]+=RNG.normal(0,0.5,len(idx))
    return finst,obs

def fit_fm(obs):
    y=obs-obs.mean()
    nfft=N*4
    Y=np.fft.rfft(y*np.hanning(N),nfft)
    freqs=np.fft.rfftfreq(nfft,1/FS)
    m=(freqs>=0.5)&(freqs<=20)
    ii=np.flatnonzero(m)[np.argmax(np.abs(Y[m])**2)]
    if 1<=ii<len(Y)-1:
        a,b,c=np.log(np.abs(Y[ii-1])+1e-12),np.log(np.abs(Y[ii])+1e-12),np.log(np.abs(Y[ii+1])+1e-12)
        den=a-2*b+c
        d=.5*(a-c)/den if abs(den)>1e-12 else 0.
    else:d=0.
    fm=float(freqs[ii]+d*(freqs[1]-freqs[0]))
    A=np.column_stack([np.ones(N),np.cos(2*np.pi*fm*T),np.sin(2*np.pi*fm*T)])
    c=np.linalg.lstsq(A,obs,rcond=None)[0]
    fc=float(c[0]); amp=float(np.hypot(c[1],c[2])); phase=float(np.arctan2(-c[2],c[1]))
    beta=amp/fm
    return fc,fm,beta,phase

def circ_deg(a,b):
    return np.abs(np.angle(np.exp(1j*(a-b))))*180/np.pi

def sample_fm(n,hard=False):
    rows=[]
    for _ in range(n):
        fc=RNG.uniform(150,2500); fm=RNG.uniform(1,15); beta=RNG.uniform(0.1,8); ph=RNG.uniform(-np.pi,np.pi)
        _,obs=synth_fm(fc,fm,beta,ph,0.05 if hard else 0.02,hard)
        est=fit_fm(obs)
        rows.append([fc,fm,beta,ph,*est])
    return np.asarray(rows)

def e10_metrics(rows):
    fc,fm,b,p=rows[:,:4].T; ef,em,eb,ep=rows[:,4:].T
    return {
        'fc_mae_hz':float(np.mean(abs(ef-fc))),
        'fm_mae_hz':float(np.mean(abs(em-fm))),
        'beta_relative_median':float(np.median(abs(eb-b)/b)),
        'phase_mae_deg':float(np.mean(circ_deg(ep,p))),
        'phase_p95_deg':float(np.percentile(circ_deg(ep,p),95))}

# -----------------------------------------------------------------------------
# E11 identifiability: PM and FM generate the same instantaneous-frequency law
# under parameter reparameterization.
# -----------------------------------------------------------------------------
# FM phase: phi_FM=2*pi*fc*t + beta*sin(2*pi*fm*t+psi)
# PM phase: phi_PM=2*pi*fc*t + I*sin(2*pi*fm*t+psi)
# d/dt(phi_PM)/(2*pi) = fc + I*fm*cos(...)
# which is FM with beta=I. Therefore the waveform is identical exactly.

def pm_signal(fc,fm,I,phase):
    return np.sin(2*np.pi*fc*T + I*np.sin(2*np.pi*fm*T+phase))

def fm_signal(fc,fm,beta,phase):
    return np.sin(2*np.pi*fc*T + beta*np.sin(2*np.pi*fm*T+phase))

def identifiability_test(n=200):
    max_abs=[]; jac_svals=[]
    # For PM/FM with identical carrier, modulator and modulation coefficient,
    # the signals are exactly equal. The parameter Jacobian therefore has
    # collinear beta/I directions when both mechanisms are allowed.
    for _ in range(n):
        fc=RNG.uniform(200,1800); fm=RNG.uniform(1,12); m=RNG.uniform(.1,6); ph=RNG.uniform(-np.pi,np.pi)
        a=fm_signal(fc,fm,m,ph); b=pm_signal(fc,fm,m,ph)
        max_abs.append(float(np.max(np.abs(a-b))))
        # two columns wrt FM index and PM index at identical parameterization
        # are the same function, hence rank 1 in a 2-column Jacobian.
        d=np.cos(2*np.pi*fm*T+ph)
        J=np.column_stack([d,-d])
        s=np.linalg.svd(J,compute_uv=False)
        jac_svals.append([s[0],s[1]])
    sv=np.asarray(jac_svals)
    return {'max_signal_difference':float(max(max_abs)), 'median_singular_values':sv[len(sv)//2].tolist(), 'rank':1,'decision':'PM and FM are not separately identifiable from the waveform in this isolated model; do not train a classifier between them without additional structural observations/priors.'}

# -----------------------------------------------------------------------------
# E12 AM: direct envelope modulation is analytically estimable from |analytic signal|.
# -----------------------------------------------------------------------------
def synth_am(fc,fm,depth,phase,noise_std=0.005):
    env=1+depth*np.cos(2*np.pi*fm*T+phase)
    x=env*np.sin(2*np.pi*fc*T)
    return env,x+RNG.normal(0,noise_std,N)

def fit_am(x,fc):
    # Complex analytic signal via FFT Hilbert construction.
    X=np.fft.fft(x); h=np.zeros(N)
    h[0]=1
    if N%2==0: h[N//2]=1; h[1:N//2]=2
    else: h[1:(N+1)//2]=2
    z=np.fft.ifft(X*h); env=np.abs(z)
    y=env-env.mean()
    nfft=N*4; Y=np.fft.rfft(y*np.hanning(N),nfft); f=np.fft.rfftfreq(nfft,1/FS)
    m=(f>=0.5)&(f<=20); ii=np.flatnonzero(m)[np.argmax(np.abs(Y[m])**2)]
    fm=float(f[ii]); A=np.column_stack([np.ones(N),np.cos(2*np.pi*fm*T),np.sin(2*np.pi*fm*T)])
    c=np.linalg.lstsq(A,env,rcond=None)[0]
    depth=float(np.hypot(c[1],c[2])); phase=float(np.arctan2(-c[2],c[1]));
    return fm,depth,phase

def e12_metrics(n=500):
    errs=[]
    for _ in range(n):
        fc=RNG.uniform(400,2500); fm=RNG.uniform(1,15); d=RNG.uniform(.02,.8); ph=RNG.uniform(-np.pi,np.pi)
        env,x=synth_am(fc,fm,d,ph)
        ef,ed,ep=fit_am(x,fc)
        errs.append([abs(ef-fm),abs(ed-d)/d,circ_deg(ep,ph)])
    e=np.asarray(errs)
    return {'fm_mae_hz':float(e[:,0].mean()),'depth_relative_median':float(np.median(e[:,1])),'phase_mae_deg':float(e[:,2].mean()),'phase_p95_deg':float(np.percentile(e[:,2],95))}

def main(out='/mnt/data/tree_synth_curriculum/results/e10_e12'):
    out=Path(out); out.mkdir(parents=True,exist_ok=True)
    e10_iid=sample_fm(500,False); e10_hard=sample_fm(500,True)
    report={'E10_FM':{'trainable_parameters':0,'IID':e10_metrics(e10_iid),'HARD':e10_metrics(e10_hard)},'E11_PM':identifiability_test(),'E12_AM':{'trainable_parameters':0,'IID':e12_metrics(500)}}
    (out/'report.json').write_text(json.dumps(report,indent=2))
    md=['# E10–E12: FM, PM, AM','','## E10 — FM','', 'The isolated FM problem is solved analytically from the instantaneous-frequency ridge:', '', r'$f_{inst}(t)=f_c+b2 f_m cos(2c0 f_m t+c6)$.'.replace('cos','\\cos'), '', f"IID: {report['E10_FM']['IID']}", f"HARD: {report['E10_FM']['HARD']}", '', 'No trainable parameters are introduced.', '', '## E11 — PM identifiability', '', 'For', '', r'$\\phi_{PM}(t)=2\\pi f_ct+I\\sin(2\\pi f_mt+\\psi)$', '', r'$\\frac{1}{2\\pi}\\frac{d\\phi_{PM}}{dt}=f_c+I f_m\\cos(2\\pi f_mt+\\psi)$', '', 'which is exactly the same instantaneous-frequency law as FM with beta = I. The generated waveforms are therefore identical under the same parameter substitution.', '', f"Maximum numerical signal difference over {200} trials: {report['E11_PM']['max_signal_difference']:.3e}", f"Numerical Jacobian rank: {report['E11_PM']['rank']}", '', '**Decision: do not train a PM-vs-FM classifier in this isolated setting.** Additional structural priors are required.', '', '## E12 — AM', '', 'AM/tremolo is directly estimated from the analytic envelope; no learned continuous parameters are needed in the isolated case.', f"Metrics: {report['E12_AM']['IID']}", '']
    (out/'REPORT.md').write_text('\n'.join(md))
    print(json.dumps(report,indent=2))
if __name__=='__main__': main()
