"""
E45: Gaussian Process versus High-Order Taylor Jet Horizon Comparison.
Compares deterministic polynomial Taylor expansions (J0, J1, J2) with causal
SDE/GP predictive filters on dynamic acoustic trajectories (vibrato, pitch bends).
Demonstrates Taylor divergence outside local radius vs bounded GP stability and MDL cost.
"""

from __future__ import annotations
import numpy as np
from scipy.linalg import expm
from typing import Tuple, Union, Dict, Any, List


class TaylorJetPredictor:
    """
    Local polynomial Taylor Jet extrapolator at anchor t0:
        J0: x_hat(t) = x0
        J1: x_hat(t) = x0 + dot_x0 * Delta t
        J2: x_hat(t) = x0 + dot_x0 * Delta t + 0.5 * ddot_x0 * Delta t^2
    """

    def __init__(
        self,
        x0: float,
        x_dot0: float = 0.0,
        x_ddot0: float = 0.0
    ) -> None:
        self.x0 = float(x0)
        self.x_dot0 = float(x_dot0)
        self.x_ddot0 = float(x_ddot0)

    def predict(
        self,
        delta_t: Union[float, np.ndarray],
        order: int = 1
    ) -> Union[float, np.ndarray]:
        """
        Extrapolate trajectory using Taylor Jet of specified order (0, 1, or 2).
        """
        dt = np.asarray(delta_t, dtype=np.float64)
        is_scalar = dt.ndim == 0
        dt = np.atleast_1d(dt)

        if order == 0:
            pred = np.full_like(dt, self.x0)
        elif order == 1:
            pred = self.x0 + self.x_dot0 * dt
        elif order == 2:
            pred = self.x0 + self.x_dot0 * dt + 0.5 * self.x_ddot0 * (dt ** 2)
        else:
            raise ValueError(f"Unsupported Taylor jet order: {order}. Must be 0, 1, or 2.")

        if is_scalar:
            return float(pred[0])
        return pred

    def compute_mdl_cost(
        self,
        ground_truth: np.ndarray,
        predictions: np.ndarray,
        residual_std: float = 5.0
    ) -> float:
        """
        Compute total description length (in bits) to transmit residuals under fixed Gaussian variance.
            L(x | M) = sum_i 0.5 * log2(2 * pi * e * sigma^2) + (x_i - x_hat_i)^2 / (2 * sigma^2 * ln 2)
        """
        gt = np.asarray(ground_truth, dtype=np.float64)
        pred = np.asarray(predictions, dtype=np.float64)
        sig2 = max(float(residual_std) ** 2, 1e-12)

        # Standard negative log2 likelihood
        n = len(gt)
        mse = np.sum((gt - pred) ** 2)
        log_term = n * np.log2(2.0 * np.pi * np.e * sig2)
        quad_term = mse / (sig2 * np.log(2.0))
        return float(0.5 * (log_term + quad_term))

    def compute_valid_horizon(
        self,
        t_grid: np.ndarray,
        ground_truth: np.ndarray,
        order: int = 1,
        tolerance: float = 20.0
    ) -> float:
        """
        Find maximum elapsed time where absolute prediction error <= tolerance.
        """
        preds = self.predict(t_grid, order=order)
        errors = np.abs(ground_truth - preds)
        exceed_idx = np.where(errors > tolerance)[0]
        if len(exceed_idx) == 0:
            return float(t_grid[-1])
        return float(t_grid[exceed_idx[0]])


class GPPredictor:
    """
    Continuous-Time Gauss-Markov / Kalman SDE State-Space Predictor.
    Models acoustic frequency trajectory as a damped stochastic oscillator:
        d[x - x_nom] = dot_x * dt
        d[dot_x] = (-gamma * dot_x - lambda * (x - x_nom)) * dt + sigma_p * dw(t)
    """

    def __init__(
        self,
        x0: float,
        x_dot0: float = 0.0,
        nominal_mean: float = 440.0,
        damping: float = 8.0,
        lambda_revert: float = (2.0 * np.pi * 6.0) ** 2,
        sigma_process: float = 150.0,
        P0: np.ndarray | None = None
    ) -> None:
        self.x0 = float(x0)
        self.x_dot0 = float(x_dot0)
        self.nominal_mean = float(nominal_mean)
        self.damping = max(float(damping), 1e-6)
        self.lambda_revert = max(float(lambda_revert), 1e-6)
        self.sigma_p = float(sigma_process)

        # State vector: [displacement from nominal, velocity]
        self.s0 = np.array([self.x0 - self.nominal_mean, self.x_dot0], dtype=np.float64)

        if P0 is None:
            self.P0 = np.zeros((2, 2), dtype=np.float64)
        else:
            self.P0 = np.asarray(P0, dtype=np.float64)

        # Continuous drift matrix F
        self.F = np.array([
            [0.0, 1.0],
            [-self.lambda_revert, -self.damping]
        ], dtype=np.float64)

        # Stationary covariance P_inf
        # P_inf satisfies F * P_inf + P_inf * F^T + Q_c = 0
        # Q_c = [[0, 0], [0, q]]
        q = self.sigma_p ** 2
        p11 = q / (2.0 * self.damping * self.lambda_revert)
        p22 = q / (2.0 * self.damping)
        self.P_inf = np.array([
            [p11, 0.0],
            [0.0, p22]
        ], dtype=np.float64)

    def predict(
        self,
        delta_t: Union[float, np.ndarray]
    ) -> Tuple[Union[float, np.ndarray], Union[float, np.ndarray]]:
        """
        Compute predicted trajectory mean and predictive standard deviation at elapsed time delta_t >= 0.
        """
        dt = np.asarray(delta_t, dtype=np.float64)
        is_scalar = dt.ndim == 0
        dt = np.atleast_1d(dt)

        means = np.empty_like(dt)
        stds = np.empty_like(dt)

        for i, t in enumerate(dt):
            t_val = float(max(t, 0.0))
            if t_val == 0.0:
                means[i] = self.x0
                stds[i] = np.sqrt(max(self.P0[0, 0], 1e-12))
                continue

            # State transition matrix via matrix exponential
            Phi = expm(self.F * t_val)
            s_t = Phi @ self.s0
            pred_mean = self.nominal_mean + s_t[0]

            # Process noise covariance: Qd(t) = P_inf - Phi * P_inf * Phi^T
            Qd = self.P_inf - Phi @ self.P_inf @ Phi.T
            Pt = Phi @ self.P0 @ Phi.T + Qd
            var_x = max(Pt[0, 0], 1e-12)

            means[i] = pred_mean
            stds[i] = np.sqrt(var_x)

        if is_scalar:
            return float(means[0]), float(stds[0])
        return means, stds

    def compute_mdl_cost(
        self,
        ground_truth: np.ndarray,
        predictions: np.ndarray,
        predictive_stds: np.ndarray
    ) -> float:
        """
        Compute total description length under predictive Gaussian distribution:
            L(x | GP) = sum_i 0.5 * [ log2(2 * pi * e * sigma_i^2) + (x_i - mu_i)^2 / (sigma_i^2 * ln 2) ]
        """
        gt = np.asarray(ground_truth, dtype=np.float64)
        mu = np.asarray(predictions, dtype=np.float64)
        std = np.maximum(np.asarray(predictive_stds, dtype=np.float64), 1e-12)

        sig2 = std ** 2
        log_terms = np.log2(2.0 * np.pi * np.e * sig2)
        quad_terms = ((gt - mu) ** 2) / (sig2 * np.log(2.0))
        total_bits = 0.5 * np.sum(log_terms + quad_terms)
        return float(total_bits)

    def compute_valid_horizon(
        self,
        t_grid: np.ndarray,
        ground_truth: np.ndarray,
        tolerance: float = 20.0
    ) -> float:
        """
        Find maximum elapsed time where absolute prediction error <= tolerance.
        """
        means, _ = self.predict(t_grid)
        errors = np.abs(ground_truth - means)
        exceed_idx = np.where(errors > tolerance)[0]
        if len(exceed_idx) == 0:
            return float(t_grid[-1])
        return float(t_grid[exceed_idx[0]])


def evaluate_gp_vs_jet_benchmarks() -> Dict[str, Any]:
    """
    Execute comprehensive E45 benchmarks comparing J0, J1, J2 and GP on vibrato trajectory.
    """
    fc = 440.0
    delta_f = 25.0
    fm = 6.0

    gt_fn = lambda t: fc + delta_f * np.sin(2.0 * np.pi * fm * t)
    f0 = gt_fn(0.0)
    f_dot0 = delta_f * (2.0 * np.pi * fm) * np.cos(0.0)
    f_ddot0 = 0.0

    jet = TaylorJetPredictor(x0=f0, x_dot0=f_dot0, x_ddot0=f_ddot0)
    omega_m = 2.0 * np.pi * fm
    gp = GPPredictor(x0=f0, x_dot0=f_dot0, nominal_mean=fc, damping=8.0, lambda_revert=omega_m**2, sigma_process=150.0)

    # Time horizons
    t_eval = np.array([0.01, 0.02, 0.05, 0.10, 0.15, 0.20, 0.25, 0.30])
    gt_eval = np.array([gt_fn(t) for t in t_eval])

    j0_preds = np.array([jet.predict(t, order=0) for t in t_eval])
    j1_preds = np.array([jet.predict(t, order=1) for t in t_eval])
    j2_preds = np.array([jet.predict(t, order=2) for t in t_eval])
    gp_preds, gp_stds = gp.predict(t_eval)

    # Errors
    err_j0 = np.abs(gt_eval - j0_preds)
    err_j1 = np.abs(gt_eval - j1_preds)
    err_j2 = np.abs(gt_eval - j2_preds)
    err_gp = np.abs(gt_eval - gp_preds)

    # MDL costs over 300ms window
    dense_t = np.linspace(0.005, 0.30, 60)
    dense_gt = np.array([gt_fn(t) for t in dense_t])
    dense_j0 = np.array([jet.predict(t, order=0) for t in dense_t])
    dense_j1 = np.array([jet.predict(t, order=1) for t in dense_t])
    dense_j2 = np.array([jet.predict(t, order=2) for t in dense_t])
    dense_gp, dense_stds = gp.predict(dense_t)

    mdl_j0 = jet.compute_mdl_cost(dense_gt, dense_j0, residual_std=5.0)
    mdl_j1 = jet.compute_mdl_cost(dense_gt, dense_j1, residual_std=5.0)
    mdl_j2 = jet.compute_mdl_cost(dense_gt, dense_j2, residual_std=5.0)
    mdl_gp = gp.compute_mdl_cost(dense_gt, dense_gp, dense_stds)

    # Valid horizons with tolerance = 15 Hz
    tol = 15.0
    t_grid = np.linspace(0.001, 0.50, 500)
    gt_grid = np.array([gt_fn(t) for t in t_grid])

    tau_j0 = jet.compute_valid_horizon(t_grid, gt_grid, order=0, tolerance=tol)
    tau_j1 = jet.compute_valid_horizon(t_grid, gt_grid, order=1, tolerance=tol)
    tau_j2 = jet.compute_valid_horizon(t_grid, gt_grid, order=2, tolerance=tol)
    tau_gp = gp.compute_valid_horizon(t_grid, gt_grid, tolerance=tol)

    eval_table = []
    for i, t in enumerate(t_eval):
        eval_table.append({
            'delta_t_ms': float(t * 1000.0),
            'ground_truth_hz': float(gt_eval[i]),
            'err_j0_hz': float(err_j0[i]),
            'err_j1_hz': float(err_j1[i]),
            'err_j2_hz': float(err_j2[i]),
            'err_gp_hz': float(err_gp[i]),
            'gp_predicted_std_hz': float(gp_stds[i])
        })

    return {
        'horizon_table': eval_table,
        'mdl_costs_bits': {
            'J0_constant': float(mdl_j0),
            'J1_linear': float(mdl_j1),
            'J2_parabolic': float(mdl_j2),
            'GP_stochastic': float(mdl_gp),
            'gp_savings_vs_j1_percent': float((mdl_j1 - mdl_gp) / mdl_j1 * 100.0)
        },
        'valid_prediction_horizons_ms': {
            'tolerance_hz': float(tol),
            'tau_J0_ms': float(tau_j0 * 1000.0),
            'tau_J1_ms': float(tau_j1 * 1000.0),
            'tau_J2_ms': float(tau_j2 * 1000.0),
            'tau_GP_ms': float(tau_gp * 1000.0)
        }
    }
