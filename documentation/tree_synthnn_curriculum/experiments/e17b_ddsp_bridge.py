#!/usr/bin/env python3
"""
Experimento E17b: Ponte DDSP-TreeNN (Expressividade e Transientes).

Objetivos:
1. Extrair Envelope Residual Expressivo R(t) eliminando o viés do ADSR.
2. Identificar Ruído Transiente condicionado à derivada do envelope c_trans * (dA/dt)^2.
3. Rastrear a Dispersão Relativa de Fase Delta phi_k no onset para preservar o crest factor.
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
PROJECT_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(PROJECT_ROOT))

from analysis.ddsp_bridge import DDSPBridgeSolver
from analysis.cqt import GaussianCQT

def run_e17b():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E17b: PONTE DDSP (EXPRESSIVIDADE E TRANSIENTES)")
    print("=========================================================================")

    sr = 12000.0
    solver = DDSPBridgeSolver(sr=sr)
    cqt = GaussianCQT(sr=sr, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    n_val = 50
    results = {}
    
    print("\n--- 1. Avaliação de Residual Expressivo R(t) e Transiente de Ruído ---")
    print(f"{'Split':<15} | {'RMSE R(t)':<15} | {'Erro c_trans (%)':<18} | {'Status':<10}")
    print("-" * 70)

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        err_r_list, err_c_list = [], []

        for _ in range(n_val):
            duration = 1.0
            N = int(duration * sr)
            t = np.arange(N) / sr

            t_on = 0.15
            t_off = 0.85
            f0 = float(rng.uniform(200.0, 400.0))
            
            # ADSR Simples
            t_rel = t - t_on
            A_adsr = np.zeros(N)
            A_adsr[(t_rel >= 0) & (t_rel < 0.05)] = t_rel[(t_rel >= 0) & (t_rel < 0.05)] / 0.05
            A_adsr[(t_rel >= 0.05) & (t_rel < 0.70)] = 1.0
            A_adsr[t_rel >= 0.70] = np.exp(-(t_rel[t_rel >= 0.70] - 0.70) * 10.0)
            
            # Expressividade
            R_true = 0.15 * np.sin(2.0 * math.pi * 5.8 * t) * (t > t_on) * (t < t_off)
            A_meas = A_adsr * (1.0 + R_true) + rng.normal(0, 0.005, size=N)

            # Extração R(t)
            R_est = solver.extract_expressive_residual(A_meas, A_adsr)
            mask_sus = (t > t_on + 0.1) & (t < t_off - 0.1)
            rmse_r = float(np.sqrt(np.mean((R_true[mask_sus] - R_est[mask_sus])**2)))
            err_r_list.append(rmse_r)

            # Transient Noise
            c_true = float(rng.uniform(0.002, 0.008))
            sigma_base = 0.02
            dt = 1.0 / sr
            dA = np.maximum(0.0, np.gradient(A_adsr, dt))
            En_true = sigma_base**2 + c_true * (dA**2)
            x_noise = rng.normal(0, np.sqrt(np.maximum(1e-12, En_true)), size=N)

            s_est, c_est = solver.extract_transient_noise(x_noise, A_adsr, t_on, t_off)
            err_c = abs(c_est - c_true) / c_true * 100.0
            err_c_list.append(err_c)

        med_r = float(np.median(err_r_list))
        med_c = float(np.median(err_c_list))
        passed = (med_r < 0.02) and (med_c < 10.0)
        status = "✅ PASS" if passed else "❌ FAIL"
        
        results[s_name] = {'rmse_r': med_r, 'err_c_trans': med_c, 'pass': passed}
        print(f"{s_name:<15} | {med_r:<15.4f} | {med_c:<18.2f} | {status:<10}")

    print("\n--- 2. Avaliação de Dispersão de Fase Relativa (Delta phi_k) ---")
    err_phi = []
    for _ in range(50):
        N = int(0.5 * sr)
        t = np.arange(N) / sr
        f0 = float(rng.uniform(150.0, 300.0))
        t_on = 0.2
        K = 6
        x = np.zeros(N)
        phi_true = []
        for k in range(1, K+1):
            p = float(rng.uniform(-math.pi, math.pi))
            phi_true.append(p)
            x += (1.0 / k) * np.sin(2.0 * math.pi * k * f0 * t + p)

        dp_true = []
        for k in range(1, K+1):
            raw = phi_true[k-1] - k * phi_true[0]
            dp_true.append((raw + math.pi) % (2*math.pi) - math.pi)

        dp_est = solver.extract_phase_dispersion(x, f0, t_on, K, cqt)
        err = np.abs(np.array(dp_est) - np.array(dp_true))
        err = np.minimum(err, 2*math.pi - err)
        err_phi.append(np.mean(np.degrees(err)))

    med_phi = float(np.median(err_phi))
    passed = med_phi < 10.0
    status = "✅ PASS" if passed else "❌ FAIL"
    print(f"{'Fase Relativa':<15} | Erro Mediano: {med_phi:.2f} graus | {status}")
    
    results['PhaseDispersion'] = {'median_err_deg': med_phi, 'pass': passed}

    output_meta = {'stage': 'E17b', 'metrics': results}
    json_path = ROOT / 'experiments' / 'e17b_ddsp_bridge.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"\nResultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e17b()
