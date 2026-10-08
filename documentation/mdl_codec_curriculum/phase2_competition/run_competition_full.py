#!/usr/bin/env python3
import json
import sys
import numpy as np
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase1_signal_laboratory'))

from signal_laboratory import SignalLaboratory
from competition_framework import CompetitionFramework
from m1_cqt_ridge import M1_CQTRidge
from m1b_cqt_jet_ridge import M1b_CQTJetRidge
from m2_chirplet_ridge import M2_ChirpletRidge
from m3_matrix_pencil import M3_MatrixPencil
from m4_hankel_ssa import M4_HankelSSA
from m5_matching_pursuit import M5_MatchingPursuit
from m6_wavelet_maxima import M6_WaveletMaxima
from m7_afd import M7_AdaptiveFourierDecomposition as M7_AFD
from m8_dmd import M8_DMD
from m9_vmd import M9_VMD
from m11_state_space_gp import M11_StateSpaceGPSDE as M11_StateSpaceGP
from m12_invertible_flow import M12_InvertibleFlow

def main():
    print("=========================================================")
    print("🏆 FASE 2: MEGA-COMPETIÇÃO ESTRUTURAL (M1 a M12)")
    print("=========================================================")
    
    lab = SignalLaboratory(sr=12000)
    dur = 0.25 
    
    sinais = {
        'E06_PureTone': lab.e06_pure_tone(440.0, 1.0, 0.0, dur),
        'E08_Chirp': lab.e08_linear_chirp(200.0, 400.0, 1.0, 0.0, dur),
        'E10_Polo': lab.e10_damped_exponential(300.0, 10.0, 1.0, 0.0, dur),
        'E12_Crossing': lab.e12_crossing_ridges(300.0, 500.0, 500.0, 300.0, dur),
        'E14_Transient': lab.e14_transient(0.125, dur, kind='noise_burst')
    }
    
    comp = CompetitionFramework(sr=12000)
    comp.register_competitor(M1_CQTRidge(sr=12000, hop_length=128, n_peaks=2))
    comp.register_competitor(M1b_CQTJetRidge(sr=12000, hop_length=128, n_peaks=2))
    comp.register_competitor(M2_ChirpletRidge(sr=12000, hop_length=128))
    comp.register_competitor(M3_MatrixPencil(sr=12000, hop_length=128, K=4))
    comp.register_competitor(M4_HankelSSA(sr=12000, window_len=128, rank=4))
    comp.register_competitor(M5_MatchingPursuit(sr=12000, max_iter=20, dict_size=500))
    comp.register_competitor(M6_WaveletMaxima(sr=12000, num_scales=16))
    comp.register_competitor(M7_AFD(sr=12000, num_atoms=8))
    comp.register_competitor(M8_DMD(sr=12000, window_len=128, rank=4))
    comp.register_competitor(M9_VMD(sr=12000, K=4, alpha=1500.0))
    comp.register_competitor(M11_StateSpaceGP(sr=12000, num_resonators=2))
    comp.register_competitor(M12_InvertibleFlow(sr=12000, hop_length=128, n_layers=4))
    
    all_results = {}
    
    # Run fully parallelized execution over models
    for nome, (x, truth) in sinais.items():
        print(f"\n--- Avaliando Sinal: {nome} ---")
        # Framework updated internally by conductor to support parallel evaluation
        res = comp.evaluate_signal(nome, x, truth, parallel=True, max_workers=6)
        all_results[nome] = {}
        for c_name, c_metrics in res.items():
            sisdr = c_metrics['sisdr_db']
            t_ms = c_metrics['compute_time_s'] * 1000
            
            # Simple fallback parameters count
            num_params = c_metrics.get('num_params', 0)
            print(f"[{c_name:<18}] SI-SDR: {sisdr:>7.2f} dB | Tempo: {t_ms:>6.1f} ms | Params: {num_params}")
            all_results[nome][c_name] = {'sisdr': sisdr, 'time': t_ms, 'num_params': num_params}
            
    out_dir = Path(__file__).parent
    with open(out_dir / "competition_results_full.json", "w") as f:
        json.dump(all_results, f, indent=2)

if __name__ == '__main__':
    main()
