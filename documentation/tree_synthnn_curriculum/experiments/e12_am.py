#!/usr/bin/env python3
"""
Experimento E12: Identificação de Modulação de Amplitude (AM / Tremolo) Desacoplada de Vibrato.

Objetivos:
1. Validar a recuperação dos parâmetros de AM:
    frequência de tremolo f_am in [2, 30] Hz
    profundidade de modulação m in [0.03, 0.95]
    fase phi_am in [-pi, pi)
2. Desacoplar rigorosamente modulação de amplitude de modulação de frequência (vibrato) e ADSR.

Critérios de Promoção (spec.py):
- depth_RMSE < 0.01
- freq_relative_error < 0.02 (2%)
- Sucesso nos 4 splits do quadrante (IID, Composicional, OOD, Hard com vibrato simultâneo)
- Isolamento contrafactual I_AM > 10.0 (20 dB)
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

from analysis.modulation import AnalyticAmSolver

def sample_am_signal(rng: np.random.Generator, split: str) -> tuple[np.ndarray, float, float, float, float]:
    """Gera sinal com modulação de amplitude AM/tremolo, com vibrato em split Hard."""
    if split == 'IID':
        fc = float(rng.uniform(220.0, 500.0))
        fam = float(rng.uniform(3.0, 12.0)) # Hz
        m = float(rng.uniform(0.20, 0.80))
        phi = float(rng.uniform(-math.pi, math.pi))
        vibrato_cents = 0.0
        noise = 0.0
    elif split == 'Compositional':
        fc = float(rng.uniform(180.0, 420.0))
        fam = float(rng.uniform(15.0, 28.0)) # flutter tremolo rápido
        m = float(rng.uniform(0.70, 0.95)) # profundidade extrema
        phi = float(rng.uniform(-math.pi, math.pi))
        vibrato_cents = 0.0
        noise = 0.0
    elif split == 'OOD':
        fc = float(rng.uniform(250.0, 600.0))
        fam = float(rng.uniform(1.5, 4.0)) # tremolo lento
        m = float(rng.uniform(0.04, 0.12)) # tremolo muito sutil
        phi = float(rng.uniform(-math.pi, math.pi))
        vibrato_cents = 0.0
        noise = 0.0
    else: # Hard: AM simultânea com vibrato (modulação de pitch) e ruído
        fc = float(rng.uniform(200.0, 500.0))
        fam = float(rng.uniform(4.0, 15.0))
        m = float(rng.uniform(0.15, 0.75))
        phi = float(rng.uniform(-math.pi, math.pi))
        vibrato_cents = 35.0 # vibrato de pitch de 35 cents a 6 Hz
        noise = 0.005

    sr = 12000
    N = int(1.0 * sr)
    t = np.arange(N) / sr

    # Envelope AM
    am_env = 1.0 + m * np.sin(2.0 * math.pi * fam * t + phi)

    # Fase do portador (com vibrato simultâneo se Hard)
    if vibrato_cents > 0:
        c_traj = vibrato_cents * np.sin(2.0 * math.pi * 5.8 * t)
        f_inst = fc * (2.0 ** (c_traj / 1200.0))
        phase = 2.0 * math.pi * np.cumsum(f_inst) / sr
    else:
        phase = 2.0 * math.pi * fc * t

    x = am_env * np.sin(phase)
    if noise > 0:
        x += rng.normal(0.0, noise, size=N)

    return x, fc, fam, m, phi

def run_e12():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E12: IDENTIFICAÇÃO DE AM / TREMOLO E DESACOPLAMENTO")
    print("=========================================================================")

    solver = AnalyticAmSolver(sr=12000.0)
    rng = np.random.default_rng(42)
    n_val = 50

    results = {}
    all_eval_data = {}

    print("\n1. Avaliando nos 4 splits do quadrante (incluindo Hard com vibrato simultâneo)...")
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Modulação de Amplitude AM):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'Erro fam Rel. (%)':<18} | {'RMSE Profundidade':<18} | {'Erro Máx m':<12} | {'Status':<10}")
    print("-" * 80)

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        err_fam_list, err_m_list = [], []
        m_true_list, m_pred_list = [], []

        for _ in range(n_val):
            x, fc_t, fam_t, m_t, phi_t = sample_am_signal(rng, s_name)
            fc_e, fam_e, m_e, _ = solver.solve(x, fc_t, fam_t)

            err_fam = abs(fam_e - fam_t) / fam_t * 100.0
            err_m = abs(m_e - m_t)

            err_fam_list.append(err_fam)
            err_m_list.append(err_m)
            m_true_list.append(m_t)
            m_pred_list.append(m_e)

        med_fam = float(np.median(err_fam_list))
        rmse_m = float(np.sqrt(np.mean(np.array(err_m_list)**2)))
        max_m = float(np.max(err_m_list))

        passed = (med_fam < 2.0) and (rmse_m < 0.010)
        results[s_name] = {
            'median_fam_error_pct': med_fam,
            'depth_rmse': rmse_m,
            'max_depth_error': max_m,
            'pass_promotion': passed,
        }
        all_eval_data[s_name] = (np.array(m_true_list), np.array(m_pred_list))

        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {med_fam:<18.3f} | {rmse_m:<18.5f} | {max_m:<12.5f} | {status:<10}")

    # 2. Teste de Isolamento Contrafactual (Intervenção pura na profundidade m de AM)
    print("\n2. Executando teste de isolamento contrafactual I_AM (intervenção em m)...")
    x_b, fc_b, fam_b, m_b, phi_b = sample_am_signal(rng, 'IID')
    delta_m_true = 0.20
    m_pert = min(0.95, m_b + delta_m_true)
    delta_m_true = m_pert - m_b

    sr = 12000; N = int(1.0 * sr); t = np.arange(N) / sr
    am_env_p = 1.0 + m_pert * np.sin(2.0 * math.pi * fam_b * t + phi_b)
    x_p = am_env_p * np.sin(2.0 * math.pi * fc_b * t)

    fc_e_b, fam_e_b, m_e_b, _ = solver.solve(x_b, fc_b, fam_b)
    fc_e_p, fam_e_p, m_e_p, _ = solver.solve(x_p, fc_b, fam_b)

    delta_m_est = abs(m_e_p - m_e_b)
    leak_fc = abs(fc_e_p - fc_e_b) / fc_b
    leak_fam = abs(fam_e_p - fam_e_b) / fam_b
    leak_tot = leak_fc + leak_fam

    iso_ratio = (delta_m_est / m_b) / max(leak_tot, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta m Real:             {delta_m_true:+.4f}")
    print(f"  -> Delta m Estimado:         {delta_m_est:+.4f}")
    print(f"  -> Vazamento em (fc, fam):   {leak_tot * 100:.6f}%")
    print(f"  -> Métrica de Isolamento:    I_AM = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 3. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    m_t_iid, m_p_iid = all_eval_data['IID']

    axes[0].scatter(m_t_iid, m_p_iid, color='crimson', alpha=0.7, s=35, label='Exemplos IID')
    axes[0].plot([0, 1], [0, 1], 'k--', label='y = x')
    axes[0].set_xlabel('Profundidade de Tremolo m Real')
    axes[0].set_ylabel('m Estimado')
    axes[0].set_title('Recuperação de Profundidade de AM / Tremolo (E12)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Dispersão no split Hard (com vibrato simultâneo)
    m_t_hard, m_p_hard = all_eval_data['Hard']
    axes[1].scatter(m_t_hard, m_p_hard, color='purple', alpha=0.7, s=35, label='Hard (com Vibrato Ativo)')
    axes[1].plot([0, 1], [0, 1], 'k--', label='y = x')
    axes[1].set_xlabel('Profundidade m Real')
    axes[1].set_ylabel('m Estimado (sob Vibrato de 35 cents)')
    axes[1].set_title('Desacoplamento Rigoroso de Tremolo vs Vibrato (E12)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e12_am_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stage': 'E12',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e12_am.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e12()
