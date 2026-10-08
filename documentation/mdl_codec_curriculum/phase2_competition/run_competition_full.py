#!/usr/bin/env python3
import json
import sys
import numpy as np
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase1_signal_laboratory'))

from signal_laboratory import SignalLaboratory
from competition_framework import CompetitionFramework
from m1_cqt_ridge import M1_CQTRidge
from m3_matrix_pencil import M3_MatrixPencil
from m5_matching_pursuit import M5_MatchingPursuit

def main():
    print("=========================================================")
    print("🏆 FASE 2: COMPETIÇÃO ENTRE REPRESENTAÇÕES ESTRUTURAIS")
    print("=========================================================")
    
    lab = SignalLaboratory(sr=12000)
    dur = 0.25 
    
    sinais = {
        'E06_PureTone': lab.e06_pure_tone(440.0, 1.0, 0.0, dur),
        'E08_Chirp': lab.e08_linear_chirp(200.0, 400.0, 1.0, 0.0, dur),
        'E10_Polo': lab.e10_damped_exponential(300.0, 10.0, 1.0, 0.0, dur),
        'E12_Crossing': lab.e12_crossing_ridges(300.0, 500.0, 500.0, 300.0, dur),
        'E14_Transient_Click': lab.e14_transient(0.125, dur, kind='click')
    }
    
    comp = CompetitionFramework(sr=12000)
    comp.register_competitor(M1_CQTRidge(sr=12000, hop_length=128, n_peaks=2))
    comp.register_competitor(M3_MatrixPencil(sr=12000, hop_length=128, K=4))
    comp.register_competitor(M5_MatchingPursuit(sr=12000, max_iter=20, dict_size=1000))
    
    all_results = {}
    
    for nome, (x, truth) in sinais.items():
        print(f"\n--- Avaliando Sinal: {nome} ---")
        res = comp.evaluate_signal(nome, x, truth)
        all_results[nome] = {}
        for c_name, c_metrics in res.items():
            sisdr = c_metrics['sisdr_db']
            t_ms = c_metrics['compute_time_s'] * 1000
            print(f"[{c_name:<18}] SI-SDR: {sisdr:>7.2f} dB | Tempo: {t_ms:>6.1f} ms")
            
            # Contagem ingênua de features/params para avaliar o "Description Length"
            num_params = 0
            if c_name == 'M1_CQTRidge':
                # f, a, p * len(t)
                num_params = len(c_metrics['params']['f']) * len(c_metrics['params']['f'][0]) * 3
            elif c_name == 'M3_MatrixPencil':
                num_params = len(c_metrics['params']['blocks']) * 4 * 4 # K=4 polos * 4 attr (f, g, a, p)
            elif c_name == 'M5_MatchingPursuit':
                num_params = len(c_metrics['params']['atoms']) * 2 # idx, c
                
            all_results[nome][c_name] = {'sisdr': sisdr, 'time': t_ms, 'num_params': num_params}
            
    out_dir = Path(__file__).parent
    with open(out_dir / "competition_results_full.json", "w") as f:
        json.dump(all_results, f, indent=2)

if __name__ == '__main__':
    main()
