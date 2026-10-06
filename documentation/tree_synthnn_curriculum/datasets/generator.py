from __future__ import annotations

from dataclasses import replace
from pathlib import Path
from typing import Dict, Iterable, Tuple

import numpy as np
import torch

from synthnn import (
    EffectConfig,
    EnvelopeConfig,
    LFOConfig,
    NoteConfig,
    SourceConfig,
    SpectralEnvelopeConfig,
    SynthConfig,
    VoiceConfig,
)


def rng_for(seed: int) -> np.random.Generator:
    return np.random.default_rng(seed)


def sample_log_uniform(rng, lo: float, hi: float) -> float:
    return float(np.exp(rng.uniform(np.log(lo), np.log(hi))))


def sample_note(rng, duration: float = 1.5, onset: float = 0.25, midi_range=(36, 84)) -> NoteConfig:
    return NoteConfig(
        onset=onset,
        duration=duration,
        midi=float(rng.uniform(*midi_range)),
        velocity=float(rng.uniform(.5, 1.0)),
        attack=float(rng.uniform(.005, .10)),
        decay=float(rng.uniform(.02, .30)),
        sustain=float(rng.uniform(.25, .95)),
        release=float(rng.uniform(.03, .35)),
        detune_cents=float(rng.uniform(-30, 30)),
        vibrato_depth_cents=0.0,
        vibrato_rate=5.0,
        phase=float(rng.uniform(-np.pi, np.pi)),
        pan=float(rng.uniform(-.7, .7)),
    )


def base_voice(rng, *, duration=2.0, source_kind="sine", harmonic_count=1) -> VoiceConfig:
    amps = [0.0] * harmonic_count
    amps[0] = 1.0
    return VoiceConfig(
        name="v0",
        source=SourceConfig(
            kind=source_kind,
            harmonic_count=harmonic_count,
            harmonic_amplitudes=amps,
            harmonic_phases=[0.0] * harmonic_count,
            spectral_envelope=SpectralEnvelopeConfig(
                centers_hz=[100, 300, 1000, 3000, 8000],
                values_db=[0, 0, 0, 0, 0],
                sigma_oct=.30,
            ),
        ),
        effects=EffectConfig(),
        envelopes=EnvelopeConfig(),
        notes=[sample_note(rng, duration=min(1.2, duration - .3))],
        gain=.7,
    )


def make_stage_config(stage: str, seed: int = 0) -> Tuple[SynthConfig, Dict]:
    rng = rng_for(seed)
    duration = 2.0
    meta: Dict = {"stage": stage, "seed": seed}

    if stage == "E03":
        voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
        voice.notes[0] = replace(voice.notes[0], midi=float(rng.uniform(45, 75)), attack=.02, decay=.08, sustain=.8, release=.1)
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage == "E04":
        voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
        amp = float(sample_log_uniform(rng, 0.01, 1.0))
        voice.gain = amp
        voice.notes[0] = replace(voice.notes[0], midi=float(rng.uniform(40, 80)), velocity=1.0, attack=.01, decay=.01, sustain=1.0, release=.01)
        meta["true_A"] = amp
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage == "E05":
        voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
        voice.notes[0] = sample_note(rng, duration=1.25)
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage == "E06":
        K = int(rng.choice([2, 4, 8, 12, 16]))
        amps = [1.0] + [float(sample_log_uniform(rng, .02, .8)) for _ in range(K - 1)]
        voice = base_voice(rng, duration=duration, source_kind="additive", harmonic_count=K)
        voice.source.harmonic_amplitudes = amps
        voice.source.harmonic_phases = [float(rng.uniform(-np.pi, np.pi)) for _ in range(K)]
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage == "E08":
        K = 12
        voice = base_voice(rng, duration=duration, source_kind="additive", harmonic_count=K)
        voice.source.inharmonicity = float(10 ** rng.uniform(-6, -3))
        voice.source.harmonic_amplitudes = [1.0] + [1.0 / k for k in range(2, K + 1)]
        meta["true_B"] = voice.source.inharmonicity
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage == "E09":
        voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
        voice.notes[0] = replace(
            voice.notes[0],
            vibrato_depth_cents=float(rng.uniform(2, 80)),
            vibrato_rate=float(rng.uniform(.5, 12)),
            vibrato_phase=float(rng.uniform(-np.pi, np.pi)),
        )
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    if stage in {"E10", "E11", "E12"}:
        voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
        if stage == "E10":
            voice.source.fm_ratio = float(sample_log_uniform(rng, .25, 8.0))
            voice.source.fm_index = float(rng.uniform(.01, 8.0))
            voice.source.fm_phase = float(rng.uniform(-np.pi, np.pi))
        elif stage == "E11":
            voice.source.pm_ratio = float(sample_log_uniform(rng, .25, 8.0))
            voice.source.pm_index = float(rng.uniform(.01, 8.0))
            voice.source.pm_phase = float(rng.uniform(-np.pi, np.pi))
        else:
            voice.notes[0] = replace(
                voice.notes[0],
                tremolo_depth=float(rng.uniform(.01, .9)),
                tremolo_rate=float(rng.uniform(.2, 12)),
                tremolo_phase=float(rng.uniform(-np.pi, np.pi)),
            )
        return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta

    # Conservative fallback: simple sine note. More elaborate generators can be
    # added stage-by-stage without changing the training protocol.
    voice = base_voice(rng, duration=duration, source_kind="sine", harmonic_count=1)
    return SynthConfig(sample_rate=12000, duration=duration, seed=seed, voices=[voice]), meta


def render_example(stage: str, seed: int = 0):
    from synthnn import SynthNN
    cfg, meta = make_stage_config(stage, seed)
    model = SynthNN(cfg)
    y = model.render(normalize=False)
    return y.detach().cpu(), meta, cfg
