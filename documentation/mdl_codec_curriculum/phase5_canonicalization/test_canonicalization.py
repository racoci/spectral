#!/usr/bin/env python3
"""
Unit and Integration Tests for Phase 5: Canonicalization and Gauges (E39-E41).
Tests exact equivalence classes, canonical gauge projections, bit savings, and physical ambiguity distributions.
"""

import unittest
import numpy as np
import sys
from pathlib import Path

# Add project root and curriculum to path
CURRICULUM_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(CURRICULUM_DIR))
sys.path.insert(0, str(CURRICULUM_DIR / 'phase5_canonicalization'))
sys.path.insert(0, str(CURRICULUM_DIR / 'phase3_residual_cost'))

from equivalence_classes import (
    time_shift_vs_phase,
    fm_vs_pm_angle_modulation,
    filter_vs_spectral_envelope,
    polar_amplitude_phase,
    compute_gauge_jacobian_nullity
)

from canonical_gauge import (
    CanonicalTimePhase,
    CanonicalAngleModulation,
    CanonicalFilterEnvelope,
    canonicalize_parameter_vector,
    evaluate_canonical_mdl_savings
)

from ambiguity_distribution import (
    HarmonicCollisionAmbiguity,
    ResonatorVsNoiseAmbiguity,
    CausalLookaheadDisambiguator
)


class TestEquivalenceClasses(unittest.TestCase):
    """E39: Mathematical Verification of Exact Equivalence Classes D(theta_1) = D(theta_2)."""

    def setUp(self):
        self.sr = 16000.0
        self.duration = 0.1
        self.t = np.linspace(0, self.duration, int(self.sr * self.duration), endpoint=False)

    def test_time_shift_vs_phase(self):
        """Verify that time delay dt and phase offset dphi = -2*pi*f*dt are identical to machine precision."""
        f0 = 440.0
        dt = 0.0025  # 2.5 ms
        phi0 = 0.35
        
        diff, x_time, x_phase = time_shift_vs_phase(f0=f0, dt=dt, phi0=phi0, duration=self.duration, sr=self.sr)
        self.assertLess(diff, 1e-12, f"Time shift vs phase max diff {diff:.3e} exceeds 1e-12")
        np.testing.assert_allclose(x_time, x_phase, atol=1e-12)

    def test_fm_vs_pm(self):
        """Verify that FM with deviation df and PM with index I = df/fm are identical waveforms."""
        fc = 800.0
        fm = 15.0
        beta = 2.4
        psi = 0.65
        
        diff, x_fm, x_pm = fm_vs_pm_angle_modulation(fc=fc, fm=fm, beta=beta, psi=psi, duration=self.duration, sr=self.sr)
        self.assertEqual(diff, 0.0, f"FM vs PM max diff {diff} is non-zero")
        np.testing.assert_array_equal(x_fm, x_pm)

    def test_filter_vs_spectral_envelope(self):
        """Verify that LTI filter convolution and harmonic envelope weighting are identical."""
        f0 = 220.0
        num_harmonics = 8
        cutoff_hz = 1200.0
        
        diff, y_filt, y_env = filter_vs_spectral_envelope(
            f0=f0, num_harmonics=num_harmonics, cutoff_hz=cutoff_hz, duration=self.duration, sr=self.sr
        )
        self.assertLess(diff, 1e-12, f"Filter vs spectral envelope max diff {diff:.3e} exceeds 1e-12")
        np.testing.assert_allclose(y_filt, y_env, atol=1e-12)

    def test_polar_amplitude_phase(self):
        """Verify that -A cos(wt + phi) == A cos(wt + phi + pi)."""
        A = -1.5
        phi = 0.4
        f = 330.0
        diff, x_neg, x_canon = polar_amplitude_phase(A=A, phi=phi, f=f, duration=self.duration, sr=self.sr)
        self.assertLess(diff, 1e-13)
        np.testing.assert_allclose(x_neg, x_canon, atol=1e-13)

    def test_jacobian_nullity(self):
        """Verify that the Jacobian for redundant parameterization has a null singular value."""
        nullity_result = compute_gauge_jacobian_nullity(f0=440.0, duration=0.05, sr=self.sr)
        self.assertTrue(nullity_result['has_nullity'])
        self.assertLess(nullity_result['singular_value_ratio'], 1e-14)


class TestCanonicalGauge(unittest.TestCase):
    """E40: Verification of Canonical Gauge Projections and Bit Savings."""

    def setUp(self):
        self.sr = 16000.0
        self.duration = 0.1

    def test_canonical_time_phase_idempotence(self):
        """Test that canonical projection G(G(theta)) == G(theta)."""
        canon_proj = CanonicalTimePhase(sr=self.sr)
        raw_params = {'A': -2.5, 'f0': 440.0, 't0': 0.015, 'phi0': 1.2}
        
        g1 = canon_proj.project(raw_params)
        g2 = canon_proj.project(g1)
        
        self.assertGreaterEqual(g1['A'], 0.0)
        self.assertEqual(g1['t0'], 0.0)
        self.assertAlmostEqual(g1['A'], g2['A'], places=10)
        self.assertAlmostEqual(g1['phi0'], g2['phi0'], places=10)

    def test_canonical_time_phase_waveform_invariance(self):
        """Test that D(G(theta)) == D(theta) to machine precision."""
        canon_proj = CanonicalTimePhase(sr=self.sr)
        raw_params = {'A': -1.8, 'f0': 500.0, 't0': 0.003, 'phi0': 0.7}
        
        x_raw = canon_proj.synthesize(raw_params, duration=self.duration)
        x_canon = canon_proj.synthesize(canon_proj.project(raw_params), duration=self.duration)
        
        diff = float(np.max(np.abs(x_raw - x_canon)))
        self.assertLess(diff, 1e-12)

    def test_canonical_angle_modulation(self):
        """Test collapsing FM and PM into canonical AngleModulation."""
        canon_am = CanonicalAngleModulation(sr=self.sr)
        
        # Unconstrained candidate specifying PM
        pm_cand = {'type': 'PM', 'fc': 440.0, 'fm': 6.0, 'I': 1.8, 'psi': 0.3}
        # Unconstrained candidate specifying FM
        fm_cand = {'type': 'FM', 'fc': 440.0, 'fm': 6.0, 'delta_f': 1.8 * 6.0, 'psi': 0.3}
        
        c_pm = canon_am.project(pm_cand)
        c_fm = canon_am.project(fm_cand)
        
        # Both must collapse to identical canonical parameters
        self.assertAlmostEqual(c_pm['beta'], c_fm['beta'], places=10)
        self.assertAlmostEqual(c_pm['fc'], c_fm['fc'], places=10)
        self.assertAlmostEqual(c_pm['fm'], c_fm['fm'], places=10)
        self.assertAlmostEqual(c_pm['psi'], c_fm['psi'], places=10)

        # Synthesize waveforms and verify equality
        x_pm = canon_am.synthesize(c_pm, duration=self.duration)
        x_fm = canon_am.synthesize(c_fm, duration=self.duration)
        self.assertEqual(float(np.max(np.abs(x_pm - x_fm))), 0.0)

    def test_canonical_filter_envelope_mdl(self):
        """Test that canonical filter-envelope selects minimal description length."""
        canon_fe = CanonicalFilterEnvelope(sr=self.sr)
        
        # Case A: Low-order all-pole filter (2 poles) shaping 16 harmonics -> Filter should win
        decision_a = canon_fe.select_and_project(num_poles=2, num_harmonics=16, f0=200.0)
        self.assertEqual(decision_a['chosen_family'], 'FILTER')
        self.assertLess(decision_a['L_chosen_bits'], decision_a['L_alternative_bits'])

        # Case B: Arbitrary 3 harmonics vs 8th order filter -> Envelope should win
        decision_b = canon_fe.select_and_project(num_poles=8, num_harmonics=3, f0=200.0)
        self.assertEqual(decision_b['chosen_family'], 'HARMONIC_ENVELOPE')
        self.assertLess(decision_b['L_chosen_bits'], decision_b['L_alternative_bits'])

    def test_mdl_bit_savings(self):
        """Verify that overall canonicalization strictly decreases total description length."""
        savings = evaluate_canonical_mdl_savings(num_blocks=20, sr=self.sr)
        self.assertGreater(savings['bit_savings_total'], 0.0)
        self.assertLess(savings['compression_ratio_canonical'], savings['compression_ratio_unconstrained'])
        self.assertLess(savings['max_reconstruction_error'], 1e-12)


class TestAmbiguityDistribution(unittest.TestCase):
    """E41: Verification of True Physical Ambiguity and Uncertainty Posterior."""

    def setUp(self):
        self.sr = 16000.0
        self.duration = 0.1

    def test_harmonic_collision_posterior(self):
        """Test posterior covariance and differential entropy for overlapping harmonics."""
        h_amb = HarmonicCollisionAmbiguity(sr=self.sr)
        res = h_amb.compute_posterior(f1=220.0, f2=440.0, observed_amplitude=1.5, noise_std=0.05)
        
        self.assertIn('mean_A1', res)
        self.assertIn('mean_A2', res)
        self.assertIn('covariance', res)
        self.assertIn('entropy_bits', res)
        self.assertTrue(np.isfinite(res['entropy_bits']))
        self.assertLess(res['entropy_bits'], res['prior_entropy_bits'])
        self.assertGreater(res['information_gain_bits'], 0.0)
        # Covariance must show strong anti-correlation along A1 + A2 = A_obs
        cov = np.array(res['covariance'])
        self.assertLess(cov[0, 1], 0.0, "Expected negative correlation between overlapping components")

    def test_resonator_vs_noise_likelihood(self):
        """Test posterior likelihood distinguishing high-Q resonator vs filtered noise."""
        r_amb = ResonatorVsNoiseAmbiguity(sr=self.sr)
        
        # Resonator signal
        x_res = r_amb.generate_resonator(f0=500.0, decay_rate=25.0, duration=self.duration)
        eval_res = r_amb.evaluate_likelihood(x_res)
        self.assertGreater(eval_res['p_resonator'], eval_res['p_noise'])

        # Noise burst
        x_noise = r_amb.generate_noise_grain(center_f=500.0, bandwidth=50.0, duration=self.duration)
        eval_noise = r_amb.evaluate_likelihood(x_noise)
        self.assertGreater(eval_noise['p_noise'], eval_noise['p_resonator'])

    def test_causal_lookahead_entropy_decay(self):
        """Verify that extending lookahead horizon causes ambiguity entropy to monotonically decay."""
        lookahead = CausalLookaheadDisambiguator(sr=self.sr)
        horizons = [0.02, 0.05, 0.1, 0.2]  # seconds
        entropy_trace = lookahead.simulate_horizon_expansion(f1=220.0, f2=440.0, horizons=horizons)
        
        # Check monotonic decrease
        for i in range(len(entropy_trace) - 1):
            self.assertGreaterEqual(
                entropy_trace[i]['entropy_bits'],
                entropy_trace[i+1]['entropy_bits'],
                f"Entropy increased from horizon {horizons[i]}s to {horizons[i+1]}s"
            )


if __name__ == '__main__':
    unittest.main()
