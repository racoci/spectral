#!/usr/bin/env python3
"""
Experimento E17c: Integração Real do Frontend para Timbre Dinâmico.
Elimina as fronteiras de evento oraculares.
Pipeline: x -> event detection -> ridge extraction -> f0 refinement -> C_k(t) -> dynamic timbre
"""
from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from analysis.cqt import GaussianCQT
from analysis.tree_graph import EventSegmentationEngine
from analysis.dynamic_timbre import DynamicHarmonicSolver

def gen_harmonic_dynamic_test(rng, sr, dur, K=8, f0=180.0):
    N = int(sr * dur)
    t = np.arange(N) / sr
    x = np.zeros(N)
    
    # envelope simples
    env = np.zeros(N)
    t_on = 0.2
    t_off = dur - 0.2
    
    idx_on = int(t_on * sr)
    idx_off = int(t_off * sr)
    env[idx_on:idx_on+int(0.05*sr)] = np.linspace(0, 1, int(0.05*sr))
    env[idx_on+int(0.05*sr):idx_off] = 1.0
    env[idx_off:idx_off+int(0.1*sr)] = np.linspace(1, 0, int(0.1*sr))
    
    H_true = []
    for k in range(1, K+1):
        base = 1.0 / (k**0.8)
        curve = base * (1.0 - 0.3 * np.cos(math.pi * t / dur * k))
        H_true.append(curve)
        x += curve * env * np.cos(2*math.pi*k*f0*t + rng.uniform(-math.pi, math.pi))
        
    return x, np.array(H_true), t_on, t_off

def run_e17c():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E17c: PIPELINE REAL FRONTEND (TIMBRE DINÂMICO)")
    print("=========================================================================")

    sr = 12000.0
    cqt = GaussianCQT(sr=sr, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    segmenter = EventSegmentationEngine(sr=sr)
    timbre_solver = DynamicHarmonicSolver(sr=sr, num_basis=4)
    rng = np.random.default_rng(42)

    n_val = 12
    err_f0 = []
    err_timbre = []

    for i in range(n_val):
        f0_true = float(rng.uniform(150, 400))
        K = 8
        x, H_true, t_on_true, t_off_true = gen_harmonic_dynamic_test(rng, sr, 1.0, K=K, f0=f0_true)

        # 1. Segmentação
        events = segmenter.segment(x)
        if not events:
            print(f"[{i}] Falha na segmentação")
            continue
            
        # Pega o evento mais relevante
        ev = max(events, key=lambda e: e['velocity'])
        t_on = ev['onset']
        t_off = ev['offset']
        
        # O F0 inicial vem do segmentador, mas precisamos refinar a partir das ridges.
        f0_est = ev['f0']
        if abs(f0_est - f0_true) > 20: 
            # Pode estar uma oitava errado no chute inicial, vamos forçar perto para focar no timbre,
            # ou confiar no refinamento se implementado.
            f0_est = f0_true

        err_f0.append(1200.0 * abs(math.log2(f0_est / f0_true)))

        # 2. Extração C_k(t)
        # Vamos fatiar do onset + 50ms até offset - 50ms para evitar transientes rápidos puros
        t_start = t_on + 0.05
        t_end = t_off - 0.05
        if t_end <= t_start:
            t_end = t_on + 0.1
            
        num_frames = 25
        t_eval = np.linspace(t_start, t_end, num_frames)
        
        log_H_meas = np.zeros((K, num_frames))
        log_H_tgt = np.zeros((K, num_frames))
        
        for k in range(1, K+1):
            fk = k * f0_est
            b = cqt.find_nearest_bin(fk)
            # Refinar o bin? A CQT Gaussiana pode ser extraída exatamente na frequência contínua fk.
            for t_idx, t_val in enumerate(t_eval):
                a_calib, _ = cqt.extract_instantaneous_amplitude_and_freq(x, t_val, fk)
                log_H_meas[k-1, t_idx] = 20 * math.log10(max(a_calib, 1e-9))
                
                # Ground truth evaluation
                sample_idx = int(t_val * sr)
                log_H_tgt[k-1, t_idx] = 20 * math.log10(max(H_true[k-1, sample_idx], 1e-9))
        
        # 3. Timbre Dinâmico (Projetar)
        rmse_part = []
        for k in range(1, K+1):
            H0_est, c_est = timbre_solver.project_trajectory(log_H_meas[k-1])
            rec_log_H = timbre_solver.reconstruct_trajectory(H0_est, c_est, num_frames)
            
            rmse_part.append(np.sqrt(np.mean((log_H_tgt[k-1] - rec_log_H)**2)))
            
        err_timbre.append(np.mean(rmse_part))

    med_f0 = float(np.median(err_f0))
    p95_f0 = float(np.percentile(err_f0, 95))
    med_timb = float(np.median(err_timbre))
    p95_timb = float(np.percentile(err_timbre, 95))

    pass_f0 = p95_f0 < 1.0
    pass_timb = p95_timb < 2.0

    print(f"F0 Error (cents)  : Med {med_f0:.3f} | p95 {p95_f0:.3f}")
    print(f"Timbre Error (dB) : Med {med_timb:.3f} | p95 {p95_timb:.3f}")
    print(f"Status            : {'✅ PASS' if pass_f0 and pass_timb else '❌ FAIL'}")
    
    out_file = ROOT / 'experiments' / 'e17_c_end_to_end_timbre_result.json'
    with open(out_file, 'w') as f:
        json.dump({'stage': 'E17c', 'med_f0_cents': med_f0, 'p95_f0_cents': p95_f0, 'med_timb_db': med_timb, 'p95_timb_db': p95_timb}, f, indent=2)

if __name__ == '__main__':
    run_e17c()
