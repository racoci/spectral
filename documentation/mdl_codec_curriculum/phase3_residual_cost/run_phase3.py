#!/usr/bin/env python3
import json
import sys
from pathlib import Path

# Inserir paths
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase1_signal_laboratory'))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase2_competition'))

from signal_laboratory import SignalLaboratory
from competition_framework import CompetitionFramework

# Competidores "best in class" das categorias
from m4_hankel_ssa import M4_HankelSSA
from m6_wavelet_maxima import M6_WaveletMaxima
from m12_invertible_flow import M12_InvertibleFlow

from residual_evaluator import ResidualEvaluator

def main():
    print("=========================================================")
    print("📊 FASE 3: AVALIAÇÃO DE CUSTO DO RESÍDUO (MDL)")
    print("=========================================================")
    
    sr = 12000.0
    lab = SignalLaboratory(sr=sr)
    dur = 0.25
    N = int(sr * dur)
    
    sinais = {
        'E06_PureTone': lab.e06_pure_tone(440.0, 0.5, 0.0, dur), # amplitude reduzida pra não clipar no float
        'E14_Transient': lab.e14_transient(0.125, dur, kind='noise_burst')
    }
    
    # Roda a extração
    comp = CompetitionFramework(sr=sr)
    comp.register_competitor(M4_HankelSSA(sr=sr, window_len=128, rank=2))
    comp.register_competitor(M6_WaveletMaxima(sr=sr, num_scales=16))
    comp.register_competitor(M12_InvertibleFlow(sr=sr, hop_length=128, n_layers=4))
    
    evaluator = ResidualEvaluator(bit_depth=16)
    
    all_results = {}
    
    for nome, (x, truth) in sinais.items():
        print(f"\n--- Sinal: {nome} ---")
        # Roda inferência
        res = comp.evaluate_signal(nome, x, truth, parallel=False) # Roda linear pra debugar
        
        all_results[nome] = {}
        
        # Recupera as predições e os thetas (precisamos do x_hat e do num_params)
        for comp_obj in comp.competitors:
            c_name = comp_obj.name
            c_metrics = res[c_name]
            
            if not c_metrics['success']:
                continue
            
            # Recalcula do objeto
            # Precisamos do x_hat
            theta, x_hat = comp_obj.fit_and_reconstruct(x)
            
            # Conta params
            num_params = c_metrics.get('num_params', 0)
            if num_params == 0:
                if c_name == 'M4_HankelSSA':
                    num_params = len(theta.get('U_k', [])) * len(theta.get('U_k', [[]])[0]) + len(theta.get('V_k', [])) * len(theta.get('V_k', [[]])[0]) + len(theta.get('S_k', []))
                elif c_name == 'M6_WaveletMaxima':
                    num_params = len(theta.get('maxima', [])) * 3
                elif c_name == 'M12_InvertibleFlow':
                    num_params = len(theta.get('latent_z', [])) * len(theta.get('latent_z', [[]])[0])
            
            mdl = evaluator.evaluate(x, x_hat, num_params)
            
            all_results[nome][c_name] = {
                'sisdr': c_metrics['sisdr_db'],
                'mdl': mdl
            }
            
            print(f"[{c_name:<18}] SI-SDR: {c_metrics['sisdr_db']:>6.1f} dB")
            print(f"    L(theta): {mdl['L_theta_bits']/1024:>6.1f} kb | L(R): {mdl['L_residual_bits']/1024:>6.1f} kb | L_tot: {mdl['L_total_bits']/1024:>6.1f} kb")
            print(f"    Comp. Ratio: {mdl['compression_ratio']:>6.2f}x (Raw: {mdl['L_raw_baseline_bits']/1024:>6.1f} kb)")

    out_dir = Path(__file__).parent
    with open(out_dir / "phase3_residual_cost.json", "w") as f:
        json.dump(all_results, f, indent=2)

if __name__ == '__main__':
    main()
