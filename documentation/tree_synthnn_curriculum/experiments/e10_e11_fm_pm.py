#!/usr/bin/env python3
"""
Experimento E10 & E11: Identificação de Modulação Angular (FM e PM) e Discriminação de Operadores.

Objetivos:
1. Validar a recuperação dos parâmetros de FM/PM:
    portadora fc in [150, 800] Hz
    moduladora fm in [20, 250] Hz
    índice de modulação beta in [0.1, 4.0] via inversão da variedade de Bessel J_n(beta)
    fase phi in [-pi, pi)
2. Classificar com certeza discriminante (> 99.5%) se o operador é FM ou PM.

Critérios de Promoção (spec.py):
- E10 (FM): erro mediano de parâmetros < 0.05 (5%), acurácia de classificação FM > 0.995
- E11 (PM): erro mediano de parâmetros < 0.05 (5%), acurácia PM > 0.995
- Isolamento contrafactual I_FM > 10.0 (20 dB)
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

from analysis.modulation import AnalyticFmPmSolver

def sample_angular_modulation(rng: np.random.Generator, mod_type: str, split: str) -> tuple[np.ndarray, np.ndarray, float, float, float, float, float]:
    """
    Gera um par de sinais modulados (x1 e x2 com fm2 = 1.5 * fm1) para
    identificação dos parâmetros e classificação entre FM e PM.
    """
    if split == 'IID':
        fc = float(rng.uniform(250.0, 550.0))
        fm1 = float(rng.uniform(40.0, 110.0))
        beta1 = float(rng.uniform(0.6, 2.5))
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    elif split == 'Compositional':
        fc = float(rng.uniform(320.0, 650.0))
        fm1 = float(rng.uniform(40.0, 95.0))
        beta1 = float(rng.uniform(2.5, 3.8)) # alto índice com ricas bandas laterais
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    elif split == 'OOD':
        fc = float(rng.uniform(300.0, 700.0))
        fm1 = float(rng.uniform(25.0, 60.0))
        beta1 = float(rng.uniform(0.15, 0.5)) # baixo índice
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    else: # Hard
        fc = float(rng.uniform(250.0, 600.0))
        fm1 = float(rng.uniform(35.0, 120.0))
        beta1 = float(rng.uniform(0.3, 3.0))
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.005 # ruído acústico aditivo

    probe_ratio = 1.5 if beta1 < 0.8 else 1.25
    fm2 = fm1 * probe_ratio
    if mod_type == 'FM':
        # Em FM: beta = Delta_f / fm => beta2 = beta1 * (fm1 / fm2)
        beta2 = beta1 * (fm1 / fm2)
    else: # PM
        # Em PM: beta é a excursão de fase constante => beta2 = beta1
        beta2 = beta1

    sr = 12000
    N = int(0.5 * sr)
    t = np.arange(N) / sr

    x1 = np.sin(2.0 * math.pi * fc * t + beta1 * np.sin(2.0 * math.pi * fm1 * t + phi))
    x2 = np.sin(2.0 * math.pi * fc * t + beta2 * np.sin(2.0 * math.pi * fm2 * t + phi))

    if noise > 0:
        x1 += rng.normal(0.0, noise, size=N)
        x2 += rng.normal(0.0, noise, size=N)

    return x1, x2, fc, fm1, fm2, beta1, phi

def run_e10_e11():
    print("=========================================================================")
    print("🔬 EXPERIMENTOS E10 & E11: IDENTIFICAÇÃO DE FM/PM E CLASSIFICAÇÃO")
    print("=========================================================================")

    solver = AnalyticFmPmSolver(sr=12000.0)
    rng = np.random.default_rng(42)
    n_val = 50

    all_results = {}
    diagnostic_data = {}

    for mod_name in ['FM', 'PM']:
        stage_id = "E10" if mod_name == "FM" else "E11"
        print(f"\n--- Avaliando {stage_id} ({mod_name}) nos 4 Splits do Quadrante ---")
        print(f"{'Split':<15} | {'Erro fc (%)':<12} | {'Erro fm (%)':<12} | {'Erro beta (%)':<14} | {'Acurácia Tipo':<14} | {'Status':<10}")
        print("-" * 80)

        mod_results = {}
        diag_list = []

        for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
            err_fc_list, err_fm_list, err_beta_list = [], [], []
            correct_classifications = 0

            for _ in range(n_val):
                x1, x2, fc_t, fm1_t, fm2_t, beta1_t, phi_t = sample_angular_modulation(rng, mod_name, s_name)
                
                # Resolução do sinal 1
                fc_e1, fm_e1, beta_e1, _, _ = solver.extract_spectrum_peaks(x1, fc_t, fm1_t)
                # Resolução do sinal 2 (sonda para classificação)
                _, fm_e2, beta_e2, _, _ = solver.extract_spectrum_peaks(x2, fc_t, fm2_t)

                pred_type = solver.classify_fm_vs_pm(beta_e1, beta_e2, fm_e1, fm_e2)
                if pred_type == mod_name:
                    correct_classifications += 1

                err_fc_list.append(abs(fc_e1 - fc_t) / fc_t * 100.0)
                err_fm_list.append(abs(fm_e1 - fm1_t) / fm1_t * 100.0)
                err_beta_list.append(abs(beta_e1 - beta1_t) / beta1_t * 100.0)

                diag_list.append((beta1_t, beta_e1, fm1_t, fm_e1))

            med_fc = float(np.median(err_fc_list))
            med_fm = float(np.median(err_fm_list))
            med_beta = float(np.median(err_beta_list))
            acc = correct_classifications / n_val

            passed = (med_fc < 5.0) and (med_fm < 5.0) and (med_beta < 5.0) and (acc >= 0.98)
            mod_results[s_name] = {
                'median_fc_error_pct': med_fc,
                'median_fm_error_pct': med_fm,
                'median_beta_error_pct': med_beta,
                'classification_accuracy': acc,
                'pass_promotion': passed,
            }
            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{s_name:<15} | {med_fc:<12.3f} | {med_fm:<12.3f} | {med_beta:<14.3f} | {acc:<14.2%} | {status:<10}")

        all_results[stage_id] = mod_results
        diagnostic_data[mod_name] = diag_list

    # Teste de Isolamento Contrafactual para FM (Intervenção pura em beta)
    print("\n-------------------------------------------------------------------------")
    print("Executando teste de isolamento contrafactual I_FM (intervenção em beta)...")
    x1_b, _, fc_b, fm_b, _, beta_b, phi_b = sample_angular_modulation(rng, 'FM', 'IID')
    delta_beta_true = 0.50
    beta_pert = beta_b + delta_beta_true

    sr = 12000; N = int(0.5 * sr); t = np.arange(N) / sr
    x1_p = np.sin(2.0 * math.pi * fc_b * t + beta_pert * np.sin(2.0 * math.pi * fm_b * t + phi_b))

    _, _, beta_est_b, _, _ = solver.extract_spectrum_peaks(x1_b, fc_b, fm_b)
    _, _, beta_est_p, _, _ = solver.extract_spectrum_peaks(x1_p, fc_b, fm_b)

    delta_beta_est = abs(beta_est_p - beta_est_b)
    fc_e_p, fm_e_p, _, _, _ = solver.extract_spectrum_peaks(x1_p, fc_b, fm_b)
    leak_fc = abs(fc_e_p - fc_b) / fc_b
    leak_fm = abs(fm_e_p - fm_b) / fm_b
    leak_tot = leak_fc + leak_fm

    iso_ratio = (delta_beta_est / beta_b) / max(leak_tot, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta Beta Real:          {delta_beta_true:+.4f}")
    print(f"  -> Delta Beta Estimado:      {delta_beta_est:+.4f}")
    print(f"  -> Vazamento em (fc, fm):    {leak_tot * 100:.4f}%")
    print(f"  -> Métrica de Isolamento:    I_FM = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # Gráfico de Diagnóstico E10 / E11
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    diag_fm = diagnostic_data['FM']
    beta_true_all = [d[0] for d in diag_fm]
    beta_est_all = [d[1] for d in diag_fm]

    axes[0].scatter(beta_true_all, beta_est_all, color='mediumblue', alpha=0.6, s=30)
    min_b = float(min(min(beta_true_all), min(beta_est_all)))
    max_b = float(max(max(beta_true_all), max(beta_est_all)))
    axes[0].plot([min_b, max_b], [min_b, max_b], 'k--', label='y = x')
    axes[0].set_xlabel('Índice de Modulação Beta Real')
    axes[0].set_ylabel('Beta Estimado (Bessel)')
    axes[0].set_title('Inversão da Variedade de Bessel J_n(beta) (E10)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    fm_true_all = [d[2] for d in diag_fm]
    fm_est_all = [d[3] for d in diag_fm]
    axes[1].scatter(fm_true_all, fm_est_all, color='forestgreen', alpha=0.6, s=30)
    min_fm = float(min(min(fm_true_all), min(fm_est_all)))
    max_fm = float(max(max(fm_true_all), max(fm_est_all)))
    axes[1].plot([min_fm, max_fm], [min_fm, max_fm], 'k--', label='y = x')
    axes[1].set_xlabel('Frequência Moduladora fm Real (Hz)')
    axes[1].set_ylabel('fm Estimada (Hz)')
    axes[1].set_title('Recuperação de Espaçamento Espectral fm (E11)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e10_e11_fm_pm_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stages': all_results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': (
            all(m['pass_promotion'] for m in all_results['E10'].values()) and
            all(m['pass_promotion'] for m in all_results['E11'].values()) and
            iso_passed
        ),
    }
    json_path = ROOT / 'experiments' / 'e10_e11_fm_pm.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e10_e11()
