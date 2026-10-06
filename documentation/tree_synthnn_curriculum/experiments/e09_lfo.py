#!/usr/bin/env python3
"""
Experimento E09: Identificação de Modulação Periódica LFO (Vibrato/Pitch).

Objetivo:
Validar a recuperação dos parâmetros contínuos de modulação periódica:
    taxa f_m in [2, 11] Hz
    profundidade d in [10, 80] cents
    fase phi in [-pi, pi) via vetor unitário (cos phi, sin phi) em S^1
através de desmodulação temporal precisa e refinamento neural por cabeça de vetor de fase.

Critérios de Promoção (spec.py):
- Erro relativo de frequência f_m < 0.02 (2%)
- Erro circular de fase < 5 graus (5.0 deg = 0.087 rad)
- Erro relativo de profundidade < 0.02 (2%)
- Sucesso nos splits do quadrante (IID, Composicional, OOD, Hard)
- Isolamento contrafactual I_LFO > 10.0
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

from analysis.lfo import AnalyticLfoEstimator, LfoParametricHead

def sample_lfo_signal(rng: np.random.Generator, split: str) -> tuple[np.ndarray, float, float, float, float]:
    """Gera um sinal senoidal com modulação periódica de pitch (vibrato LFO)."""
    if split == 'IID':
        f0 = float(rng.uniform(160.0, 340.0))
        fm = float(rng.uniform(3.5, 7.5)) # Hz
        depth_cents = float(rng.uniform(20.0, 55.0)) # cents
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    elif split == 'Compositional':
        f0 = float(rng.uniform(120.0, 240.0))
        fm = float(rng.uniform(8.0, 11.0)) # vibrato rápido
        depth_cents = float(rng.uniform(50.0, 75.0)) # vibrato profundo
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    elif split == 'OOD':
        f0 = float(rng.uniform(220.0, 420.0))
        fm = float(rng.uniform(2.0, 3.2)) # vibrato lento
        depth_cents = float(rng.uniform(12.0, 20.0)) # vibrato raso
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.0
    else: # Hard
        f0 = float(rng.uniform(150.0, 350.0))
        fm = float(rng.uniform(3.0, 9.0))
        depth_cents = float(rng.uniform(15.0, 60.0))
        phi = float(rng.uniform(-math.pi, math.pi))
        noise = 0.005 # ruído acústico aditivo

    sr = 12000
    duration = 1.0
    t = np.arange(int(duration * sr)) / sr

    cents_traj = depth_cents * np.sin(2.0 * math.pi * fm * t + phi)
    f_inst_traj = f0 * (2.0 ** (cents_traj / 1200.0))
    phase_traj = 2.0 * math.pi * np.cumsum(f_inst_traj) / sr

    x = np.sin(phase_traj)
    if noise > 0:
        x += rng.normal(0.0, noise, size=len(x))

    return x, f0, fm, depth_cents, phi

def build_lfo_features(t_eval: np.ndarray, y_eval: np.ndarray, d_ana: float) -> np.ndarray:
    """Gera vetor de 64 features interpoladas no domínio do tempo."""
    t_grid = np.linspace(0.25, 0.75, 32)
    dt = t_grid[1] - t_grid[0]
    
    # Interpolar para grade uniforme de 32 pontos
    y_interp = np.interp(t_grid, t_eval, y_eval)
    dy = np.gradient(y_interp, dt)

    feat = np.zeros(64, dtype=np.float32)
    scale = max(1.0, d_ana)
    feat[:32] = (y_interp / scale).astype(np.float32)
    feat[32:] = (dy / (2.0 * math.pi * 5.0 * scale)).astype(np.float32)
    return feat

def run_e09():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E09: IDENTIFICAÇÃO DE MODULAÇÃO PERIÓDICA LFO")
    print("=========================================================================")

    estimator = AnalyticLfoEstimator(sr=12000.0)
    rng = np.random.default_rng(42)

    n_train = 1800
    n_val = 50

    print(f"\n1. Gerando {n_train} sinais sintetizados com LFO de treino...")
    X_train_list = []
    fm_ana_train, d_ana_train, vec_ana_train = [], [], []
    Targets_train_list = []

    for _ in range(n_train):
        x, f0, fm, depth_c, phi = sample_lfo_signal(rng, 'IID')
        fm_a, d_a, phi_a, t_ev, y_ev = estimator.estimate(x, f0)
        feat = build_lfo_features(t_ev, y_ev, d_a)

        X_train_list.append(feat)
        fm_ana_train.append(fm_a)
        d_ana_train.append(d_a)
        vec_ana_train.append([math.cos(phi_a), math.sin(phi_a)])
        Targets_train_list.append([fm, depth_c, math.cos(phi), math.sin(phi)])

    X_train = torch.tensor(np.array(X_train_list), dtype=torch.float32)
    fm_ana_t = torch.tensor(np.array(fm_ana_train), dtype=torch.float32)
    d_ana_t = torch.tensor(np.array(d_ana_train), dtype=torch.float32)
    vec_ana_t = torch.tensor(np.array(vec_ana_train), dtype=torch.float32)
    Targets_train = torch.tensor(np.array(Targets_train_list), dtype=torch.float32)

    val_splits = {}
    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        X_v, fm_v, d_v, vec_v, T_v = [], [], [], [], []
        for _ in range(n_val):
            x, f0, fm, depth_c, phi = sample_lfo_signal(rng, s_name)
            fm_a, d_a, phi_a, t_ev, y_ev = estimator.estimate(x, f0)
            feat = build_lfo_features(t_ev, y_ev, d_a)

            X_v.append(feat)
            fm_v.append(fm_a)
            d_v.append(d_a)
            vec_v.append([math.cos(phi_a), math.sin(phi_a)])
            T_v.append([fm, depth_c, math.cos(phi), math.sin(phi)])
        val_splits[s_name] = (
            torch.tensor(np.array(X_v), dtype=torch.float32),
            torch.tensor(np.array(fm_v), dtype=torch.float32),
            torch.tensor(np.array(d_v), dtype=torch.float32),
            torch.tensor(np.array(vec_v), dtype=torch.float32),
            torch.tensor(np.array(T_v), dtype=torch.float32)
        )

    # 2. Treinar LfoParametricHead
    print("\n2. Treinando LfoParametricHead residual em torno da estimativa analítica...")
    torch.manual_seed(42)
    model = LfoParametricHead(in_features=64)
    optimizer = torch.optim.AdamW(model.parameters(), lr=1e-3, weight_decay=1e-5)

    batch_size = 64
    n_batches = len(X_train) // batch_size
    n_epochs = 60

    for epoch in range(n_epochs):
        perm = torch.randperm(len(X_train))
        epoch_loss = 0.0
        for b in range(n_batches):
            idx = perm[b * batch_size : (b + 1) * batch_size]
            optimizer.zero_grad()
            
            fm_pred, depth_pred, phase_pred = model(
                X_train[idx], fm_ana_t[idx], d_ana_t[idx], vec_ana_t[idx]
            )
            
            target_fm = Targets_train[idx, 0]
            target_depth = Targets_train[idx, 1]
            target_vec = Targets_train[idx, 2:4]

            loss_fm = torch.mean(((fm_pred - target_fm) / target_fm)**2)
            loss_depth = torch.mean(((depth_pred - target_depth) / target_depth)**2)
            cos_sim = torch.sum(phase_pred * target_vec, dim=-1)
            loss_phase = torch.mean(1.0 - cos_sim)

            loss = loss_fm + loss_depth + loss_phase
            loss.backward()
            optimizer.step()
            epoch_loss += loss.item()

    print(f"   -> Treinamento concluído. Loss final: {epoch_loss / n_batches:.6f}")

    # 3. Avaliação no Quadrante de 4 Vias
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Modulação LFO):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'Erro fm Rel. (%)':<18} | {'Erro Fase (graus)':<18} | {'Erro Prof. (%)':<16} | {'Status':<10}")
    print("-" * 85)

    results = {}
    all_eval_data = {}

    model.eval()
    with torch.no_grad():
        for s_name, (X_val, fm_v, d_v, vec_v, T_val) in val_splits.items():
            fm_pred, depth_pred, phase_pred = model(X_val, fm_v, d_v, vec_v)
            
            fm_true = T_val[:, 0].numpy()
            d_true = T_val[:, 1].numpy()
            target_vec = T_val[:, 2:4].numpy()

            fm_p = fm_pred.numpy()
            d_p = depth_pred.numpy()
            vec_p = phase_pred.numpy()

            # Erro relativo de taxa fm
            rel_err_fm = np.abs(fm_p - fm_true) / fm_true * 100.0
            median_rel_fm = float(np.median(rel_err_fm))

            # Erro relativo de profundidade
            rel_err_d = np.abs(d_p - d_true) / d_true * 100.0
            median_rel_d = float(np.median(rel_err_d))

            # Erro circular de fase em graus
            phi_true = np.arctan2(target_vec[:, 1], target_vec[:, 0])
            phi_est = np.arctan2(vec_p[:, 1], vec_p[:, 0])
            dphi = np.abs(np.arctan2(np.sin(phi_est - phi_true), np.cos(phi_est - phi_true)))
            phase_deg = np.degrees(dphi)
            median_phase_deg = float(np.median(phase_deg))

            passed = (median_rel_fm < 2.0) and (median_phase_deg < 5.0) and (median_rel_d < 2.0)
            results[s_name] = {
                'median_relative_fm_error_pct': median_rel_fm,
                'median_phase_error_deg': median_phase_deg,
                'median_relative_depth_error_pct': median_rel_d,
                'pass_promotion': passed,
            }
            all_eval_data[s_name] = (fm_true, fm_p, phi_true, phi_est, d_true, d_p)

            status = "✅ PASS" if passed else "❌ FAIL"
            print(f"{s_name:<15} | {median_rel_fm:<18.2f} | {median_phase_deg:<18.2f} | {median_rel_d:<16.2f} | {status:<10}")

    # 4. Teste de Isolamento Contrafactual (Intervenção pura na profundidade d)
    print("\n4. Executando teste de isolamento contrafactual I_LFO...")
    x_base, f0_b, fm_b, depth_b, phi_b = sample_lfo_signal(rng, 'IID')
    depth_pert = depth_b + 15.0 # +15 cents
    delta_d_true = depth_pert - depth_b

    # Sintetizar par contrafactual
    sr = 12000; duration = 1.0; t = np.arange(int(duration * sr)) / sr
    c_traj_p = depth_pert * np.sin(2.0 * math.pi * fm_b * t + phi_b)
    f_traj_p = f0_b * (2.0 ** (c_traj_p / 1200.0))
    p_traj_p = 2.0 * math.pi * np.cumsum(f_traj_p) / sr
    x_pert = np.sin(p_traj_p)

    fm_a_b, d_a_b, phi_a_b, t_b, y_b = estimator.estimate(x_base, f0_b)
    fm_a_p, d_a_p, phi_a_p, t_p, y_p = estimator.estimate(x_pert, f0_b)

    feat_b = build_lfo_features(t_b, y_b, d_a_b)
    feat_p = build_lfo_features(t_p, y_p, d_a_p)

    with torch.no_grad():
        fm_est_b, d_pred_b, _ = model(
            torch.tensor(feat_b).unsqueeze(0),
            torch.tensor([fm_a_b]), torch.tensor([d_a_b]),
            torch.tensor([[math.cos(phi_a_b), math.sin(phi_a_b)]])
        )
        fm_est_p, d_pred_p, _ = model(
            torch.tensor(feat_p).unsqueeze(0),
            torch.tensor([fm_a_p]), torch.tensor([d_a_p]),
            torch.tensor([[math.cos(phi_a_p), math.sin(phi_a_p)]])
        )

    delta_d_est = float(abs(d_pred_p - d_pred_b).item())
    delta_fm_leak = float(abs(fm_est_p - fm_est_b).item()) / fm_b

    iso_ratio = (delta_d_est / depth_b) / max(delta_fm_leak, 1e-6)
    iso_db = 20.0 * math.log10(max(iso_ratio, 1e-4))

    print(f"  -> Delta Depth Real:          {delta_d_true:+.2f} cents")
    print(f"  -> Delta Depth Estimado:      {delta_d_est:+.2f} cents")
    print(f"  -> Vazamento Relativo em fm:  {delta_fm_leak * 100:.4f}%")
    print(f"  -> Métrica de Isolamento:     I_LFO = {iso_ratio:.1f} ({iso_db:.1f} dB)")
    iso_passed = iso_ratio > 10.0

    # 5. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    fm_t_iid, fm_p_iid, phi_t_iid, phi_p_iid, d_t_iid, d_p_iid = all_eval_data['IID']

    # Frequência de Modulação Real vs Predito
    axes[0].scatter(fm_t_iid, fm_p_iid, color='teal', alpha=0.7, s=35, label='Exemplos IID')
    min_fm = float(min(fm_t_iid.min(), fm_p_iid.min()))
    max_fm = float(max(fm_t_iid.max(), fm_p_iid.max()))
    axes[0].plot([min_fm, max_fm], [min_fm, max_fm], 'k--', label='y = x')
    axes[0].set_xlabel('Taxa fm Real (Hz)')
    axes[0].set_ylabel('Taxa fm Estimada (Hz)')
    axes[0].set_title('Recuperação de Frequência de LFO fm (E09)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Dispersão de Profundidade em cents
    axes[1].scatter(d_t_iid, d_p_iid, color='coral', alpha=0.7, s=35)
    min_d = float(min(d_t_iid.min(), d_p_iid.min()))
    max_d = float(max(d_t_iid.max(), d_p_iid.max()))
    axes[1].plot([min_d, max_d], [min_d, max_d], 'k--', label='y = x')
    axes[1].set_xlabel('Profundidade Real (cents)')
    axes[1].set_ylabel('Profundidade Estimada (cents)')
    axes[1].set_title('Recuperação de Profundidade de Vibrato (cents)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e09_lfo_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    # Salvar resultados em JSON
    output_meta = {
        'stage': 'E09',
        'metrics': results,
        'isolation_metric_ratio': float(iso_ratio),
        'isolation_metric_db': float(iso_db),
        'promotion_passed': all(m['pass_promotion'] for m in results.values()) and iso_passed,
    }
    json_path = ROOT / 'experiments' / 'e09_lfo.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e09()
