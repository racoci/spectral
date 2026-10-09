"""
Phase 6: Evolution and Uncertainty (E42-E45)
Causal Probabilistic Predictive Framework for MDL Structural Audio Codec.
"""

from .gp_anchor_evolution import (
    AmplitudeOUEvolution,
    FrequencyKinematicEvolution,
    simulate_euler_maruyama,
    evaluate_anchor_evolution_benchmarks
)

from .phase_uncertainty import (
    PhaseStochasticIntegrator,
    evaluate_phase_uncertainty_benchmarks
)

from .probabilistic_calibration import (
    CalibrationEvaluator,
    evaluate_calibration_benchmarks
)

from .gp_vs_jet_benchmark import (
    TaylorJetPredictor,
    GPPredictor,
    evaluate_gp_vs_jet_benchmarks
)

__all__ = [
    'AmplitudeOUEvolution',
    'FrequencyKinematicEvolution',
    'simulate_euler_maruyama',
    'evaluate_anchor_evolution_benchmarks',
    'PhaseStochasticIntegrator',
    'evaluate_phase_uncertainty_benchmarks',
    'CalibrationEvaluator',
    'evaluate_calibration_benchmarks',
    'TaylorJetPredictor',
    'GPPredictor',
    'evaluate_gp_vs_jet_benchmarks',
]
