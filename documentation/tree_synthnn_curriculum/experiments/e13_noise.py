#!/usr/bin/env python3
"""
Experimento E13: Identificação de Ruído Estocástico (Nível, Tilt Alfa e Joelho).

Objetivos:
1. Rejeitar componentes harmônicos determinísticos através de filtro de percentil espectral.
2. Recuperar com precisão:
    declive espectral alfa in [0.0, 2.2]
    frequência de joelho f_knee in [400, 3000] Hz
    nível de ruído em dB in [-60, -20] dB
3. Satisfazer os critérios de promoção (spec.py):
    PSD_log_RMSE < 1.0 dB
    alpha_abs_error < 0.05
    Isolamento contrafactual I_noise > 10.0 (20 dB)
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

from analysis.noise_effects import AnalyticNoiseEstimator

def sample_noise_scene(rng: np.random.Generator, split: str) -> tuple[np.ndarray, float, float, float]:
    """Gera ruído colorido sob a lei de densidade espectral com joelho, somado a tom harmônico."""
    if split == 'IID':
        alpha = float(rng.uniform(0.5, 1.5))
        fknee = float(rng.uniform(700.0, 2000.0))
        level_db = float(rng.uniform(-40.0, -22.0))
        num_harmonics = 4
    elif split == 'Compositional':
        alpha = float(rng.uniform(1.7, 2.2)) # ruído marrom com decaimento acentuado
        fknee = float(rng.uniform(450.0, 850.0))
        level_db = float(rng.uniform(-35.0, -20.0))
        num_harmonics = 4
    elif split == 'OOD':
        alpha = float(rng.uniform(0.0, 0.25)) # ruído praticamente branco (plano)
        fknee = float(rng.uniform(1200.0, 2800.0))
        level_db = float(rng.uniform(-45.0, -25.0))
        num_harmonics = 4
    else: # Hard: múltiplos parciais determinísticos fortes
        alpha = float(rng.uniform(0.4, 1.8))
        fknee = float(rng.uniform(600.0, 2200.0))
        level_db = float(rng.uniform(-45.0, -25.0))
        num_harmonics = 8

    sr = 12000
    duration = 2.5
    N = int(duration * sr)
    t = np.arange(N) / sr

    # 1. Gerar ruído colorido
    w = rng.normal(0.0, 1.0, size=N)
    fft_w = np.fft.rfft(w)
    freqs = np.fft.rfftfreq(N, d=1.0 / sr)
    freqs[0] = 1.0

    sigma = 10.0 ** (level_db / 20.0)
    psd_filter = (1.0 + (freqs / fknee)**2) ** (-alpha / 4.0)
    noise_signal = np.fft.irfft(fft_w * psd_filter, n=N) * sigma

    # 2. Somar sinal harmônico determinístico
    f0 = 240.0
    tonal = np.zeros(N)
    for k in range(1, num_harmonics + 1):
        if k * f0 < sr * 0.45:
            tonal += (0.4 / k) * np.sin(2.0 * math.pi * k * f0 * t)

    x = tonal + noise_signal
    return x, alpha, fknee, level_db

def run_e13():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E13: IDENTIFICAÇÃO DE RUÍDO ESTOCÁSTICO (PSD, ALFA, JOELHO)")
    print("=========================================================================")

    estimator = AnalyticNoiseEstimator(sr=12000.0)
    rng = np.random.default_rng(42)
    n_val = 50

    results = {}
    all_eval_data = {}

    print("\n1. Avaliando nos 4 splits do quadrante (com rejeição de picos harmônicos)...")
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Componente Estocástico de Ruído):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'Erro Abs. Alfa':<16} | {'PSD Log RMSE (dB)':<20} | {'Erro fknee (%)':<16} | {'Status':<10}")
    print("-" * 85)

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        err_alpha_list, rmse_db_list, err_fk_list = [], [], []
        alpha_true_list, alpha_pred_list = [], []

        for _ in range(n_val):
            x, alpha_t, fknee_t, level_t = sample_noise_scene(rng, s_name)
            alpha_e, fknee_e, level_e, rmse_db, _, _ = estimator.estimate(x)

            err_alpha = abs(alpha_e - alpha_t)
            err_fk = abs(fknee_e - fknee_t) / fknee_t * 100.0

            err_alpha_list.append(err_alpha)
            rmse_db_list.append(rmse_db)
            err_fk_list.append(err_fk)

            alpha_true_list.append(alpha_t)
            alpha_pred_list.append(alpha_e)

        med_alpha = float(np.median(err_alpha_list))
        med_rmse_db = float(np.median(rmse_db_list))
        med_fk = float(np.median(err_fk_list))

        passed = (med_alpha < 0.050) and (med_rmse_db < 1.0)
        results[s_name] = {
            'median_alpha_abs_error': med_alpha,
            'median_psd_log_rmse_db': med_rmse_db,
            'median_fknee_error_pct': med_fk,
            'pass_promotion': passed,
        }
        all_eval_data[s_name] = (np.array(alpha_true_list), np.array(alpha_pred_list))

        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {med_alpha:<16.4f} | {med_rmse_db:<20.4f} | {med_fk:<16.2f} | {status:<10}")

    # 2. Teste de Isolamento Contrafactual (Intervenção pura no nível de ruído)
    print("\n2. Executando teste de isolamento contrafactual I_noise (intervenção no nível)...")
    x_b, alpha_b, fknee_b, level_b = sample_noise_scene(rng, 'IID')
    delta_level_db = 10.0 # +10 dB de ruído
    level_pert = level_b + delta_level_db

    sr = 12000; duration = 2.0; N = int(duration * sr); t = np.arange(N) / sr
    w = rng.normal(0.0, 1.0, size=N)
    fft_w = np.fft.rfft(w)
    freqs = np.fft.rfftfreq(N, d=1.0 / sr); freqs[0] = 1.0
    sigma_p = 10.0 ** (level_pert / 20.0)
    psd_filter = (1.0 + (freqs / fknee_b)**2) ** (-alpha_b / 4.0)
    noise_p = np.fft.irfft(fft_w * psd_filter, n=N) * sigma_p

    tonal = np.zeros(N)
    for k in range(1, 5):
        if k * 240.0 < sr * 0.45:
            tonal += (0.4 / k) * np.sin(2.0 * math.pi * k * 240.0 * t)
    x_p = tonal + noise_p

    alpha_e_b, fknee_e_b, level_e_b, _, _, _ = estimator.estimate(x_b)
    alpha_e_p, fknee_e_p, level_e_p, _, _, _ = estimator.estimate(x_p)

    delta_level_est = abs(level_e_p - level_e_b)
    leak_alpha = abs(alpha_e_p - alpha_e_b)
    leak_fk = abs(fknee_e_p - fknee_e_b) / fknee_b
    leak_tot = leak_alpha + leak_fk

    iso_ratio = delta_level_est / max(leak_tot, 1e-4)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta Nível Real:         {delta_level_db:+.2f} dB")
    print(f"  -> Delta Nível Estimado:     {delta_level_est:+.2f} dB")
    print(f"  -> Vazamento em (alfa, fk):  {leak_tot:.5f}")
    print(f"  -> Métrica de Isolamento:    I_noise = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 3. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    alpha_t_iid, alpha_p_iid = all_eval_data['IID']

    axes[0].scatter(alpha_t_iid, alpha_p_iid, color='royalblue', alpha=0.7, s=35, label='Exemplos IID')
    axes[0].plot([0, 2.5], [0, 2.5], 'k--', label='y = x')
    axes[0].set_xlabel('Declive Alfa Real')
    axes[0].set_ylabel('Alfa Estimado')
    axes[0].set_title('Recuperação de Declive Espectral Alfa (E13)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Exemplo de Curva PSD Ajustada
    _, _, _, _, f_w, psd_db = estimator.estimate(x_b)
    mask = (f_w >= 150.0) & (f_w <= sr * 0.45)
    axes[1].plot(f_w[mask], psd_db[mask], 'gray', alpha=0.5, label='PSD Medida (com Piso)')
    u_plot = np.log10(1.0 + (f_w[mask] / fknee_e_b)**2)
    psd_model_plot = level_e_b - 5.0 * alpha_e_b * u_plot
    axes[1].plot(f_w[mask], psd_model_plot, 'r-', lw=2.0, label=f'Modelo Ajustado (alfa={alpha_e_b:.2f})')
    axes[1].set_xscale('log')
    axes[1].set_xlabel('Frequência (Hz)')
    axes[1].set_ylabel('PSD (dB/Hz)')
    axes[1].set_title('Ajuste do Piso de Ruído Estocástico (E13)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e13_noise_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stage': 'E13',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e13_noise.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e13()
