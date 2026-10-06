#!/usr/bin/env python3
"""
Experimento E04: Identificação de Amplitude Estacionária (A).

Objetivo:
Validar que a amplitude estacionária A de uma componente senoidal pura pode ser
recuperada de forma analítica exata através da CQT gaussiana de 60 bins/oitava calibrada:
    20 log10 |C(t, f0)| ~ 20 log10 A + const
com correção analítica de off-bin exp(+0.5 * (log2(f_inst/fc) / sigma_oct)^2).

Critérios de Promoção (spec.py):
- RMSE < 0.1 dB em escala logarítmica
- Isolamento contrafactual I_A > 20 dB (I_A > 10.0)
- Sucesso nos 4 splits do quadrante (IID, Composicional, OOD, Hard)
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
PROJECT_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(PROJECT_ROOT))

from synthnn import SynthNN, SynthConfig
from datasets.generator import make_stage_config, sample_log_uniform, base_voice
from analysis.cqt import GaussianCQT

def run_e04_experiment(num_trials_per_split: int = 50):
    print("=========================================================================")
    print("🔬 EXPERIMENTO E04: IDENTIFICAÇÃO DE AMPLITUDE ESTACIONÁRIA (A)")
    print("=========================================================================")

    cqt = GaussianCQT(sr=12000.0, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    splits = {
        'IID': [],
        'Compositional': [],
        'OOD': [],
        'Hard': [],
    }

    print("\n1. Gerando amostras dos 4 quadrantes de teste...")
    # IID: A in [0.05, 1.0], midi in [45, 75]
    for i in range(num_trials_per_split):
        a_true = float(sample_log_uniform(rng, 0.05, 1.0))
        midi = float(rng.uniform(45, 75))
        splits['IID'].append((a_true, midi, 'IID'))

    # Composicional: A muito baixo (0.02 a 0.05) com agudos (midi 75 a 84)
    for i in range(num_trials_per_split):
        a_true = float(sample_log_uniform(rng, 0.02, 0.05))
        midi = float(rng.uniform(75, 84))
        splits['Compositional'].append((a_true, midi, 'Compositional'))

    # OOD: Faixa dinâmica extrema A in [0.005, 0.02] (> 46 dB de atenuação)
    for i in range(num_trials_per_split):
        a_true = float(sample_log_uniform(rng, 0.005, 0.02))
        midi = float(rng.uniform(40, 80))
        splits['OOD'].append((a_true, midi, 'OOD'))

    # Hard: Frequência exatamente no meio entre dois filtros CQT (máximo erro off-bin)
    for i in range(num_trials_per_split):
        a_true = float(sample_log_uniform(rng, 0.05, 0.9))
        bin_idx = int(rng.integers(30, cqt.num_channels - 30))
        f_half = math.sqrt(cqt.freqs[bin_idx] * cqt.freqs[bin_idx + 1])
        midi = 69.0 + 12.0 * math.log2(f_half / 440.0)
        splits['Hard'].append((a_true, midi, 'Hard'))

    results = {}
    db_errors_all = {}

    for split_name, items in splits.items():
        db_errs = []
        rel_errs = []

        for a_true, midi, _ in items:
            f0_nom = 440.0 * (2.0 ** ((midi - 69.0) / 12.0))
            sr = 12000
            duration = 1.0
            N = int(duration * sr)
            t = np.arange(N) / sr

            # Síntese direta do oscilador
            x = a_true * np.sin(2.0 * math.pi * f0_nom * t)

            # Análise analítica da CQT no centro temporal t0 = 0.5s
            t0 = 0.5
            best_bin = cqt.find_nearest_bin(f0_nom)
            fc = cqt.freqs[best_bin]
            a_est, f_inst_est = cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)

            db_err = 20.0 * math.log10(max(a_est, 1e-9) / a_true)
            rel_err = abs(a_est - a_true) / a_true

            db_errs.append(db_err)
            rel_errs.append(rel_err)

        db_errs = np.array(db_errs)
        rel_errs = np.array(rel_errs)
        rmse_db = float(np.sqrt(np.mean(db_errs**2)))
        mae_db = float(np.mean(np.abs(db_errs)))
        max_db = float(np.max(np.abs(db_errs)))
        mean_rel = float(np.mean(rel_errs))

        db_errors_all[split_name] = db_errs
        results[split_name] = {
            'rmse_db': rmse_db,
            'mae_db': mae_db,
            'max_db': max_db,
            'mean_rel_error_percent': mean_rel * 100.0,
            'pass_promotion': rmse_db < 0.10,
        }

    # 2. Teste de Isolamento Contrafactual
    print("\n2. Executando teste de isolamento contrafactual I_A...")
    # Par (x, x') onde apenas a amplitude muda: A' = A + Delta_A
    a_base = 0.4
    delta_a = 0.1
    f_test = 440.0
    t = np.arange(12000) / 12000.0

    x_orig = a_base * np.sin(2.0 * math.pi * f_test * t)
    x_pert = (a_base + delta_a) * np.sin(2.0 * math.pi * f_test * t)

    best_bin = cqt.find_nearest_bin(f_test)
    fc = cqt.freqs[best_bin]
    a_hat_orig, f_hat_orig = cqt.extract_instantaneous_amplitude_and_freq(x_orig, 0.5, fc)
    a_hat_pert, f_hat_pert = cqt.extract_instantaneous_amplitude_and_freq(x_pert, 0.5, fc)

    delta_a_hat = a_hat_pert - a_hat_orig
    delta_f_hat_cents = 1200.0 * abs(math.log2(max(f_hat_pert, 1e-6) / max(f_hat_orig, 1e-6)))

    # Métrica de isolamento I_A = (|delta_A_hat| / A) / (|delta_f_hat| / f + eps)
    iso_metric = (abs(delta_a_hat) / a_base) / ((delta_f_hat_cents / 1200.0) + 1e-7)
    iso_metric_db = 20.0 * math.log10(max(iso_metric, 1e-4))

    print(f"  -> Delta A Real:           {delta_a:.4f}")
    print(f"  -> Delta A Estimado:       {delta_a_hat:.4f}")
    print(f"  -> Vazamento para Pitch:   {delta_f_hat_cents:.6f} cents")
    print(f"  -> Métrica de Isolamento:  I_A = {iso_metric:.1f} ({iso_metric_db:.1f} dB)")
    isolation_passed = iso_metric_db > 20.0

    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE DE VALIDAÇÃO (E04 - Amplitude):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'RMSE (dB)':<12} | {'MAE (dB)':<12} | {'Max (dB)':<12} | {'Erro Rel. (%)':<15} | {'Status':<10}")
    print("-" * 75)
    for name, m in results.items():
        status = "✅ PASS" if m['pass_promotion'] else "❌ FAIL"
        print(f"{name:<15} | {m['rmse_db']:<12.4f} | {m['mae_db']:<12.4f} | {m['max_db']:<12.4f} | {m['mean_rel_error_percent']:<15.3f} | {status:<10}")

    # 3. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    # Boxplot de Erros em dB
    axes[0].boxplot([db_errors_all[k] for k in splits.keys()], labels=list(splits.keys()))
    axes[0].axhline(0.1, color='r', ls='--', label='Limiar de Promoção (+0.1 dB)')
    axes[0].axhline(-0.1, color='r', ls='--', label='Limiar de Promoção (-0.1 dB)')
    axes[0].set_ylabel('Erro de Amplitude (dB)')
    axes[0].set_title('Distribuição de Erros de Amplitude por Split (E04)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Dispersão Real vs Estimado
    all_true_A = []
    all_est_A = []
    for split_items in splits.values():
        for a_true, midi, _ in split_items:
            f0_nom = 440.0 * (2.0 ** ((midi - 69.0) / 12.0))
            best_bin = cqt.find_nearest_bin(f0_nom)
            fc = cqt.freqs[best_bin]
            x = a_true * np.sin(2.0 * math.pi * f0_nom * (np.arange(12000)/12000.0))
            a_est, _ = cqt.extract_instantaneous_amplitude_and_freq(x, 0.5, fc)
            all_true_A.append(a_true)
            all_est_A.append(a_est)

    axes[1].loglog(all_true_A, all_est_A, 'b.', alpha=0.6, label='Estimativas CQT')
    axes[1].loglog([0.005, 1.0], [0.005, 1.0], 'r--', label='Identidade Perfeita (y = x)')
    axes[1].set_xlabel('Amplitude Real (A_true)')
    axes[1].set_ylabel('Amplitude Estimada (A_est)')
    axes[1].set_title('Calibração Dinâmica de Amplitude (40 dB de Faixa)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e04_amplitude_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    # Salvar métricas em JSON
    output_meta = {
        'stage': 'E04',
        'metrics': results,
        'isolation_metric_ratio': iso_metric,
        'isolation_metric_db': iso_metric_db,
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and isolation_passed,
    }
    json_file = ROOT / 'experiments' / 'e04_amplitude.json'
    with open(json_file, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_file}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e04_experiment()
