#!/usr/bin/env python3
"""
E02 - Expansão de Intervalo Dinâmico (Dynamic Range Expansion)
Testa se a transformação de tempo para tempo-frequência (ou pseudo-lifting)
infla as magnitudes a um ponto que exija muitos bits adicionais.
Para compressão lossless (MDL), cada bit gerado pela expansão estrutural é custo real.
"""
import numpy as np
import json
from pathlib import Path

def evaluate_dynamic_range_expansion():
    print("=========================================================")
    print("🔬 E02: EXPANSÃO DE INTERVALO DINÂMICO (DYNAMIC RANGE)")
    print("=========================================================")
    
    sr = 12000
    dur = 1.0
    N = int(sr * dur)
    rng = np.random.default_rng(42)
    
    # Geramos ruído branco cobrindo quase toda a faixa 16-bit
    x = rng.integers(-30000, 30000, size=N, dtype=np.int32)
    
    original_bits = np.log2(np.max(np.abs(x)) + 1)
    
    # STFT de referência (hanning, hop N/4)
    import scipy.signal
    f, t, Zxx = scipy.signal.stft(x.astype(float), fs=sr, window='hann', nperseg=256)
    
    # Expand range: max magnitude da STFT
    mag = np.abs(Zxx)
    max_mag = np.max(mag)
    
    transformed_bits = np.log2(max_mag + 1)
    expansion_ratio = transformed_bits / original_bits
    
    print(f"Original Bit Depth Requerido : {original_bits:.2f} bits")
    print(f"STFT Mag Bit Depth Requerido : {transformed_bits:.2f} bits")
    print(f"Taxa de Expansão (Ratio)     : {expansion_ratio:.2f}x")
    
    # Em abordagens integer-to-integer MDSCT, busca-se ratio próximo a 1.0.
    # Em STFT redundante, o crescimento e a redundância (frames overlap) multiplicam
    # a quantidade bruta de bits do campo C(t, u) antes do enxugamento estrutural.
    
    # Aqui fazemos um sanity check
    passed = expansion_ratio < 2.0
    print(f"Expansão Controlada (< 2x)   : {'✅ SIM' if passed else '❌ NÃO'}")
    
    out_dir = Path(__file__).parent
    with open(out_dir / "e02_dynamic_range.json", "w") as f:
        json.dump({
            "stage": "E02",
            "original_bits": float(original_bits),
            "transformed_bits": float(transformed_bits),
            "expansion_ratio": float(expansion_ratio),
            "passed": bool(passed)
        }, f, indent=2)

if __name__ == '__main__':
    evaluate_dynamic_range_expansion()
