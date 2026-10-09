"""
E44: Rigorous Probabilistic Calibration Validation.
Verifies that the SDE/GP continuous predictive model is statistically honest:
    - 95.0% +/- 1.5% of ground-truth test trajectories fall within the +/- 1.96 * sigma envelope.
    - Normalized innovation Z = (x_gt - mu) / sigma is standard normal N(0, 1).
    - Probability Integral Transform (PIT) U = Phi(Z) is strictly uniform U[0, 1] via Kolmogorov-Smirnov test.
"""

from __future__ import annotations
import numpy as np
from scipy import stats, special
from typing import Dict, Any, Union


class CalibrationEvaluator:
    """
    Evaluator for probabilistic forecast calibration and statistical integrity.
    """

    def __init__(self, tolerance_95: float = 0.015) -> None:
        self.tolerance_95 = float(tolerance_95)

    def evaluate_calibration(
        self,
        ground_truth: np.ndarray,
        predicted_mean: np.ndarray,
        predicted_std: np.ndarray
    ) -> Dict[str, Any]:
        """
        Evaluate empirical coverage against theoretical Gaussian confidence intervals.
        """
        gt = np.asarray(ground_truth, dtype=np.float64).flatten()
        mu = np.asarray(predicted_mean, dtype=np.float64).flatten()
        std = np.asarray(predicted_std, dtype=np.float64).flatten()

        if len(gt) != len(mu) or len(mu) != len(std):
            raise ValueError(f"Length mismatch: gt={len(gt)}, mu={len(mu)}, std={len(std)}")

        # Guard against zero or negative standard deviations
        std_safe = np.maximum(std, 1e-12)

        # Standardized innovations (Z-scores)
        z_scores = (gt - mu) / std_safe
        z_abs = np.abs(z_scores)

        # Empirical coverages
        cov_50 = float(np.mean(z_abs <= 0.67448975))
        cov_68 = float(np.mean(z_abs <= 1.00000000))
        cov_90 = float(np.mean(z_abs <= 1.64485363))
        cov_95 = float(np.mean(z_abs <= 1.95996398))
        cov_99 = float(np.mean(z_abs <= 2.57582930))

        z_mean = float(np.mean(z_scores))
        z_std = float(np.std(z_scores, ddof=1))

        # Probability Integral Transform (PIT)
        pit_values = 0.5 * (1.0 + special.erf(z_scores / np.sqrt(2.0)))
        ks_stat, ks_pval = stats.kstest(pit_values, 'uniform')

        # Diagnosis
        if cov_95 < (0.95 - self.tolerance_95):
            diagnosis = 'OVERCONFIDENT'
        elif cov_95 > (0.95 + self.tolerance_95):
            diagnosis = 'UNDERCONFIDENT'
        else:
            diagnosis = 'WELL_CALIBRATED'

        is_calibrated = (diagnosis == 'WELL_CALIBRATED')

        return {
            'total_samples': len(gt),
            'z_mean': z_mean,
            'z_std': z_std,
            'coverage_50_percent': cov_50,
            'coverage_68_sigma': cov_68,
            'coverage_90_percent': cov_90,
            'coverage_1_96_sigma': cov_95,
            'coverage_99_percent': cov_99,
            'pit_ks_statistic': float(ks_stat),
            'pit_ks_pvalue': float(ks_pval),
            'diagnosis': diagnosis,
            'is_calibrated': is_calibrated
        }


def evaluate_calibration_benchmarks() -> Dict[str, Any]:
    """
    Run comprehensive E44 calibration benchmarks across synthetic signals and SDE trajectories.
    """
    from gp_anchor_evolution import AmplitudeOUEvolution, simulate_euler_maruyama
    from phase_uncertainty import PhaseStochasticIntegrator

    np.random.seed(42)
    evaluator = CalibrationEvaluator()

    # 1. Amplitude OU trajectory calibration across 10,000 runs
    A0 = 1.0
    A_inf = 1.3
    lambda_A = 2.0
    sigma_A = 0.3
    ou = AmplitudeOUEvolution(A0=A0, A_inf=A_inf, lambda_A=lambda_A, sigma_A=sigma_A)

    dt_eval = 0.20
    mu_a, var_a = ou.predict(dt_eval)
    std_a = np.sqrt(var_a)

    trajs = simulate_euler_maruyama(
        x0=A0,
        drift_fn=lambda x: -lambda_A * (x - A_inf),
        diffusion_fn=lambda x: sigma_A,
        t_max=dt_eval,
        num_steps=100,
        num_trajectories=10000
    )
    final_amps = trajs[:, -1]

    amp_calib = evaluator.evaluate_calibration(
        ground_truth=final_amps,
        predicted_mean=np.full_like(final_amps, mu_a),
        predicted_std=np.full_like(final_amps, std_a)
    )

    # 2. Phase stochastic trajectory calibration across 10,000 runs
    integrator = PhaseStochasticIntegrator(phi0=0.0, f0=440.0, sigma_w=3.0)
    dt_phase = 0.05
    mu_phi, var_phi = integrator.predict_phase_distribution(dt_phase)
    std_phi = np.sqrt(var_phi)

    phase_trajs = integrator.simulate_phase_trajectories(t_max=dt_phase, num_steps=150, num_trajectories=10000)
    final_phases = phase_trajs[:, -1]

    phase_calib = evaluator.evaluate_calibration(
        ground_truth=final_phases,
        predicted_mean=np.full_like(final_phases, mu_phi),
        predicted_std=np.full_like(final_phases, std_phi)
    )

    # 3. Deliberately distorted test cases (overconfident vs underconfident)
    norm_gt = np.random.normal(0.0, 1.0, size=5000)
    overconfident = evaluator.evaluate_calibration(norm_gt, np.zeros_like(norm_gt), np.full_like(norm_gt, 0.6))
    underconfident = evaluator.evaluate_calibration(norm_gt, np.zeros_like(norm_gt), np.full_like(norm_gt, 1.7))

    return {
        'amplitude_ou_calibration': amp_calib,
        'phase_stochastic_calibration': phase_calib,
        'control_experiments': {
            'overconfident': overconfident,
            'underconfident': underconfident
        }
    }
