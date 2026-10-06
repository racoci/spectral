from __future__ import annotations

from dataclasses import dataclass, asdict
from typing import Dict, List, Tuple


@dataclass(frozen=True)
class StageSpec:
    id: str
    name: str
    purpose: str
    trainable_groups: Tuple[str, ...]
    frozen_groups: Tuple[str, ...]
    train_examples: int
    val_examples: int
    iid_test_examples: int
    compositional_test_examples: int
    structural_ood_examples: int
    hard_test_examples: int
    duration_s: float
    sample_rate: int
    max_voices: int
    max_nodes: int
    max_edges: int
    jet_order: int
    architecture: str
    optimizer: str
    lr: float
    weight_decay: float
    loss_terms: Tuple[str, ...]
    promotion: Tuple[str, ...]
    notes: str = ""

    def to_dict(self) -> Dict:
        d = asdict(self)
        for k in ("trainable_groups", "frozen_groups", "loss_terms", "promotion"):
            d[k] = list(d[k])
        return d


# Global parameter groups refer to semantic controls, not raw tensor names.
PARAM_GROUPS = (
    "master",
    "events",
    "pitch",
    "envelope",
    "harmonics",
    "spectral_envelope",
    "inharmonicity",
    "lfo",
    "fm",
    "pm",
    "am",
    "noise",
    "source_family",
    "filter",
    "delay",
    "chorus",
    "reverb",
    "spatial",
    "modulation_graph",
    "graph_structure",
    "analysis_residual",
)


def _common(
    stage_id: str,
    name: str,
    purpose: str,
    trainable: Tuple[str, ...],
    *,
    train_examples: int,
    val_examples: int,
    iid: int,
    comp: int,
    struct: int,
    hard: int,
    max_voices: int = 1,
    max_nodes: int = 1,
    max_edges: int = 0,
    jet_order: int = 2,
    architecture: str = "analytic / no trainable network",
    optimizer: str = "none",
    lr: float = 0.0,
    weight_decay: float = 0.0,
    losses: Tuple[str, ...] = ("L_x",),
    promotion: Tuple[str, ...] = (),
    notes: str = "",
) -> StageSpec:
    frozen = tuple(g for g in PARAM_GROUPS if g not in trainable)
    return StageSpec(
        id=stage_id,
        name=name,
        purpose=purpose,
        trainable_groups=trainable,
        frozen_groups=frozen,
        train_examples=train_examples,
        val_examples=val_examples,
        iid_test_examples=iid,
        compositional_test_examples=comp,
        structural_ood_examples=struct,
        hard_test_examples=hard,
        duration_s=2.0 if max_voices == 1 else 4.0,
        sample_rate=12000,
        max_voices=max_voices,
        max_nodes=max_nodes,
        max_edges=max_edges,
        jet_order=jet_order,
        architecture=architecture,
        optimizer=optimizer,
        lr=lr,
        weight_decay=weight_decay,
        loss_terms=losses,
        promotion=promotion,
        notes=notes,
    )


STAGES: List[StageSpec] = [
    _common("E00", "identifiability", "Measure rank/conditioning before learning.", (), train_examples=4000, val_examples=500, iid=1000, comp=1000, struct=0, hard=1000, architecture="Jacobian + SVD", losses=("rank", "condition_number"), promotion=("full_rank_on_target_subspace", "min_sigma > 1e-3 * max_sigma")),
    _common("E01", "cqt_and_jets", "Validate analytic CQT/reassignment/jets against ground truth.", (), train_examples=10000, val_examples=1000, iid=2000, comp=2000, struct=0, hard=2000, jet_order=4, architecture="fixed frontend", losses=("L_f", "L_df", "L_d2f", "L_d3f", "L_d4f"), promotion=("median_pitch_error < 1_cent", "median_derivative_relative_error < 0.02")),
    _common("E02", "ridge", "Detect/track one ridge.", ("analysis_residual",), train_examples=12000, val_examples=1500, iid=3000, comp=3000, struct=0, hard=3000, architecture="linear -> 16 -> 1", optimizer="AdamW", lr=3e-3, weight_decay=1e-4, losses=("L_f", "L_track", "L_smooth"), promotion=("IID_cents_RMSE < 2", "hard_cents_RMSE < 8", "track_F1 > 0.995")),
    _common("E03", "pitch", "Recover f0 from a clean mono-sine component.", ("pitch",), train_examples=16000, val_examples=2000, iid=4000, comp=4000, struct=0, hard=4000, architecture="32 -> 16 -> 3 heads", optimizer="AdamW", lr=2e-3, weight_decay=1e-4, losses=("L_f", "L_df", "L_d2f"), promotion=("IID_RMSE < 1_cent", "OOD_RMSE < 3_cents", "no systematic_bias")),
    _common("E04", "amplitude", "Recover scalar amplitude only.", ("envelope",), train_examples=10000, val_examples=1000, iid=2000, comp=2000, struct=0, hard=2000, architecture="linear -> 16 -> 1", optimizer="AdamW", lr=2e-3, weight_decay=1e-4, losses=("L_logA",), promotion=("RMSE < 0.1_dB", "intervention_isolation > 20dB")),
    _common("E05", "adsr", "Recover attack/decay/sustain/release.", ("envelope",), train_examples=24000, val_examples=3000, iid=5000, comp=5000, struct=0, hard=5000, architecture="64 -> 32 -> 4 constrained heads", optimizer="AdamW", lr=1e-3, weight_decay=1e-4, losses=("L_onset", "L_tau", "L_sustain", "L_env"), promotion=("median_ADSR_relative_error < 0.05", "waveform_RMSE_gain < 1e-3")),
    _common("E06", "harmonics", "Recover H_k with shared harmonic decoder.", ("harmonics",), train_examples=30000, val_examples=4000, iid=6000, comp=6000, struct=0, hard=6000, architecture="harmonic embedding + shared MLP", optimizer="AdamW", lr=1e-3, weight_decay=1e-4, losses=("L_H", "L_CQT", "L_x"), promotion=("H_RMSE < 0.01", "k>8_extrapolation_RMSE < 0.03")),
    _common("E07", "spectral_envelope", "Recover low-dimensional E(f) basis coefficients.", ("spectral_envelope",), train_examples=30000, val_examples=4000, iid=6000, comp=6000, struct=0, hard=6000, architecture="basis projection: 2 -> 3 -> spline", optimizer="AdamW", lr=8e-4, weight_decay=1e-4, losses=("L_Edb", "L_CQT", "L_x"), promotion=("median_envelope_error < 0.75_dB", "gauge_constraints < 1e-4")),
    _common("E08", "inharmonicity", "Recover B after harmonic/envelope disentanglement.", ("inharmonicity",), train_examples=16000, val_examples=2000, iid=4000, comp=4000, struct=0, hard=4000, architecture="analytic LS + residual MLP(16,8)", optimizer="AdamW", lr=5e-4, weight_decay=1e-5, losses=("L_B", "L_freq"), promotion=("median_relative_B_error < 0.05", "no_sign_errors")),
    _common("E09", "lfo", "Recover sinusoidal modulation parameters.", ("lfo",), train_examples=24000, val_examples=3000, iid=5000, comp=5000, struct=0, hard=5000, architecture="64 -> 32 -> 4 + phase vector head", optimizer="AdamW", lr=1e-3, weight_decay=1e-4, losses=("L_depth", "L_freq", "L_phase", "L_track"), promotion=("freq_relative_error < 0.02", "phase_circular_error < 5deg")),
    _common("E10", "fm", "Recover FM carrier/modulator/index/phase.", ("fm",), train_examples=30000, val_examples=4000, iid=6000, comp=6000, struct=0, hard=6000, architecture="96 -> 48 -> 8", optimizer="AdamW", lr=8e-4, weight_decay=1e-4, losses=("L_FM_params", "L_J", "L_x"), promotion=("median_parameter_error < 0.05", "FM_classification_accuracy > 0.995")),
    _common("E11", "pm", "Recover PM parameters in isolation from FM.", ("pm",), train_examples=24000, val_examples=3000, iid=5000, comp=5000, struct=0, hard=5000, architecture="96 -> 48 -> 8", optimizer="AdamW", lr=8e-4, weight_decay=1e-4, losses=("L_PM_params", "L_J", "L_x"), promotion=("median_parameter_error < 0.05", "PM_accuracy > 0.995")),
    _common("E12", "am", "Recover AM/tremolo separately.", ("am",), train_examples=24000, val_examples=3000, iid=5000, comp=5000, struct=0, hard=5000, architecture="64 -> 32 -> 6", optimizer="AdamW", lr=1e-3, weight_decay=1e-4, losses=("L_AM_params", "L_env", "L_x"), promotion=("depth_RMSE < 0.01", "freq_relative_error < 0.02")),
    _common("E13", "noise", "Recover noise level and spectral slope/knee.", ("noise",), train_examples=30000, val_examples=4000, iid=6000, comp=6000, struct=0, hard=6000, architecture="64 -> 32 -> 6", optimizer="AdamW", lr=7e-4, weight_decay=1e-4, losses=("L_noise_psd", "L_alpha", "L_knee", "L_x"), promotion=("PSD_log_RMSE < 1dB", "alpha_abs_error < 0.05")),
    _common("E14", "effects", "Identify nonlocal effects one at a time.", ("filter", "delay", "chorus", "reverb", "spatial"), train_examples=50000, val_examples=6000, iid=10000, comp=10000, struct=0, hard=10000, architecture="effect-specific heads", optimizer="AdamW", lr=5e-4, weight_decay=1e-4, losses=("L_multiscale_STFT", "L_CQT", "L_x"), promotion=("single_effect_parameter_error < 5_percent", "effect_on_off_accuracy > 0.99"), notes="Train each effect separately before combined-effects curriculum."),
    _common("E15", "events", "Recover note onsets, offsets and pitch.", ("events",), train_examples=50000, val_examples=6000, iid=10000, comp=10000, struct=0, hard=10000, architecture="temporal conv/MLP + onset offset heads", optimizer="AdamW", lr=8e-4, weight_decay=1e-4, losses=("L_onset", "L_offset", "L_pitch", "L_velocity"), promotion=("onset_F1 > 0.995", "median_onset_error < 2ms", "pitch_error < 3c")),
    _common("E16", "known_tree", "Learn modulation parameters with topology fixed.", ("modulation_graph",), train_examples=40000, val_examples=5000, iid=8000, comp=8000, struct=0, hard=8000, max_nodes=4, max_edges=4, architecture="message-passing tree, topology supplied", optimizer="AdamW", lr=5e-4, weight_decay=1e-4, losses=("L_edge_param", "L_J", "L_x"), promotion=("edge_strength_RMSE < 0.02", "audio_RMSE < 1e-3")),
    _common("E17", "edge_inference", "Infer edges while node identities stay known.", ("graph_structure",), train_examples=60000, val_examples=8000, iid=12000, comp=12000, struct=6000, hard=12000, max_nodes=6, max_edges=8, architecture="shared node encoder + pairwise edge head", optimizer="AdamW", lr=5e-4, weight_decay=2e-4, losses=("L_edge", "L_degree", "L_topology"), promotion=("edge_F1 > 0.99", "exact_topology > 0.95", "OOD_edge_F1 > 0.95")),
    _common("E18", "variable_topology", "Infer variable-width/depth trees.", ("graph_structure",), train_examples=100000, val_examples=12000, iid=20000, comp=20000, struct=12000, hard=20000, max_nodes=10, max_edges=18, architecture="autoregressive TreeNN", optimizer="AdamW", lr=3e-4, weight_decay=2e-4, losses=("L_graph", "L_node_type", "L_edge", "L_struct_cost"), promotion=("exact_topology > 0.90", "GED_normalized < 0.05", "OOD_depth_accuracy > 0.80")),
    _common("E19", "module_selection", "Infer module type among FM/PM/AM/LFO/noise.", ("graph_structure",), train_examples=120000, val_examples=15000, iid=25000, comp=25000, struct=15000, hard=25000, max_nodes=12, max_edges=22, architecture="TreeNN + Gumbel-Softmax type head", optimizer="AdamW", lr=2e-4, weight_decay=2e-4, losses=("L_type", "L_graph", "L_CQT", "L_x"), promotion=("module_type_accuracy > 0.98", "exact_graph > 0.90", "temperature_anneal_stable")),
    _common("E20", "analysis_by_synthesis", "Jointly refine inferred graph/parameters through SynthNN.", ("graph_structure", "analysis_residual"), train_examples=150000, val_examples=20000, iid=30000, comp=30000, struct=20000, hard=30000, max_nodes=12, max_edges=22, architecture="encoder + TreeNN + frozen SynthNN + residual heads", optimizer="AdamW", lr=1e-4, weight_decay=1e-5, losses=("L_x", "L_CQT", "L_J", "L_graph", "L_sparsity"), promotion=("audio_SI-SDR > 30dB", "CQT_log_RMSE < 1dB", "graph_exact > 0.90")),
    _common("E21", "predictive_codec", "Predict future windows without observing future samples.", ("analysis_residual", "graph_structure"), train_examples=200000, val_examples=25000, iid=40000, comp=40000, struct=25000, hard=40000, max_voices=8, max_nodes=16, max_edges=30, jet_order=4, architecture="causal state predictor + TreeNN + SynthNN", optimizer="AdamW", lr=1e-4, weight_decay=1e-5, losses=("L_future_wave", "L_future_CQT", "L_future_J", "L_rate", "L_graph"), promotion=("report E(H) curve", "beats_baseline_at_same_bits", "stable_online_update")),
]


def stages_as_dict() -> List[Dict]:
    return [s.to_dict() for s in STAGES]
