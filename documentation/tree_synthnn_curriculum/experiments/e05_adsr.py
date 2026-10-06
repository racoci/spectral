#!/usr/bin/env python3
"""
Experimento E05: Identificação de Envelope ADSR Paramétrico.

Arquitetura (conforme curriculum/spec.py):
- Entrada: 64 features (32 pontos temporais adaptativos de log-magnitude mu_A + 32 derivadas temporais d_mu_A)
- Modelo: 64 -> 32 -> 16 -> 4 constrained heads (sigmoid com limites físicos garantidos)
- Normalização temporal: tau = t / duration

Critérios de Promoção (spec.py):
- Erro relativo mediano de parâmetros < 5% (0.05)
- Sucesso nos splits do quadrante (IID, Composicional, OOD, Hard)
- Isolamento contrafactual I_ADSR > 10.0
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

from analysis.envelope import adsr_curve
from analysis.cqt import GaussianCQT

# =============================================================================
# 1. CABEÇA PREDITIVA ADSR CONSTRANGIDA (64 -> 32 -> 16 -> 4)
# =============================================================================

class ParametricAdsrHead(nn.Module):
    def __init__(self, in_features: int = 64):
        super().__init__()
        self.net = nn.Sequential(
            nn.Linear(in_features, 32),
            nn.ReLU(),
            nn.Linear(32, 16),
            nn.ReLU(),
            nn.Linear(16, 4)
        )

    def forward(self, z: torch.Tensor) -> torch.Tensor:
        raw = self.net(z)
        # Limites físicos garantidos cobrindo faixas amplas:
        tau_a = torch.sigmoid(raw[..., 0]) * 0.50 + 0.002 # [0.002, 0.502]
        tau_d = torch.sigmoid(raw[..., 1]) * 0.50 + 0.005 # [0.005, 0.505]
        sustain = torch.sigmoid(raw[..., 2]) * 0.96 + 0.02 # [0.020, 0.980]
        tau_r = torch.sigmoid(raw[..., 3]) * 0.65 + 0.010 # [0.010, 0.660]
        return torch.stack([tau_a, tau_d, sustain, tau_r], dim=-1)

# =============================================================================
# 2. EXTRATOR DE FEATURES TEMPORAIS ADAPTATIVAS (64 DIMENSÕES)
# =============================================================================

def extract_adsr_64_features(cqt: GaussianCQT, x: np.ndarray, t_on: float, duration: float, f0: float) -> np.ndarray:
    """
    Extrai 64 features do envelope:
    - 32 valores de mu_A amostrados uniformemente no envelope da nota
    - 32 derivadas temporais discretas d_mu_A
    """
    fc = cqt.freqs[cqt.find_nearest_bin(f0)]
    t_eval = np.linspace(t_on, t_on + duration * 1.50, 32)
    
    mags = []
    for t_i in t_eval:
        C_val, gain = cqt.evaluate_coefficient_and_gain(x, t_i, fc)
        mag = abs(C_val) / max(gain, 1e-12)
        mags.append(math.log(max(mag, 1e-4)))
        
    mags = np.array(mags, dtype=np.float32)
    mags_norm = mags - np.max(mags) # Normalizado ao pico
    d_mags = np.gradient(mags_norm).astype(np.float32)
    
    return np.concatenate([mags_norm, d_mags])

# =============================================================================
# 3. GERADOR DE EXEMPLOS E DATASETS
# =============================================================================

def generate_sample(rng: np.random.Generator, split: str, cqt: GaussianCQT):
    duration = 1.0
    t_on = 0.2
    sr = 12000
    N = int(2.2 * sr)
    t = np.arange(N) / sr
    
    if split == 'IID':
        tau_a = float(np.exp(rng.uniform(np.log(0.008), np.log(0.25))))
        tau_d = float(rng.uniform(0.03, 0.30))
        sustain = float(rng.uniform(0.20, 0.85))
        tau_r = float(rng.uniform(0.03, 0.35))
        f0 = float(rng.uniform(250.0, 900.0))
        noise = 0.0
    elif split == 'Compositional':
        tau_a = float(rng.uniform(0.01, 0.04))
        tau_d = float(rng.uniform(0.20, 0.35))
        sustain = float(rng.uniform(0.25, 0.60))
        tau_r = float(rng.uniform(0.15, 0.35))
        f0 = float(rng.uniform(300.0, 1200.0))
        noise = 0.0
    elif split == 'OOD':
        tau_a = float(rng.uniform(0.20, 0.35))
        tau_d = float(rng.uniform(0.03, 0.15))
        sustain = float(rng.uniform(0.15, 0.90))
        tau_r = float(rng.uniform(0.30, 0.45))
        f0 = float(rng.uniform(200.0, 1500.0))
        noise = 0.0
    else: # Hard
        tau_a = float(rng.uniform(0.02, 0.25))
        tau_d = float(rng.uniform(0.05, 0.25))
        sustain = float(rng.uniform(0.20, 0.80))
        tau_r = float(rng.uniform(0.05, 0.30))
        f0 = float(rng.uniform(200.0, 1200.0))
        noise = 0.015
        
    t_att = tau_a * duration
    t_dec = tau_d * duration
    t_rel = tau_r * duration
    
    env = adsr_curve(t, t_on, duration, t_att, t_dec, sustain, t_rel)
    x = env * np.sin(2.0 * math.pi * f0 * t)
    if noise > 0:
        x += rng.normal(0.0, noise, size=len(x))
        
    z_feat = extract_adsr_64_features(cqt, x, t_on, duration, f0)
    target = np.array([tau_a, tau_d, sustain, tau_r], dtype=np.float32)
    return z_feat, target

# =============================================================================
# 4. EXECUÇÃO DO EXPERIMENTO E05
# =============================================================================

def run_e05():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E05: IDENTIFICAÇÃO DE ENVELOPE ADSR (64 -> 32 -> 16 -> 4)")
    print("=========================================================================")

    cqt = GaussianCQT(sr=12000.0, bins_per_octave=60, fmin=40.0, fmax=5500.0)
    rng = np.random.default_rng(42)

    n_train = 2000
    n_val = 60

    print(f"\n1. Sintetizando {n_train} perfis de treinamento (64 features)...")
    X_train_list, Y_train_list = [], []
    for _ in range(n_train):
        z, tgt = generate_sample(rng, 'IID', cqt)
        X_train_list.append(z)
        Y_train_list.append(tgt)

    X_train = torch.tensor(np.array(X_train_list), dtype=torch.float32)
    Y_train = torch.tensor(np.array(Y_train_list), dtype=torch.float32)

    val_splits = {}
    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        X_v, Y_v = [], []
        for _ in range(n_val):
            z, tgt = generate_sample(rng, s_name, cqt)
            X_v.append(z)
            Y_v.append(tgt)
        val_splits[s_name] = (
            torch.tensor(np.array(X_v), dtype=torch.float32),
            torch.tensor(np.array(Y_v), dtype=torch.float32)
        )

    # 2. Treinamento
    print("\n2. Treinando ParametricAdsrHead (AdamW, lr=2.5e-3)...")
    torch.manual_seed(42)
    model = ParametricAdsrHead(in_features=64)
    optimizer = torch.optim.AdamW(model.parameters(), lr=2.5e-3, weight_decay=1e-4)
    loss_fn = nn.MSELoss()

    batch_size = 64
    n_batches = len(X_train) // batch_size
    n_epochs = 140

    for epoch in range(n_epochs):
        perm = torch.randperm(len(X_train))
        epoch_loss = 0.0
        for b in range(n_batches):
            idx = perm[b * batch_size : (b + 1) * batch_size]
            optimizer.zero_grad()
            pred = model(X_train[idx])
            loss = loss_fn(pred, Y_train[idx])
            loss.backward()
            optimizer.step()
            epoch_loss += loss.item()

    print(f"   -> Treinamento concluído. Loss MSE final: {epoch_loss / n_batches:.5f}")

    # 3. Avaliação no Quadrante
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE DE VALIDAÇÃO (E05 - ADSR):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'RMSE Total':<12} | {'Med. Rel. Err':<15} | {'tau_A Err (%)':<15} | {'Sustain Err':<12} | {'Status':<10}")
    print("-" * 80)

    results = {}
    all_preds = {}

    model.eval()
    with torch.no_grad():
        for s_name, (X_val, Y_val) in val_splits.items():
            pred = model(X_val)
            err = pred - Y_val
            rmse_total = float(torch.sqrt(torch.mean(err**2)).item())
            
            # Erro relativo por parâmetro
            rel_err_matrix = torch.abs(err) / torch.clamp(Y_val, min=0.01)
            med_rel_err = float(torch.median(rel_err_matrix).item())
            
            tau_a_err_pct = float(torch.mean(torch.abs(err[:, 0]) / torch.clamp(Y_val[:, 0], min=0.01)).item()) * 100.0
            sustain_err = float(torch.mean(torch.abs(err[:, 2])).item())

            passed = med_rel_err < 0.05
            results[s_name] = {
                'rmse_total': rmse_total,
                'median_relative_error': med_rel_err,
                'tau_a_err_percent': tau_a_err_pct,
                'sustain_mae': sustain_err,
                'pass_promotion': passed
            }
            all_preds[s_name] = (Y_val.numpy(), pred.numpy())

            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{s_name:<15} | {rmse_total:<12.4f} | {med_rel_err*100:<14.2f}% | {tau_a_err_pct:<14.2f}% | {sustain_err:<12.4f} | {status:<10}")

    # 4. Teste de Isolamento Contrafactual
    print("\n4. Executando teste de isolamento contrafactual I_ADSR...")
    z_base, y_base = generate_sample(rng, 'IID', cqt)
    duration = 1.0; t_on = 0.2; sr = 12000
    t = np.arange(int(2.2 * sr)) / sr
    t_att = y_base[0] * duration; t_dec = y_base[1] * duration; s_orig = y_base[2]; t_rel = y_base[3] * duration
    s_pert = float(np.clip(s_orig + 0.20, 0.0, 1.0))
    delta_s_real = s_pert - s_orig

    env_pert = adsr_curve(t, t_on, duration, t_att, t_dec, s_pert, t_rel)
    x_pert = env_pert * np.sin(2.0 * math.pi * 440.0 * t)
    z_pert = extract_adsr_64_features(cqt, x_pert, t_on, duration, 440.0)

    with torch.no_grad():
        pred_base = model(torch.tensor(z_base).unsqueeze(0))[0].numpy()
        pred_pert = model(torch.tensor(z_pert).unsqueeze(0))[0].numpy()

    delta_preds = np.abs(pred_pert - pred_base)
    delta_s_est = delta_preds[2]
    cross_leak = delta_preds[0] + delta_preds[1] + delta_preds[3]

    iso_ratio = delta_s_est / max(cross_leak, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta Sustain Real:        {delta_s_real:+.4f}")
    print(f"  -> Delta Sustain Estimado:    {delta_s_est:+.4f}")
    print(f"  -> Vazamento Cruzado (A,D,R): {cross_leak:.6f}")
    print(f"  -> Métrica de Isolamento:     I_ADSR = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 5. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(2, 2, figsize=(11, 10))
    param_names = [r'$\tau_A$ (Ataque)', r'$\tau_D$ (Decay)', 'Sustain', r'$\tau_R$ (Release)']
    y_true_iid, y_pred_iid = all_preds['IID']

    for p_idx in range(4):
        ax = axes[p_idx // 2, p_idx % 2]
        ax.scatter(y_true_iid[:, p_idx], y_pred_iid[:, p_idx], color='teal', alpha=0.7, edgecolors='k', s=35)
        min_v = float(min(y_true_iid[:, p_idx].min(), y_pred_iid[:, p_idx].min()))
        max_v = float(max(y_true_iid[:, p_idx].max(), y_pred_iid[:, p_idx].max()))
        ax.plot([min_v, max_v], [min_v, max_v], 'r--', lw=1.5, label='y = x (Exato)')
        ax.set_xlabel(f'{param_names[p_idx]} Real')
        ax.set_ylabel(f'{param_names[p_idx]} Estimado')
        ax.set_title(f'Convergência {param_names[p_idx]} (IID)')
        ax.legend()
        ax.grid(True, alpha=0.3)

    plt.tight_layout()
    plot_path = ROOT / 'experiments' / 'e05_adsr_results.png'
    plt.savefig(plot_path, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_path}")

    # Salvar resultados em JSON
    output_meta = {
        'stage': 'E05',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e05_adsr.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e05()
