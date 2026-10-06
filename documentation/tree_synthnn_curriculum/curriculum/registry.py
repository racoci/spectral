from __future__ import annotations

import re
from typing import Dict, Iterable, List, Tuple

from synthnn import SynthNN


# The registry is intentionally semantic. It is used for two things:
# (1) validating which SynthNN parameters belong to an oracle subproblem; and
# (2) freezing/unfreezing exact tensor groups during analysis-by-synthesis.
# The inverse neural network heads are separate and should use the same group names.
RULES: List[Tuple[str, str]] = [
    ("master", r"^master_(gain|drive|pan)$"),
    ("master", r"^global_"),
    ("master", r"^voices\.\d+\.gain$"),
    ("events", r"\.note_params\.\d+\.(onset|duration|midi|velocity)$"),
    ("pitch", r"\.note_params\.\d+\.(detune_cents|pitch_bend_cents|pitch_slide_cents|phase)$"),
    ("lfo", r"\.note_params\.\d+\.(vibrato_depth_cents|vibrato_rate|vibrato_phase)$"),
    ("am", r"\.note_params\.\d+\.(tremolo_depth|tremolo_rate|tremolo_phase)$"),
    ("envelope", r"\.note_params\.\d+\.(attack|decay|sustain|release)$"),
    ("envelope", r"\.default_(attack|decay|sustain|release)$"),
    ("harmonics", r"\.timbre\.(harmonic_amplitudes|harmonic_phases)$"),
    ("spectral_envelope", r"\.timbre\.(spectral_centers_log2|spectral_db|spectral_sigma)$"),
    ("inharmonicity", r"\.timbre\.inharmonicity$"),
    ("fm", r"\.timbre\.(fm_ratio|fm_index|fm_phase)$"),
    ("pm", r"\.timbre\.(pm_ratio|pm_index|pm_phase)$"),
    ("noise", r"\.timbre\.(source_noise|fractal_alpha|fractal_knee|noise_mask_strength|vocal_aspiration)$"),
    ("source_family", r"\.timbre\.(vocal_voicing|vocal_open_quotient|vocal_spectral_tilt|vocal_jitter|vocal_shimmer|formant_hz|formant_bw|formant_gain_db|pluck_position|damping|granular_density|granular_size|wavetable)$"),
    ("filter", r"\.effects\.filter_(cutoff|resonance)$"),
    ("filter", r"\.effects\.drive$"),
    ("delay", r"\.effects\.delay_(time|feedback|mix)$"),
    ("chorus", r"\.effects\.chorus_"),
    ("reverb", r"\.effects\.reverb_"),
    ("spatial", r"\.effects\.pan$"),
    ("spatial", r"\.note_params\.\d+\.pan$"),
    ("modulation_graph", r"\.modulation\.params\."),
]


def ensure_runtime_parameters(model: SynthNN) -> None:
    """Materialize per-note ParameterDicts before inspecting named_parameters()."""
    for voice in model.voices:
        voice._ensure_note_parameters(len(voice.runtime_notes))


def classify(name: str) -> str:
    for group, pattern in RULES:
        if re.search(pattern, name):
            return group
    return "unassigned"


def parameter_groups(model: SynthNN) -> Dict[str, List[str]]:
    ensure_runtime_parameters(model)
    groups: Dict[str, List[str]] = {}
    for name, _ in model.named_parameters():
        groups.setdefault(classify(name), []).append(name)
    return groups


def set_trainable_groups(model: SynthNN, trainable: Iterable[str]) -> Dict[str, int]:
    ensure_runtime_parameters(model)
    trainable = set(trainable)
    counts: Dict[str, int] = {}
    for name, p in model.named_parameters():
        group = classify(name)
        p.requires_grad_(group in trainable)
        counts[group] = counts.get(group, 0) + p.numel()
    return counts


def trainable_summary(model: SynthNN) -> Dict[str, int]:
    ensure_runtime_parameters(model)
    out: Dict[str, int] = {}
    for name, p in model.named_parameters():
        if p.requires_grad:
            g = classify(name)
            out[g] = out.get(g, 0) + p.numel()
    return out
