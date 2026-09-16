import os
os.environ["MPLCONFIGDIR"] = "/tmp/mpl"
import numpy as np
import matplotlib.pyplot as plt
from scipy.io import wavfile
from scipy.signal import stft
from scipy.optimize import minimize
from pathlib import Path

# ============================================================================
# V7: Reversible FIR Lifting Approximation of a 60-bin/octave CQT Lattice
# ============================================================================

wav_path = Path("/home/racoci/Projects/audio2image/public/voice.wav")
out_dir = Path("/home/racoci/Projects/audio2image/tests/test-outputs")
out_dir.mkdir(parents=True, exist_ok=True)
out_path = out_dir / "cqt_stft_vs_optimized_integer_lifting.png"

fs, x = wavfile.read(wav_path)
if x.ndim > 1:
    x = np.mean(x, axis=1)

# Exact integer PCM-like domain.
if np.issubdtype(x.dtype, np.floating):
    peak = float(np.max(np.abs(x)))
    pcm_scale = (2**23 - 1) / peak if peak else 1.0
    x_int = np.rint(x * pcm_scale).astype(np.int64)
else:
    x_int = x.astype(np.int64)

x_float = x_int.astype(np.float64)
x_float /= max(np.max(np.abs(x_float)), 1.0)

# ------------------------------------------------------------
# 60-bin/octave log-frequency grid
# ------------------------------------------------------------
BPO = 60
FMIN = 20.0
FMAX = min(11025.0, fs/2.0) # Nyquist target
freqs = FMIN * 2.0**( np.arange(int(np.floor(BPO*np.log2(FMAX/FMIN))) + 1) / BPO )
freqs = freqs[freqs <= FMAX]
N = len(freqs)

# ------------------------------------------------------------
# Reference CQT from STFT
# ------------------------------------------------------------
nperseg = 4096
noverlap = 3072
f_lin, times, Z = stft(
    x_float,
    fs=fs,
    window="hann",
    nperseg=nperseg,
    noverlap=noverlap,
    boundary="zeros",
    padded=True
)

# Gaussian CQT in log-frequency
du = 1.0 / BPO
sigma = (1.65*du)/2.355
D = np.log2(np.maximum(f_lin, 1e-12))[:, None] - np.log2(freqs)[None, :]
G = np.exp(-0.5*(D/sigma)**2)
G /= np.maximum(G.sum(axis=0, keepdims=True), 1e-30)

valid = (f_lin >= FMIN) & (f_lin <= FMAX)
C_ref = G[valid, :].T @ Z[valid, :]
ref_db = 20*np.log10(np.maximum(np.abs(C_ref), 1e-12))
ref_db -= ref_db.max()

# ------------------------------------------------------------
# Higher-Order FIR Lifting Factorization
# ------------------------------------------------------------
# We model the Predictor P and Updater U as symmetric FIR filters of order K=2
# (Total of 6 degrees of freedom: 3 for P, 3 for U).
# This allows high-fidelity approximation of a smooth Gaussian window response:
#
# P(e)[n] = p0 * e[n] + p1 * (e[n-1] + e[n+1]) + p2 * (e[n-2] + e[n+2])
# U(d)[n] = u0 * d[n] + u1 * (d[n-1] + d[n+1]) + u2 * (d[n-2] + d[n+2])

target_sigma_bins = 1.65 / 2.355

def lift_matrix_fir(n, p, u_coefs):
    """Return full reversible analysis matrix for one FIR lifting stage."""
    # Build operation by applying to basis vectors
    def stage(v):
        e = v[0::2].copy()
        o = v[1::2].copy()
        
        pred = np.zeros_like(o)
        for k in range(len(o)):
            # Symmetric edge extensions
            em2 = e[max(k-2, 0)] if k-2 >= 0 else e[0]
            em1 = e[max(k-1, 0)] if k-1 >= 0 else e[0]
            ec  = e[min(k, len(e)-1)]
            ep1 = e[min(k+1, len(e)-1)]
            ep2 = e[min(k+2, len(e)-1)]
            
            pred[k] = p[0]*ec + p[1]*(em1 + ep1) + p[2]*(em2 + ep2)
            
        d = o - pred
        
        upd = np.zeros_like(e)
        for k in range(len(e)):
            dm2 = d[max(k-2, 0)] if k-2 >= 0 else d[0] if len(d) else 0
            dm1 = d[max(k-1, 0)] if k-1 >= 0 else d[0] if len(d) else 0
            dc  = d[min(k, len(d)-1)] if len(d) else 0
            dp1 = d[min(k+1, len(d)-1)] if len(d) else 0
            dp2 = d[min(k+2, len(d)-1)] if len(d) else 0
            
            if len(d):
                upd[k] = u_coefs[0]*dc + u_coefs[1]*(dm1 + dp1) + u_coefs[2]*(dm2 + dp2)
                
        s = e + upd
        out = np.empty_like(v)
        out[0::2] = s
        out[1::2] = d
        return out
        
    cols = [stage(np.eye(n)[:,k]) for k in range(n)]
    return np.column_stack(cols)

# We optimize the 6 parameters: 3 for P, 3 for U
n_test = 61
center = n_test//2
delta = np.zeros(n_test)
delta[center] = 1.0

def objective_fir(theta):
    p0, p1, p2, u0, u1, u2 = theta
    p = [p0, p1, p2]
    u_coefs = [u0, u1, u2]
    
    A = lift_matrix_fir(n_test, p, u_coefs)
    # Cascade the FIR stage 3 times to get a smooth Gaussian profile
    cur = A @ delta
    for _ in range(2):
        cur = A @ cur
        
    y = np.abs(cur)
    y /= max(y.max(), 1e-30)
    
    u_idx = np.arange(n_test) - center
    gauss_target = np.exp(-0.5*(u_idx/target_sigma_bins)**2)
    
    yd = 20*np.log10(np.maximum(y, 1e-8))
    gd = 20*np.log10(np.maximum(gauss_target, 1e-8))
    
    w = np.exp(-0.5*(u_idx/12.0)**2)
    return float(np.sum(w * (yd - gd)**2))

# Optimize the FIR parameters starting from a smooth CDF 5/3 baseline
res = minimize(
    objective_fir,
    x0=np.array([0.25, 0.125, 0.05, 0.25, 0.125, 0.05]),
    method="Nelder-Mead",
    options={"maxiter": 200, "xatol": 1e-5, "fatol": 1e-5}
)

p_opt = res.x[0:3]
u_opt = res.x[3:6]

# Convert optimized float parameters to Q20 fixed-point integers
FBITS = 20
p_fix = [int(np.rint(val * (1 << FBITS))) for val in p_opt]
u_fix = [int(np.rint(val * (1 << FBITS))) for val in u_opt]

def round_fixed(x, bits):
    den = 1 << bits
    out = np.empty_like(x, dtype=np.int64)
    pos = x >= 0
    out[pos] = (x[pos] + den//2)//den
    out[~pos] = -((-x[~pos] + den//2)//den)
    return out

# Reversible Integer FIR Predictor
def pred_fir_int(e):
    if len(e) == 0:
        return e.copy()
    p = np.empty(len(e), dtype=np.int64)
    for k in range(len(e)):
        em2 = e[max(k-2, 0)]
        em1 = e[max(k-1, 0)]
        ec  = e[k]
        ep1 = e[min(k+1, len(e)-1)]
        ep2 = e[min(k+2, len(e)-1)]
        
        acc = (
            p_fix[0] * int(ec) +
            p_fix[1] * (int(em1) + int(ep1)) +
            p_fix[2] * (int(em2) + int(ep2))
        )
        p[k] = round_fixed(np.array([acc], dtype=np.int64), FBITS)[0]
    return p

# Reversible Integer FIR Updater
def upd_fir_int(d):
    if len(d) == 0:
        return np.zeros(0, dtype=np.int64)
    u = np.empty(len(d), dtype=np.int64)
    for k in range(len(d)):
        dm2 = d[max(k-2, 0)]
        dm1 = d[max(k-1, 0)]
        dc  = d[k]
        dp1 = d[min(k+1, len(d)-1)]
        dp2 = d[min(k+2, len(d)-1)]
        
        acc = (
            u_fix[0] * int(dc) +
            u_fix[1] * (int(dm1) + int(dp1)) +
            u_fix[2] * (int(dm2) + int(dp2))
        )
        u[k] = round_fixed(np.array([acc], dtype=np.int64), FBITS)[0]
    return u

def lift_forward_fir(v):
    e = v[0::2].copy()
    o = v[1::2].copy()
    p = pred_fir_int(e[:len(o)])
    d = o - p
    u = upd_fir_int(d)
    s = e.copy()
    s[:len(u)] += u
    return s, d

def lift_inverse_fir(s, d, original_len):
    u = upd_fir_int(d)
    e = s.copy()
    e[:len(u)] -= u
    p = pred_fir_int(e[:len(d)])
    o = d + p
    out = np.empty(original_len, dtype=np.int64)
    out[0::2] = e
    out[1::2] = o
    return out

def cascade_forward_fir(v, stages=3):
    cur = v.copy()
    details = []
    lengths = []
    for _ in range(stages):
        if len(cur) < 6:
            break
        s, d = lift_forward_fir(cur)
        details.append(d)
        lengths.append(len(cur))
        cur = s
    return cur, details, lengths

def cascade_inverse_fir(low, details, lengths):
    cur = low
    for k in range(len(details)-1, -1, -1):
        cur = lift_inverse_fir(cur, details[k], lengths[k])
    return cur

# Quantize reference CQT coefficients to exact 24-bit integer grid
COEF_BITS = 24
Cr = np.rint(C_ref.real*(1<<COEF_BITS)).astype(np.int64)
Ci = np.rint(C_ref.imag*(1<<COEF_BITS)).astype(np.int64)

Lift = np.zeros_like(C_ref.real)
max_coef_err = 0
coef_mismatch = 0

# Apply FIR Lifting Cascade to each frame (W)
stages = 3
for j in range(Cr.shape[1]):
    lo_r, ds_r, sz_r = cascade_forward_fir(Cr[:,j], stages=stages)
    lo_i, ds_i, sz_i = cascade_forward_fir(Ci[:,j], stages=stages)
    
    rr = cascade_inverse_fir(lo_r, ds_r, sz_r)
    ii = cascade_inverse_fir(lo_i, ds_i, sz_i)
    
    max_coef_err = max(
        max_coef_err,
        int(np.max(np.abs(rr-Cr[:,j]))),
        int(np.max(np.abs(ii-Ci[:,j])))
    )
    coef_mismatch += int(np.count_nonzero(rr != Cr[:,j]))
    coef_mismatch += int(np.count_nonzero(ii != Ci[:,j]))
    
    # Accumulate detail energies back at their log-frequency scale
    for level, (dr, di) in enumerate(zip(ds_r, ds_i), start=1):
        scale = 2**level
        energy = dr.astype(np.float64)**2 + di.astype(np.float64)**2
        src_pos = (np.arange(len(energy)) + 0.5)*scale - 0.5
        valid_m = (src_pos >= 0) & (src_pos <= N-1)
        if np.any(valid_m):
            e_interp = np.interp(
                np.arange(N),
                src_pos[valid_m],
                energy[valid_m],
                left=0.0,
                right=0.0
            )
            Lift[:,j] += e_interp
            
    if len(lo_r):
        low_energy = lo_r.astype(np.float64)**2 + lo_i.astype(np.float64)**2
        src_pos = (np.arange(len(lo_r)) + 0.5)*(2**len(ds_r)) - 0.5
        low_interp = np.interp(
            np.arange(N),
            src_pos,
            low_energy,
            left=0.0,
            right=0.0
        )
        Lift[:,j] += low_interp

assert max_coef_err == 0 and coef_mismatch == 0
lift_db = 10*np.log10(np.maximum(Lift, 1e-30))
lift_db -= lift_db.max()

# ------------------------------------------------------------
# 1D Filter Impulse Response Check
# ------------------------------------------------------------
imp = np.zeros(121, dtype=np.int64)
imp[60] = 1<<FBITS
s_imp, d_imp = lift_forward_fir(imp)
for _ in range(2):
    s_imp, _ = lift_forward_fir(s_imp)
    
smooth = s_imp.astype(np.float64)
smooth /= max(np.abs(smooth).max(), 1e-30)

freq_index = np.arange(len(smooth)) - 60
gauss = np.exp(-0.5*(freq_index/target_sigma_bins)**2)
gauss /= gauss.max()
filter_rmse = float(np.sqrt(np.mean((smooth-gauss)**2)))

# ------------------------------------------------------------
# Plot and Save
# ------------------------------------------------------------
fig, axes = plt.subplots(3, 1, figsize=(15, 15), constrained_layout=True)

im0 = axes[0].pcolormesh(
    times, freqs, ref_db, shading="auto", cmap="magma", vmin=-80, vmax=0
)
axes[0].set_yscale("log")
axes[0].set_ylim(FMIN, FMAX)
axes[0].set_ylabel("Frequência (Hz)")
axes[0].set_title(
    f"CQT de Referência (STFT + Gaussianas Logarítmicas) - {BPO} bins/oitava"
)
fig.colorbar(im0, ax=axes[0], label="Magnitude (dB)")

im1 = axes[1].pcolormesh(
    times, freqs, lift_db, shading="auto", cmap="magma", vmin=-80, vmax=0
)
axes[1].set_yscale("log")
axes[1].set_ylim(FMIN, FMAX)
axes[1].set_ylabel("Frequência (Hz)")
axes[1].set_title(
    "CQT Reversível Otimizada (Lifting FIR K=2) - Reversibilidade Perfeita"
)
fig.colorbar(im1, ax=axes[1], label="Energia (dB)")

# Plot 1D impulse response
xx = np.arange(len(smooth)) - 60
mask = np.abs(xx) <= 18
axes[2].plot(
    xx[mask], 20*np.log10(np.maximum(np.abs(gauss[mask]), 1e-8)),
    linewidth=2.5, label="Gaussiana CQT Alvo"
)
axes[2].plot(
    xx[mask], 20*np.log10(np.maximum(np.abs(smooth[mask]), 1e-8)),
    "--", linewidth=2.0, label="Cascata Lifting FIR (Otimizada)"
)
axes[2].set_ylim(-60, 3)
axes[2].set_xlabel("Deslocamento em bins de log-frequência (1/60 oitava)")
axes[2].set_ylabel("Magnitude (dB)")
axes[2].set_title(
    f"Resposta Espectral de Impulso - RMSE Linear = {filter_rmse:.6g}"
)
axes[2].legend()
axes[2].grid(True, alpha=0.15)

fig.suptitle(
    f"Estudo de CQT Reversível por Cascata de Lifting FIR | 60 bins/oitava, {FMIN:.0f}–{FMAX:.0f} Hz\n"
    f"Reversibilidade dos Coeficientes: Erro Máximo = {max_coef_err}, Mismatches = {coef_mismatch} | "
    f"Sinal Original: {fs} Hz, {x_int.shape[0]} amostras"
)

fig.savefig(out_path, dpi=180)
print(f"Sucesso! Gráfico salvo em: {out_path}")
print(f"Parâmetros Otimizados P: p0 = {p_opt[0]:.6f}, p1 = {p_opt[1]:.6f}, p2 = {p_opt[2]:.6f}")
print(f"Parâmetros Otimizados U: u0 = {u_opt[0]:.6f}, u1 = {u_opt[1]:.6f}, u2 = {u_opt[2]:.6f}")
print(f"Parâmetros de Ponto Fixo Q20 P: p0 = {p_fix[0]}, p1 = {p_fix[1]}, p2 = {p_fix[2]}")
print(f"Parâmetros de Ponto Fixo Q20 U: u0 = {u_fix[0]}, u1 = {u_fix[1]}, u2 = {u_fix[2]}")
print(f"Bijeção Inteira Perfeita de Coeficientes: {max_coef_err == 0 and coef_mismatch == 0}")
