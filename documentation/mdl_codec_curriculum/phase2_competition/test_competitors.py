#!/usr/bin/env python3
import unittest
import numpy as np
import sys
from pathlib import Path

cur_dir = Path(__file__).resolve().parent
sys.path.insert(0, str(cur_dir))
sys.path.insert(0, str(cur_dir.parents[0] / 'phase1_signal_laboratory'))

from competition_framework import CompetitionFramework, StructuralRepresentation, count_structural_parameters
from signal_laboratory import SignalLaboratory
from m1_cqt_ridge import M1_CQTRidge
from m1b_cqt_jet_ridge import M1b_CQTJetRidge
from m2_chirplet_ridge import M2_ChirpletRidge
from m3_matrix_pencil import M3_MatrixPencil
from m4_hankel_ssa import M4_HankelSSA
from m5_matching_pursuit import M5_MatchingPursuit
from m6_wavelet_maxima import M6_WaveletMaxima
from m7_afd import M7_AdaptiveFourierDecomposition
from m8_dmd import M8_DMD
from m9_vmd import M9_VMD

class TestFramework(unittest.TestCase):
    def setUp(self):
        self.sr = 12000.0
        self.lab = SignalLaboratory(sr=self.sr)
        self.x, self.truth = self.lab.e06_pure_tone(440.0, 1.0, 0.0, 0.1)

    def test_parameter_counting(self):
        d = {'a': 1.0, 'b': [2.0, 3.0], 'c': {'d': np.array([4.0, 5.0, 6.0])}}
        self.assertEqual(count_structural_parameters(d), 6)

    def test_framework_evaluation_serial_and_parallel(self):
        framework = CompetitionFramework(sr=self.sr)
        framework.register_competitor(M1b_CQTJetRidge(sr=self.sr))
        
        # Test serial
        res_serial = framework.evaluate_signal('E06', self.x, self.truth, parallel=False)
        self.assertIn('M1b_CQTJetRidge', res_serial)
        self.assertTrue(res_serial['M1b_CQTJetRidge']['success'])
        self.assertGreater(res_serial['M1b_CQTJetRidge']['sisdr_db'], 10.0)

        # Test parallel
        res_parallel = framework.evaluate_signal('E06', self.x, self.truth, parallel=True, max_workers=2)
        self.assertIn('M1b_CQTJetRidge', res_parallel)
        self.assertTrue(res_parallel['M1b_CQTJetRidge']['success'])

class TestBaselineCompetitors(unittest.TestCase):
    def setUp(self):
        self.sr = 12000.0
        self.lab = SignalLaboratory(sr=self.sr)
        self.x_pure, _ = self.lab.e06_pure_tone(440.0, 1.0, 0.0, 0.1)

    def test_m1_cqt_ridge(self):
        m1 = M1_CQTRidge(sr=self.sr)
        theta, x_hat = m1.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('f', theta)

    def test_m1b_cqt_jet_ridge(self):
        m1b = M1b_CQTJetRidge(sr=self.sr)
        theta, x_hat = m1b.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('f_inst', theta)

    def test_m2_chirplet_ridge(self):
        m2 = M2_ChirpletRidge(sr=self.sr)
        x_chirp, _ = self.lab.e08_linear_chirp(200.0, 400.0, 1.0, 0.0, 0.1)
        theta, x_hat = m2.fit_and_reconstruct(x_chirp)
        self.assertEqual(len(x_hat), len(x_chirp))
        self.assertIn('f_dot', theta)
        mse = np.mean((x_chirp - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(x_chirp**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 15.0)

    def test_m3_matrix_pencil(self):
        m3 = M3_MatrixPencil(sr=self.sr, K=2)
        theta, x_hat = m3.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('blocks', theta)

    def test_m4_hankel_ssa(self):
        m4 = M4_HankelSSA(sr=self.sr, window_len=64, rank=4)
        theta, x_hat = m4.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('singular_values', theta)
        mse = np.mean((self.x_pure - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(self.x_pure**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 15.0)

    def test_m5_matching_pursuit(self):
        m5 = M5_MatchingPursuit(sr=self.sr, max_iter=5, dict_size=50)
        theta, x_hat = m5.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('atoms', theta)

    def test_m6_wavelet_maxima(self):
        m6 = M6_WaveletMaxima(sr=self.sr, num_scales=8)
        x_click, _ = self.lab.e14_transient(0.05, 0.1, kind='click')
        theta, x_hat = m6.fit_and_reconstruct(x_click)
        self.assertEqual(len(x_hat), len(x_click))
        self.assertIn('total_maxima', theta)
        mse = np.mean((x_click - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(x_click**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 15.0)

    def test_m7_afd(self):
        m7 = M7_AdaptiveFourierDecomposition(sr=self.sr, num_atoms=8)
        theta, x_hat = m7.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('poles_re', theta)
        mse = np.mean((self.x_pure - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(self.x_pure**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 10.0)

    def test_m8_dmd(self):
        m8 = M8_DMD(sr=self.sr, window_len=48, rank=4)
        theta, x_hat = m8.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('frequencies_hz', theta)
        mse = np.mean((self.x_pure - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(self.x_pure**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 20.0)

    def test_m9_vmd(self):
        m9 = M9_VMD(sr=self.sr, K=2)
        theta, x_hat = m9.fit_and_reconstruct(self.x_pure)
        self.assertEqual(len(x_hat), len(self.x_pure))
        self.assertIn('center_freqs_hz', theta)
        mse = np.mean((self.x_pure - x_hat)**2)
        sisdr = 10 * np.log10(np.mean(self.x_pure**2) / max(mse, 1e-12))
        self.assertGreater(sisdr, 15.0)

if __name__ == '__main__':
    unittest.main()
