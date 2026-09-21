import numpy as np, matplotlib.pyplot as plt, json, time
from scipy.ndimage import gaussian_filter1d
from scipy.signal import stft
from pathlib import Path
out=Path('/mnt/data/direct_log_cqt_jet'); out.mkdir(exist_ok=True)
Fs=40960.; N=8192; dur=N/Fs; B=96; K=960; fmin=20.; fmax=20480.; sigma=.95/B
rng=np.random.default_rng(20260920); t=np.arange(N)/Fs; ut=t/dur
x0=np.zeros(N,complex); chirps=[]
for i in range(50):
 z=gaussian_filter1d(rng.standard_normal(N),sigma=100); z/=z.std()
 lf=np.clip(rng.uniform(np.log2(25),np.log2(17000))+rng.uniform(-2,2)*(ut-.5)+rng.uniform(.12,.75)*z/3,np.log2(20),np.log2(20000))
 fi=2**lf; ph=2*np.pi*np.cumsum(fi)/Fs+rng.uniform(0,2*np.pi)
 env=.5+.5*np.sin(2*np.pi*rng.uniform(.2,1)*ut+rng.uniform(0,2*np.pi))**2
 x0 += rng.uniform(.01,.05)*env*np.exp(1j*ph); chirps.append(fi)
chirps=np.array(chirps); rms=np.sqrt(np.mean(abs(x0)**2))
wn=(rng.standard_normal(N)+1j*rng.standard_normal(N))/np.sqrt(2); wn*=rms*10**(-30/20)
n=(rng.standard_normal(N)+1j*rng.standard_normal(N))/np.sqrt(2); nn=np.fft.fft(n); ff=np.fft.fftfreq(N,1/Fs); nn/=np.sqrt(np.maximum(abs(ff),1)); n=np.fft.ifft(nn); n*=rms*10**(-35/20)/np.sqrt(np.mean(abs(n)**2)); x0+=wn+n
scale=.8/max(abs(x0.real).max(),abs(x0.imag).max()); qr=np.round(x0.real*scale*32767).astype(np.int16); qi=np.round(x0.imag*scale*32767).astype(np.int16); x=qr.astype(float)/32767+1j*qi.astype(float)/32767
freq=np.fft.fftfreq(N,1/Fs); af=abs(freq); valid=af>=fmin; y=np.zeros(N); y[valid]=np.log2(af[valid]/fmin); yc=np.arange(K)/B; centers=fmin*2**yc; X=np.fft.fft(x)
# Frame operator first, then synthesis. Coefficients are generated blockwise; no dense 960*N tensor is kept.
S=np.zeros(N)
for sign in (1,-1):
 for a in range(0,K,32):
  uu=y[None,:]-yc[a:a+32,None]; G=np.exp(-.5*(uu/sigma)**2); G[:,~valid]=0
  G[:,freq*sign<=0]=0
  S += (G*G).sum(0)
S[0]+=1; S[N//2]+=1; Ss=np.maximum(S,1e-30)
# Generate coefficient jet for positive side, decimated only for visualization.
D=4; Wv=np.empty((K,N//D),np.complex64); Wyv=np.empty_like(Wv)
Xhat=np.zeros(N,complex); tic=time.time()
for sign in (1,-1):
 for a in range(0,K,32):
  yc_b=yc[a:a+32]; uu=y[None,:]-yc_b[:,None]; G=np.exp(-.5*(uu/sigma)**2); G[:,~valid]=0; G[:,freq*sign<=0]=0
  C=np.fft.ifft(X[None,:]*G,axis=1)
  if sign==1:
   Wv[a:a+32]=C[:,::D].astype(np.complex64)
   Gy=-(uu/sigma**2)*G; Cy=np.fft.ifft(X[None,:]*Gy,axis=1); Wyv[a:a+32]=Cy[:,::D].astype(np.complex64)
  Cspec=np.fft.fft(C,axis=1); Xhat += (Cspec*(G/Ss[None,:])).sum(0)
Xhat[0]+=X[0]; Xhat[N//2]+=X[N//2]; xr=np.fft.ifft(Xhat); err=xr-x
mae=np.mean(abs(err)); rmse=np.sqrt(np.mean(abs(err)**2)); rel=np.linalg.norm(err)/np.linalg.norm(x); snr=20*np.log10(np.sqrt(np.mean(abs(x)**2))/rmse); exact=np.mean((np.round(np.clip(xr.real,-1,1)*32767).astype(np.int16)==qr)&(np.round(np.clip(xr.imag,-1,1)*32767).astype(np.int16)==qi))
# phase/log amplitude + derivative fields
A=abs(Wv); phase=np.angle(Wv); logA=np.log(np.maximum(A,1e-30)); ratio=Wyv/np.where(A>1e-20,Wv,1+0j); dlog=np.real(ratio); dphi=np.imag(ratio)
lo,hi=np.percentile(logA,[2,99.5]); V=np.clip((logA-lo)/(hi-lo),0,1); H=(phase+np.pi)/(2*np.pi)
# HSV conversion
h6=(H%1)*6; ii=np.floor(h6).astype(int); f=h6-ii; q=V*(1-f); r=V*f; rgb=np.empty(H.shape+(3,),np.float32)
for k in range(6):
 m=ii==k
 vals=[(V,r,0),(q,V,0),(0,V,r),(0,q,V),(r,0,V),(V,0,q)][k]
 rgb[m]=np.stack([v[m] if np.ndim(v) else np.full(m.sum(),v) for v in vals],-1)
# plots
plt.figure(figsize=(11,6)); [plt.plot(t,c,lw=.7,alpha=.55) for c in chirps]; plt.yscale('log'); plt.ylim(20,20000); plt.xlabel('Time (s)'); plt.ylabel('Frequency (Hz)'); plt.title('50 synthetic chirps — expected instantaneous frequency'); plt.tight_layout(); plt.savefig(out/'01_expected_chirps.png',dpi=180); plt.close()
plt.figure(figsize=(12,7)); plt.imshow(rgb,origin='lower',aspect='auto',extent=[0,dur,0,10],interpolation='nearest'); plt.xlabel('Time (s)'); plt.ylabel('y = log2(f/20)'); plt.title('Complex log-frequency CQT: hue = phase, brightness = log amplitude'); plt.tight_layout(); plt.savefig(out/'02_phase_logamplitude_rgb.png',dpi=180); plt.close()
plt.figure(figsize=(12,7)); plt.imshow(logA,origin='lower',aspect='auto',extent=[0,dur,0,10],interpolation='nearest'); plt.xlabel('Time (s)'); plt.ylabel('y = log2(f/20)'); plt.title('log |W(t,y)| — exactly 96 bins/octave'); plt.colorbar(label='log amplitude'); plt.tight_layout(); plt.savefig(out/'03_log_amplitude.png',dpi=180); plt.close()
plt.figure(figsize=(12,7)); plt.imshow(phase,origin='lower',aspect='auto',extent=[0,dur,0,10],interpolation='nearest'); plt.xlabel('Time (s)'); plt.ylabel('y = log2(f/20)'); plt.title('arg W(t,y)'); plt.colorbar(label='phase (rad)'); plt.tight_layout(); plt.savefig(out/'04_phase.png',dpi=180); plt.close()
f,ts,Z=stft(xr,fs=Fs,window='hann',nperseg=512,noverlap=384,return_onesided=False,boundary=None); mask=(abs(f)>=20)&(abs(f)<=20000); fm=abs(f[mask]); o=np.argsort(fm); M=20*np.log10(np.maximum(abs(Z[mask][o]),1e-15)); M-=M.max(); plt.figure(figsize=(12,7)); plt.pcolormesh(ts,fm[o],M,shading='auto',vmin=-80,vmax=0); plt.yscale('log'); plt.ylim(20,20000); [plt.plot(t,c,lw=.45,alpha=.22) for c in chirps]; plt.xlabel('Time (s)'); plt.ylabel('Frequency (Hz)'); plt.title('Reconstructed signal: STFT + expected chirps'); plt.colorbar(label='relative magnitude (dB)'); plt.tight_layout(); plt.savefig(out/'05_reconstructed_stft.png',dpi=180); plt.close()
plt.figure(figsize=(11,5)); plt.plot(t,abs(err),lw=.7); plt.yscale('log'); plt.xlabel('Time (s)'); plt.ylabel('|error|'); plt.title('Complex reconstruction error'); plt.tight_layout(); plt.savefig(out/'06_reconstruction_error.png',dpi=180); plt.close()
metrics={'Fs':Fs,'N':N,'duration_s':dur,'bins_per_octave':B,'octaves':10,'positive_channels':K,'negative_channels':K,'spacing_octaves':1/B,'MAE_complex':float(mae),'RMSE_complex':float(rmse),'relative_L2':float(rel),'SNR_dB':float(snr),'bit_exact_sample_fraction':float(exact),'max_abs_error':float(abs(err).max()),'mean_component_error_LSB':float(.5*(np.mean(abs(err.real))+np.mean(abs(err.imag)))*32767),'jet_fields':['W','W_y'],'runtime_s':time.time()-tic}
json.dump(metrics,open(out/'metrics.json','w'),indent=2)
print(json.dumps(metrics,indent=2))
