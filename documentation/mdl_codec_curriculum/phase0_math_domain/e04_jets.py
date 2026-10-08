#!/usr/bin/env python3
"""
E04 - Jets (Geometria Diferencial Local)
Definindo como calcularemos os jatos diferenciais na transformada.
"""
import numpy as np
import json
from pathlib import Path

def compute_1d_jet(x, n_orders=2, dx=1.0):
    # Finite differences for order 0, 1, 2
    # J_0: x
    # J_1: dx/dt
    # J_2: d2x/dt2
    N = len(x)
    J = np.zeros((N, n_orders + 1))
    J[:, 0] = x
    if n_orders >= 1:
        J[:, 1] = np.gradient(x, dx)
    if n_orders >= 2:
        J[:, 2] = np.gradient(J[:, 1], dx)
    return J

def test_jets():
    print("=========================================================")
    print("🔬 E04: JETS (GEOMETRIA DIFERENCIAL LOCAL)")
    print("=========================================================")
    
    t = np.linspace(0, 1, 100)
    dt = t[1] - t[0]
    
    # f(t) = t^2
    # J_1 = 2t
    # J_2 = 2
    x = t**2
    J = compute_1d_jet(x, n_orders=2, dx=dt)
    
    # Avaliar o jet em t=0.5 (índice 50)
    idx = 50
    print(f"t = {t[idx]}")
    print(f"J_0 esperado: 0.25 | obtido: {J[idx, 0]:.4f}")
    print(f"J_1 esperado: 1.0  | obtido: {J[idx, 1]:.4f}")
    print(f"J_2 esperado: 2.0  | obtido: {J[idx, 2]:.4f}")
    
    out_dir = Path(__file__).parent
    with open(out_dir / "e04_jets.json", "w") as f:
        json.dump({
            "stage": "E04",
            "j0_err": float(abs(0.25 - J[idx, 0])),
            "j1_err": float(abs(1.0 - J[idx, 1])),
            "j2_err": float(abs(2.0 - J[idx, 2]))
        }, f, indent=2)

if __name__ == '__main__':
    test_jets()
