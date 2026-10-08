#!/usr/bin/env python3
"""
E00 - Contrato Digital
Garante a reconstrução bit-exact de um array de inteiros (PCM 16-bit).
"""
import numpy as np
import json
from pathlib import Path

def test_digital_contract():
    # Parâmetros contratuais
    sr = 12000
    bit_depth = 16
    dur = 1.0
    N = int(sr * dur)
    
    print("=========================================================")
    print("🔬 E00: CONTRATO DIGITAL (BIT-EXACT INTEGER RECONSTRUCTION)")
    print("=========================================================")
    
    rng = np.random.default_rng(42)
    # Gerando sinal no domínio alvo (-32768 a 32767)
    x = rng.integers(-32768, 32767, size=N, dtype=np.int16)
    
    # "Encode" e "Decode" dummy (identidade) para estabelecer o baseline
    z = x.copy()
    x_hat = z
    
    diff = np.abs(x.astype(np.int32) - x_hat.astype(np.int32))
    max_diff = np.max(diff)
    
    passed = (max_diff == 0)
    
    print(f"Formato     : PCM {bit_depth}-bit, {sr} Hz")
    print(f"Max Diff    : {max_diff}")
    print(f"Bit-Exact   : {'✅ SIM' if passed else '❌ NÃO'}")
    
    # Salvar resultados
    out_dir = Path(__file__).parent
    with open(out_dir / "e00_digital_contract.json", "w") as f:
        json.dump({
            "stage": "E00",
            "max_diff": int(max_diff),
            "bit_exact": bool(passed)
        }, f, indent=2)

if __name__ == '__main__':
    test_digital_contract()
