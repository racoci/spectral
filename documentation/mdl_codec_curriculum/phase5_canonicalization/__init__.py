"""
Phase 5: Canonicalization and Gauges (E39-E41)
Minimum Description Length (MDL) Structural Codec Curriculum
"""

from .equivalence_classes import (
    time_shift_vs_phase,
    fm_vs_pm_angle_modulation,
    filter_vs_spectral_envelope,
    polar_amplitude_phase,
    compute_gauge_jacobian_nullity
)

from .canonical_gauge import (
    CanonicalTimePhase,
    CanonicalAngleModulation,
    CanonicalFilterEnvelope,
    canonicalize_parameter_vector,
    evaluate_canonical_mdl_savings
)

from .ambiguity_distribution import (
    HarmonicCollisionAmbiguity,
    ResonatorVsNoiseAmbiguity,
    CausalLookaheadDisambiguator
)
