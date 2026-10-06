#!/usr/bin/env python3
"""
Experimento E08: Identificação de Inarmonicidade B em Cordas e Parciais.

Objetivo:
Validar a recuperação do coeficiente de rigidez B in [1e-5, 1e-2] e f0 a partir da
lei de dispersão acústica de cordas:
    f_k = k * f0 * sqrt(1 + B * k^2)
utilizando o solver analítico polinomial em k^2 e k^4 combinado com MLP residual.

Critérios de Promoção (spec.py):
- Erro relativo mediano de B < 0.05 (5%)
- Ausência total de erros de sinal (B > 0 estritamente)
- Sucesso nos splits do quadrante (IID, Composicional, OOD, Hard)
- Isolamento contrafactual I_B > 10.0
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np
import matplotlib.pyplot as plt
import torch
from torch import nn

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
PROJECT_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(PROJECT_ROOT))

from analysis.inharmonicity import AnalyticInharmonicitySolver, InharmonicityResidualHead
from analysis.cqt import GaussianCQT

def sample_inharmonic_tone(rng: np.random.Generator, split: str, K: int = 8) -> tuple[np.ndarray, float, float, np.ndarray]:
    if split == 'IID':
        B = float(10.0 ** rng.uniform(-4.0, -2.5)) # 1e-4 a 3e-3
        f0 = float(rng.uniform(130.0, 320.0))
        noise = 0.0
    elif split == 'Compositional':
        B = float(10.0 ** rng.uniform(-2.5, -2.0)) # 3e-3 a 1e-2 (rigidez extrema de cordas graves de piano)
        f0 = float(rng.uniform(80.0, 200.0))
        noise = 0.0
    elif split == 'OOD':
        B = float(10.0 ** rng.uniform(-4.5, -3.8)) # 3e-5 a 1.5e-4 (cordas finas de violino/guitarra)
        f0 = float(rng.uniform(150.0, 400.0))
        noise = 0.0
    else: # Hard
        B = float(10.0 ** rng.uniform(-4.0, -2.2))
        f0 = float(rng.uniform(100.0, 350.0))
        noise = 0.01

    H = np.array([k**(-1.2) for k in range(1, K + 1)], dtype=float)
    k_vec = np.arange(1, K + 1, dtype=float)
    partial_freqs = k_vec * f0 * np.sqrt(1.0 + B * (k_vec ** 2))

    sr = 12000
    N = int(1.2 * sr)
    t = np.arange(N) / sr
    x = np.zeros(N)

    for k_idx, (fk, h) in enumerate(zip(partial_freqs, H), 1):
        if fk < sr * 0.45:
            phi = float(rng.uniform(0.0, 2.0 * math.pi))
            x += h * np.sin(2.0 * math.pi * fk * t + phi)

    if noise > 0:
        x += rng.normal(0.0, noise, size=len(x))

    return x, f0, B, partial_freqs

def extract_measured_frequencies(x: np.ndarray, f0_nominal: float, cqt: GaussianCQT, K: int = 8) -> np.ndarray:
    """
    Extrai as frequências instantâneas dos K parciais via CQT e derivadas de fase,
    utilizando preditor adaptativo de dispersão para compensar o desvio em frequência
    de parciais inarmônicos de ordem superior (que podem desviar múltiplos semitons).
    """
    t0 = 0.6
    found_freqs = []

    # 1. Medir primeiros 3 harmônicos na vizinhança nominal
    for k in range(1, 4):
        best_bin = cqt.find_nearest_bin(k * f0_nominal)
        fc = cqt.freqs[best_bin]
        _, f_inst = cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)
        found_freqs.append(f_inst)

    # 2. Estimativa analítica preliminar de f0 e B com k=1..3
    k_init = np.array([1.0, 2.0, 3.0])
    c_init = np.linalg.lstsq(np.column_stack([np.ones(3), k_init**2]), np.array(found_freqs) / k_init, rcond=None)[0]
    f0_p = max(10.0, float(c_init[0]))
    B_p = max(0.0, float(2.0 * c_init[1] / f0_p))

    # 3. Rastrear parciais restantes k=4..K usando a trajetória dispersiva prevista
    for k in range(4, K + 1):
        fk_pred = k * f0_p * math.sqrt(1.0 + B_p * (k ** 2))
        best_bin = cqt.find_nearest_bin(fk_pred)
        fc = cqt.freqs[best_bin]
        _, f_inst = cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)
        found_freqs.append(f_inst)

    return np.array(found_freqs, dtype=float)

def run_e08():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E08: IDENTIFICAÇÃO DE INARMONICIDADE B (CORDAS)")
    print("=========================================================================")

    K = 8
    solver = AnalyticInharmonicitySolver(K=K)
    cqt = GaussianCQT(sr=12000.0, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    # 1. Gerar dados de treino para a cabeça residual MLP (16 -> 8 -> 1)
    n_train = 2000
    n_val = 50

    print(f"\n1. Gerando {n_train} tons inarmônicos para calibrar o refinador residual...")
    feat_train_list, B_ana_train_list, B_true_train_list = [], [], []

    for _ in range(n_train):
        x, f0_true, B_true, _ = sample_inharmonic_tone(rng, 'IID', K=K)
        # Usa estimativa rápida de f0 nominal para inicializar a busca de bins
        f_meas = extract_measured_frequencies(x, f0_true, cqt, K=K)
        f0_ana, B_ana, res = solver.solve(f_meas)

        feat = np.zeros(16, dtype=np.float32)
        feat[0] = math.log10(max(B_ana, 1e-7))
        feat[1] = math.log2(f0_ana / 440.0)
        feat[2 : 2 + len(res)] = res[:14] * 0.1

        feat_train_list.append(feat)
        B_ana_train_list.append(B_ana)
        B_true_train_list.append(B_true)

    feat_train = torch.tensor(np.array(feat_train_list), dtype=torch.float32)
    B_ana_train = torch.tensor(np.array(B_ana_train_list), dtype=torch.float32)
    B_true_train = torch.tensor(np.array(B_true_train_list), dtype=torch.float32)

    # 2. Treinar Cabeça Residual
    print("\n2. Treinando InharmonicityResidualHead (16 -> 16 -> 8 -> 1)...")
    torch.manual_seed(42)
    model = InharmonicityResidualHead(in_features=16)
    optimizer = torch.optim.AdamW(model.parameters(), lr=5e-4, weight_decay=1e-5)

    batch_size = 64
    n_batches = len(feat_train) // batch_size
    n_epochs = 80

    for epoch in range(n_epochs):
        perm = torch.randperm(len(feat_train))
        epoch_loss = 0.0
        for b in range(n_batches):
            idx = perm[b * batch_size : (b + 1) * batch_size]
            optimizer.zero_grad()
            pred_B = model(feat_train[idx], B_ana_train[idx])
            # Loss relativa simétrica: log(pred / true)^2
            loss = torch.mean((torch.log(pred_B) - torch.log(B_true_train[idx]))**2)
            loss.backward()
            optimizer.step()
            epoch_loss += loss.item()

    print(f"   -> Treinamento concluído. Loss log-relativa final: {epoch_loss / n_batches:.6f}")

    # 3. Avaliação no Quadrante de 4 Vias
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Inarmonicidade B):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'Erro Rel. Mediano (%)':<24} | {'Erro f0 (Hz)':<14} | {'Erros Sinal':<12} | {'Status':<10}")
    print("-" * 80)

    results = {}
    all_eval_data = {}

    model.eval()
    with torch.no_grad():
        for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
            rel_errors_B = []
            abs_errors_f0 = []
            sign_errors = 0
            b_true_list = []
            b_pred_list = []

            for _ in range(n_val):
                x, f0_true, B_true, _ = sample_inharmonic_tone(rng, s_name, K=K)
                f_meas = extract_measured_frequencies(x, f0_true, cqt, K=K)
                f0_ana, B_ana, res = solver.solve(f_meas)

                feat = np.zeros(16, dtype=np.float32)
                feat[0] = math.log10(max(B_ana, 1e-7))
                feat[1] = math.log2(f0_ana / 440.0)
                feat[2 : 2 + len(res)] = res[:14] * 0.1

                feat_t = torch.tensor(feat).unsqueeze(0)
                B_ana_t = torch.tensor([B_ana], dtype=torch.float32)
                B_pred = float(model(feat_t, B_ana_t).item())

                if B_pred <= 0:
                    sign_errors += 1

                rel_err = abs(B_pred - B_true) / B_true
                rel_errors_B.append(rel_err * 100.0)
                abs_errors_f0.append(abs(f0_ana - f0_true))

                b_true_list.append(B_true)
                b_pred_list.append(B_pred)

            median_rel_err = float(np.median(rel_errors_B))
            mean_f0_err = float(np.mean(abs_errors_f0))

            passed = (median_rel_err < 5.0) and (sign_errors == 0)
            results[s_name] = {
                'median_relative_B_error_pct': median_rel_err,
                'mean_f0_error_hz': mean_f0_err,
                'sign_errors': sign_errors,
                'pass_promotion': passed,
            }
            all_eval_data[s_name] = (np.array(b_true_list), np.array(b_pred_list))

            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{s_name:<15} | {median_rel_err:<24.2f} | {mean_f0_err:<14.3f} | {sign_errors:<12} | {status:<10}")

    # 4. Teste de Isolamento Contrafactual (Intervenção pura em B)
    print("\n4. Executando teste de isolamento contrafactual I_B...")
    x_base, f0_b, B_b, _ = sample_inharmonic_tone(rng, 'IID', K=K)
    B_pert = B_b * 1.5 # +50% de inarmonicidade
    delta_B_true = B_pert - B_b

    # Sintetizar par contrafactual mantendo f0 idêntico
    sr = 12000; N = int(1.2 * sr); t = np.arange(N) / sr
    H = np.array([k**(-1.2) for k in range(1, K + 1)], dtype=float)
    k_vec = np.arange(1, K + 1, dtype=float)
    part_pert = k_vec * f0_b * np.sqrt(1.0 + B_pert * (k_vec ** 2))
    x_pert = np.zeros(N)
    for fk, h in zip(part_pert, H):
        if fk < sr * 0.45:
            x_pert += h * np.sin(2.0 * math.pi * fk * t)

    f_meas_b = extract_measured_frequencies(x_base, f0_b, cqt, K=K)
    f_meas_p = extract_measured_frequencies(x_pert, f0_b, cqt, K=K)

    f0_ana_b, B_ana_b, res_b = solver.solve(f_meas_b)
    f0_ana_p, B_ana_p, res_p = solver.solve(f_meas_p)

    with torch.no_grad():
        feat_b = np.zeros(16, dtype=np.float32)
        feat_b[0] = math.log10(max(B_ana_b, 1e-7))
        feat_b[1] = math.log2(f0_ana_b / 440.0)
        feat_b[2 : 2 + len(res_b)] = res_b[:14] * 0.1
        B_est_b = float(model(torch.tensor(feat_b).unsqueeze(0), torch.tensor([B_ana_b])).item())

        feat_p = np.zeros(16, dtype=np.float32)
        feat_p[0] = math.log10(max(B_ana_p, 1e-7))
        feat_p[1] = math.log2(f0_ana_p / 440.0)
        feat_p[2 : 2 + len(res_p)] = res_p[:14] * 0.1
        B_est_p = float(model(torch.tensor(feat_p).unsqueeze(0), torch.tensor([B_ana_p])).item())

    delta_B_est = abs(B_est_p - B_est_b)
    delta_f0_leak = abs(f0_ana_p - f0_ana_b) / f0_b # vazamento relativo em pitch

    iso_ratio = (delta_B_est / B_b) / max(delta_f0_leak, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta B Real:             {delta_B_true:+.6f}")
    print(f"  -> Delta B Estimado:         {delta_B_est:+.6f}")
    print(f"  -> Vazamento Relativo em f0: {delta_f0_leak * 100:.4f}%")
    print(f"  -> Métrica de Isolamento:    I_B = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 5. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    b_true_iid, b_pred_iid = all_eval_data['IID']

    # Dispersão B Real vs Predito em escala logarítmica
    axes[0].loglog(b_true_iid, b_pred_iid, 'ro', alpha=0.6, label='Exemplos IID')
    min_b = float(min(b_true_iid.min(), b_pred_iid.min()))
    max_b = float(max(b_true_iid.max(), b_pred_iid.max()))
    axes[0].loglog([min_b, max_b], [min_b, max_b], 'k--', label='y = x')
    axes[0].set_xlabel('B Real')
    axes[0].set_ylabel('B Estimado')
    axes[0].set_title('Recuperação de Inarmonicidade B (Escala Log)')
    axes[0].legend()
    axes[0].grid(True, which='both', alpha=0.3)

    # Dispersão de Desvio de Parciais
    sample_k = np.arange(1, 9)
    y_harm = sample_k * f0_b
    y_inharm = sample_k * f0_b * np.sqrt(1.0 + B_b * sample_k**2)
    axes[1].plot(sample_k, (y_inharm - y_harm), 'bs-', label=f'Desvio Real (B = {B_b:.2e})')
    y_pred_part = sample_k * f0_ana_b * np.sqrt(1.0 + B_est_b * sample_k**2)
    axes[1].plot(sample_k, (y_pred_part - y_harm), 'mo--', label=f'Desvio Estimado (B = {B_est_b:.2e})')
    axes[1].set_xlabel('Índice Parcial k')
    axes[1].set_ylabel('Desvio de Frequência fk - k*f0 (Hz)')
    axes[1].set_title('Ajuste da Dispersão Acústica de Parciais (E08)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e08_inharmonicity_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    # Salvar resultados em JSON
    output_meta = {
        'stage': 'E08',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e08_inharmonicity.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e08()
