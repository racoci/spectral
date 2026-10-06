from __future__ import annotations

# Explicit parameter distributions used by the curriculum generator.
# Each dictionary documents the *synthetic ground-truth* domain, not an
# unbounded prior used by the neural network.

DISTRIBUTIONS = {
    "pitch": {
        "midi": [36.0, 84.0],
        "detune_cents": [-30.0, 30.0],
        "pitch_bend_cents": [-200.0, 200.0],
        "pitch_slide_cents": [-300.0, 300.0],
        "phase": [-3.141592653589793, 3.141592653589793],
        "f0_hz_effective": [65.4064, 1046.5023],
    },
    "amplitude": {
        "gain_log10": [-1.0, 0.0],
        "log_amplitude_db": [-24.0, 0.0],
    },
    "adsr": {
        "attack_s": [0.005, 0.10],
        "decay_s": [0.02, 0.30],
        "sustain": [0.25, 0.95],
        "release_s": [0.03, 0.35],
    },
    "harmonics": {
        "K_choices": [2, 4, 8, 12, 16],
        "Hk_log10": [-1.7, -0.1],
        "harmonic_phase": [-3.141592653589793, 3.141592653589793],
    },
    "spectral_envelope": {
        "center_hz": [70.0, 10000.0],
        "db": [-18.0, 6.0],
        "sigma_oct": [0.12, 0.60],
    },
    "inharmonicity": {
        "B_log10": [-6.0, -3.0],
    },
    "lfo": {
        "depth_cents": [2.0, 80.0],
        "rate_hz": [0.5, 12.0],
        "phase": [-3.141592653589793, 3.141592653589793],
        "waveform": ["sine"],
    },
    "fm": {
        "ratio_log2": [-2.0, 3.0],
        "index": [0.01, 8.0],
        "phase": [-3.141592653589793, 3.141592653589793],
    },
    "pm": {
        "ratio_log2": [-2.0, 3.0],
        "index": [0.01, 8.0],
        "phase": [-3.141592653589793, 3.141592653589793],
    },
    "am": {
        "depth": [0.01, 0.90],
        "rate_hz": [0.2, 12.0],
        "phase": [-3.141592653589793, 3.141592653589793],
    },
    "noise": {
        "level": [0.0, 0.5],
        "alpha": [-2.0, 1.0],
        "knee_hz": [300.0, 6000.0],
    },
    "events": {
        "n_notes": [1, 4],
        "onset_jitter_s": [0.0, 0.020],
        "duration_s": [0.15, 1.5],
        "velocity": [0.35, 1.0],
    },
    "voices": {
        "V": [1, 8],
        "pan": [-0.8, 0.8],
    },
    "tree": {
        "width": [1, 8],
        "depth": [1, 4],
        "edge_probability": [0.1, 0.5],
        "edge_gain": [0.05, 1.0],
    },
}
