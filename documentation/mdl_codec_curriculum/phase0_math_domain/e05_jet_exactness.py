#!/usr/bin/env python3
"""
E05 - Exatidão dos Jets Matemáticos
Usa representação analítica em domínio contínuo vs transformada para provar 
a extração dos Jatos J_1 e J_2 (Frequency e Chirp Rate).
O sinal analítico (via Hilbert) demonstra a exatidão teórica isolada das janelas de frame discretos.
"""
import numpy as np
import scipy.signal
import math
import json
from pathlib import Path

def test_jet_exactness():
    print("=========================================================")
    print("🔬 E05: EXATIDÃO DOS JETS (JET EXACTNESS)")
    print("=========================================================")
    
    sr = 12000
    N = sr * 1
    t = np.arange(N) / sr
    f0 = 100.0
    alpha = 50.0
    
    x = np.cos(2 * math.pi * (f0 * t + 0.5 * alpha * t**2))
    
    # 1. Analytic Signal Extraction (Theoretical upper bound for exact transform)
    xa = scipy.signal.hilbert(x)
    phase = np.unwrap(np.angle(xa))
    
    dt = 1.0 / sr
    f_inst_est = np.gradient(phase, dt) / (2 * math.pi)
    alpha_est = np.gradient(f_inst_est, dt)
    
    # 2. Evaluation inside valid window
    valid = slice(1000, -1000)
    
    f_inst_true = f0 + alpha * t[valid]
    err_f = np.mean(np.abs(f_inst_true - f_inst_est[valid]))
    err_alpha = np.mean(np.abs(alpha - alpha_est[valid]))
    
    print(f"Erro J_1 (Freq Inst)  : {err_f:.4f} Hz")
    print(f"Erro J_2 (Chirp Rate) : {err_alpha:.4f} Hz/s")
    
    passed = err_f < 0.1 and err_alpha < 5.0
    print(f"Jets Extraídos com Sucesso: {'✅ SIM' if passed else '❌ NÃO'}")
    
    out_dir = Path(__file__).parent
    with open(out_dir / "e05_jet_exactness.json", "w") as f:
        json.dump({
            "stage": "E05",
            "err_f_hz": float(err_f),
            "err_alpha_hz_s": float(err_alpha),
            "passed": bool(passed)
        }, f, indent=2)

if __name__ == '__main__':
    test_jet_exactness()
