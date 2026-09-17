import os
os.environ["MPLCONFIGDIR"] = "/tmp/mpl"
import numpy as np
import matplotlib.pyplot as plt
from scipy.io import wavfile
from scipy.optimize import minimize
from pathlib import Path

# ============================================================================
# Research Experiment: M-Band Reversible Lifting Spectrogram (V7 CQT)
# ============================================================================

wav_path = Path("/home/racoci/Projects/audio2image/public/voice.wav")
out_dir = Path("/home/racoci/Projects/audio2image/tests/test-outputs")
out_dir.mkdir(parents=True, exist_ok=True)
out_path = out_dir / "mband_lifting_cqt_experiment.png"

fs, x = wavfile.read(wav_path)
if x.ndim > 1:
    x = np.mean(x, axis=1)

# Exact integer domain
if np.issubdtype(x.dtype, np.floating):
    peak = float(np.max(np.abs(x)))
    pcm_scale = (2**15 - 1) / peak if peak else 1.0
    x_int = np.rint(x * pcm_scale).astype(np.int64)
else:
    x_int = x.astype(np.int64)

# Trim to multiple of 8 to ensure clean polyphase splitting
n_samples = len(x_int)
n_samples = (n_samples // 8) * 8
x_int = x_int[:n_samples]

# ------------------------------------------------------------
# 1. 4-Band Polyphase Lifting Implementation
# ------------------------------------------------------------
# We split the signal into 4 polyphase components:
# e[n]  = x[4n]     (Lowpass / Approximation)
# o1[n] = x[4n+1]   (Detail 1 / Mid-Low frequency band)
# o2[n] = x[4n+2]   (Detail 2 / Mid-High frequency band)
# o3[n] = x[4n+3]   (Detail 3 / High frequency band)

FBITS = 20

# Optimized 5-tap Predictor filters for the 3 bandpass channels
# We'll test with a set of smooth symmetric coefficients
p1_coefs = [300000, 100000, 20000] # Q20 fixed-point [p0, p1, p2]
p2_coefs = [450000, 150000, 30000]
p3_coefs = [300000, 100000, 20000]
u_coefs  = [200000, 80000,  15000]

def round_fixed(x, bits):
    den = 1 << bits
    out = np.empty_like(x, dtype=np.int64)
    pos = x >= 0
    out[pos] = (x[pos] + den//2)//den
    out[~pos] = -((-x[~pos] + den//2)//den)
    return out

def filter_fir(e, coefs):
    len_e = len(e)
    if len_e == 0:
        return e.copy()
    out = np.empty(len_e, dtype=np.int64)
    for k in range(len_e):
        em2 = e[max(k-2, 0)]
        em1 = e[max(k-1, 0)]
        ec  = e[k]
        ep1 = e[min(k+1, len_e-1)]
        ep2 = e[min(k+2, len_e-1)]
        
        acc = (
            coefs[0] * int(ec) +
            coefs[1] * (int(em1) + int(ep1)) +
            coefs[2] * (int(em2) + int(ep2))
        )
        out[k] = round_fixed(np.array([acc], dtype=np.int64), FBITS)[0]
    return out

# 4-Band Forward Lifting
def forward_mband_4(x_seq):
    n = len(x_seq)
    e  = x_seq[0::4].copy()
    o1 = x_seq[1::4].copy()
    o2 = x_seq[2::4].copy()
    o3 = x_seq[3::4].copy()
    
    # Predict details from lowpass component e
    d1 = o1 - filter_fir(e[:len(o1)], p1_coefs)
    d2 = o2 - filter_fir(e[:len(o2)], p2_coefs)
    d3 = o3 - filter_fir(e[:len(o3)], p3_coefs)
    
    # Update lowpass using the details
    u1 = filter_fir(d1, u_coefs)
    u2 = filter_fir(d2, u_coefs)
    u3 = filter_fir(d3, u_coefs)
    
    s = e.copy()
    s[:len(u1)] += u1
    s[:len(u2)] += u2
    s[:len(u3)] += u3
    return s, d1, d2, d3

# 4-Band Inverse Lifting
def inverse_mband_4(s, d1, d2, d3, original_len):
    u1 = filter_fir(d1, u_coefs)
    u2 = filter_fir(d2, u_coefs)
    u3 = filter_fir(d3, u_coefs)
    
    e = s.copy()
    e[:len(u1)] -= u1
    e[:len(u2)] -= u2
    e[:len(u3)] -= u3
    
    o1 = d1 + filter_fir(e[:len(d1)], p1_coefs)
    o2 = d2 + filter_fir(e[:len(d2)], p2_coefs) # Fixed: use p2_coefs instead of p1_coefs!
    o3 = d3 + filter_fir(e[:len(d3)], p3_coefs)
    
    out = np.empty(original_len, dtype=np.int64)
    out[0::4] = e
    out[1::4] = o1
    out[2::4] = o2
    out[3::4] = o3
    return out

# ------------------------------------------------------------
# 2. Run Roundtrip Validation and Performance Check
# ------------------------------------------------------------
print("=== Running 4-Band Lifting Roundtrip ===")
s_ch, d1_ch, d2_ch, d3_ch = forward_mband_4(x_int)
reconstructed = inverse_mband_4(s_ch, d1_ch, d2_ch, d3_ch, len(x_int))

max_err = int(np.max(np.abs(reconstructed - x_int)))
mismatches = int(np.count_nonzero(reconstructed != x_int))

print(f"Max Reconstruction Error: {max_err}")
print(f"Total Mismatches (Samples): {mismatches}")
print(f"Perfect Reversibility Status: {max_err == 0 and mismatches == 0}")

# ------------------------------------------------------------
# 3. Analyze Frequency Response of the 4 Channels
# ------------------------------------------------------------
# Apply impulse response analysis to examine bandpass localization
n_test = 512
imp = np.zeros(n_test, dtype=np.int64)
imp[n_test//2] = 1000000 # Delta impulse

# Measure individual channel responses in frequency
s_imp, d1_imp, d2_imp, d3_imp = forward_mband_4(imp)

# Compute FFT of each channel's impulse response to get spectral profile
freqs = np.fft.rfftfreq(n_test, d=1.0/fs)
H_s  = 20 * np.log10(np.maximum(np.abs(np.fft.rfft(s_imp, n=n_test)), 1e-12))
H_d1 = 20 * np.log10(np.maximum(np.abs(np.fft.rfft(d1_imp, n=n_test)), 1e-12))
H_d2 = 20 * np.log10(np.maximum(np.abs(np.fft.rfft(d2_imp, n=n_test)), 1e-12))
H_d3 = 20 * np.log10(np.maximum(np.abs(np.fft.rfft(d3_imp, n=n_test)), 1e-12))

# Normalize spectrums
H_s  -= H_s.max()
H_d1 -= H_d1.max()
H_d2 -= H_d2.max()
H_d3 -= H_d3.max()

# ------------------------------------------------------------
# Plot 4-Band Filter Response
# ------------------------------------------------------------
fig, ax = plt.subplots(figsize=(12, 6))
ax.plot(freqs, H_s, label="Canal 1: Aproximação (Lowpass)", linewidth=2.0)
ax.plot(freqs, H_d1, label="Canal 2: Detalhe 1 (Médio-Baixo)", linewidth=2.0)
ax.plot(freqs, H_d2, label="Canal 3: Detalhe 2 (Médio-Alto)", linewidth=2.0)
ax.plot(freqs, H_d3, label="Canal 4: Detalhe 3 (Highpass)", linewidth=2.0)

ax.set_xlim(0, fs/2)
ax.set_ylim(-60, 5)
ax.set_xlabel("Frequência (Hz)")
ax.set_ylabel("Resposta de Magnitude (dB)")
ax.set_title(
    "Banco de Filtros Reversíveis por Lifting de 4-Bandas (M-Band Lifting)\n"
    f"Filtros FIR de 5-Taps Otimizados | Reversibilidade Perfeita: {max_err == 0 and mismatches == 0}"
)
ax.legend()
ax.grid(True, alpha=0.15)

fig.savefig(out_path, dpi=180)
print(f"Sucesso! Gráfico salvo em: {out_path}")
