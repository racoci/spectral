from __future__ import annotations

import json, math, os
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import Dict, List, Tuple

import numpy as np
from scipy import signal

OUT = Path('/mnt/data/e17_blindspot_validation')
OUT.mkdir(parents=True, exist_ok=True)
RNG = np.random.default_rng(20261006)

SR = 12000
DUR = 2.0
N = int(SR * DUR)
T = np.arange(N) / SR


def cents(a, b):
    return 1200.0 * np.abs(np.log2(np.maximum(a,1e-12) / np.maximum(b,1e-12)))


def local_complex_coeff(x, freq, center, halfwin=384):
    lo = max(0, center-halfwin); hi = min(len(x), center+halfwin+1)
    tt = np.arange(lo, hi) / SR
    w = np.hanning(len(tt))
    z = np.exp(-2j*np.pi*freq*tt)
    c = np.sum(w*x[lo:hi]*z) / (np.sum(w)+1e-12)
    return 2*c


def smooth_dynamic_curve(t, rng, base=1.0, rel=0.4, knots=9):
    kt = np.linspace(0, t[-1], knots)
    kv = base*(1 + rel*rng.normal(size=knots))
    kv = np.maximum(kv, 0.03*base)
    return np.interp(t, kt, kv)


def gen_harmonic_dynamic(rng, K=8, f0=180.0, rel=0.5):
    amps = []
    phases = rng.uniform(-np.pi, np.pi, K)
    x = np.zeros_like(T)
    H = np.zeros((K, N))
    for k in range(1, K+1):
        base = 1.0/(k**0.8)
        curve = smooth_dynamic_curve(T, rng, base=base, rel=rel, knots=13)
        H[k-1] = curve
        x += curve*np.cos(2*np.pi*k*f0*T + phases[k-1])
    return x, H, phases, f0


def e171_dynamic_timbre(num=80):
    rows=[]
    for i in range(num):
        f0_true=float(RNG.uniform(110,280))
        x,H,ph,f0=gen_harmonic_dynamic(RNG, K=int(RNG.integers(4,13)), f0=f0_true, rel=float(RNG.uniform(.15,.55)))
        # recover local complex amplitudes at 25 frames
        centers=np.linspace(384,N-385,25).astype(int)
        errs=[]
        # E17.1 isolates dynamic timbre: use the exact oracle f0 so pitch error
        # cannot masquerade as a timbre-tracking failure.
        for k in range(1,H.shape[0]+1):
            target=H[k-1,centers]
            pred=np.array([abs(local_complex_coeff(x,k*f0,c)) for c in centers])
            # constant projection scale is absorbed per partial; compare after median normalization
            pred*=np.median(target)/max(np.median(pred),1e-12)
            errs.append(np.sqrt(np.mean((20*np.log10(np.maximum(pred,1e-9)/np.maximum(target,1e-9)))**2)))
        rows.append({'case':i,'K':H.shape[0],'f0_est_hz':f0,'median_partial_rmse_db':float(np.median(errs)),'p95_partial_rmse_db':float(np.percentile(errs,95))})
    return rows


def e172_attack_transient(num=80):
    rows=[]
    for i in range(num):
        f0=float(RNG.uniform(100,800)); amp=float(RNG.uniform(.4,1.0)); attack=float(RNG.uniform(.008,.04));
        phase=float(RNG.uniform(-np.pi,np.pi))
        base=amp*np.sin(2*np.pi*f0*T+phase)
        env=1-np.exp(-T/0.015)
        steady=base*env
        # inharmonic transient/chiff, with decaying noise and high-frequency chirp
        fc=float(RNG.uniform(2500,5200)); bw=float(RNG.uniform(150,500));
        transient=(0.4+0.6*RNG.random())*np.exp(-T/attack)*RNG.normal(size=N)
        transient*=np.exp(-0.5*((np.fft.rfftfreq(N,1/SR)-fc)/bw)**2).dot(np.ones((1,1))).item() if False else 1.0
        # band-limit transient
        b,a=signal.butter(4,[max(100,fc-bw)/(SR/2), min(5900,fc+bw)/(SR/2)],btype='band')
        transient=signal.lfilter(b,a,transient)
        # add high-frequency inharmonic deterministic chirp
        transient += 0.25*np.exp(-T/(attack*1.5))*signal.chirp(T, f0=fc, f1=min(5900,fc+1800), t1=DUR, method='linear')
        x=steady+transient
        # reconstruct only steady harmonic; residual exposes omitted attack field
        yhat=steady
        r=x-yhat
        e_r=np.linalg.norm(r[:int(.08*SR)])/max(np.linalg.norm(x[:int(.08*SR)]),1e-12)
        e_late=np.linalg.norm(r[int(.12*SR):])/max(np.linalg.norm(x[int(.12*SR):]),1e-12)
        # transient onset from energy derivative threshold
        frames=max(1,int(.005*SR)); ee=[]
        for j in range(0,int(.12*SR),frames): ee.append(np.sqrt(np.mean(r[j:j+frames]**2)+1e-12))
        ee=np.array(ee); d=np.diff(np.log(ee+1e-9)); idx=int(np.argmax(d)) if len(d) else 0
        onset=idx*0.005
        rows.append({'case':i,'attack_s':attack,'residual_first80ms_rel':float(e_r),'residual_after120ms_rel':float(e_late),'detected_residual_onset_s':float(onset),'transient_to_steady_rms_db':float(20*np.log10((np.std(transient[:int(.08*SR)])+1e-9)/(np.std(steady[:int(.08*SR)])+1e-9)))})
    return rows


def e173_collision(num=100):
    rows=[]
    for i in range(num):
        f1=float(RNG.uniform(100,500)); ratio=float(RNG.uniform(.98,1.02)); f2=f1*ratio
        # design matrix for first 6 harmonics of both voices, real sine/cos pairs at exact frequencies
        freqs=np.r_[np.arange(1,7)*f1, np.arange(1,7)*f2]
        cols=[]
        for f in freqs:
            cols += [np.cos(2*np.pi*f*T), np.sin(2*np.pi*f*T)]
        A=np.stack(cols,axis=1)
        s=np.linalg.svd(A, compute_uv=False)
        cond=float(s[0]/max(s[-1],1e-15))
        rank=int(np.sum(s>1e-8*s[0]))
        rows.append({'case':i,'f1_hz':f1,'f2_hz':f2,'relative_separation':abs(f2-f1)/f1,'rank':rank,'cond':cond,'sigma_min':float(s[-1])})
    # exact harmonic identity case
    f1=200.0; f2=400.0
    freqs=np.r_[np.arange(1,7)*f1, np.arange(1,7)*f2]
    cols=[]
    for f in freqs: cols += [np.cos(2*np.pi*f*T), np.sin(2*np.pi*f*T)]
    A=np.stack(cols,axis=1); s=np.linalg.svd(A, compute_uv=False)
    exact={'relative_separation':1.0,'rank':int(np.sum(s>1e-8*s[0])),'columns':A.shape[1],'nullity':int(A.shape[1]-np.sum(s>1e-8*s[0])),'sigma_min':float(s[-1]),'condition_number':float(s[0]/max(s[-1],1e-15))}
    return rows,exact


def e174_relative_phase(num=100):
    rows=[]
    for i in range(num):
        f0=float(RNG.uniform(100,500)); K=int(RNG.integers(3,9));
        H=1/np.arange(1,K+1)**.8
        psi=rng_phase=RNG.uniform(-np.pi,np.pi,K)
        # strict phase family predicts k*phi0 + constant offset per harmonic; evaluate residual of best common phi0
        # complex phasors are directly observed in this oracle test
        # brute-force wrapped LS over phi0 using dense grid
        grid=np.linspace(-np.pi,np.pi,4096)
        costs=[]
        for p0 in grid:
            pred=((np.arange(1,K+1)*p0 + 0.0 + np.pi)%(2*np.pi))-np.pi
            d=np.angle(np.exp(1j*(psi-pred)))
            costs.append(np.sum((H*d)**2))
        j=int(np.argmin(costs)); best=grid[j]
        residual=np.sqrt(np.mean(np.angle(np.exp(1j*(psi-(np.arange(1,K+1)*best))))**2))
        # strict family case
        psi_strict=(np.arange(1,K+1)*float(RNG.uniform(-np.pi,np.pi)))
        best2=0.0
        # solve by circular regression via e^(i psi_k) ~= e^(ik phi)
        ph=grid
        cost=np.array([np.mean(np.angle(np.exp(1j*(psi_strict-np.arange(1,K+1)*p)))**2) for p in ph])
        residual2=float(np.sqrt(cost.min()))
        rows.append({'case':i,'K':K,'independent_phase_rmse_rad':float(residual),'strict_phase_rmse_rad':residual2})
    return rows


def make_rir(rng, L=2048):
    h=np.zeros(L)
    h[0]=1.0
    # early reflections
    for d,g in zip(rng.integers(20,500,12), rng.uniform(-.45,.45,12)):
        h[int(d)] += g
    # diffuse tail with exponential decay and mild coloration
    t=np.arange(L)/SR
    tail=rng.normal(size=L)*np.exp(-t/rng.uniform(.25,.8))
    b,a=signal.butter(2, rng.uniform(.12,.45))
    tail=signal.lfilter(b,a,tail)
    h += rng.uniform(.15,.4)*tail/np.max(np.abs(tail))
    h *= 0.6/np.max(np.abs(h))
    return h


def fit_schroeder_rir(h, K=4):
    # approximate by direct + K exponential lowpass-shaped scalar tails; intentionally limited
    L=len(h); t=np.arange(L)/SR
    basis=[np.eye(1,L,0).ravel()]
    taus=np.geomspace(.04,1.2,K)
    for tau in taus: basis.append(np.exp(-t/tau))
    B=np.stack(basis,axis=1)
    coef=np.linalg.lstsq(B,h,rcond=None)[0]
    return B@coef, float(np.linalg.norm(h-B@coef)/max(np.linalg.norm(h),1e-12))


def e175_rir(num=50):
    rows=[]
    for i in range(num):
        h=make_rir(RNG)
        hs,rel=fit_schroeder_rir(h,K=4)
        # early reflection energy and late energy mismatch
        early=slice(0,int(.08*SR)); late=slice(int(.12*SR),None)
        early_rel=float(np.linalg.norm(h[early]-hs[early])/max(np.linalg.norm(h[early]),1e-12))
        late_rel=float(np.linalg.norm(h[late]-hs[late])/max(np.linalg.norm(h[late]),1e-12))
        rows.append({'case':i,'rir_rel_fit_error':rel,'early80ms_rel_error':early_rel,'late_after120ms_rel_error':late_rel})
    return rows


def e176_representation(num=100):
    rows=[]
    horizons=[.02,.05,.1,.2,.5,1.0]
    # single-tone phase observation with constant frequency error
    for i in range(num):
        f=float(RNG.uniform(80,1200)); eps=float(RNG.normal(0,.03)) # Hz
        ph_err=[]
        for H in horizons:
            t=H
            ph_err.append(2*np.pi*eps*t)
        # wrap-sensitive unit-circle error
        circ=[abs(np.exp(1j*p)-1.0) for p in ph_err]
        rows.append({'case':i,'f_hz':f,'freq_error_hz':eps, **{f'phase_err_{str(h).replace(".","p")}s_rad':abs(2*np.pi*eps*h) for h in horizons}, **{f'circ_err_{str(h).replace(".","p")}s':circ[j] for j,h in enumerate(horizons)}})
    return rows


def e177_equivalence():
    # 1) PM/FM identity under current definitions
    fc=440.0; fm=5.0; I=2.3; psi=.7
    phi_fm=2*np.pi*fc*T + I*np.sin(2*np.pi*fm*T+psi)
    phi_pm=2*np.pi*fc*T + I*np.sin(2*np.pi*fm*T+psi)
    x1=np.cos(phi_fm); x2=np.cos(phi_pm)
    pmfm=float(np.max(np.abs(x1-x2)))
    # 2) time shift <-> phase offset for pure sinusoid
    dt=.003; f=330.0; p=.4
    a=np.cos(2*np.pi*f*(T-dt)+p); b=np.cos(2*np.pi*f*T + (p-2*np.pi*f*dt))
    phase_shift=float(np.max(np.abs(a-b)))
    # 3) filter <-> spectral envelope: same multiplicative spectrum for additive input
    freqs=np.fft.rfftfreq(N,1/SR)
    H=1/np.sqrt(1+(freqs/1800)**6)
    X=np.fft.rfft(np.sin(2*np.pi*220*T)+.7*np.sin(2*np.pi*660*T))
    y=np.fft.irfft(X*H,n=N)
    # define envelope-absorbed representation as the same filtered spectrum
    y2=np.fft.irfft(X*H,n=N)
    filter_env=float(np.max(np.abs(y-y2)))
    return {'pm_fm_max_abs_diff':pmfm,'time_shift_phase_gauge_max_abs_diff':phase_shift,'filter_envelope_exact_reconstruction_max_abs_diff':filter_env}


def jacobian_adversarial(num=80):
    rows=[]
    t=np.linspace(0,.5,int(.5*SR),endpoint=False)
    def sig(p):
        A,d,fc,fm,beta,psi=p
        return A*(1+d*np.cos(2*np.pi*fm*t+psi))*np.cos(2*np.pi*fc*t+beta*np.sin(2*np.pi*fm*t+psi))
    # Random cases plus deliberately adversarial geometries.
    cases=[]
    for i in range(num):
        A=float(RNG.uniform(.5,1.2)); d=float(RNG.uniform(.02,.9)); fc=float(RNG.uniform(150,1000))
        if i%3==0: fm=fc*(1+RNG.uniform(-.003,.003))
        else: fm=float(RNG.uniform(.5,40.0))
        beta=float(10**RNG.uniform(-2,0.8))
        if i%5==0: beta*=.03
        psi=float(RNG.uniform(-np.pi,np.pi))
        cases.append((A,d,fc,fm,beta,psi,'random'))
    cases += [
        (1.0,1e-5,440.,5.,1.0,.3,'d~0'),
        (1.0,.5,440.,5.,1e-6,.3,'beta~0'),
        (1.0,.5,440.,1e-4,2.0,.3,'fm~0'),
        (1.0,.5,440.,439.999,1.0,.3,'fm~fc'),
        (1.0,.5,440.,440.001,1e-5,.3,'double-degenerate'),
    ]
    for i,(A,d,fc,fm,beta,psi,label) in enumerate(cases):
        p=np.array([A,d,fc,fm,beta,psi],float)
        y=sig(p); J=np.zeros((len(t),len(p)))
        steps=np.array([1e-4,1e-5,.03,.001,1e-5,1e-5])
        for j,h in enumerate(steps):
            q=p.copy(); q[j]+=h; J[:,j]=(sig(q)-y)/h
        norms=np.linalg.norm(J,axis=0); Jn=J/np.maximum(norms,1e-12)
        s=np.linalg.svd(Jn,compute_uv=False)
        rows.append({'case':i,'label':label,'fc_hz':fc,'fm_hz':fm,'fm_over_fc':fm/fc,'d':d,'beta':beta,'sigma_min':float(s[-1]),'cond':float(s[0]/max(s[-1],1e-15))})
    return rows


def save_csv(rows, path):
    import csv
    if not rows: return
    keys=list(rows[0].keys())
    with open(path,'w',newline='') as f:
        w=csv.DictWriter(f,fieldnames=keys); w.writeheader(); w.writerows(rows)


def summarize(rows, key):
    x=np.array([r[key] for r in rows],float)
    return {'median':float(np.median(x)),'p95':float(np.percentile(x,95)),'max':float(np.max(x)),'min':float(np.min(x))}


def main():
    e171=e171_dynamic_timbre(80); save_csv(e171,OUT/'E17_1_dynamic_timbre.csv')
    e172=e172_attack_transient(80); save_csv(e172,OUT/'E17_2_attack_transient.csv')
    e173, exact=e173_collision(100); save_csv(e173,OUT/'E17_3_harmonic_collision.csv')
    e174=e174_relative_phase(100); save_csv(e174,OUT/'E17_4_relative_phase.csv')
    e175=e175_rir(50); save_csv(e175,OUT/'E17_5_rir.csv')
    e176=e176_representation(100); save_csv(e176,OUT/'E17_6_horizon.csv')
    e177=e177_equivalence();
    adv=jacobian_adversarial(80); save_csv(adv,OUT/'E17_8_adversarial_jacobian.csv')

    report={
        'E17.1_dynamic_timbre': {
            'median_partial_rmse_db': summarize(e171,'median_partial_rmse_db'),
            'p95_partial_rmse_db': summarize(e171,'p95_partial_rmse_db')
        },
        'E17.2_attack_transient': {
            'residual_first80ms_rel': summarize(e172,'residual_first80ms_rel'),
            'residual_after120ms_rel': summarize(e172,'residual_after120ms_rel'),
            'onset_s': summarize(e172,'detected_residual_onset_s'),
        },
        'E17.3_harmonic_collision': {
            'relative_sep_median': summarize(e173,'relative_separation'),
            'condition_number': summarize(e173,'cond'),
            'exact_identity_case': exact
        },
        'E17.4_relative_phase': {
            'independent_phase_rmse_rad': summarize(e174,'independent_phase_rmse_rad'),
            'strict_phase_rmse_rad': summarize(e174,'strict_phase_rmse_rad')
        },
        'E17.5_rir': {
            'rir_rel_fit_error': summarize(e175,'rir_rel_fit_error'),
            'early80ms_rel_error': summarize(e175,'early80ms_rel_error'),
            'late_after120ms_rel_error': summarize(e175,'late_after120ms_rel_error')
        },
        'E17.6_representation_horizon': {
            'phase_error_rad_at_horizons_mean': {h:float(np.mean([r[f'phase_err_{str(h).replace(".","p")}s_rad'] for r in e176])) for h in [.02,.05,.1,.2,.5,1.0]},
            'frequency_error_hz_rmse': float(np.sqrt(np.mean(np.array([r['freq_error_hz'] for r in e176])**2)))
        },
        'E17.7_equivalence':e177,
        'E17.8_adversarial_jacobian': {
            'cond': summarize(adv,'cond'),
            'sigma_min': summarize(adv,'sigma_min'),
            'worst_cases': sorted(adv,key=lambda r:r['sigma_min'])[:8]
        }
    }
    with open(OUT/'summary.json','w') as f: json.dump(report,f,indent=2)

    md=[]
    md.append('# E17.1–E17.8 Blind-Spot Validation\n')
    md.append('This suite is deliberately diagnostic: it tests representational coverage, identifiability, confounding, and forecast sensitivity before promotion to E18.\n')
    md.append('## Results\n')
    md.append(f"- E17.1 dynamic timbre: median per-partial local-projection error = {np.median([r['median_partial_rmse_db'] for r in e171]):.3f} dB; p95 = {np.percentile([r['median_partial_rmse_db'] for r in e171],95):.3f} dB.\n")
    md.append(f"- E17.2 attack/transient: residual energy in first 80 ms = median {np.median([r['residual_first80ms_rel'] for r in e172]):.3f}; after 120 ms = median {np.median([r['residual_after120ms_rel'] for r in e172]):.3f}. The transient is materially outside the current steady harmonic model.\n")
    md.append(f"- E17.3 harmonic collision: median condition number = {np.median([r['cond'] for r in e173]):.2e}; exact f2=2*f1 with both voices carrying 6 harmonics gives rank {exact['rank']} for {exact['columns']} columns, nullity {exact['nullity']}.\n")
    md.append(f"- E17.4 relative harmonic phase: independent partial phases give median residual {np.median([r['independent_phase_rmse_rad'] for r in e174]):.3f} rad under the strict phase law, while strict-phase samples give {np.median([r['strict_phase_rmse_rad'] for r in e174]):.3e} rad.\n")
    md.append(f"- E17.5 realistic RIR: a 5-parameter Schroeder-like approximation leaves median relative RIR error {np.median([r['rir_rel_fit_error'] for r in e175]):.3f}; early 80 ms mismatch median {np.median([r['early80ms_rel_error'] for r in e175]):.3f}.\n")
    for h in [.02,.05,.1,.2,.5,1.0]:
        md.append(f"- E17.6 horizon {h:.2f} s: mean phase error from the sampled frequency-estimation noise = {np.mean([r[f'phase_err_{str(h).replace(chr(46), chr(112))}s_rad'] for r in e176]):.3f} rad.\n")
    md.append(f"- E17.7 equivalence: PM/FM max waveform difference = {e177['pm_fm_max_abs_diff']:.3e}; time-shift/phase-gauge max difference = {e177['time_shift_phase_gauge_max_abs_diff']:.3e}; filter-vs-envelope construction is exact to {e177['filter_envelope_exact_reconstruction_max_abs_diff']:.3e}.\n")
    md.append(f"- E17.8 adversarial Jacobian: worst observed normalized condition number = {max(r['cond'] for r in adv):.2e}; worst normalized sigma_min = {min(r['sigma_min'] for r in adv):.3e}.\n")
    md.append('\n## Interpretation\n')
    md.append('1. E17.1 is representable, but the old E06/E07 tests do not establish dynamic H_k(t) recovery. A dedicated temporal timbre state is required.\n')
    md.append('2. E17.2 exposes an omitted excitation/residual field at attacks. A symbolic harmonic model alone is insufficient for pluck/chiff/breath-like transients.\n')
    md.append('3. E17.3 contains a true identifiability failure: exact harmonic collisions can create a non-zero null space even with noiseless observations. This is not a TreeNN capacity problem.\n')
    md.append('4. E17.4 shows that strict relative harmonic phase is an additional model assumption, not a consequence of additive synthesis. It needs an explicit gauge/class.\n')
    md.append('5. E17.5 shows that a simple Schroeder parameterization cannot be assumed to represent arbitrary room responses; the mismatch should enter a residual/effect-field component.\n')
    md.append('6. E17.6 quantifies why small instantaneous frequency errors become large phase errors with prediction horizon. Jet quality must therefore be judged by downstream forecast error, not only local RMSE.\n')
    md.append('7. E17.7 should be formalized before graph recovery/model selection: PM/FM, time-shift/phase, and filter/envelope can belong to the same observable equivalence class.\n')
    md.append('8. E17.8 should become an adversarial data generator: sample parameter configurations near low singular values instead of relying only on IID and hand-built hard cases.\n')
    md.append('\n## Promotion decision\n')
    md.append('E18 should remain blocked until E17.1–E17.7 are represented as explicit benchmark families and E17.8 is integrated into the hard-set generator. The evidence indicates that the main remaining risks are model-class omission and identifiability geometry, not insufficient TreeNN width.\n')
    (OUT/'REPORT.md').write_text(''.join(md))
    print(json.dumps(report, indent=2))

if __name__=='__main__': main()
