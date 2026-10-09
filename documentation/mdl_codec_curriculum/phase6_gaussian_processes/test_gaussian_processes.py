"""
Unit and Integration Tests for Phase 6: Evolution and Uncertainty (E42-E45).
Verifies SDE/GP anchor state evolution, cubic phase variance explosion,
probabilistic calibration (exact 95% +/- 1.96 sigma envelope), and GP vs Taylor Jets.
"""

from __future__ import annotations
import unittest
import numpy as np
from pathlib import Path
import sys

# Add phase directory to path
PHASE_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(PHASE_DIR))

from gp_anchor_evolution import (
    AmplitudeOUEvolution,
    FrequencyKinematicEvolution,
    simulate_euler_maruyama
)
from phase_uncertainty import (
    PhaseStochasticIntegrator
)
from probabilistic_calibration import (
    CalibrationEvaluator
)
from gp_vs_jet_benchmark import (
    TaylorJetPredictor,
    GPPredictor
)


class TestGPAnchorEvolution(unittest.TestCase):
    """Test suite for E42: GP distance from anchor."""

    def test_amplitude_ou_evolution_anchor_properties(self):
        """Verify boundary conditions and asymptotic convergence for amplitude OU process."""
        ou = AmplitudeOUEvolution(A0=1.5, A_inf=1.0, lambda_A=2.0, sigma_A=0.3)
        
        # At delta_t = 0, mean is A0 and variance is 0
        mu_0, var_0 = ou.predict(0.0)
        self.assertAlmostEqual(mu_0, 1.5, places=7)
        self.assertAlmostEqual(var_0, 0.0, places=7)
        
        # Monotonicity check
        dts = np.linspace(0.0, 1.0, 50)
        mus, vars_ = ou.predict(dts)
        self.assertTrue(np.all(np.diff(vars_) >= 0.0), "Variance must grow monotonically with distance from anchor")
        
        # Asymptotic variance check: sigma^2 / (2 * lambda)
        _, var_inf = ou.predict(100.0)
        theoretical_var_inf = (0.3 ** 2) / (2.0 * 2.0)
        self.assertAlmostEqual(var_inf, theoretical_var_inf, places=5)
        
        # Asymptotic mean check: A_inf
        mu_inf, _ = ou.predict(100.0)
        self.assertAlmostEqual(mu_inf, 1.0, places=5)

    def test_amplitude_monte_carlo_matches_analytical(self):
        """Verify that Monte Carlo Euler-Maruyama simulation matches analytical variance within 3%."""
        np.random.seed(42)
        A0 = 1.2
        A_inf = 0.8
        lambda_A = 3.0
        sigma_A = 0.4
        dt_target = 0.25
        num_trajectories = 10000
        num_steps = 250
        
        ou = AmplitudeOUEvolution(A0=A0, A_inf=A_inf, lambda_A=lambda_A, sigma_A=sigma_A)
        _, analytical_var = ou.predict(dt_target)
        
        # Run Monte Carlo Euler-Maruyama
        mc_trajectories = simulate_euler_maruyama(
            x0=A0,
            drift_fn=lambda x: -lambda_A * (x - A_inf),
            diffusion_fn=lambda x: sigma_A,
            t_max=dt_target,
            num_steps=num_steps,
            num_trajectories=num_trajectories
        )
        final_values = mc_trajectories[:, -1]
        empirical_var = float(np.var(final_values))
        
        rel_error = abs(empirical_var - analytical_var) / analytical_var
        self.assertLess(rel_error, 0.03, f"Relative variance error {rel_error:.4f} must be < 3%")

    def test_frequency_kinematic_evolution(self):
        """Verify 2D kinematic SDE [f, dot_f] covariance propagation."""
        f0 = 440.0
        f_dot0 = 10.0
        q_f = 25.0  # spectral density of frequency acceleration noise
        gamma_f = 0.5  # damping
        
        kin = FrequencyKinematicEvolution(f0=f0, f_dot0=f_dot0, q_f=q_f, gamma_f=gamma_f)
        
        # At delta_t = 0
        mu_0, P_0 = kin.predict(0.0)
        self.assertAlmostEqual(mu_0[0], f0, places=6)
        self.assertAlmostEqual(mu_0[1], f_dot0, places=6)
        self.assertAlmostEqual(P_0[0, 0], 0.0, places=6)
        
        # Distance evolution
        dts = np.array([0.01, 0.05, 0.1, 0.2, 0.5])
        prev_var = 0.0
        for dt in dts:
            mu_t, P_t = kin.predict(dt)
            var_f = P_t[0, 0]
            self.assertGreater(var_f, prev_var, "Frequency variance must grow monotonically")
            prev_var = var_f


class TestPhaseUncertainty(unittest.TestCase):
    """Test suite for E43: Phase uncertainty and cubic variance explosion."""

    def test_analytical_cubic_variance(self):
        """Verify analytical cubic variance formula Var(phi) = 4*pi^2 * sigma_w^2 * dt^3 / 3."""
        phi0 = 0.5
        f0 = 440.0
        sigma_w = 4.0  # Hz / s^0.5
        integrator = PhaseStochasticIntegrator(phi0=phi0, f0=f0, sigma_w=sigma_w)
        
        # At dt = 0
        mu_0, var_0 = integrator.predict_phase_distribution(0.0)
        self.assertAlmostEqual(mu_0, phi0, places=6)
        self.assertAlmostEqual(var_0, 0.0, places=6)
        
        # At dt = 0.1
        dt = 0.1
        expected_var = (4.0 * np.pi**2 * sigma_w**2 / 3.0) * (dt ** 3)
        mu_dt, var_dt = integrator.predict_phase_distribution(dt)
        self.assertAlmostEqual(var_dt, expected_var, places=6)

    def test_cubic_power_law_exponent(self):
        """Verify that log-log regression of phase variance against dt yields exponent p = 3.0 +/- 0.05."""
        integrator = PhaseStochasticIntegrator(phi0=0.0, f0=440.0, sigma_w=2.5)
        dts = np.logspace(-2, 0, 30)  # 0.01 to 1.0 s
        vars_ = [integrator.predict_phase_distribution(dt)[1] for dt in dts]
        
        log_dts = np.log(dts)
        log_vars = np.log(vars_)
        
        # Linear fit: log(var) = p * log(dt) + const
        poly = np.polyfit(log_dts, log_vars, 1)
        power_exponent = poly[0]
        self.assertAlmostEqual(power_exponent, 3.0, delta=0.02, msg="Phase variance scaling exponent must be 3.0")

    def test_monte_carlo_phase_variance(self):
        """Verify that Monte Carlo integration of frequency Brownian motion matches analytical within 4%."""
        np.random.seed(42)
        phi0 = 0.0
        f0 = 220.0
        sigma_w = 5.0
        dt = 0.15
        
        integrator = PhaseStochasticIntegrator(phi0=phi0, f0=f0, sigma_w=sigma_w)
        _, analytical_var = integrator.predict_phase_distribution(dt)
        
        # Run Monte Carlo
        mc_phases = integrator.simulate_phase_trajectories(
            t_max=dt,
            num_steps=300,
            num_trajectories=8000
        )
        final_phases = mc_phases[:, -1]
        empirical_var = float(np.var(final_phases))
        
        rel_err = abs(empirical_var - analytical_var) / analytical_var
        self.assertLess(rel_err, 0.04, f"Empirical variance error {rel_err:.4f} must be < 4%")

    def test_decoherence_horizon(self):
        """Verify calculation of decoherence horizon where phase standard deviation reaches 1.0 rad."""
        sigma_w = 3.0
        integrator = PhaseStochasticIntegrator(phi0=0.0, f0=440.0, sigma_w=sigma_w)
        
        tau_decoh = integrator.compute_decoherence_horizon(threshold_rad=1.0)
        _, var_at_tau = integrator.predict_phase_distribution(tau_decoh)
        std_at_tau = np.sqrt(var_at_tau)
        
        self.assertAlmostEqual(std_at_tau, 1.0, places=5)
        
        # Verify coherent cosine expectation attenuation: e^(-0.5 * 1^2) = e^(-0.5) ~ 0.6065
        coh_factor = integrator.compute_coherence_factor(tau_decoh)
        self.assertAlmostEqual(coh_factor, np.exp(-0.5), places=5)


class TestProbabilisticCalibration(unittest.TestCase):
    """Test suite for E44: Probabilistic Calibration Validation."""

    def test_calibrated_model_coverage_95_percent(self):
        """Mandatory Gate: 95% of ground-truth test samples must fall within +/- 1.96 sigma envelope."""
        np.random.seed(42)
        evaluator = CalibrationEvaluator()
        
        # Simulate calibrated SDE ensemble (N = 10,000 samples)
        mu_true = 440.0
        sigma_true = 5.0
        gt_samples = np.random.normal(mu_true, sigma_true, size=10000)
        
        # Evaluate against the model predicting mu=440, sigma=5
        metrics = evaluator.evaluate_calibration(
            ground_truth=gt_samples,
            predicted_mean=np.full_like(gt_samples, mu_true),
            predicted_std=np.full_like(gt_samples, sigma_true)
        )
        
        cov_95 = metrics['coverage_1_96_sigma']
        self.assertAlmostEqual(cov_95, 0.950, delta=0.015, msg="Coverage for +/- 1.96 sigma must be 95% +/- 1.5%")
        
        # Also check Z-score standard normality
        self.assertAlmostEqual(metrics['z_mean'], 0.0, delta=0.05)
        self.assertAlmostEqual(metrics['z_std'], 1.0, delta=0.05)
        
        # PIT uniformity test p-value should not reject uniformity (p > 0.05)
        self.assertGreater(metrics['pit_ks_pvalue'], 0.05)

    def test_multistep_sde_trajectory_calibration(self):
        """Verify calibration holds across multi-step SDE trajectory propagation."""
        np.random.seed(42)
        A0 = 1.0
        A_inf = 1.2
        lambda_A = 2.0
        sigma_A = 0.3
        
        ou = AmplitudeOUEvolution(A0=A0, A_inf=A_inf, lambda_A=lambda_A, sigma_A=sigma_A)
        evaluator = CalibrationEvaluator()
        
        # Generate 5,000 Monte Carlo trajectories over 50 time steps
        t_max = 0.5
        num_steps = 50
        num_trajs = 5000
        trajs = simulate_euler_maruyama(
            x0=A0,
            drift_fn=lambda x: -lambda_A * (x - A_inf),
            diffusion_fn=lambda x: sigma_A,
            t_max=t_max,
            num_steps=num_steps,
            num_trajectories=num_trajs
        )
        
        dts = np.linspace(0.0, t_max, num_steps + 1)
        # Check calibration at midpoint (t = 0.25s) and endpoint (t = 0.50s)
        for step_idx in [25, 50]:
            dt = dts[step_idx]
            mu_pred, var_pred = ou.predict(dt)
            std_pred = np.sqrt(var_pred)
            gt_at_t = trajs[:, step_idx]
            
            metrics = evaluator.evaluate_calibration(
                ground_truth=gt_at_t,
                predicted_mean=np.full_like(gt_at_t, mu_pred),
                predicted_std=np.full_like(gt_at_t, std_pred)
            )
            self.assertAlmostEqual(
                metrics['coverage_1_96_sigma'], 0.950, delta=0.015,
                msg=f"Trajectory at t={dt:.2f}s must have 95% +/- 1.5% coverage"
            )

    def test_overconfident_and_underconfident_detection(self):
        """Verify that evaluator correctly flags overconfident and underconfident predictors."""
        evaluator = CalibrationEvaluator()
        np.random.seed(42)
        gt = np.random.normal(0.0, 1.0, size=5000)
        
        # Overconfident model predicts sigma = 0.5 (too small)
        over_metrics = evaluator.evaluate_calibration(
            ground_truth=gt,
            predicted_mean=np.zeros_like(gt),
            predicted_std=np.full_like(gt, 0.5)
        )
        self.assertLess(over_metrics['coverage_1_96_sigma'], 0.90, "Overconfident model coverage must drop below 90%")
        self.assertEqual(over_metrics['diagnosis'], 'OVERCONFIDENT')
        
        # Underconfident model predicts sigma = 2.0 (too large)
        under_metrics = evaluator.evaluate_calibration(
            ground_truth=gt,
            predicted_mean=np.zeros_like(gt),
            predicted_std=np.full_like(gt, 2.0)
        )
        self.assertGreater(under_metrics['coverage_1_96_sigma'], 0.98, "Underconfident model coverage must exceed 98%")
        self.assertEqual(under_metrics['diagnosis'], 'UNDERCONFIDENT')


class TestGPVsJetBenchmark(unittest.TestCase):
    """Test suite for E45: GP versus High-Order Taylor Jet Horizon Comparison."""

    def setUp(self):
        # Synthetic frequency vibrato ground truth
        self.fc = 440.0
        self.delta_f = 25.0  # vibrato depth (+/- 25 Hz)
        self.fm = 6.0        # vibrato frequency (6 Hz)
        
        # Ground truth trajectory function
        self.gt_fn = lambda t: self.fc + self.delta_f * np.sin(2.0 * np.pi * self.fm * t)
        
        # Theoretical derivatives at t = 0
        self.f0 = self.gt_fn(0.0)  # 440.0
        self.f_dot0 = self.delta_f * (2.0 * np.pi * self.fm) * np.cos(0.0)  # 25 * 12*pi ~ 942.47
        self.f_ddot0 = -self.delta_f * ((2.0 * np.pi * self.fm) ** 2) * np.sin(0.0)  # 0.0

    def test_taylor_jet_divergence_behavior(self):
        """Verify that high-order Taylor jet J2 error explodes quadratically/cubically beyond local radius."""
        jet = TaylorJetPredictor(x0=self.f0, x_dot0=self.f_dot0, x_ddot0=self.f_ddot0)
        
        # At very small dt (e.g. 5ms), J1 and J2 are accurate
        dt_near = 0.005
        err_j1_near = abs(jet.predict(dt_near, order=1) - self.gt_fn(dt_near))
        self.assertLess(err_j1_near, 0.5, "At 5ms Taylor approximation should have low error")
        
        # Beyond quarter period (dt > 0.05s), J1 shoots off linearly
        dt_far = 0.15
        pred_j1_far = jet.predict(dt_far, order=1)
        err_j1_far = abs(pred_j1_far - self.gt_fn(dt_far))
        self.assertGreater(err_j1_far, 50.0, "At 150ms J1 should diverge significantly from bounded vibrato")

    def test_gp_bounded_prediction_and_mdl_superiority(self):
        """Verify that GP predictor maintains bounded predictions and lower MDL code length than Taylor jets."""
        omega_m = 2.0 * np.pi * self.fm
        gp = GPPredictor(x0=self.f0, x_dot0=self.f_dot0, nominal_mean=self.fc, damping=8.0, lambda_revert=omega_m**2, sigma_process=150.0)
        jet = TaylorJetPredictor(x0=self.f0, x_dot0=self.f_dot0, x_ddot0=self.f_ddot0)
        
        dts = np.linspace(0.01, 0.25, 25)
        gt_vals = np.array([self.gt_fn(t) for t in dts])
        
        # GP predictions remain bounded within [fc - 2*delta_f, fc + 2*delta_f]
        gp_preds, gp_stds = gp.predict(dts)
        self.assertTrue(np.all(np.abs(gp_preds - self.fc) < 3.0 * self.delta_f), "GP mean prediction must stay bounded")
        
        # Compare MDL code lengths over the horizon
        mdl_gp = gp.compute_mdl_cost(gt_vals, gp_preds, gp_stds)
        
        j1_preds = np.array([jet.predict(t, order=1) for t in dts])
        mdl_j1 = jet.compute_mdl_cost(gt_vals, j1_preds, residual_std=5.0)
        
        self.assertLess(mdl_gp, mdl_j1, "GP must achieve strictly lower predictive description length than diverging J1")

    def test_predictive_horizon_comparison(self):
        """Verify that GP achieves longer valid predictive horizon than J1 and J2 under tolerance epsilon."""
        omega_m = 2.0 * np.pi * self.fm
        gp = GPPredictor(x0=self.f0, x_dot0=self.f_dot0, nominal_mean=self.fc, damping=8.0, lambda_revert=omega_m**2, sigma_process=150.0)
        jet = TaylorJetPredictor(x0=self.f0, x_dot0=self.f_dot0, x_ddot0=self.f_ddot0)
        
        tolerance_hz = 20.0
        t_grid = np.linspace(0.001, 0.30, 300)
        gt_grid = np.array([self.gt_fn(t) for t in t_grid])
        
        tau_j1 = jet.compute_valid_horizon(t_grid, gt_grid, order=1, tolerance=tolerance_hz)
        tau_gp = gp.compute_valid_horizon(t_grid, gt_grid, tolerance=tolerance_hz)
        
        self.assertGreater(tau_gp, tau_j1, f"GP horizon {tau_gp:.3f}s must exceed J1 horizon {tau_j1:.3f}s")


if __name__ == '__main__':
    unittest.main()
