#!/usr/bin/env python3
"""
E03 - Condicionamento do Operador Transformada
Mede o condicionamento (kappa) da transformada T para avaliar a estabilidade à reconstrução.
Transformadas muito redundantes podem ser invertíveis, mas mal condicionadas,
amplificando pequenos erros de quantização de compressão num ruído gigante de áudio.
"""
import numpy as np
import json
from pathlib import Path
from scipy.signal import stft, istft

def evaluate_conditioning():
    print("=========================================================")
    print("🔬 E03: CONDICIONAMENTO DA TRANSFORMADA (STABILITY)")
    print("=========================================================")
    
    sr = 12000
    N = 256 # Small size to make explicitly constructing the operator matrix viable
    
    # We will compute the implicit matrix operator of the transformation x -> C
    # for a standard STFT frame to measure condition number.
    
    T_matrix = []
    
    # Impulse response per sample
    for i in range(N):
        x = np.zeros(N)
        x[i] = 1.0
        # Compute STFT
        f, t, Zxx = stft(x, fs=sr, window='boxcar', nperseg=N, noverlap=0)
        # Flatten complex to [Real, Imag]
        Zxx_flat = np.concatenate([Zxx.real.flatten(), Zxx.imag.flatten()])
        T_matrix.append(Zxx_flat)
        
    T_matrix = np.array(T_matrix).T # Columns are impulse responses
    
    # SVD
    U, s, Vh = np.linalg.svd(T_matrix, full_matrices=False)
    
    sigma_max = np.max(s)
    sigma_min = np.min(s[s > 1e-10]) # ignore null space if any
    
    kappa = sigma_max / sigma_min
    
    # Test stability to quantization noise
    rng = np.random.default_rng(42)
    x = rng.normal(0, 1.0, N)
    f, t, Zxx = stft(x, fs=sr, window='boxcar', nperseg=N, noverlap=0)
    
    # Add quantization noise uniformly distributed in [-0.05, 0.05]
    Zxx_quant = Zxx + (rng.random(Zxx.shape) - 0.5) * 0.1 + 1j * (rng.random(Zxx.shape) - 0.5) * 0.1
    
    _, x_hat = istft(Zxx_quant, fs=sr, window='boxcar', nperseg=N, noverlap=0)
    x_hat = x_hat[:N]
    
    reconstruction_err = np.max(np.abs(x - x_hat))
    
    print(f"Sigma Max : {sigma_max:.4f}")
    print(f"Sigma Min : {sigma_min:.4f}")
    print(f"Kappa (k) : {kappa:.4f}")
    print(f"Erro Quant: {reconstruction_err:.4f}")
    
    passed = kappa < 10.0
    print(f"Bem condicionado (k < 10): {'✅ SIM' if passed else '❌ NÃO'}")
    
    out_dir = Path(__file__).parent
    with open(out_dir / "e03_conditioning.json", "w") as f:
        json.dump({
            "stage": "E03",
            "kappa": float(kappa),
            "quantization_error": float(reconstruction_err),
            "passed": bool(passed)
        }, f, indent=2)

if __name__ == '__main__':
    evaluate_conditioning()
