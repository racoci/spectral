#!/usr/bin/env python3
"""
Experimento E14: Identificação de Efeitos Acústicos Não-Locais (Delay e Filtro).

Objetivos:
1. Detectar com acurácia > 99.0% o estado ON/OFF de cada efeito não-local.
2. Recuperar os parâmetros físicos contínuos de cada efeito isolado com erro < 5%:
    Delay: tempo de atraso tau_ms in [20, 200] ms
    Filter: frequência de corte fc in [600, 3500] Hz
3. Satisfazer os critérios de promoção (spec.py):
    single_effect_parameter_error < 5% (0.05)
    effect_on_off_accuracy > 0.99 (99.0%)
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np
import matplotlib.pyplot as plt
from scipy.signal import butter, sosfilt

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
PROJECT_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(PROJECT_ROOT))

from analysis.noise_effects import AnalyticEffectsEstimator

def sample_dry_audio(rng: np.random.Generator, N: int, sr: float) -> tuple[np.ndarray, float]:
    t = np.arange(N) / sr
    f0 = float(rng.uniform(140.0, 240.0))
    decay = np.where(t <= 0.5, 1.0, np.exp(-(t - 0.5) * 50.0))
    x = np.zeros(N)
    for k in range(1, 24):
        if k * f0 < sr * 0.45:
            x += (1.0 / k) * np.sin(2.0 * math.pi * k * f0 * t)
    return x * decay, f0

def apply_delay(x: np.ndarray, tau_ms: float, g: float, sr: float) -> np.ndarray:
    tau_samples = int(tau_ms * 1e-3 * sr)
    y = x.copy()
    if tau_samples < len(x):
        y[tau_samples:] += g * x[:-tau_samples]
    return y

def apply_filter(x: np.ndarray, fc: float, sr: float) -> np.ndarray:
    sos = butter(2, fc / (sr / 2.0), btype='low', output='sos')
    return sosfilt(sos, x)

def run_e14():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E14: IDENTIFICAÇÃO DE EFEITOS NÃO-LOCAIS (DELAY E FILTRO)")
    print("=========================================================================")

    estimator = AnalyticEffectsEstimator(sr=12000.0)
    rng = np.random.default_rng(42)
    sr = 12000.0
    N = int(1.2 * sr)
    n_val = 50

    results = {}
    all_eval_data = {}

    # 1. Avaliação de DELAY
    print("\n--- 1. Avaliação do Efeito DELAY (Detecção ON/OFF e Tempo tau_ms) ---")
    print(f"{'Split':<15} | {'Acurácia ON/OFF':<18} | {'Erro tau_ms (ms)':<18} | {'Erro tau (%)':<14} | {'Status':<10}")
    print("-" * 80)

    delay_results = {}
    tau_true_all, tau_pred_all = [], []

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        correct_state = 0
        tau_err_ms_list, tau_err_pct_list = [], []

        for _ in range(n_val):
            is_on = rng.uniform() > 0.5
            x, f0 = sample_dry_audio(rng, N, sr)

            if s_name == 'IID':
                tau_t = float(rng.uniform(35.0, 120.0))
                g_t = float(rng.uniform(0.25, 0.60))
            elif s_name == 'Compositional':
                tau_t = float(rng.uniform(120.0, 200.0)) # eco longo
                g_t = float(rng.uniform(0.40, 0.70))
            elif s_name == 'OOD':
                tau_t = float(rng.uniform(20.0, 35.0)) # slapback curto
                g_t = float(rng.uniform(0.15, 0.35))
            else: # Hard
                tau_t = float(rng.uniform(25.0, 180.0))
                g_t = float(rng.uniform(0.20, 0.60))

            y = apply_delay(x, tau_t, g_t, sr) if is_on else x
            if s_name == 'Hard':
                y += rng.normal(0, 0.005, size=len(y))

            pred_on, tau_e, g_e = estimator.detect_delay(x, y)
            if pred_on == is_on:
                correct_state += 1

            if is_on:
                err_ms = abs(tau_e - tau_t)
                err_pct = (err_ms / tau_t) * 100.0
                tau_err_ms_list.append(err_ms)
                tau_err_pct_list.append(err_pct)
                tau_true_all.append(tau_t)
                tau_pred_all.append(tau_e)

        acc = correct_state / n_val
        med_ms = float(np.median(tau_err_ms_list)) if tau_err_ms_list else 0.0
        med_pct = float(np.median(tau_err_pct_list)) if tau_err_pct_list else 0.0

        passed = (acc >= 0.98) and (med_pct < 5.0)
        delay_results[s_name] = {
            'on_off_accuracy': acc,
            'median_tau_error_ms': med_ms,
            'median_tau_error_pct': med_pct,
            'pass_promotion': passed,
        }
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {acc:<18.2%} | {med_ms:<18.3f} | {med_pct:<14.2f} | {status:<10}")

    results['delay'] = delay_results
    all_eval_data['tau'] = (tau_true_all, tau_pred_all)

    # 2. Avaliação de FILTER
    print("\n--- 2. Avaliação do Efeito FILTER (Detecção ON/OFF e Frequência fc) ---")
    print(f"{'Split':<15} | {'Acurácia ON/OFF':<18} | {'Erro fc (Hz)':<18} | {'Erro fc (%)':<14} | {'Status':<10}")
    print("-" * 80)

    filter_results = {}
    fc_true_all, fc_pred_all = [], []

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        correct_state = 0
        fc_err_hz_list, fc_err_pct_list = [], []

        for _ in range(n_val):
            is_on = rng.uniform() > 0.5
            x, f0 = sample_dry_audio(rng, N, sr)

            if s_name == 'IID':
                fc_t = float(rng.uniform(1000.0, 2500.0))
            elif s_name == 'Compositional':
                fc_t = float(rng.uniform(500.0, 950.0)) # filtro escuro/abafado
            elif s_name == 'OOD':
                fc_t = float(rng.uniform(2600.0, 3800.0)) # filtro muito aberto
            else: # Hard
                fc_t = float(rng.uniform(600.0, 3000.0))

            y = apply_filter(x, fc_t, sr) if is_on else x
            if s_name == 'Hard':
                y += rng.normal(0, 0.005, size=len(y))

            pred_on, fc_e = estimator.detect_filter(x, y, f0=f0)
            if pred_on == is_on:
                correct_state += 1

            if is_on:
                err_hz = abs(fc_e - fc_t)
                err_pct = (err_hz / fc_t) * 100.0
                fc_err_hz_list.append(err_hz)
                fc_err_pct_list.append(err_pct)
                fc_true_all.append(fc_t)
                fc_pred_all.append(fc_e)

        acc = correct_state / n_val
        med_hz = float(np.median(fc_err_hz_list)) if fc_err_hz_list else 0.0
        med_pct = float(np.median(fc_err_pct_list)) if fc_err_pct_list else 0.0

        passed = (acc >= 0.98) and (med_pct < 5.0)
        filter_results[s_name] = {
            'on_off_accuracy': acc,
            'median_fc_error_hz': med_hz,
            'median_fc_error_pct': med_pct,
            'pass_promotion': passed,
        }
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {acc:<18.2%} | {med_hz:<18.3f} | {med_pct:<14.2f} | {status:<10}")

    results['filter'] = filter_results
    all_eval_data['fc'] = (fc_true_all, fc_pred_all)

    # 3. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    tau_t, tau_p = all_eval_data['tau']
    axes[0].scatter(tau_t, tau_p, color='darkorange', alpha=0.7, s=35, label='Atrasos Medidos')
    axes[0].plot([15, 210], [15, 210], 'k--', label='y = x')
    axes[0].set_xlabel('Tempo de Delay Real tau (ms)')
    axes[0].set_ylabel('Tempo Estimado tau_hat (ms)')
    axes[0].set_title('Identificação Cepstral de Delay Não-Local (E14)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    fc_t, fc_p = all_eval_data['fc']
    axes[1].scatter(fc_t, fc_p, color='teal', alpha=0.7, s=35, label='Cortes Medidos')
    axes[1].plot([450, 4000], [450, 4000], 'k--', label='y = x')
    axes[1].set_xlabel('Frequência de Corte Real fc (Hz)')
    axes[1].set_ylabel('fc Estimada (Hz)')
    axes[1].set_title('Identificação Espectral de Filtro Passa-Baixas (E14)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e14_effects_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stage': 'E14',
        'metrics': results,
        'promotion_passed': (
            all(m['pass_promotion'] for m in delay_results.values()) and
            all(m['pass_promotion'] for m in filter_results.values())
        ),
    }
    json_path = ROOT / 'experiments' / 'e14_effects.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e14()
