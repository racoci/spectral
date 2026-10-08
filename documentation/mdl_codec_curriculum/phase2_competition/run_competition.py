#!/usr/bin/env python3
import json
import sys
from pathlib import Path

# Fix python path
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase1_signal_laboratory'))

from signal_laboratory import SignalLaboratory
from competition_framework import CompetitionFramework
from m1_cqt_ridge import M1_CQTRidge
from m3_matrix_pencil import M3_MatrixPencil

def main():
    print("=========================================================")
    print("🏆 FASE 2: COMPETIÇÃO ENTRE REPRESENTAÇÕES (M1 vs M3)")
    print("=========================================================")
    
    lab = SignalLaboratory(sr=12000)
    dur = 0.25 # Sinal curto para rapidez na SVD do Matrix Pencil
    
    # Gerar Sinais Ground Truth
    print("Gerando Laboratório de Sinais...")
    sinais = {
        'E06_PureTone': lab.e06_pure_tone(440.0, 1.0, 0.0, dur),
        'E08_Chirp': lab.e08_linear_chirp(200.0, 400.0, 1.0, 0.0, dur),
        'E10_Polo': lab.e10_damped_exponential(300.0, 10.0, 1.0, 0.0, dur),
        'E12_Crossing': lab.e12_crossing_ridges(300.0, 500.0, 500.0, 300.0, dur)
    }
    
    comp = CompetitionFramework(sr=12000)
    # n_peaks = 2, K=4 (para 2 senoides reais -> 4 polos conjugados)
    comp.register_competitor(M1_CQTRidge(sr=12000, hop_length=128, n_peaks=2))
    comp.register_competitor(M3_MatrixPencil(sr=12000, hop_length=128, K=4))
    
    all_results = {}
    
    for nome, (x, truth) in sinais.items():
        print(f"\n--- Avaliando Sinal: {nome} ---")
        res = comp.evaluate_signal(nome, x, truth)
        all_results[nome] = {}
        for c_name, c_metrics in res.items():
            sisdr = c_metrics['sisdr_db']
            t_ms = c_metrics['compute_time_s'] * 1000
            print(f"[{c_name:<16}] SI-SDR: {sisdr:>7.2f} dB | Tempo: {t_ms:>6.1f} ms")
            all_results[nome][c_name] = {'sisdr': sisdr, 'time': t_ms}
            
    out_dir = Path(__file__).parent
    with open(out_dir / "competition_results_m1_m3.json", "w") as f:
        json.dump(all_results, f, indent=2)

if __name__ == '__main__':
    main()
