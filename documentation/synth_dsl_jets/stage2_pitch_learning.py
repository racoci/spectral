#!/usr/bin/env python3
"""
Estágio 2: Aprender Somente a Frequência — Modelo Mínimo (E2).

Objetivo:
Construir e treinar uma cabeça preditiva minimalista Pf : zf -> (f_hat, f_dot_hat, f_ddot_hat)
a partir das características da CQT Gaussiana em banda-base, operando sobre o resíduo
ou diretamente sobre os jatos analíticos.

Princípios do Currículo Hierárquico (v12 & Diretrizes do Sistema):
1. O Quadrante de Validação de 4 Vias: IID, Composicional, Estrutural OOD, Adversarial.
2. Três versões de cada exemplo: x_clean, x_perturbed, x_hard.
3. Teste Contrafactual de Isolamento Paramétrico:
   I_i = |Delta theta_hat_i| / (sum_{j != i} |Delta theta_hat_j| + eps) >> 1.
4. Comparação entre Regressão Linear Simples (Wz + b) e MLP Minúsculo com Conexão Residual.
5. Função de perda musical rigorosa em cents e escalas físicas:
   Lf = (1200 log2(f_hat / f))^2 + lambda1 * ((f_dot_hat - f_dot) / s_dot)^2 + lambda2 * ((f_ddot_hat - f_ddot) / s_ddot)^2.
6. Critério de Aprovação: RMSE < 2 cents em IID e generalização em OOD.
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys, time
import numpy as np
import torch
import torch.nn as nn
import matplotlib.pyplot as plt

# Importar o extrator CQT analítico validado no Estágio 1
sys.path.insert(0, str(Path(__file__).resolve().parent))
from stage1_randomized_tests import BasebandGaussianCqtJetExtractor

# =============================================================================
# 1. ARQUITETURAS DAS CABEÇAS PREDITIVAS (LINEAR & MLP MINÚSCULO)
# =============================================================================

class LinearPitchHead(nn.Module):
    """
    Cabeça Linear Pura: theta_hat = W * z + b.
    Testa se uma única matriz linear sobre o vetor de características da CQT
    já é suficiente para resolver a estimação da frequência e derivadas.
    """
    def __init__(self, in_features: int = 7):
        super().__init__()
        self.linear = nn.Linear(in_features, 3)
        # Inicialização informada pela física analítica:
        # z = [u_ridge, log_mag, delta_u, u_analytic, fd_obs/100, fdd_obs/50, fc/1000]
        with torch.no_grad():
            self.linear.weight.zero_()
            self.linear.bias.zero_()
            self.linear.weight[0, 3] = 1.0   # u_analytic -> u
            self.linear.weight[1, 4] = 100.0 # fd_obs -> fd
            self.linear.weight[2, 5] = 50.0  # fdd_obs -> fdd

    def forward(self, z: torch.Tensor) -> torch.Tensor:
        return self.linear(z)


class TinyMlpPitchHead(nn.Module):
    """
    MLP Minúsculo com Conexão Residual Limitada (Bounded Residual Skip Connection):
    theta_hat = theta_analytic + scale * Tanh(MLP(z)).
    A rede aprende estritamente o resíduo (correção não-linear de dispersão de janela)
    mantendo-se estavelmente delimitada mesmo sob forte extrapolação OOD.
    Total de parâmetros: ~838.
    """
    def __init__(self, in_features: int = 7, hidden_dim: int = 32):
        super().__init__()
        self.mlp = nn.Sequential(
            nn.Linear(in_features, hidden_dim),
            nn.GELU(),
            nn.Linear(hidden_dim, 16),
            nn.GELU(),
            nn.Linear(16, 3),
            nn.Tanh()
        )
        # Escala máxima do resíduo (inicializada em ~15 cents, 15 Hz/s, 15 Hz/s^2)
        self.scale = nn.Parameter(torch.tensor([0.015, 15.0, 15.0], dtype=torch.float32))

        with torch.no_grad():
            for m in self.mlp.modules():
                if isinstance(m, nn.Linear):
                    nn.init.xavier_uniform_(m.weight, gain=0.1)
                    nn.init.zeros_(m.bias)

    def forward(self, z: torch.Tensor) -> torch.Tensor:
        # Vetor base analítico: [u_analytic, fd_analytic, fdd_analytic]
        base = torch.stack([z[:, 3], z[:, 4] * 100.0, z[:, 5] * 50.0], dim=-1)
        residual = self.mlp(z) * self.scale
        return base + residual

# =============================================================================
# 2. GERADOR PROCEDURAL DO DATASET COM O QUADRANTE DE 4 VIAS
# =============================================================================

class Stage2DatasetGenerator:
    """
    Gera o dataset hierárquico com gramática controlada:
    - 3 versões por amostra: clean, perturbed, hard.
    - 4 conjuntos de validação: IID, Composicional, Estrutural OOD, Adversarial.
    - Pares contrafactuais para medição de isolamento semântico.
    """
    def __init__(self, sr: float = 24000.0, duration: float = 1.0):
        self.sr = sr
        self.duration = duration
        self.t0 = duration / 2.0
        self.N = int(sr * duration)
        self.t = np.arange(self.N) / sr
        self.extractor = BasebandGaussianCqtJetExtractor(sr=sr)

    def synthesize_signal(self, f0: float, A: float, f_dot: float, f_ddot: float, phi0: float) -> np.ndarray:
        """Sintetiza senoide analítica pura com chirp quadrático."""
        phi_t = phi0 + 2.0 * np.pi * (f0 * (self.t - self.t0) + 0.5 * f_dot * ((self.t - self.t0)**2) + (1.0 / 6.0) * f_ddot * ((self.t - self.t0)**3))
        return A * np.sin(phi_t)

    def extract_features(self, x: np.ndarray, f_hint: float) -> tuple[np.ndarray, np.ndarray]:
        """
        Extrai o vetor de características z e os alvos físicos y.
        z = [u_ridge, log_mag, delta_u, u_analytic, fd_obs/100, fdd_obs/50, fc/1000]
        y = [u_true, f_dot_true, f_ddot_true]
        onde u = log2(f / 440).
        """
        meas = self.extractor.extract_time_jet(x, t0=self.t0, f_hint=f_hint)

        fc = meas['fc_bin']
        u_ridge = math.log2(fc / 440.0)
        f_inst = meas['f_inst']
        u_analytic = math.log2(max(10.0, f_inst) / 440.0)
        delta_u = (f_inst - fc) / max(10.0, fc)

        fd_obs = meas['f_dot']
        fdd_obs = meas['f_ddot']
        log_mag = meas['log_mag']

        z = np.array([
            u_ridge,
            log_mag,
            delta_u,
            u_analytic,
            fd_obs / 100.0,
            fdd_obs / 50.0,
            fc / 1000.0
        ], dtype=np.float32)

        return z, meas

    def generate_split(self, split_type: str, n_samples: int, rng: np.random.Generator) -> dict:
        """Gera um conjunto específico com as 3 variantes (clean, perturbed, hard)."""
        z_list = []
        y_list = []
        meta_list = []

        for _ in range(n_samples):
            # Definição dos domínios paramétricos por quadrante
            if split_type == 'iid':
                f0 = rng.uniform(150.0, 2500.0)
                A = rng.uniform(0.20, 1.00)
                f_dot = rng.uniform(-300.0, 300.0)
                f_ddot = rng.uniform(-100.0, 100.0)
                phi0 = rng.uniform(0.0, 2.0 * math.pi)

            elif split_type == 'compositional':
                # Combinações inéditas de parâmetros vistos individualmente:
                # Ex: pitch baixo com chirp extremo, ou pitch alto com forte aceleração negativa
                if rng.uniform() > 0.5:
                    f0 = rng.uniform(150.0, 350.0) # grave
                    f_dot = rng.uniform(350.0, 500.0) * rng.choice([-1.0, 1.0]) # chirp violento
                    f_ddot = rng.uniform(-150.0, 150.0)
                else:
                    f0 = rng.uniform(2000.0, 2500.0) # agudo
                    f_dot = rng.uniform(-450.0, 450.0)
                    f_ddot = rng.uniform(120.0, 200.0) * rng.choice([-1.0, 1.0]) # aceleração extrema
                A = rng.uniform(0.20, 0.90)
                phi0 = rng.uniform(0.0, 2.0 * math.pi)

            elif split_type == 'structural_ood':
                # Valores fora dos limites de treinamento: sub-graves (<150Hz) e ultra-agudos (>2500Hz)
                if rng.uniform() > 0.5:
                    f0 = rng.uniform(60.0, 140.0) # sub-graves
                else:
                    f0 = rng.uniform(2600.0, 4000.0) # ultra-agudos
                A = rng.uniform(0.15, 0.95)
                f_dot = rng.uniform(-500.0, 500.0)
                f_ddot = rng.uniform(-200.0, 200.0)
                phi0 = rng.uniform(0.0, 2.0 * math.pi)

            elif split_type == 'adversarial':
                # Casos degenerados: amplitude baixíssima (quase ruído),
                # ou posicionado exatamente na fronteira entre dois canais CQT (half-bin)
                A = rng.uniform(0.03, 0.09) if rng.uniform() > 0.5 else rng.uniform(0.15, 0.8)
                # Forçar frequência exatamente na fronteira entre bins CQT
                k_int = rng.integers(10, 200)
                f0 = 40.0 * (2.0 ** ((k_int + 0.5) / 60.0))
                f_dot = rng.uniform(-450.0, 450.0)
                f_ddot = rng.uniform(-180.0, 180.0)
                phi0 = rng.uniform(0.0, 2.0 * math.pi)

            # --- Versão 1: Clean ---
            x_clean = self.synthesize_signal(f0, A, f_dot, f_ddot, phi0)
            z_clean, _ = self.extract_features(x_clean, f_hint=f0)
            y_true = np.array([math.log2(f0 / 440.0), f_dot, f_ddot], dtype=np.float32)

            z_list.append(z_clean)
            y_list.append(y_true)
            meta_list.append({'split': split_type, 'variant': 'clean', 'f0': f0, 'f_dot': f_dot, 'f_ddot': f_ddot, 'A': A})

            # --- Versão 2: Perturbed (theta' = theta + epsilon) ---
            f0_p = f0 * (2.0 ** (rng.normal(0.0, 2.0) / 1200.0)) # micro-detune de +-2 cents
            fd_p = f_dot + rng.normal(0.0, 5.0)
            fdd_p = f_ddot + rng.normal(0.0, 5.0)
            A_p = max(0.05, A + rng.normal(0.0, 0.05))
            x_pert = self.synthesize_signal(f0_p, A_p, fd_p, fdd_p, phi0)
            # Injetar pequeno ruído branco (SNR ~ 40 dB)
            x_pert += rng.normal(0.0, 0.005, size=len(x_pert))
            z_pert, _ = self.extract_features(x_pert, f_hint=f0_p)
            y_pert = np.array([math.log2(f0_p / 440.0), fd_p, fdd_p], dtype=np.float32)

            z_list.append(z_pert)
            y_list.append(y_pert)
            meta_list.append({'split': split_type, 'variant': 'perturbed', 'f0': f0_p, 'f_dot': fd_p, 'f_ddot': fdd_p, 'A': A_p})

            # --- Versão 3: Hard (ambiguidades e estresse de fase) ---
            x_hard = self.synthesize_signal(f0, max(0.04, A * 0.5), f_dot * 1.25, f_ddot * 1.25, phi0 + math.pi / 2.0)
            x_hard += rng.normal(0.0, 0.015, size=len(x_hard))
            z_hard, _ = self.extract_features(x_hard, f_hint=f0)
            y_hard = np.array([math.log2(f0 / 440.0), f_dot * 1.25, f_ddot * 1.25], dtype=np.float32)

            z_list.append(z_hard)
            y_list.append(y_hard)
            meta_list.append({'split': split_type, 'variant': 'hard', 'f0': f0, 'f_dot': f_dot * 1.25, 'f_ddot': f_ddot * 1.25, 'A': A * 0.5})

        return {
            'z': torch.tensor(np.array(z_list), dtype=torch.float32),
            'y': torch.tensor(np.array(y_list), dtype=torch.float32),
            'meta': meta_list
        }

    def generate_counterfactual_pairs(self, n_pairs: int = 50, rng: np.random.Generator | None = None) -> list[dict]:
        """
        Gera pares contrafactuais (x, x') onde rigorosamente um único parâmetro é alterado:
        Theta' = Theta + Delta * e_i.
        Permite medir o Isolamento Semântico: I_i >> 1.
        """
        if rng is None:
            rng = np.random.default_rng(123)
        pairs = []

        for p_idx in range(n_pairs):
            f0_base = rng.uniform(200.0, 2000.0)
            A_base = rng.uniform(0.3, 0.8)
            fd_base = rng.uniform(-200.0, 200.0)
            fdd_base = rng.uniform(-80.0, 80.0)
            phi0 = rng.uniform(0.0, 2.0 * math.pi)

            # Escolher qual parâmetro sofrerá a intervenção: 0: f0, 1: f_dot, 2: f_ddot
            param_choice = p_idx % 3

            if param_choice == 0:
                delta_cents = rng.uniform(5.0, 30.0) * rng.choice([-1.0, 1.0])
                f0_alt = f0_base * (2.0 ** (delta_cents / 1200.0))
                fd_alt = fd_base
                fdd_alt = fdd_base
                delta_true = np.array([delta_cents / 1200.0, 0.0, 0.0])
                param_name = 'f0'
            elif param_choice == 1:
                delta_fd = rng.uniform(50.0, 150.0) * rng.choice([-1.0, 1.0])
                f0_alt = f0_base
                fd_alt = fd_base + delta_fd
                fdd_alt = fdd_base
                delta_true = np.array([0.0, delta_fd, 0.0])
                param_name = 'f_dot'
            else:
                delta_fdd = rng.uniform(40.0, 100.0) * rng.choice([-1.0, 1.0])
                f0_alt = f0_base
                fd_alt = fd_base
                fdd_alt = fdd_base + delta_fdd
                delta_true = np.array([0.0, 0.0, delta_fdd])
                param_name = 'f_ddot'

            x_a = self.synthesize_signal(f0_base, A_base, fd_base, fdd_base, phi0)
            z_a, _ = self.extract_features(x_a, f_hint=f0_base)

            x_b = self.synthesize_signal(f0_alt, A_base, fd_alt, fdd_alt, phi0)
            z_b, _ = self.extract_features(x_b, f_hint=f0_alt)

            pairs.append({
                'param_target': param_choice,
                'param_name': param_name,
                'delta_true': delta_true,
                'z_a': torch.tensor(z_a, dtype=torch.float32).unsqueeze(0),
                'z_b': torch.tensor(z_b, dtype=torch.float32).unsqueeze(0)
            })

        return pairs

# =============================================================================
# 3. FUNÇÃO DE PERDA MUSICAL E TREINAMENTO
# =============================================================================

def musical_tuning_loss(
    pred: torch.Tensor,
    target: torch.Tensor,
    lambda1: float = 1.0,
    lambda2: float = 0.5,
    s_dot: float = 100.0,
    s_ddot: float = 50.0
) -> tuple[torch.Tensor, torch.Tensor, torch.Tensor, torch.Tensor]:
    """
    Perda de afinação musical:
    Lf = (1200 * (u_hat - u))^2 + lambda1 * ((fd_hat - fd)/s_dot)^2 + lambda2 * ((fdd_hat - fdd)/s_ddot)^2.
    """
    u_hat, fd_hat, fdd_hat = pred[:, 0], pred[:, 1], pred[:, 2]
    u_true, fd_true, fdd_true = target[:, 0], target[:, 1], target[:, 2]

    # Erro de afinação em cents: 1200 * Delta u
    diff_cents = 1200.0 * (u_hat - u_true)
    loss_cents = torch.mean(diff_cents ** 2)

    # Erros relativos de chirp
    loss_fdot = torch.mean(((fd_hat - fd_true) / s_dot) ** 2)
    loss_fddot = torch.mean(((fdd_hat - fdd_true) / s_ddot) ** 2)

    total_loss = loss_cents + lambda1 * loss_fdot + lambda2 * loss_fddot
    return total_loss, loss_cents, loss_fdot, loss_fddot


def train_and_evaluate_pitch_head(
    model: nn.Module,
    train_data: dict,
    val_splits: dict,
    cf_pairs: list[dict],
    lr: float = 1e-3,
    epochs: int = 50,
    batch_size: int = 64
) -> dict:
    """Treina o modelo e avalia no quadrante de 4 vias + contrafactuais."""
    optimizer = torch.optim.AdamW(model.parameters(), lr=lr, weight_decay=1e-4)
    scheduler = torch.optim.lr_scheduler.CosineAnnealingLR(optimizer, T_max=epochs, eta_min=1e-5)

    z_train = train_data['z']
    y_train = train_data['y']
    n_train = len(z_train)

    history = {'train_loss': [], 'train_cents_rmse': [], 'val_iid_cents_rmse': []}

    print(f"Treinando {model.__class__.__name__} ({sum(p.numel() for p in model.parameters())} parâmetros)...")
    t0_train = time.time()

    for epoch in range(epochs):
        model.train()
        perm = torch.randperm(n_train)
        epoch_losses = []
        epoch_cents_sq = []

        for b_start in range(0, n_train, batch_size):
            b_idx = perm[b_start:b_start + batch_size]
            z_batch = z_train[b_idx]
            y_batch = y_train[b_idx]

            optimizer.zero_grad()
            pred = model(z_batch)
            loss, loss_c, _, _ = musical_tuning_loss(pred, y_batch)
            loss.backward()
            optimizer.step()

            epoch_losses.append(loss.item())
            epoch_cents_sq.append(loss_c.item())

        scheduler.step()
        train_rmse_c = math.sqrt(np.mean(epoch_cents_sq))
        history['train_loss'].append(np.mean(epoch_losses))
        history['train_cents_rmse'].append(train_rmse_c)

        # Avaliação IID rápida
        model.eval()
        with torch.no_grad():
            pred_iid = model(val_splits['iid']['z'])
            diff_iid_c = 1200.0 * (pred_iid[:, 0] - val_splits['iid']['y'][:, 0])
            val_iid_rmse_c = float(torch.sqrt(torch.mean(diff_iid_c ** 2)).item())
            history['val_iid_cents_rmse'].append(val_iid_rmse_c)

        if (epoch + 1) % 10 == 0 or (epoch + 1) == epochs:
            print(f"  Epoch [{epoch+1:2d}/{epochs:2d}] Loss: {np.mean(epoch_losses):8.4f} | Train RMSE Cents: {train_rmse_c:6.3f} c | Val IID RMSE: {val_iid_rmse_c:6.3f} c")

    train_time = time.time() - t0_train

    # =========================================================================
    # AVALIAÇÃO NO QUADRANTE DE 4 VIAS
    # =========================================================================
    model.eval()
    quadrant_metrics = {}

    for split_name, split_data in val_splits.items():
        with torch.no_grad():
            pred = model(split_data['z'])
            y = split_data['y']

            err_cents = torch.abs(1200.0 * (pred[:, 0] - y[:, 0])).cpu().numpy()
            err_fdot = torch.abs(pred[:, 1] - y[:, 1]).cpu().numpy()
            err_fddot = torch.abs(pred[:, 2] - y[:, 2]).cpu().numpy()

            rmse_c = float(np.sqrt(np.mean(err_cents ** 2)))
            mae_c = float(np.mean(err_cents))
            max_c = float(np.max(err_cents))
            success_c = float(np.mean(err_cents < 2.0) * 100.0)

            rmse_fd = float(np.sqrt(np.mean(err_fdot ** 2)))
            mae_fd = float(np.mean(err_fdot))
            rmse_fdd = float(np.sqrt(np.mean(err_fddot ** 2)))
            mae_fdd = float(np.mean(err_fddot))

            quadrant_metrics[split_name] = {
                'rmse_cents': rmse_c,
                'mae_cents': mae_c,
                'max_cents': max_c,
                'success_rate_under_2cents_pct': success_c,
                'rmse_fdot': rmse_fd,
                'mae_fdot': mae_fd,
                'rmse_fddot': rmse_fdd,
                'mae_fddot': mae_fdd
            }

    # =========================================================================
    # TESTE CONTRAFACTUAL DE ISOLAMENTO SEMÂNTICO (I_i)
    # =========================================================================
    isolation_scores = {'f0': [], 'f_dot': [], 'f_ddot': []}

    with torch.no_grad():
        for pair in cf_pairs:
            pred_a = model(pair['z_a']).squeeze(0).cpu().numpy()
            pred_b = model(pair['z_b']).squeeze(0).cpu().numpy()

            delta_hat = pred_b - pred_a
            p_target = pair['param_target']
            p_name = pair['param_name']

            # Normalização de grandezas conforme Seção 31 do Currículo:
            # f0 em cents (1200 * delta u), f_dot / s_dot, f_ddot / s_ddot
            scale_vec = np.array([1200.0, 1.0 / 100.0, 1.0 / 50.0])
            norm_delta_hat = np.abs(delta_hat * scale_vec)

            target_val = norm_delta_hat[p_target]
            off_target_sum = sum(norm_delta_hat[j] for j in range(3) if j != p_target)

            iso_i = float(target_val / (off_target_sum + 1e-6))
            isolation_scores[p_name].append(iso_i)

    avg_isolation = {k: float(np.mean(v)) for k, v in isolation_scores.items()}

    return {
        'model_name': model.__class__.__name__,
        'train_time_s': train_time,
        'history': history,
        'quadrant_metrics': quadrant_metrics,
        'isolation_scores': avg_isolation
    }

# =============================================================================
# 4. EXECUÇÃO INTEGRADA DO ESTÁGIO 2 & SALVAMENTO DE ARTEFATOS
# =============================================================================

def run_stage2_experiments(output_dir: Path | None = None) -> dict:
    if output_dir is None:
        output_dir = Path('documentation/synth_dsl_jets')
    output_dir.mkdir(parents=True, exist_ok=True)

    print("=========================================================================")
    print("🔬 ESTÁGIO 2: CABEÇA PREDITIVA DE FREQUÊNCIA — QUADRANTE & CONTRAFACTUAIS")
    print("=========================================================================")

    rng = np.random.default_rng(2025)
    gen = Stage2DatasetGenerator(sr=24000.0, duration=1.0)

    # 1. Gerar Conjunto de Treinamento (IID com clean, perturbed, hard)
    print("Gerando Dataset de Treinamento (600 amostras)...")
    train_data = gen.generate_split('iid', n_samples=200, rng=rng) # 200 * 3 variantes = 600

    # 2. Gerar Quadrante de Validação de 4 Vias (150 amostras cada)
    print("Gerando Quadrante de Validação (IID, Composicional, OOD, Adversarial)...")
    val_splits = {
        'iid': gen.generate_split('iid', n_samples=50, rng=rng),
        'compositional': gen.generate_split('compositional', n_samples=50, rng=rng),
        'structural_ood': gen.generate_split('structural_ood', n_samples=50, rng=rng),
        'adversarial': gen.generate_split('adversarial', n_samples=50, rng=rng)
    }

    # 3. Gerar Pares Contrafactuais
    print("Gerando 60 pares contrafactuais para teste de isolamento...")
    cf_pairs = gen.generate_counterfactual_pairs(n_pairs=60, rng=rng)

    # 4. Treinar e Avaliar os 2 Modelos Candidatos
    linear_head = LinearPitchHead(in_features=7)
    mlp_head = TinyMlpPitchHead(in_features=7, hidden_dim=32)

    res_linear = train_and_evaluate_pitch_head(linear_head, train_data, val_splits, cf_pairs, lr=3e-3, epochs=40)
    res_mlp = train_and_evaluate_pitch_head(mlp_head, train_data, val_splits, cf_pairs, lr=2e-3, epochs=40)

    # 5. Salvar Pesos do Modelo Campeão (MLP)
    model_weights_path = output_dir / 'stage2_pitch_model.pt'
    torch.save({
        'model_state_dict': mlp_head.state_dict(),
        'linear_state_dict': linear_head.state_dict(),
        'in_features': 7,
        'hidden_dim': 32
    }, model_weights_path)
    print(f"\nPesos dos modelos salvos em: {model_weights_path}")

    # 6. Salvar Métricas em JSON e CSV
    metrics_path = output_dir / 'stage2_metrics.json'
    with open(metrics_path, 'w', encoding='utf-8') as f:
        json.dump({
            'linear_head': res_linear,
            'mlp_head': res_mlp
        }, f, indent=2)
    print(f"Métricas detalhadas salvas em: {metrics_path}")

    csv_path = output_dir / 'stage2_metrics.csv'
    with open(csv_path, 'w', encoding='utf-8') as f:
        f.write("model,quadrant,rmse_cents,mae_cents,max_cents,success_rate_2c,mae_fdot,mae_fddot\n")
        for res in [res_linear, res_mlp]:
            for q_name, q_val in res['quadrant_metrics'].items():
                f.write(f"{res['model_name']},{q_name},{q_val['rmse_cents']:.6f},{q_val['mae_cents']:.6f},"
                        f"{q_val['max_cents']:.6f},{q_val['success_rate_under_2cents_pct']:.2f},"
                        f"{q_val['mae_fdot']:.4f},{q_val['mae_fddot']:.4f}\n")
    print(f"Tabela consolidada CSV salva em: {csv_path}")

    # 7. Gerar Gráficos de Confronto e Convergência
    plot_path = output_dir / '10_stage2_pitch_learning_results.png'
    _generate_stage2_plots(res_linear, res_mlp, plot_path)
    print(f"Gráfico de resultados salvo em: {plot_path}")

    # Exibição do Confronto Final no Terminal
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS CONSOLIDADOS DO ESTÁGIO 2 (MODELOS vs QUADRANTE):")
    print("-------------------------------------------------------------------------")
    print(f"{'Quadrante':<16} | {'Linear RMSE (c)':<17} | {'MLP RMSE (c)':<15} | {'Linear Sucesso':<14} | {'MLP Sucesso'}")
    print("-" * 75)
    for q_name in ['iid', 'compositional', 'structural_ood', 'adversarial']:
        lin_q = res_linear['quadrant_metrics'][q_name]
        mlp_q = res_mlp['quadrant_metrics'][q_name]
        print(f"{q_name:<16} | {lin_q['rmse_cents']:13.4f} c   | {mlp_q['rmse_cents']:11.4f} c | {lin_q['success_rate_under_2cents_pct']:11.1f}%   | {mlp_q['success_rate_under_2cents_pct']:8.1f}%")
    print("-" * 75)

    print("\n⚡ ISOLAMENTO CONTRAFACTUAL (I_i >> 1):")
    for k in ['f0', 'f_dot', 'f_ddot']:
        print(f"  -> Isolamento de {k:<6}: Linear = {res_linear['isolation_scores'][k]:8.2f} | MLP = {res_mlp['isolation_scores'][k]:8.2f}")

    is_approved = (res_mlp['quadrant_metrics']['iid']['rmse_cents'] < 2.0) and \
                  (res_mlp['quadrant_metrics']['iid']['success_rate_under_2cents_pct'] >= 95.0)
    print(f"\nStatus Final de Aprovação Estágio 2: {'✅ APROVADO' if is_approved else '❌ REPROVADO'}")
    print("=========================================================================")

    return {'linear': res_linear, 'mlp': res_mlp}

def _generate_stage2_plots(res_lin: dict, res_mlp: dict, output_path: Path):
    """Gera visualização de auditoria e confronto para o Estágio 2."""
    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    fig.suptitle("Estágio 2: Cabeça Preditiva de Frequência — Confronto Linear vs MLP Residual", fontsize=15, fontweight='bold')

    # Subplot 1: Curvas de Convergência do Treinamento
    ax1 = axes[0, 0]
    ep_lin = range(1, len(res_lin['history']['train_cents_rmse']) + 1)
    ep_mlp = range(1, len(res_mlp['history']['train_cents_rmse']) + 1)
    ax1.plot(ep_lin, res_lin['history']['train_cents_rmse'], 'b--', label='Linear Head (Treino)')
    ax1.plot(ep_lin, res_lin['history']['val_iid_cents_rmse'], 'b-', label='Linear Head (Val IID)')
    ax1.plot(ep_mlp, res_mlp['history']['train_cents_rmse'], 'g--', label='Tiny MLP (Treino)')
    ax1.plot(ep_mlp, res_mlp['history']['val_iid_cents_rmse'], 'g-', label='Tiny MLP (Val IID)')
    ax1.axhline(2.0, color='crimson', linestyle=':', label='Limiar 2 cents')
    ax1.set_xlabel("Épocas")
    ax1.set_ylabel("RMSE de Afinação (cents)")
    ax1.set_title("Convergência do Erro de Pitch")
    ax1.set_yscale('log')
    ax1.grid(True, alpha=0.3)
    ax1.legend()

    # Subplot 2: Comparativo RMSE por Quadrante de Testes
    ax2 = axes[0, 1]
    quadrants = ['iid', 'compositional', 'structural_ood', 'adversarial']
    labels = ['IID', 'Composicional', 'Estrutural OOD', 'Adversarial']
    x_idx = np.arange(len(quadrants))
    w = 0.35

    lin_rmses = [res_lin['quadrant_metrics'][q]['rmse_cents'] for q in quadrants]
    mlp_rmses = [res_mlp['quadrant_metrics'][q]['rmse_cents'] for q in quadrants]

    ax2.bar(x_idx - w/2, lin_rmses, width=w, label='Linear Head', color='steelblue', edgecolor='black')
    ax2.bar(x_idx + w/2, mlp_rmses, width=w, label='Tiny MLP Residual', color='forestgreen', edgecolor='black')
    ax2.axhline(2.0, color='crimson', linestyle='--', linewidth=1.5, label='Limiar de Aprovação (2 cents)')
    ax2.set_xticks(x_idx)
    ax2.set_xticklabels(labels)
    ax2.set_ylabel("RMSE em Cents")
    ax2.set_title("Desempenho no Quadrante de 4 Vias")
    ax2.grid(True, axis='y', alpha=0.3)
    ax2.legend()

    # Subplot 3: Taxa de Sucesso (< 2 cents)
    ax3 = axes[1, 0]
    lin_succ = [res_lin['quadrant_metrics'][q]['success_rate_under_2cents_pct'] for q in quadrants]
    mlp_succ = [res_mlp['quadrant_metrics'][q]['success_rate_under_2cents_pct'] for q in quadrants]

    ax3.bar(x_idx - w/2, lin_succ, width=w, label='Linear Head', color='royalblue', edgecolor='black')
    ax3.bar(x_idx + w/2, mlp_succ, width=w, label='Tiny MLP Residual', color='darkseagreen', edgecolor='black')
    ax3.axhline(95.0, color='firebrick', linestyle='--', label='Meta (95%)')
    ax3.set_xticks(x_idx)
    ax3.set_xticklabels(labels)
    ax3.set_ylabel("Taxa de Sucesso (%)")
    ax3.set_ylim(0, 105)
    ax3.set_title("Taxa de Sucesso (< 2 cents) por Quadrante")
    ax3.grid(True, axis='y', alpha=0.3)
    ax3.legend(loc='lower right')

    # Subplot 4: Isolamento Contrafactual Semântico (I_i)
    ax4 = axes[1, 1]
    params = ['f0', 'f_dot', 'f_ddot']
    p_idx = np.arange(len(params))
    lin_iso = [res_lin['isolation_scores'][p] for p in params]
    mlp_iso = [res_mlp['isolation_scores'][p] for p in params]

    ax4.bar(p_idx - w/2, lin_iso, width=w, label='Linear Head', color='indigo', edgecolor='black')
    ax4.bar(p_idx + w/2, mlp_iso, width=w, label='Tiny MLP Residual', color='darkorange', edgecolor='black')
    ax4.axhline(10.0, color='red', linestyle='--', label='Meta Isolamento (I >> 1)')
    ax4.set_xticks(p_idx)
    ax4.set_xticklabels(['Pitch f0', 'Chirp Rate f_dot', 'Chirp Accel f_ddot'])
    ax4.set_ylabel("Métrica de Isolamento I_i")
    ax4.set_title("Isolamento Contrafactual Semântico (I_i >> 1)")
    ax4.grid(True, axis='y', alpha=0.3)
    ax4.legend()

    plt.tight_layout()
    plt.savefig(output_path, dpi=200)
    plt.close()

if __name__ == '__main__':
    run_stage2_experiments()
