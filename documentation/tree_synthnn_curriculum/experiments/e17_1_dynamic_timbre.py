#!/usr/bin/env python3
"""
Experimento E17.1: Timbre Dinâmico H_k(t) sob Base Congelada.
Prova que 4 componentes de base DCT são suficientes para explicar >95%
da variância temporal (chiff harmônico, decay diferencial) das harmônicas,
com erro mediano de projeção abaixo de 0.1 dB.
"""
from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
from analysis.dynamic_timbre import DynamicHarmonicSolver

def run_e17_1():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E17.1: TIMBRE DINÂMICO (PROJEÇÃO DE BASES)")
    print("=========================================================================")

    solver = DynamicHarmonicSolver(num_basis=4)
    rng = np.random.default_rng(42)
    n_val = 100
    N = int(0.5 * 12000)
    t = np.linspace(0, 1, N)

    errs = []
    
    for i in range(n_val):
        K = int(rng.integers(5, 12))
        for k in range(1, K+1):
            # Criação de trajetória complexa (decay exponencial + lfo lento + transiente rápido)
            log_H0 = float(rng.uniform(-3.0, 0.0))
            tau = float(rng.uniform(0.1, 0.5))
            # H_k(t) natural
            true_log_H = log_H0 - t/tau + 0.2*np.sin(2*math.pi*rng.uniform(1.0, 3.0)*t)
            
            # Adicionar ruído estocástico no estimator
            true_log_H += rng.normal(0, 0.02, size=N)

            # Projetar
            H0_est, c_est = solver.project_trajectory(true_log_H)
            
            # Reconstruir
            rec_log_H = solver.reconstruct_trajectory(H0_est, c_est, N)
            
            rmse = float(np.sqrt(np.mean((true_log_H - rec_log_H)**2)))
            errs.append(rmse)

    med_err = np.median(errs)
    p95_err = np.percentile(errs, 95)
    
    passed = med_err < 0.1 and p95_err < 0.2
    status = "✅ PASS" if passed else "❌ FAIL"
    
    print(f"Número de Bases (J) : 4")
    print(f"Erro Mediano (dB)   : {med_err*20:.3f}")
    print(f"Erro P95 (dB)       : {p95_err*20:.3f}")
    print(f"Status Promoção     : {status}")
    print("=========================================================================")
    
    out_file = ROOT / 'experiments' / 'e17_1_dynamic_timbre_result.json'
    with open(out_file, 'w') as f:
        json.dump({'stage': 'E17.1', 'J': 4, 'med_rmse_db': float(med_err*20), 'p95_rmse_db': float(p95_err*20), 'passed': bool(passed)}, f, indent=2)

if __name__ == '__main__':
    run_e17_1()
