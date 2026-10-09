"""
E43: Phase Uncertainty and Stochastic Integration.
Analyzes the continuous stochastic integration of frequency variation into instantaneous phase:
    phi(t) = phi_0 + 2*pi * \\int_{t_0}^t f(tau) dtau
Demonstrates and proves the cubic variance explosion:
    Var(phi(Delta t)) = (4 * pi^2 * sigma_w^2 / 3) * Delta t^3
and establishes the critical phase decoherence horizon tau_decoh.
"""

from __future__ import annotations
import numpy as np
from typing import Tuple, Union, Dict, Any


class PhaseStochasticIntegrator:
    """
    Stochastic integrator for instantaneous acoustic phase:
        f(t) = f_0 + sigma_w * B(t - t_0)
        phi(t) = phi_0 + 2*pi * \\int_{t_0}^t f(tau) dtau
    """

    def __init__(
        self,
        phi0: float = 0.0,
        f0: float = 440.0,
        sigma_w: float = 2.0
    ) -> None:
        self.phi0 = float(phi0)
        self.f0 = float(f0)
        self.sigma_w = max(float(sigma_w), 1e-9)

    def predict_phase_distribution(
        self,
        delta_t: Union[float, np.ndarray]
    ) -> Tuple[Union[float, np.ndarray], Union[float, np.ndarray]]:
        """
        Compute analytical conditional phase mean and cubic variance at delta_t >= 0:
            E[phi(Delta t)] = phi_0 + 2*pi * f_0 * Delta t
            Var(phi(Delta t)) = (4 * pi^2 * sigma_w^2 / 3) * Delta t^3
        """
        dt = np.asarray(delta_t, dtype=np.float64)
        is_scalar = dt.ndim == 0
        dt = np.atleast_1d(dt)
        dt = np.maximum(dt, 0.0)

        # Expected phase (unwrapped)
        mu_phi = self.phi0 + 2.0 * np.pi * self.f0 * dt

        # Cubic variance
        coeff = (4.0 * (np.pi ** 2) * (self.sigma_w ** 2)) / 3.0
        var_phi = coeff * (dt ** 3)

        if is_scalar:
            return float(mu_phi[0]), float(var_phi[0])
        return mu_phi, var_phi

    def simulate_phase_trajectories(
        self,
        t_max: float,
        num_steps: int,
        num_trajectories: int
    ) -> np.ndarray:
        """
        Simulate stochastic phase trajectories using trapezoidal stochastic integration.
        Returns: array of shape (num_trajectories, num_steps + 1)
        """
        dt = float(t_max) / float(num_steps)
        sqrt_dt = np.sqrt(dt)

        # Preallocate trajectories
        phase_traj = np.empty((num_trajectories, num_steps + 1), dtype=np.float64)
        phase_traj[:, 0] = self.phi0

        current_f = np.full(num_trajectories, self.f0, dtype=np.float64)
        current_phi = np.full(num_trajectories, self.phi0, dtype=np.float64)

        for step in range(num_steps):
            # Brownian increment for frequency
            df = np.random.normal(0.0, self.sigma_w * sqrt_dt, size=num_trajectories)
            next_f = current_f + df
            
            # Trapezoidal integration for phase
            avg_f = 0.5 * (current_f + next_f)
            current_phi = current_phi + 2.0 * np.pi * avg_f * dt
            
            current_f = next_f
            phase_traj[:, step + 1] = current_phi

        return phase_traj

    def compute_decoherence_horizon(
        self,
        threshold_rad: float = 1.0
    ) -> float:
        """
        Calculate the critical time horizon tau_decoh where sigma_phi(tau) = threshold_rad:
            tau_decoh = [ 3 * threshold_rad^2 / (4 * pi^2 * sigma_w^2) ]^(1/3)
        """
        thresh = float(threshold_rad)
        coeff = (4.0 * (np.pi ** 2) * (self.sigma_w ** 2)) / 3.0
        tau = (thresh ** 2 / coeff) ** (1.0 / 3.0)
        return float(tau)

    def compute_coherence_factor(
        self,
        delta_t: Union[float, np.ndarray]
    ) -> Union[float, np.ndarray]:
        """
        Calculate the coherent attenuation factor:
            E[cos(phi)] = cos(E[phi]) * exp(-0.5 * Var(phi))
        Returns exp(-0.5 * Var(phi)).
        """
        _, var_phi = self.predict_phase_distribution(delta_t)
        if isinstance(var_phi, np.ndarray):
            return np.exp(-0.5 * var_phi)
        return float(np.exp(-0.5 * var_phi))


def evaluate_phase_uncertainty_benchmarks() -> Dict[str, Any]:
    """
    Execute comprehensive E43 benchmarks for reporting.
    """
    np.random.seed(42)
    f0 = 440.0
    phi0 = 0.0
    sigma_w = 3.5  # Hz / s^0.5

    integrator = PhaseStochasticIntegrator(phi0=phi0, f0=f0, sigma_w=sigma_w)

    # 1. Scaling vs Delta t
    dts = np.array([0.005, 0.01, 0.02, 0.05, 0.10, 0.20])
    scaling_records = []
    for dt in dts:
        mu, var = integrator.predict_phase_distribution(dt)
        std = np.sqrt(var)
        coh = integrator.compute_coherence_factor(dt)
        scaling_records.append({
            'delta_t': float(dt),
            'phase_mean_rad': float(mu),
            'phase_variance_rad2': float(var),
            'phase_std_rad': float(std),
            'phase_std_deg': float(np.degrees(std)),
            'coherence_attenuation': float(coh)
        })

    # 2. Power law regression
    fit_dts = np.logspace(-2.5, -0.5, 40)
    fit_vars = np.array([integrator.predict_phase_distribution(t)[1] for t in fit_dts])
    poly = np.polyfit(np.log(fit_dts), np.log(fit_vars), 1)
    power_exponent = float(poly[0])

    # 3. Monte Carlo verification at 50ms
    mc_dt = 0.05
    _, ana_var = integrator.predict_phase_distribution(mc_dt)
    mc_trajs = integrator.simulate_phase_trajectories(t_max=mc_dt, num_steps=200, num_trajectories=10000)
    emp_var = float(np.var(mc_trajs[:, -1]))
    rel_error = abs(emp_var - ana_var) / ana_var

    # 4. Decoherence Horizons for different thresholds
    tau_1rad = integrator.compute_decoherence_horizon(1.0)
    tau_pi = integrator.compute_decoherence_horizon(np.pi)

    return {
        'scaling_table': scaling_records,
        'power_law_exponent': power_exponent,
        'mc_verification': {
            'delta_t': mc_dt,
            'analytical_var': float(ana_var),
            'empirical_var': float(emp_var),
            'relative_error': float(rel_error)
        },
        'decoherence_horizons': {
            'tau_1_radian_s': float(tau_1rad),
            'tau_1_radian_ms': float(tau_1rad * 1000.0),
            'tau_pi_s': float(tau_pi),
            'tau_pi_ms': float(tau_pi * 1000.0)
        }
    }
