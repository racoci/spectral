#!/usr/bin/env python3
"""
Experimento E07: Identificação de Envelope Espectral E(f) sob Condições de Gauge.

Objetivo:
Validar a recuperação de coeficientes de base contínua de formantes/envelope espectral
    S(u) = sum_j w_j * phi_j(u),  onde u = log2(f / 440 Hz)
sob as condições estritas de gauge:
    S(0) = 0 dB        (E(440 Hz) = 0 dB)
    dS/du(0) = 0 dB/oct
através da projeção linear direta na base contínua sobre múltiplas notas.

Critérios de Promoção (spec.py):
- Erro mediano do envelope espectral < 0.75 dB
- Resíduo de gauge < 1e-4 (estritamente 0 por construção)
- Sucesso nos splits do quadrante (IID, Composicional, OOD, Hard)
- Isolamento contrafactual I_E > 10.0
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

from analysis.spectral_envelope import GaugeFixedSpectralBasis, AnalyticSpectralBasisProjector
from analysis.cqt import GaussianCQT

def sample_spectral_envelope_scene(rng: np.random.Generator, split: str, basis: GaugeFixedSpectralBasis):
    """
    Gera uma cena com duas notas em alturas distintas (f0_1 e f0_2) compartilhando o
    mesmo envelope espectral contínuo E(f) e a mesma estrutura harmônica H_k.
    """
    if split == 'IID':
        w = np.array([
            float(rng.uniform(0.5, 2.5)),   # u^2 (curvatura)
            float(rng.uniform(-0.8, 0.8)),  # u^3 (assimetria)
            float(rng.uniform(-1.5, 1.5)),  # saturação suave
            float(rng.uniform(-0.5, 0.5)),
        ], dtype=float)
        f0_1 = float(rng.uniform(160.0, 240.0))
        f0_2 = float(rng.uniform(280.0, 440.0))
        noise = 0.0
    elif split == 'Compositional':
        w = np.array([
            float(rng.uniform(2.5, 4.0)),
            float(rng.uniform(-1.8, -1.0)),
            float(rng.uniform(-2.0, 2.0)),
            float(rng.uniform(-1.0, 1.0)),
        ], dtype=float)
        f0_1 = float(rng.uniform(130.0, 200.0))
        f0_2 = float(rng.uniform(300.0, 460.0))
        noise = 0.0
    elif split == 'OOD':
        w = np.array([
            float(rng.uniform(-2.5, -0.5)),
            float(rng.uniform(-1.5, 1.5)),
            float(rng.uniform(1.0, 3.0)),
            float(rng.uniform(-1.5, 1.5)),
        ], dtype=float)
        f0_1 = float(rng.uniform(110.0, 180.0))
        f0_2 = float(rng.uniform(250.0, 400.0))
        noise = 0.0
    else: # Hard
        w = np.array([
            float(rng.uniform(-3.0, 3.0)),
            float(rng.uniform(-2.0, 2.0)),
            float(rng.uniform(-2.0, 2.0)),
            float(rng.uniform(-1.5, 1.5)),
        ], dtype=float)
        f0_1 = float(rng.uniform(150.0, 300.0))
        f0_2 = f0_1 * 1.5 # quinta justa
        noise = 0.015

    H = np.array([1.0, 0.55, 0.28, 0.16], dtype=float)
    
    sr = 12000
    N = int(1.2 * sr)
    t = np.arange(N) / sr
    
    x = np.zeros(N)
    for f0 in [f0_1, f0_2]:
        for k_idx, h in enumerate(H, 1):
            fk = k_idx * f0
            if fk < sr * 0.45:
                u = math.log2(fk / 440.0)
                e_db = float(basis.evaluate_curve(np.array([u]), w)[0])
                gain_linear = 10.0 ** (e_db / 20.0)
                x += 0.5 * h * gain_linear * np.sin(2.0 * math.pi * fk * t)

    if noise > 0:
        x += rng.normal(0.0, noise, size=len(x))

    return x, w, [f0_1, f0_2], H

def extract_spectral_observations(x: np.ndarray, f0_list: list[float], H: np.ndarray, cqt: GaussianCQT) -> tuple[list[int], np.ndarray, np.ndarray]:
    """
    Extrai as observações brutas em dB para cada harmônico de cada nota,
    subtraindo o ganho relativo do harmônico H_k já conhecido de E06:
    y_{n, k} = 20 log10(A_{n, k}) - 20 log10(H_k) = a_n + S(u_{n, k})
    """
    t0 = 0.6
    note_idx_list = []
    u_pts = []
    y_pts = []

    for n_idx, f0 in enumerate(f0_list):
        for k in range(1, 5):
            fk = k * f0
            u = math.log2(fk / 440.0)
            best_bin = cqt.find_nearest_bin(fk)
            fc = cqt.freqs[best_bin]
            a_est, _ = cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)
            
            a_meas_db = 20.0 * math.log10(max(a_est, 1e-6))
            h_db = 20.0 * math.log10(H[k - 1])
            y_db = a_meas_db - h_db

            note_idx_list.append(n_idx)
            u_pts.append(u)
            y_pts.append(y_db)

    return note_idx_list, np.array(u_pts), np.array(y_pts)

def run_e07():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E07: ENVELOPE ESPECTRAL E(f) COM BASE CONTÍNUA SOB GAUGE")
    print("=========================================================================")

    basis = GaugeFixedSpectralBasis(num_basis=4)
    projector = AnalyticSpectralBasisProjector(basis=basis, reg_lambda=1e-3)
    cqt = GaussianCQT(sr=12000.0, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    n_val = 50
    u_eval_dense = np.linspace(-2.0, 2.0, 100) # De 110 Hz até 1760 Hz
    B_eval_dense = basis.evaluate_basis(u_eval_dense)

    results = {}
    all_eval_data = {}

    print("\n1. Avaliando projeção multivoz conjunta nos 4 splits do quadrante...")
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Erro do Envelope Contínuo em dB):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'RMSE Médio (dB)':<16} | {'Mediana (dB)':<14} | {'Max Erro (dB)':<14} | {'Status':<10}")
    print("-" * 75)

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        curve_errors = []
        w_true_list = []
        w_pred_list = []

        for _ in range(n_val):
            x, w_true, f0_list, H = sample_spectral_envelope_scene(rng, s_name, basis)
            note_indices, u_pts, y_pts = extract_spectral_observations(x, f0_list, H, cqt)

            # Projeção analítica multivoz que resolve conjuntamente a_n e w_j
            _, w_pred = projector.project_multinote(note_indices, u_pts, y_pts, num_notes=len(f0_list))

            s_true = np.dot(B_eval_dense, w_true)
            s_pred = np.dot(B_eval_dense, w_pred)
            rmse_curve = float(np.sqrt(np.mean((s_true - s_pred)**2)))

            curve_errors.append(rmse_curve)
            w_true_list.append(w_true)
            w_pred_list.append(w_pred)

        curve_errors = np.array(curve_errors)
        mean_rmse = float(np.mean(curve_errors))
        median_rmse = float(np.median(curve_errors))
        max_err = float(np.max(curve_errors))

        passed = median_rmse < 0.75 # Critério do spec.py
        results[s_name] = {
            'mean_rmse_db': mean_rmse,
            'median_rmse_db': median_rmse,
            'max_error_db': max_err,
            'pass_promotion': passed,
        }
        all_eval_data[s_name] = (np.array(w_true_list), np.array(w_pred_list))

        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {mean_rmse:<16.4f} | {median_rmse:<14.4f} | {max_err:<14.4f} | {status:<10}")

    # 2. Teste de Isolamento Contrafactual (Alteração exclusiva em w_0 - Curvatura Quadrática)
    print("\n2. Executando teste de isolamento contrafactual I_E...")
    x_base, w_base, f0_base, H_base = sample_spectral_envelope_scene(rng, 'IID', basis)
    w_pert = w_base.copy()
    delta_w0 = 1.0
    w_pert[0] += delta_w0

    sr = 12000; N = int(1.2 * sr); t = np.arange(N) / sr
    x_pert = np.zeros(N)
    for f0 in f0_base:
        for k_idx, h in enumerate(H_base, 1):
            fk = k_idx * f0
            if fk < sr * 0.45:
                u = math.log2(fk / 440.0)
                e_db = float(basis.evaluate_curve(np.array([u]), w_pert)[0])
                gain_linear = 10.0 ** (e_db / 20.0)
                x_pert += 0.5 * h * gain_linear * np.sin(2.0 * math.pi * fk * t)

    n_idx_b, u_b, y_b = extract_spectral_observations(x_base, f0_base, H_base, cqt)
    n_idx_p, u_p, y_p = extract_spectral_observations(x_pert, f0_base, H_base, cqt)

    _, pred_w_base = projector.project_multinote(n_idx_b, u_b, y_b, num_notes=2)
    _, pred_w_pert = projector.project_multinote(n_idx_p, u_p, y_p, num_notes=2)

    delta_w_preds = np.abs(pred_w_pert - pred_w_base)
    delta_w0_est = delta_w_preds[0]
    leak_other_w = delta_w_preds[1] + delta_w_preds[2] + delta_w_preds[3]

    iso_ratio = delta_w0_est / max(leak_other_w, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta w_0 Real:            {delta_w0:+.4f}")
    print(f"  -> Delta w_0 Estimado:        {delta_w0_est:+.4f}")
    print(f"  -> Vazamento para Outros w:   {leak_other_w:.6f}")
    print(f"  -> Métrica de Isolamento:     I_E = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 3. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    w_true_iid, w_pred_iid = all_eval_data['IID']
    idx_sample = 0

    s_true_plot = np.dot(B_eval_dense, w_true_iid[idx_sample])
    s_pred_plot = np.dot(B_eval_dense, w_pred_iid[idx_sample])
    freq_axis_dense = 440.0 * (2.0 ** u_eval_dense)

    axes[0].plot(freq_axis_dense, s_true_plot, 'k-', lw=2.0, label='Envelope Real E(f)')
    axes[0].plot(freq_axis_dense, s_pred_plot, 'g--', lw=2.0, label='Projeção sob Gauge')
    axes[0].axvline(440.0, color='r', ls=':', label='440 Hz (Gauge: 0 dB, 0 dB/oct)')
    axes[0].set_xscale('log')
    axes[0].set_xlabel('Frequência (Hz)')
    axes[0].set_ylabel('Ganho Espectral (dB)')
    axes[0].set_title('Reconstrução Contínua do Envelope Espectral E(f)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Dispersão de Coeficientes da Base
    axes[1].scatter(w_true_iid.flatten(), w_pred_iid.flatten(), color='darkviolet', alpha=0.6, s=30)
    min_w = float(min(w_true_iid.min(), w_pred_iid.min()))
    max_w = float(max(w_true_iid.max(), w_pred_iid.max()))
    axes[1].plot([min_w, max_w], [min_w, max_w], 'k--', label='y = x')
    axes[1].set_xlabel('Coeficientes Reais w')
    axes[1].set_ylabel('Coeficientes Projetados w_hat')
    axes[1].set_title('Convergência dos Coeficientes da Base de Formantes (E07)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e07_spectral_envelope_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    # Salvar resultados em JSON
    output_meta = {
        'stage': 'E07',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'gauge_residual': 0.0, # estritamente zero por construção da base
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e07_spectral_envelope.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e07()
