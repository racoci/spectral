#!/usr/bin/env python3
"""
Orquestrador Central de Testes de Integração em Python (MDL Codec & Neural Synthesis Curriculum).
Executa todas as suítes em Python garantindo código de saída 0 apenas com 100% de sucesso.
"""

import sys
import time
import subprocess
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent

PYTHON_TEST_SUITES = [
    ("Fase 7: Probabilistic Ridge Tracking & EKF (E46-E50)", "test_probabilistic_ridge_tracking.py"),
    ("Fase 8: Learned Ridge Trajectory Codec & Surprise Encoding (E51-E59)", "test_learned_ridge_codec.py"),
    ("Fase 12: SynthNN & TreeNN Hierarchical Causal Graph (E60-E63)", "test_treenn_causal_graph.py"),
    ("Fase 13-14: Amortized Inverse Synthesis & Relational Topology (E64-E72)", "test_amortized_inverse_synthesis.py")
]

def run_all():
    print("=" * 80)
    print("🚀 EXECUTANDO SUÍTE COMPLETA DE TESTES EM PYTHON (CURRÍCULO MDL)")
    print("=" * 80)

    total_start = time.perf_counter()
    failed = []

    for idx, (title, filename) in enumerate(PYTHON_TEST_SUITES, 1):
        test_path = TESTS_DIR / filename
        print(f"\n[{idx}/{len(PYTHON_TEST_SUITES)}] ▶️  {title} ({filename})...")
        suite_start = time.perf_counter()

        res = subprocess.run([sys.executable, str(test_path)])
        suite_time = (time.perf_counter() - suite_start) * 1000.0

        if res.returncode == 0:
            print(f"    ✅ Sucesso ({suite_time:.1f} ms)")
        else:
            print(f"    ❌ FALHOU com código de saída {res.returncode}")
            failed.append(filename)

    total_time = (time.perf_counter() - total_start) * 1000.0
    print("\n" + "=" * 80)
    if failed:
        print(f"❌ {len(failed)} suíte(s) falharam: {', '.join(failed)}")
        print("=" * 80)
        sys.exit(1)
    else:
        print(f"🎉 TODAS AS {len(PYTHON_TEST_SUITES)} SUÍTES EM PYTHON PASSARAM COM 100% DE SUCESSO! ({total_time:.1f} ms)")
        print("=" * 80)
        sys.exit(0)

if __name__ == '__main__':
    run_all()
