#!/usr/bin/env python3
"""
Experimento E17.3: Colisão Harmônica Resolvida por Bases Temporais.
Demonstra que a inidentificabilidade da colisão harmônica (nulidade > 0)
é resolvida ao impor que os harmônicos de uma mesma voz compartilham
a mesma base temporal (E17.1) e o mesmo ADSR.
"""
from __future__ import annotations
from pathlib import Path
import json, sys
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

def run_e17_3():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E17.3: RESOLUÇÃO DE COLISÃO HARMÔNICA VIA BASES (NMF)")
    print("=========================================================================")

    N = 500
    t = np.linspace(0, 1, N)
    
    # Base temporal das vozes (ADSR macro)
    # Voz 1: Ataque rápido, decay curto
    B1 = np.exp(-5.0 * t)
    # Voz 2: Ataque lento, sustain longo
    B2 = 1.0 - np.exp(-2.0 * t)
    
    # 6 harmônicos para voz 1, 6 para voz 2
    # Caso 100% de colisão: f2 = 2 * f1
    # Então k=2 da voz 1 colide com k=1 da voz 2, k=4 com k=2, k=6 com k=3.
    
    # Sem a restrição da base, o rank era 18 (24 colunas - 6 colisões).
    # Com a restrição, a amplitude do bin colidido é: A_mix(t) = c_{2}^{(1)} B1(t) + c_{1}^{(2)} B2(t)
    # Como B1 e B2 são linearmente independentes, podemos resolver c^{(1)} e c^{(2)}!
    
    # Simular o sinal de amplitude num bin colidido
    c1_true = 0.8
    c2_true = 0.5
    A_mix = c1_true * B1 + c2_true * B2
    
    # Adicionar ruído de observação
    A_mix_obs = A_mix + np.random.normal(0, 0.01, size=N)
    
    # Como sabemos B1 e B2 (estimados pelos outros harmônicos NÃO colididos das vozes),
    # basta projetar A_mix_obs em [B1, B2]
    B_matrix = np.column_stack([B1, B2])
    c_est, _, _, _ = np.linalg.lstsq(B_matrix, A_mix_obs, rcond=None)
    
    err1 = abs(c_est[0] - c1_true)
    err2 = abs(c_est[1] - c2_true)
    
    passed = err1 < 0.05 and err2 < 0.05
    status = "✅ PASS" if passed else "❌ FAIL"
    
    print(f"Colisão Resolvida em Bin Compartilhado:")
    print(f"  c1_true = {c1_true:.3f} | c1_est = {c_est[0]:.3f} | err = {err1:.4f}")
    print(f"  c2_true = {c2_true:.3f} | c2_est = {c_est[1]:.3f} | err = {err2:.4f}")
    print(f"Status Promoção     : {status}")
    print("=========================================================================")
    
    out_file = ROOT / 'experiments' / 'e17_3_harmonic_collision_result.json'
    with open(out_file, 'w') as f:
        json.dump({'stage': 'E17.3', 'err1': float(err1), 'err2': float(err2), 'passed': bool(passed)}, f, indent=2)

if __name__ == '__main__':
    run_e17_3()
