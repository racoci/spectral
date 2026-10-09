"""
E42: Gaussian Process and SDE Anchor State Evolution.
Continuous-time Gauss-Markov state-space models for amplitude A(t) and frequency f(t).
Computes analytical variance propagation sigma^2(Delta t) away from conditioned anchors.
"""

from __future__ import annotations
import numpy as np
from typing import Tuple, Union, Callable, Dict, Any


class AmplitudeOUEvolution:
    """
    Ornstein-Uhlenbeck (OU) model for acoustic amplitude evolution:
        dA(t) = -lambda_A * (A(t) - A_inf) * dt + sigma_A * dw(t)
    
    Given anchor A(t_0) = A0, the conditional distribution is Gaussian:
        mu_A(Delta t) = A_inf + (A0 - A_inf) * exp(-lambda_A * Delta t)
        sigma_A^2(Delta t) = (sigma_A^2 / (2 * lambda_A)) * (1 - exp(-2 * lambda_A * Delta t))
    """

    def __init__(
        self,
        A0: float = 1.0,
        A_inf: float = 1.0,
        lambda_A: float = 1.0,
        sigma_A: float = 0.2
    ) -> None:
        self.A0 = float(A0)
        self.A_inf = float(A_inf)
        self.lambda_A = max(float(lambda_A), 1e-9)
        self.sigma_A = float(sigma_A)

    def predict(
        self,
        delta_t: Union[float, np.ndarray]
    ) -> Tuple[Union[float, np.ndarray], Union[float, np.ndarray]]:
        """
        Compute analytical conditional mean and variance at elapsed time delta_t >= 0.
        """
        dt = np.asarray(delta_t, dtype=np.float64)
        is_scalar = dt.ndim == 0
        dt = np.atleast_1d(dt)
        dt = np.maximum(dt, 0.0)

        # Conditional mean
        exp_neg = np.exp(-self.lambda_A * dt)
        mu = self.A_inf + (self.A0 - self.A_inf) * exp_neg

        # Conditional variance
        stationary_var = (self.sigma_A ** 2) / (2.0 * self.lambda_A)
        var = stationary_var * (1.0 - np.exp(-2.0 * self.lambda_A * dt))

        if is_scalar:
            return float(mu[0]), float(var[0])
        return mu, var


class FrequencyKinematicEvolution:
    """
    2D Kinematic Continuous-Time SDE for Instantaneous Frequency:
        state s(t) = [f(t), dot_f(t)]^T
        ds(t) = F * s(t) * dt + L * dw(t)
        
        F = [[0, 1], [0, -gamma_f]]
        L = [[0], [1]]
        w(t) has diffusion spectral density q_f.
    """

    def __init__(
        self,
        f0: float = 440.0,
        f_dot0: float = 0.0,
        q_f: float = 10.0,
        gamma_f: float = 0.1,
        P0: np.ndarray | None = None
    ) -> None:
        self.s0 = np.array([float(f0), float(f_dot0)], dtype=np.float64)
        self.q_f = float(q_f)
        self.gamma_f = float(gamma_f)
        if P0 is None:
            self.P0 = np.zeros((2, 2), dtype=np.float64)
        else:
            self.P0 = np.asarray(P0, dtype=np.float64)

    def compute_transition_and_covariance(
        self,
        delta_t: float
    ) -> Tuple[np.ndarray, np.ndarray]:
        """
        Compute discrete state transition matrix Phi(Delta t) and process covariance Qd(Delta t).
        """
        dt = float(max(delta_t, 0.0))
        if dt == 0.0:
            return np.eye(2, dtype=np.float64), np.zeros((2, 2), dtype=np.float64)

        gamma = self.gamma_f
        q = self.q_f

        if abs(gamma) < 1e-7:
            # Undamped double integrator
            Phi = np.array([
                [1.0, dt],
                [0.0, 1.0]
            ], dtype=np.float64)
            Qd = q * np.array([
                [dt**3 / 3.0, dt**2 / 2.0],
                [dt**2 / 2.0, dt]
            ], dtype=np.float64)
        else:
            # Damped kinematic oscillator
            e_g = np.exp(-gamma * dt)
            e_2g = np.exp(-2.0 * gamma * dt)
            g_tau = (1.0 - e_g) / gamma

            Phi = np.array([
                [1.0, g_tau],
                [0.0, e_g]
            ], dtype=np.float64)

            # Analytical Qd integral
            q11 = (q / (gamma ** 2)) * (dt - (2.0 / gamma) * (1.0 - e_g) + (1.0 / (2.0 * gamma)) * (1.0 - e_2g))
            q12 = (q / gamma) * ((1.0 - e_g) / gamma - (1.0 - e_2g) / (2.0 * gamma))
            q22 = (q / (2.0 * gamma)) * (1.0 - e_2g)

            Qd = np.array([
                [q11, q12],
                [q12, q22]
            ], dtype=np.float64)

        return Phi, Qd

    def predict(
        self,
        delta_t: float
    ) -> Tuple[np.ndarray, np.ndarray]:
        """
        Compute conditional mean state vector and 2x2 covariance matrix at delta_t >= 0.
        """
        Phi, Qd = self.compute_transition_and_covariance(delta_t)
        mu = Phi @ self.s0
        P = Phi @ self.P0 @ Phi.T + Qd
        # Enforce exact symmetry
        P = 0.5 * (P + P.T)
        return mu, P


def simulate_euler_maruyama(
    x0: float,
    drift_fn: Callable[[float], float],
    diffusion_fn: Callable[[float], float],
    t_max: float,
    num_steps: int,
    num_trajectories: int
) -> np.ndarray:
    """
    Vectorized Euler-Maruyama simulation for 1D SDE:
        dx(t) = a(x) dt + b(x) dw(t)
    Returns: array of shape (num_trajectories, num_steps + 1)
    """
    dt = float(t_max) / float(num_steps)
    sqrt_dt = np.sqrt(dt)
    
    trajectories = np.empty((num_trajectories, num_steps + 1), dtype=np.float64)
    trajectories[:, 0] = x0
    
    current_x = np.full(num_trajectories, x0, dtype=np.float64)
    
    for step in range(num_steps):
        # Sample standard normal increments
        dw = np.random.normal(0.0, sqrt_dt, size=num_trajectories)
        drift = drift_fn(current_x)
        diffusion = diffusion_fn(current_x)
        current_x = current_x + drift * dt + diffusion * dw
        trajectories[:, step + 1] = current_x
        
    return trajectories


def evaluate_anchor_evolution_benchmarks() -> Dict[str, Any]:
    """
    Run comprehensive E42 benchmarks for report generation and JSON output.
    """
    np.random.seed(42)
    # 1. Amplitude OU Evolution
    ou = AmplitudeOUEvolution(A0=1.5, A_inf=1.0, lambda_A=2.5, sigma_A=0.35)
    dts = np.array([0.01, 0.05, 0.10, 0.25, 0.50, 1.0])
    amp_results = []
    
    for dt in dts:
        mu, var = ou.predict(dt)
        # Monte Carlo verification
        mc = simulate_euler_maruyama(
            x0=1.5,
            drift_fn=lambda x: -2.5 * (x - 1.0),
            diffusion_fn=lambda x: 0.35,
            t_max=dt,
            num_steps=100,
            num_trajectories=5000
        )
        emp_var = float(np.var(mc[:, -1]))
        rel_err = abs(emp_var - var) / var if var > 0 else 0.0
        amp_results.append({
            'delta_t': float(dt),
            'analytical_mu': float(mu),
            'analytical_var': float(var),
            'analytical_std': float(np.sqrt(var)),
            'empirical_var': float(emp_var),
            'relative_error': float(rel_err)
        })
        
    # 2. Frequency Kinematic Evolution
    kin = FrequencyKinematicEvolution(f0=440.0, f_dot0=5.0, q_f=20.0, gamma_f=0.2)
    freq_results = []
    for dt in dts:
        mu, P = kin.predict(dt)
        freq_results.append({
            'delta_t': float(dt),
            'f_mean': float(mu[0]),
            'f_dot_mean': float(mu[1]),
            'var_f': float(P[0, 0]),
            'std_f': float(np.sqrt(P[0, 0])),
            'cov_f_fdot': float(P[0, 1]),
            'var_fdot': float(P[1, 1])
        })
        
    return {
        'amplitude_ou': amp_results,
        'frequency_kinematic': freq_results
    }
