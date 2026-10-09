#!/usr/bin/env python3
import json
import sys
from pathlib import Path
import numpy as np

# Inserir paths
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase1_signal_laboratory'))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase2_competition'))
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'phase3_residual_cost'))

from signal_laboratory import SignalLaboratory
from m1_cqt_ridge import M1_CQTRidge
from m3_matrix_pencil import M3_MatrixPencil
from m4_hankel_ssa import M4_HankelSSA
from m6_wavelet_maxima import M6_WaveletMaxima

from residual_evaluator import ResidualEvaluator
from hybrid_router import StructuralRouter

def main():
    print("=========================================================")
    print("🔀 FASE 4: A HIPÓTESE HÍBRIDA (STRUCTURAL ROUTING)")
    print("=========================================================")
    
    sr = 12000.0
    lab = SignalLaboratory(sr=sr)
    
    # Criar um sinal Híbrido (1.0 seg): 
    # [0.0 - 0.25] Pure Tone (Boa para Hankel/Matrix Pencil)
    # [0.25 - 0.5] Crossing Ridges (Boa para Matrix Pencil/Ridge)
    # [0.5 - 0.75] Transient Noise Burst (Desastre para paramétricos, Raw vence)
    # [0.75 - 1.0] Chirp (Boa para Ridge)
    
    dur_block = 0.25
    N_b = int(sr * dur_block)
    
    print("Gerando Sinal Musical Composto (Oscilador -> Polifonia -> Transiente -> Chirp)...")
    x0, _ = lab.e06_pure_tone(440.0, 0.6, 0.0, dur_block)
    x1, _ = lab.e12_crossing_ridges(300.0, 500.0, 500.0, 300.0, dur_block)
    x1 *= 0.3 # Scale down
    x2, _ = lab.e14_transient(0.1, dur_block, kind='noise_burst')
    x3, _ = lab.e08_linear_chirp(200.0, 600.0, 0.6, 0.0, dur_block)
    
    x_hybrid = np.concatenate([x0, x1, x2, x3])
    
    evaluator = ResidualEvaluator(bit_depth=16)
    competitors = [
        M1_CQTRidge(sr=sr, hop_length=128, n_peaks=2),
        M3_MatrixPencil(sr=sr, hop_length=128, K=4),
        M4_HankelSSA(sr=sr, window_len=128, rank=2),
        M6_WaveletMaxima(sr=sr, num_scales=16)
    ]
    
    router = StructuralRouter(block_size=N_b, evaluator=evaluator, competitors=competitors)
    
    print("Executando Roteamento Exaustivo (MDL)...")
    x_hat, decisions = router.process_signal(x_hybrid)
    
    L_total_hybrid = sum(d['L_bits'] for d in decisions)
    
    # Avaliando Baseline Estrita (Apenas Raw Áudio)
    x_int = evaluator.to_pcm(x_hybrid)
    h_x = evaluator.shannon_entropy(x_int)
    L_total_raw = h_x * len(x_hybrid)
    
    print("\nResultados do Roteador por Bloco:")
    for d in decisions:
        block_name = ['Pure Tone', 'Crossing', 'Transient', 'Chirp'][d['block']]
        print(f"  Bloco {d['block']} ({block_name:<10}): Escolhido -> {d['method']:<18} | Custo: {d['L_bits']/1024:>6.1f} kb")
        
    print(f"\nCusto Total Raw Audio    : {L_total_raw/1024:>6.1f} kb")
    print(f"Custo Total Codec Híbrido: {L_total_hybrid/1024:>6.1f} kb")
    ratio = L_total_hybrid / max(L_total_raw, 1.0)
    print(f"Compression Ratio (MDL)  : {ratio:>6.2f}x")
    
    out_dir = Path(__file__).parent
    with open(out_dir / "phase4_hybrid_routing.json", "w") as f:
        json.dump({
            "stage": "E37",
            "decisions": decisions,
            "L_raw_kb": float(L_total_raw/1024),
            "L_hybrid_kb": float(L_total_hybrid/1024),
            "compression_ratio": float(ratio)
        }, f, indent=2)

if __name__ == '__main__':
    main()
