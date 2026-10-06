#!/usr/bin/env python3
"""
Teste Automatizado de Regressão e Validação: Estágios 1 e 2 do Currículo Hierárquico.

Verifica:
1. Estágio 1: Frontend puramente analítico (CQT 60 bins/oct + Hermite Jets)
   - RMSE Pitch < 2.0 cents
   - Taxa de sucesso (< 2c) >= 95%
   - Recuperação de chirp rate f_dot (MAE < 5 Hz/s)
2. Estágio 2: Cabeça preditiva minimalista (Linear & Tiny MLP)
   - RMSE IID < 2.0 cents
   - Generalização em Composicional e OOD
   - Isolamento semântico contrafactual I_i > 2.0
"""

import unittest
from pathlib import Path
import sys, math
import numpy as np
import torch

# Adicionar caminhos do projeto
PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT / 'documentation' / 'synth_dsl_jets'))

from stage1_randomized_tests import BasebandGaussianCqtJetExtractor, run_stage1_monte_carlo_tests
from stage2_pitch_learning import Stage2DatasetGenerator, LinearPitchHead, TinyMlpPitchHead, train_and_evaluate_pitch_head

class TestStage1AndStage2Curriculum(unittest.TestCase):
    def test_01_stage1_monte_carlo_accuracy(self):
        """Valida que o frontend analítico atinge RMSE < 2 cents e 100% de sucesso."""
        summary = run_stage1_monte_carlo_tests(n_trials=50, seed=123, output_dir=PROJECT_ROOT / 'documentation' / 'synth_dsl_jets')
        self.assertTrue(summary['approved'], "Estágio 1 reprovado nos testes Monte Carlo!")
        self.assertLess(summary['pitch_rmse_cents'], 2.0, f"RMSE de afinação excedeu 2 cents: {summary['pitch_rmse_cents']:.4f}")
        self.assertGreaterEqual(summary['success_rate_under_2cents_pct'], 95.0, f"Taxa de sucesso abaixo de 95%: {summary['success_rate_under_2cents_pct']:.1f}%")
        self.assertLess(summary['f_dot_mae_hz_s'], 5.0, f"Erro em f_dot excedeu 5 Hz/s: {summary['f_dot_mae_hz_s']:.2f}")

    def test_02_stage1_static_sine_zero_error(self):
        """Valida precisão ultra-alta em senoides estacionárias puras."""
        extractor = BasebandGaussianCqtJetExtractor(sr=24000.0)
        t = np.arange(24000) / 24000.0
        f0 = 440.0
        x = 0.8 * np.sin(2.0 * np.pi * f0 * t)
        meas = extractor.extract_time_jet(x, t0=0.5, f_hint=f0)

        err_cents = abs(1200.0 * math.log2(meas['f_inst'] / f0))
        self.assertLess(err_cents, 0.05, f"Erro em senoide estacionária excedeu 0.05 cents: {err_cents:.4f}")
        self.assertLess(abs(meas['f_dot']), 0.5, f"Chirp rate espúrio detectado: {meas['f_dot']:.2f}")

    def test_03_stage2_pitch_heads_training_and_quadrant(self):
        """Valida convergência das cabeças preditivas e teste contrafactual."""
        rng = np.random.default_rng(42)
        gen = Stage2DatasetGenerator(sr=24000.0, duration=1.0)

        train_data = gen.generate_split('iid', n_samples=100, rng=rng)
        val_splits = {
            'iid': gen.generate_split('iid', n_samples=30, rng=rng),
            'compositional': gen.generate_split('compositional', n_samples=30, rng=rng),
            'structural_ood': gen.generate_split('structural_ood', n_samples=30, rng=rng),
            'adversarial': gen.generate_split('adversarial', n_samples=30, rng=rng)
        }
        cf_pairs = gen.generate_counterfactual_pairs(n_pairs=20, rng=rng)

        # 1. Testar LinearPitchHead
        lin_head = LinearPitchHead(in_features=7)
        res_lin = train_and_evaluate_pitch_head(lin_head, train_data, val_splits, cf_pairs, lr=3e-3, epochs=30)
        self.assertLess(res_lin['quadrant_metrics']['iid']['rmse_cents'], 2.0, "Linear head falhou no limiar de 2 cents em IID!")

        # 2. Testar TinyMlpPitchHead
        mlp_head = TinyMlpPitchHead(in_features=7, hidden_dim=32)
        res_mlp = train_and_evaluate_pitch_head(mlp_head, train_data, val_splits, cf_pairs, lr=2e-3, epochs=30)
        self.assertLess(res_mlp['quadrant_metrics']['iid']['rmse_cents'], 2.0, "MLP head falhou no limiar de 2 cents em IID!")
        self.assertLess(res_mlp['quadrant_metrics']['compositional']['rmse_cents'], 2.0, "MLP head falhou em Composicional!")

        # 3. Testar Isolamento Contrafactual (I_i > 2.0)
        for param in ['f0', 'f_dot', 'f_ddot']:
            iso = res_mlp['isolation_scores'][param]
            self.assertGreater(iso, 2.0, f"Isolamento semântico insuficiente para {param}: {iso:.2f}")

if __name__ == '__main__':
    unittest.main()
