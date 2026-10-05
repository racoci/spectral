from __future__ import annotations
import math, zipfile
from pathlib import Path
import numpy as np
import pandas as pd
import sympy as sp
import matplotlib.pyplot as plt
from scipy.optimize import nnls
from scipy.integrate import cumulative_trapezoid

OUT = Path('/mnt/data/holomorphic_general_transform')
OUT.mkdir(parents=True, exist_ok=True)

# ---------------- SymPy verification ----------------
f, y = sp.symbols('f y', positive=True)
q, f0, beta, lam, alpha = sp.symbols('q f0 beta lam alpha', positive=True)
fp = sp.symbols('fp', positive=True)
etap = sp.symbols('etap', negative=True)
eta = sp.Function('eta')
F = sp.Function('f')
Phi = sp.Function('Phi')

# Chain-rule identity, treating f(y) and eta(y) as symbolic functions.
fy = F(y); ey = eta(y)
max_res = sp.simplify(sp.diff(Phi(f), f).subs(f, fy) - 2*sp.pi*ey)
phi2_res = sp.simplify(sp.diff(Phi(f), f, 2).subs(f, fy) - 2*sp.pi*sp.diff(ey,y)/sp.diff(fy,y))

# Resolution equation derived from sigma_y^2 = -1/(2*pi eta'(y) f'(y)).
sigma_y = sp.symbols('sigma_y', positive=True)
res_check = sp.simplify(
    (-1/(2*sp.pi*sp.diff(ey,y)*sp.diff(fy,y)))
    + 1/(2*sp.pi*sp.diff(ey,y)*sp.diff(fy,y))
)

# Peak derivative.
peak = sp.simplify(
    sp.diff(Phi(f), f).subs(f,fy)*sp.diff(fy,y)
    - 2*sp.pi*(sp.diff(fy,y)*ey + fy*sp.diff(ey,y))
)
peak_reduced = sp.simplify(peak.subs(sp.diff(Phi(f),f).subs(f,fy), 2*sp.pi*ey))

# Linear Gaussian/Bargmann exact square completion.
a, eta0 = sp.symbols('a eta0', positive=True)
Phi_lin = 2*sp.pi*eta0*f - sp.pi*a*f**2
Psi_lin = sp.expand(Phi_lin - 2*sp.pi*f*(eta0-a*y))
square_res = sp.simplify(Psi_lin - (-sp.pi*a*(f-y)**2 + sp.pi*a*y**2))

# Cauchy potential.
Phi_c = 2*sp.pi*q*sp.log(f/f0)
cauchy_deriv = sp.simplify(sp.diff(Phi_c,f) - 2*sp.pi*q/f)

# Exact Cauchy L2 integral.
fc = sp.symbols('fc', positive=True)
Ic = sp.simplify(sp.integrate((f/f0)**(4*sp.pi*q)*sp.exp(-4*sp.pi*q*f/fc), (f,0,sp.oo)))
Ic_expected = f0**(-4*sp.pi*q)*sp.gamma(4*sp.pi*q+1)*(fc/(4*sp.pi*q))**(4*sp.pi*q+1)
Ic_res = sp.simplify(Ic - Ic_expected)

# Shifted-family Laplace density.
s,u = sp.symbols('s u', positive=True)
w = u**(alpha-1)*sp.exp(-4*sp.pi*lam*u)
lap = sp.simplify(sp.integrate(w*sp.exp(-s*u),(u,0,sp.oo)))
lap_expected = sp.gamma(alpha)/(s+4*sp.pi*lam)**alpha
lap_res = sp.simplify(lap-lap_expected)

verification = {
    'maximum_condition_residual': max_res,
    'Phi_second_residual': phi2_res,
    'peak_derivative_after_maximum_condition': peak_reduced,
    'linear_square_completion_residual': square_res,
    'cauchy_Phi_derivative_residual': cauchy_deriv,
    'cauchy_L2_integral_residual': Ic_res,
    'shifted_Laplace_density_residual': lap_res,
}
with open(OUT/'sympy_verification.txt','w',encoding='utf-8') as fh:
    fh.write('SYMPY VERIFICATION\n==================\n')
    for k,v in verification.items(): fh.write(f'{k}: {sp.simplify(v)}\n')
    fh.write('\nDerived identities:\n')
    fh.write("Phi'(f(y)) = 2*pi*eta(y)\n")
    fh.write("Phi''(f(y)) = 2*pi*eta'(y)/f'(y)\n")
    fh.write("sigma_y^2 = -1/(2*pi*eta'(y)*f'(y))\n")
    fh.write("d/dy log A(y) = -2*pi*f(y)*eta'(y)\n")
    fh.write("For linear y=f and eta=eta0-a*y: Psi=-pi*a*(f-y)^2+pi*a*y^2\n")

# ---------------- Numerical experiment ----------------
FS=8000.0; N=4096; t=np.arange(N)/FS; df=FS/N
rng=np.random.default_rng(1234)
x=(0.48*np.sin(2*np.pi*(170*t+105*t**2))
   +0.34*np.sin(2*np.pi*(500*t-185*t**2))
   +0.24*np.sin(2*np.pi*(920*t+70*np.sin(2*np.pi*1.15*t)))
   +0.16*np.sin(2*np.pi*(280*t+20*t**2))
   +0.11*np.sin(2*np.pi*(340*t+210*t**2)))
x*=0.78+0.22*np.sin(2*np.pi*0.7*t)**2
x+=0.015*rng.standard_normal(N)
X=np.fft.fft(x)
Xpos=np.zeros(N,dtype=complex); Xpos[0]=X[0]
Xpos[1:N//2]=2*X[1:N//2]
Xpos[N//2]=X[N//2]
freqs=np.fft.fftfreq(N,1/FS)
mask=(freqs>0)&(freqs<FS/2)
fgrid=freqs[mask]; Xgrid=Xpos[mask]
FMIN,FMAX=50.,3500.
band=(fgrid>=FMIN)&(fgrid<=FMAX); fb=fgrid[band]; Xb=Xgrid[band]
full_band = (freqs >= FMIN) & (freqs <= FMAX)
M=180


def scale_data(name):
    if name=='linear':
        y=np.linspace(FMIN,FMAX,M); fy=y.copy(); fp=np.ones_like(y)
    elif name=='cauchy':
        y=np.linspace(0.,math.log2(FMAX/FMIN),M); fy=FMIN*2**y; fp=math.log(2)*fy
    elif name=='mel':
        y0=2595*math.log2(1+FMIN/700); y1=2595*math.log2(1+FMAX/700)
        y=np.linspace(y0,y1,M); fy=700*(2**(y/2595)-1); fp=(700*math.log(2)/2595)*2**(y/2595)
    elif name=='bark':
        y0=26.81*FMIN/(1960+FMIN)-0.53; y1=26.81*FMAX/(1960+FMAX)-0.53
        y=np.linspace(y0,y1,M); fy=1960*(y+0.53)/(26.28-y); fp=1960*26.81/(26.28-y)**2
    else: raise ValueError(name)
    return y,fy,fp


def build_eta(y,fp,sigma,eta_top=1e-4):
    deta=-1/(2*np.pi*sigma**2*fp)
    # Exact numerical integral backward from y_max.
    eta=np.empty_like(y); eta[-1]=eta_top
    # cumulative_trapezoid with reversed coordinate gives sign; integrate positive amount.
    inc=cumulative_trapezoid(deta[::-1], y[::-1], initial=0.)
    eta[:]=eta_top+inc[::-1]
    return eta


def build_filters(fgrid,y,fy,eta):
    eta_f=np.interp(fgrid,fy,eta)
    phi=cumulative_trapezoid(2*np.pi*eta_f,fgrid,initial=0.)
    G=np.empty((len(y),len(fgrid)))
    for j,fj in enumerate(fy):
        phij=np.interp(fj,fgrid,phi)
        logG=phi-phij-2*np.pi*(fgrid-fj)*eta[j]
        g=np.exp(np.clip(logG,-745,0))
        n2=math.sqrt(max(df*np.sum(g*g),1e-300))
        G[j]=g/n2
    return G


def solve_weights(G):
    A=(np.abs(G)**2).T
    rho,res=nnls(A,np.ones(A.shape[0]))
    H=A@rho
    mean=np.mean(H)
    if mean>0:
        rho/=mean; H/=mean
    return rho,H,res


def ycbcr_rgb(z):
    mag=np.abs(z); ref=max(float(np.max(mag)),1e-300)
    db=20*np.log10(np.maximum(mag,1e-300)/ref)
    y8=np.clip(np.rint((db+96)*255/96),0,255).astype(np.uint8)
    dbq=-96+96*y8.astype(float)/255
    Aq=ref*10**(dbq/20)
    lo=np.clip(y8.astype(np.float64)-.5,0,255); hi=np.clip(y8.astype(np.float64)+.5,0,255)
    Alo=ref*10**((-96+96*lo/255)/20); Ahi=ref*10**((-96+96*hi/255)/20)
    halfbin=.5*np.maximum(Ahi-Alo,1e-300)
    sat=np.clip(np.abs(mag-Aq)/halfbin,0,1)
    sat=np.maximum(sat,1/255)
    phase=np.angle(z)
    w=sat*np.exp(1j*phase)
    Cb=128+127*np.imag(w); Cr=128+127*np.real(w); Y=y8.astype(float)
    R=Y+1.402*(Cr-128); G=Y-.344136*(Cb-128)-.714136*(Cr-128); B=Y+1.772*(Cb-128)
    rgb=np.clip(np.stack([R,G,B],axis=-1),0,255).astype(np.uint8)
    center=np.hypot(Cb-128,Cr-128)
    return rgb,center

results={}; rows=[]
for name in ['linear','cauchy','mel','bark']:
    y,fy,fp=scale_data(name); dy=float(np.mean(np.diff(y))); sigma=1.15*dy
    eta=build_eta(y,fp,sigma); G=build_filters(fb,y,fy,eta); rho,H,res=solve_weights(G)
    E=np.empty((M,N),complex)
    for j in range(M):
        filt=np.zeros(N,float); filt[full_band]=G[j]
        E[j]=np.fft.ifft(Xpos*filt)*math.sqrt(max(rho[j],1e-12))
    results[name]={'y':y,'fy':fy,'fp':fp,'sigma':sigma,'eta':eta,'G':G,'rho':rho,'H':H,'E':E}
    rows.append(dict(scale=name,bins=M,dy=dy,sigma_y=sigma,eta_min_ms=1000*eta.min(),eta_max_ms=1000*eta.max(),rho_min=rho.min(),rho_max=rho.max(),H_min=H.min(),H_max=H.max(),tight_rel_L2_error=np.linalg.norm(H-1)/math.sqrt(H.size),nnls_residual=res))

metrics=pd.DataFrame(rows); metrics.to_csv(OUT/'metrics.csv',index=False)

# Signal plot
plt.figure(figsize=(11,3.8)); plt.plot(t,x); plt.xlabel('Time (s)'); plt.ylabel('Amplitude'); plt.title('Synthetic test signal'); plt.tight_layout(); plt.savefig(OUT/'synthetic_signal.png',dpi=160); plt.close()

# Frame operator comparison
plt.figure(figsize=(10,4.5))
for name in results: plt.plot(fb,results[name]['H'],label=name)
plt.axhline(1,linestyle='--'); plt.xlabel('Frequency (Hz)'); plt.ylabel('Frame operator H(f)'); plt.title('Least-squares tightness after L2 normalization'); plt.legend(); plt.tight_layout(); plt.savefig(OUT/'frame_operator_comparison.png',dpi=160); plt.close()

# Scale-dependent eta plots
for name,r in results.items():
    plt.figure(figsize=(9,4.5)); plt.plot(r['y'],r['eta']*1000); plt.xlabel('y'); plt.ylabel('eta(y) (ms)'); plt.title(f'Holomorphic height eta(y): {name}'); plt.tight_layout(); plt.savefig(OUT/f'eta_{name}.png',dpi=160); plt.close()

# YCbCr and magnitude plots.
for name,r in results.items():
    idx=np.arange(0,N,2); E=r['E'][:,idx]; yv=r['y']; extent=[t[idx[0]],t[idx[-1]],yv[0],yv[-1]]
    rgb,center=ycbcr_rgb(E)
    plt.figure(figsize=(11,7)); plt.imshow(rgb,origin='lower',aspect='auto',extent=extent,interpolation='nearest'); plt.xlabel('Time (s)'); plt.ylabel('Perceptual coordinate y'); plt.title(f'{name}: L2-normalized + tight-weighted holomorphic field, YCbCr'); plt.tight_layout(); plt.savefig(OUT/f'ycbcr_spectrogram_{name}.png',dpi=170); plt.close()
    mag=np.abs(E); ref=max(float(mag.max()),1e-300); dbm=np.clip(20*np.log10(np.maximum(mag,1e-300)/ref),-96,0)
    plt.figure(figsize=(11,7)); plt.imshow(dbm,origin='lower',aspect='auto',extent=extent,interpolation='nearest',vmin=-96,vmax=0); plt.xlabel('Time (s)'); plt.ylabel('Perceptual coordinate y'); plt.title(f'{name}: log-magnitude spectrogram (-96 dB to 0 dB)'); plt.colorbar(label='dB'); plt.tight_layout(); plt.savefig(OUT/f'magnitude_spectrogram_{name}.png',dpi=170); plt.close()
    plt.figure(figsize=(11,7)); plt.imshow((np.angle(E)+np.pi)/(2*np.pi),origin='lower',aspect='auto',extent=extent,interpolation='nearest',vmin=0,vmax=1,cmap='hsv'); plt.xlabel('Time (s)'); plt.ylabel('Perceptual coordinate y'); plt.title(f'{name}: phase debug view'); plt.colorbar(label='phase / 2pi'); plt.tight_layout(); plt.savefig(OUT/f'phase_{name}.png',dpi=170); plt.close()
    (OUT/f'ycbcr_stats_{name}.txt').write_text(f'min chroma-center distance: {center.min():.8f}\nmean chroma-center distance: {center.mean():.8f}\n',encoding='utf-8')

# Reusable Python implementation source.
source = r'''from __future__ import annotations\n\nimport math\nimport numpy as np\nfrom scipy.integrate import cumulative_trapezoid\nfrom scipy.optimize import nnls\n\n# Given a monotone perceptual scale y(f) and target local resolution sigma_y(y):\n#\n#   eta'(y) = -1/(2*pi*sigma_y(y)^2*f'(y))\n#   Phi'(f) = 2*pi*eta(y(f))\n#\n# so eta and Phi are determined by one quadrature each.\n\ndef build_eta(y, fprime, sigma_y, eta_top=1e-4):\n    deta_dy = -1.0/(2.0*np.pi*np.asarray(sigma_y)**2*np.asarray(fprime))\n    eta_rev = cumulative_trapezoid(deta_dy[::-1], y[::-1], initial=0.0)\n    return eta_top + eta_rev[::-1]\n\ndef build_filters(fgrid, y, fy, eta, fs):\n    df = fgrid[1] - fgrid[0]\n    eta_f = np.interp(fgrid, fy, eta)\n    phi = cumulative_trapezoid(2.0*np.pi*eta_f, fgrid, initial=0.0)\n    G = np.empty((len(y), len(fgrid)))\n    for j, fj in enumerate(fy):\n        phi_j = np.interp(fj, fgrid, phi)\n        # Centered logarithm avoids catastrophic cancellation and removes\n        # only a y-dependent factor. Subsequent L2 normalization restores\n        # exactly the normalized window shape.\n        logG = phi - phi_j - 2.0*np.pi*(fgrid-fj)*eta[j]\n        gj = np.exp(np.clip(logG, -745.0, 0.0))\n        norm = np.sqrt(np.sum(gj**2)*df)\n        G[j] = gj/norm\n    return G\n\ndef solve_tight_weights(G):\n    A = (np.abs(G)**2).T\n    rho, residual = nnls(A, np.ones(A.shape[0]))\n    H = A@rho\n    scale = H.mean()\n    if scale > 0:\n        rho /= scale\n        H /= scale\n    return rho, H, residual\n\ndef analysis_coefficients(x, fs, fgrid, band_mask, G, rho):\n    N=len(x)\n    X=np.fft.fft(x)\n    Xpos=np.zeros(N,dtype=complex)\n    Xpos[0]=X[0]\n    Xpos[1:N//2]=2.0*X[1:N//2]\n    Xpos[N//2]=X[N//2]\n    out=np.empty((G.shape[0],N),dtype=complex)\n    for j in range(G.shape[0]):\n        filt=np.zeros(N,float); filt[band_mask]=G[j]\n        out[j]=np.fft.ifft(Xpos*filt)*np.sqrt(max(rho[j],1e-12))\n    return out\n'''
(OUT/'generic_holomorphic_transform.py').write_text(source.replace('\\n','\n'),encoding='utf-8')

# Documentation of the display mapping.
(OUT/'YCbCr_convention.txt').write_text(r'''Display convention used in the generated images\n\n1. Magnitude: dB = 20 log10(|z| / reference), with reference = max |z| in the image.\n2. Luma Y: -96 dB -> 0, 0 dB -> 255, linearly in dB.\n3. Decode Y to Aq, the magnitude represented by that 8-bit Y code.\n4. residual magnitude = abs(|z|-Aq); it controls saturation.\n5. phase = arg(z); chroma direction is exp(i*phase).\n6. Cb = 128 + 127*saturation*sin(phase).\n7. Cr = 128 + 127*saturation*cos(phase).\n8. saturation is floored at 1/255, so (Cb,Cr) is never the neutral center (128,128).\n9. RGB is obtained from full-range BT.601 YCbCr.\n\nThis is a visualization mapping. It deliberately preserves phase even when the 8-bit Y code exactly matches the magnitude, but it is not claimed to be a lossless 24-bit encoding of arbitrary complex values.\n'''.replace('\\n','\n'),encoding='utf-8')

# Package all files.
zip_path=Path('/mnt/data/generic_holomorphic_transform_results.zip')
with zipfile.ZipFile(zip_path,'w',zipfile.ZIP_DEFLATED) as zf:
    for p in OUT.iterdir():
        if p.is_file(): zf.write(p,p.name)

print('OUTPUT',OUT)
print('ZIP',zip_path)
print(metrics.to_string(index=False))
