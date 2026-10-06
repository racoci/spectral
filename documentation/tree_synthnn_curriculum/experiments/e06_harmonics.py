#!/usr/bin/env python3
"""
Experimento E06: Identificação de Estrutura Harmônica (H_k) com Decoder Compartilhado.

Objetivo:
Validar a recuperação da série harmônica H_k através de um decodificador contínuo
compartilhado h_k = MLP(z, log2(k)), provando que uma rede treinada apenas em k <= 8
consegue extrapolar analiticamente para harmônicos superiores k in [9, 16].

Critérios de Promoção (spec.py):
- H_RMSE < 0.01 no conjunto k <= 8
- Extrapolação k in [9, 16] com RMSE < 0.03
- Sucesso nos splits do quadrante (IID, Composicional, OOD, Hard)
- Isolamento contrafactual I_H > 10.0
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

from analysis.harmonic import SharedHarmonicDecoder, HarmonicLatentEncoder
from analysis.cqt import GaussianCQT

def sample_harmonic_series(rng: np.random.Generator, split: str, K_max: int = 16) -> tuple[np.ndarray, float]:
    """Gera uma série harmônica sintética H_k respeitando H_1 = 1.0."""
    k_vec = np.arange(1, K_max + 1, dtype=float)
    if split == 'IID':
        alpha = float(rng.uniform(0.8, 1.8))
        f0 = float(rng.uniform(150.0, 400.0))
        pert = rng.normal(0.0, 0.05, size=K_max)
    elif split == 'Compositional':
        alpha = float(rng.uniform(1.9, 2.7)) # decaimento muito íngreme
        f0 = float(rng.uniform(100.0, 250.0))
        pert = rng.normal(0.0, 0.03, size=K_max)
    elif split == 'OOD':
        alpha = float(rng.uniform(0.3, 0.7)) # decaimento muito lento (brilho extremo)
        f0 = float(rng.uniform(120.0, 350.0))
        pert = rng.normal(0.0, 0.04, size=K_max)
    else: # Hard
        alpha = float(rng.uniform(0.6, 2.0))
        f0 = float(rng.uniform(120.0, 500.0))
        pert = rng.normal(0.0, 0.12, size=K_max) # fortes irregularidades

    pert[0] = 0.0 # H_1 sempre 1.0 por gauge
    H = (k_vec ** (-alpha)) * np.exp(pert)
    H[0] = 1.0 # estritamente 1.0
    return H, f0

def synthesize_and_extract_features(H: np.ndarray, f0: float, cqt: GaussianCQT) -> np.ndarray:
    sr = 12000
    N = int(1.0 * sr)
    t = np.arange(N) / sr
    x = np.zeros(N)
    for k_idx, h in enumerate(H, 1):
        fk = k_idx * f0
        if fk < sr * 0.45:
            x += h * np.sin(2.0 * math.pi * fk * t)

    # Extrai log(A_k / A_1) para os 4 primeiros harmônicos como features z
    t0 = 0.5
    a_vals = []
    for k in range(1, 5):
        best_bin = cqt.find_nearest_bin(k * f0)
        fc = cqt.freqs[best_bin]
        a_est, _ = cqt.extract_instantaneous_amplitude_and_freq(x, t0, fc)
        a_vals.append(max(a_est, 1e-5))

    a_vals = np.array(a_vals)
    # log(A_k) - log(A_1)
    spec_feat = np.log(a_vals) - math.log(a_vals[0])
    return spec_feat.astype(np.float32)

def run_e06():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E06: ESTRUTURA HARMÔNICA (H_k) COM DECODER COMPARTILHADO")
    print("=========================================================================")

    cqt = GaussianCQT(sr=12000.0, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    n_train = 2000
    n_val = 50

    print(f"\n1. Gerando {n_train} séries harmônicas de treinamento...")
    X_train_list, Y_train_list = [], []
    for _ in range(n_train):
        H, f0 = sample_harmonic_series(rng, 'IID', K_max=16)
        feat = synthesize_and_extract_features(H, f0, cqt)
        X_train_list.append(feat)
        Y_train_list.append(H)

    X_train = torch.tensor(np.array(X_train_list), dtype=torch.float32)
    Y_train = torch.tensor(np.array(Y_train_list), dtype=torch.float32)

    val_splits = {}
    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        X_v, Y_v = [], []
        for _ in range(n_val):
            H, f0 = sample_harmonic_series(rng, s_name, K_max=16)
            feat = synthesize_and_extract_features(H, f0, cqt)
            X_v.append(feat)
            Y_v.append(H)
        val_splits[s_name] = (
            torch.tensor(np.array(X_v), dtype=torch.float32),
            torch.tensor(np.array(Y_v), dtype=torch.float32)
        )

    # 2. Modelos: Encoder Latente e Decoder Compartilhado
    print("\n2. Treinando SharedHarmonicDecoder exclusivamente em k <= 8...")
    torch.manual_seed(42)
    encoder = HarmonicLatentEncoder(in_features=4, latent_dim=8)
    decoder = SharedHarmonicDecoder(latent_dim=8, hidden_dim=32)

    params = list(encoder.parameters()) + list(decoder.parameters())
    optimizer = torch.optim.AdamW(params, lr=2e-3, weight_decay=1e-4)

    # Índices de treino k = 1..8
    k_train_indices = torch.arange(1, 9)
    # Índices de extrapolação k = 9..16
    k_extrap_indices = torch.arange(9, 17)
    k_all_indices = torch.arange(1, 17)

    batch_size = 64
    n_batches = len(X_train) // batch_size
    n_epochs = 120

    for epoch in range(n_epochs):
        perm = torch.randperm(len(X_train))
        epoch_loss = 0.0
        for b in range(n_batches):
            idx = perm[b * batch_size : (b + 1) * batch_size]
            optimizer.zero_grad()
            z = encoder(X_train[idx])
            pred_h = decoder(z, k_train_indices) # apenas k=1..8
            target_h = Y_train[idx, 0:8]
            
            # Loss combinada linear + logarítmica
            loss_lin = torch.mean((pred_h - target_h)**2)
            loss_log = torch.mean((torch.log(pred_h + 1e-6) - torch.log(target_h + 1e-6))**2)
            loss = loss_lin + 0.1 * loss_log

            loss.backward()
            optimizer.step()
            epoch_loss += loss.item()

    print(f"   -> Treinamento concluído. Loss final: {epoch_loss / n_batches:.5f}")

    # 3. Avaliação no Quadrante e Teste Crucial de Extrapolação (k in [9, 16])
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Treino k<=8 vs Extrapolação k=9..16):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'RMSE (k<=8)':<14} | {'RMSE Extrap(k>8)':<18} | {'Max Erro k>8':<14} | {'Status':<10}")
    print("-" * 75)

    results = {}
    all_eval_data = {}

    encoder.eval()
    decoder.eval()
    with torch.no_grad():
        for s_name, (X_val, Y_val) in val_splits.items():
            z_val = encoder(X_val)
            pred_all = decoder(z_val, k_all_indices) # avalia todos os 16 harmônicos
            
            # Erro no conjunto de treino k=1..8
            err_train_band = pred_all[:, 0:8] - Y_val[:, 0:8]
            rmse_train_band = float(torch.sqrt(torch.mean(err_train_band**2)).item())

            # Teste crucial de extrapolação k=9..16
            err_extrap_band = pred_all[:, 8:16] - Y_val[:, 8:16]
            rmse_extrap_band = float(torch.sqrt(torch.mean(err_extrap_band**2)).item())
            max_extrap_err = float(torch.max(torch.abs(err_extrap_band)).item())

            passed = (rmse_train_band < 0.010) and (rmse_extrap_band < 0.030)
            results[s_name] = {
                'rmse_train_band': rmse_train_band,
                'rmse_extrap_band': rmse_extrap_band,
                'max_extrap_error': max_extrap_err,
                'pass_promotion': passed
            }
            all_eval_data[s_name] = (Y_val.numpy(), pred_all.numpy())

            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{s_name:<15} | {rmse_train_band:<14.4f} | {rmse_extrap_band:<18.4f} | {max_extrap_err:<14.4f} | {status:<10}")

    # 4. Teste de Isolamento Contrafactual
    print("\n4. Executando teste de isolamento contrafactual I_H...")
    H_base, f0_base = sample_harmonic_series(rng, 'IID', K_max=16)
    # Alteração exclusiva em H_2 (+0.10)
    H_pert = H_base.copy()
    H_pert[1] = min(0.95, H_base[1] + 0.10)
    delta_h2_true = H_pert[1] - H_base[1]

    feat_base = synthesize_and_extract_features(H_base, f0_base, cqt)
    feat_pert = synthesize_and_extract_features(H_pert, f0_base, cqt)

    with torch.no_grad():
        z_b = encoder(torch.tensor(feat_base).unsqueeze(0))
        z_p = encoder(torch.tensor(feat_pert).unsqueeze(0))
        pred_b = decoder(z_b, k_all_indices)[0].numpy()
        pred_p = decoder(z_p, k_all_indices)[0].numpy()

    delta_preds = np.abs(pred_p - pred_b)
    delta_h2_est = delta_preds[1]
    leak_other_harmonics = np.sum(delta_preds[2:8]) + delta_preds[0] # vazamento

    iso_ratio = delta_h2_est / max(leak_other_harmonics, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta H_2 Real:            {delta_h2_true:+.4f}")
    print(f"  -> Delta H_2 Estimado:        {delta_h2_est:+.4f}")
    print(f"  -> Vazamento para Outros H_k: {leak_other_harmonics:.6f}")
    print(f"  -> Métrica de Isolamento:     I_H = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 5. Geração de Gráficos de Extrapolação
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    y_true_iid, y_pred_iid = all_eval_data['IID']
    sample_idx = 0

    k_plot = np.arange(1, 17)
    axes[0].plot(k_plot[:8], y_true_iid[sample_idx, :8], 'bo-', label='Real (Banda de Treino k<=8)')
    axes[0].plot(k_plot[:8], y_pred_iid[sample_idx, :8], 'c--', label='Ajuste Treino')
    axes[0].plot(k_plot[7:], y_true_iid[sample_idx, 7:], 'ro-', label='Real (Extrapolação Não-Vista k>8)')
    axes[0].plot(k_plot[7:], y_pred_iid[sample_idx, 7:], 'm--', lw=2, label='Predição Extrapolada')
    axes[0].axvline(8.5, color='gray', ls=':', label='Fronteira de Treino')
    axes[0].set_xlabel('Índice Harmônico k')
    axes[0].set_ylabel('Amplitude H_k')
    axes[0].set_title('Extrapolação Contínua de Harmônicos k=9..16')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Dispersão Real vs Predito na Banda de Extrapolação
    axes[1].scatter(y_true_iid[:, 8:16].flatten(), y_pred_iid[:, 8:16].flatten(), color='crimson', alpha=0.6, s=25, label='Harmônicos Extrapolados')
    min_v = float(min(y_true_iid[:, 8:16].min(), y_pred_iid[:, 8:16].min()))
    max_v = float(max(y_true_iid[:, 8:16].max(), y_pred_iid[:, 8:16].max()))
    axes[1].plot([min_v, max_v], [min_v, max_v], 'k--', label='y = x')
    axes[1].set_xlabel('H_k Real (k=9..16)')
    axes[1].set_ylabel('H_k Extrapolado (k=9..16)')
    axes[1].set_title('Convergência na Faixa Extrapolada k > 8')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e06_harmonics_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    # Salvar resultados em JSON
    output_meta = {
        'stage': 'E06',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e06_harmonics.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e06()
