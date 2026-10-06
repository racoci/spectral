from __future__ import annotations

from dataclasses import dataclass, field, asdict
from typing import Any, Dict, List, Optional, Sequence, Tuple, Literal
from pathlib import Path
import json
import math

import numpy as np
import torch
import torch.nn as nn
import torch.nn.functional as F


Tensor = torch.Tensor


# -----------------------------------------------------------------------------
# Serializable configuration objects
# -----------------------------------------------------------------------------

@dataclass
class LFOConfig:
    name: str
    freq: float = 5.0
    depth: float = 1.0
    phase: float = 0.0
    waveform: str = "sine"
    delay: float = 0.0
    fade: float = 0.0
    seed: int = 0
    retrigger: bool = True
    sync: bool = False
    custom_coeffs: List[Tuple[int, float, float]] = field(default_factory=list)


@dataclass
class CurveConfig:
    points: List[Tuple[float, float]] = field(default_factory=list)
    kind: str = "linear"
    sigma: float = 0.15


@dataclass
class EnvelopeConfig:
    attack: float = 0.015
    decay: float = 0.12
    sustain: float = 0.72
    release: float = 0.18
    attack_sharpness: float = 4.0
    decay_sharpness: float = 4.0
    release_sharpness: float = 4.0
    curve: CurveConfig = field(default_factory=CurveConfig)


@dataclass
class ArpStepConfig:
    index: int
    octave: int = 0
    velocity: float = 1.0
    gate: float = 0.9
    duration_mult: float = 1.0
    accent: float = 0.0
    transpose: int = 0
    probability: float = 1.0
    ratchet: int = 1


@dataclass
class ArpeggiatorConfig:
    notes: List[int]
    base_midi: int = 60
    velocity: float = 1.0
    start: float = 0.0
    end: Optional[float] = None
    rate: float = 1 / 8
    direction: str = "up_down"
    octaves: int = 2
    gate: float = 0.8
    swing: float = 0.0
    phase: float = 0.0
    steps: List[ArpStepConfig] = field(default_factory=list)
    seed: int = 0


@dataclass
class NoteConfig:
    onset: float
    duration: float
    midi: float
    velocity: float = 1.0
    attack: float = 0.015
    decay: float = 0.12
    sustain: float = 0.72
    release: float = 0.18
    detune_cents: float = 0.0
    pitch_bend_cents: float = 0.0
    vibrato_depth_cents: float = 0.0
    vibrato_rate: float = 5.0
    vibrato_phase: float = 0.0
    tremolo_depth: float = 0.0
    tremolo_rate: float = 4.0
    tremolo_phase: float = 0.0
    portamento: float = 0.0
    glide_to: Optional[float] = None
    phase: float = 0.0
    pan: float = 0.0
    aftertouch: float = 0.0
    pitch_slide_cents: float = 0.0
    timbre: float = 0.0
    pitch_curve: CurveConfig = field(default_factory=CurveConfig)
    amp_curve: CurveConfig = field(default_factory=CurveConfig)
    timbre_curve: CurveConfig = field(default_factory=CurveConfig)


@dataclass
class ModNodeConfig:
    name: str
    kind: str = "lfo"
    target: Optional[str] = None
    inputs: List[str] = field(default_factory=list)
    depth: float = 0.0
    freq: float = 1.0
    phase: float = 0.0
    waveform: str = "sine"
    value: float = 0.0
    sigma: float = 0.2
    alpha: float = 0.0
    seed: int = 0


@dataclass
class SpectralEnvelopeConfig:
    centers_hz: List[float] = field(default_factory=lambda: [70, 180, 400, 900, 1800, 3600, 7200])
    values_db: List[float] = field(default_factory=lambda: [0, 0, 0, -1, -4, -9, -15])
    sigma_oct: float = 0.30


@dataclass
class SourceConfig:
    kind: str = "additive"
    harmonic_count: int = 16
    harmonic_amplitudes: List[float] = field(default_factory=lambda: [1.0] + [0.0] * 15)
    harmonic_phases: List[float] = field(default_factory=lambda: [0.0] * 16)
    inharmonicity: float = 0.0
    spectral_envelope: SpectralEnvelopeConfig = field(default_factory=SpectralEnvelopeConfig)

    # FM / PM
    fm_ratio: float = 1.0
    fm_index: float = 0.0
    fm_phase: float = 0.0
    pm_ratio: float = 1.0
    pm_index: float = 0.0
    pm_phase: float = 0.0

    # Noise / excitation
    source_noise: float = 0.0
    fractal_alpha: float = -0.5
    fractal_knee: float = 2000.0
    noise_mask_strength: float = 0.0
    noise_mask_mode: str = "envelope"

    # Vocal model
    vocal_voicing: float = 1.0
    vocal_open_quotient: float = 0.6
    vocal_spectral_tilt_db_oct: float = -6.0
    vocal_aspiration: float = 0.0
    vocal_jitter_cents: float = 0.0
    vocal_shimmer: float = 0.0
    formant_hz: List[float] = field(default_factory=lambda: [700.0, 1200.0, 2500.0])
    formant_bw_hz: List[float] = field(default_factory=lambda: [100.0, 120.0, 180.0])
    formant_gain_db: List[float] = field(default_factory=lambda: [0.0, -3.0, -6.0])

    # Physical / procedural source controls
    pluck_position: float = 0.2
    damping: float = 0.995
    membrane_modes: int = 6
    granular_density: float = 20.0
    granular_size: float = 0.04
    wavetable: Optional[List[float]] = None


@dataclass
class EffectConfig:
    filter_type: str = "none"
    filter_cutoff: float = 12000.0
    filter_resonance: float = 0.0
    drive: float = 0.0
    chorus_mix: float = 0.0
    chorus_depth: float = 0.0
    chorus_rate: float = 0.2
    chorus_base_delay: float = 0.012
    chorus_feedback: float = 0.0
    delay_time: float = 0.0
    delay_feedback: float = 0.0
    delay_mix: float = 0.0
    reverb_mix: float = 0.0
    reverb_decay: float = 1.5
    reverb_size: float = 0.7
    pan: float = 0.0


@dataclass
class VoiceConfig:
    name: str = "voice"
    source: SourceConfig = field(default_factory=SourceConfig)
    effects: EffectConfig = field(default_factory=EffectConfig)
    envelopes: EnvelopeConfig = field(default_factory=EnvelopeConfig)
    lfos: List[LFOConfig] = field(default_factory=list)
    modulation: List[ModNodeConfig] = field(default_factory=list)
    notes: List[NoteConfig] = field(default_factory=list)
    arpeggiators: List[ArpeggiatorConfig] = field(default_factory=list)
    gain: float = 1.0


@dataclass
class GlobalConfig:
    gain: float = 0.8
    drive: float = 0.0
    pan: float = 0.0
    delay_time: float = 0.0
    delay_feedback: float = 0.0
    delay_mix: float = 0.0
    reverb_mix: float = 0.0
    reverb_decay: float = 1.5
    reverb_size: float = 0.7


@dataclass
class SynthConfig:
    sample_rate: int = 12000
    duration: float = 8.0
    seed: int = 0
    voices: List[VoiceConfig] = field(default_factory=list)
    global_config: GlobalConfig = field(default_factory=GlobalConfig)


# -----------------------------------------------------------------------------
# Utility functions
# -----------------------------------------------------------------------------


def cents_to_ratio(cents: Tensor | float) -> Tensor:
    c = torch.as_tensor(cents, dtype=torch.get_default_dtype())
    return torch.pow(torch.tensor(2.0, dtype=c.dtype, device=c.device), c / 1200.0)


def midi_to_hz(midi: Tensor | float) -> Tensor:
    m = torch.as_tensor(midi, dtype=torch.get_default_dtype())
    return 440.0 * torch.pow(2.0, (m - 69.0) / 12.0)


def smooth_step(x: Tensor, sharpness: float = 4.0) -> Tensor:
    return 0.5 * (1.0 + torch.erf(sharpness * x / math.sqrt(2.0)))


def gaussian_rbf(t: Tensor, centers: Tensor, values: Tensor, sigma: Tensor) -> Tensor:
    d = (t[..., None] - centers[None, ...]) / torch.clamp(sigma, min=1e-7)
    w = torch.exp(-0.5 * d.square())
    return (w * values[None, :]).sum(-1) / torch.clamp(w.sum(-1), min=1e-8)


def smooth_adsr(
    tau: Tensor,
    attack: Tensor,
    decay: Tensor,
    sustain: Tensor,
    release: Tensor,
    duration: Tensor,
    attack_sharpness: float = 4.0,
    decay_sharpness: float = 4.0,
    release_sharpness: float = 4.0,
) -> Tensor:
    # C-infinity approximation formed from Gaussian-CDF transitions.
    a = torch.clamp(attack, min=1e-5)
    d = torch.clamp(decay, min=1e-5)
    r = torch.clamp(release, min=1e-5)
    ka = smooth_step(tau / a, attack_sharpness)
    kd = smooth_step((tau - a) / d, decay_sharpness)
    kr = smooth_step((tau - duration) / r, release_sharpness)
    return ka - (1.0 - sustain) * kd - sustain * kr


def waveform(ph: Tensor, kind: str) -> Tensor:
    # ph is cycles, not radians.
    p = torch.remainder(ph, 1.0)
    if kind == "sine":
        return torch.sin(2 * math.pi * p)
    if kind == "triangle":
        return 4.0 * torch.abs(p - 0.5) - 1.0
    if kind == "saw":
        return 2.0 * p - 1.0
    if kind == "reverse_saw":
        return 1.0 - 2.0 * p
    if kind == "square":
        return torch.where(p < 0.5, torch.ones_like(p), -torch.ones_like(p))
    if kind == "sample_hold":
        return torch.sin(2 * math.pi * torch.floor(p * 16.0) / 16.0)
    if kind == "noise":
        return torch.sin(2 * math.pi * (12.9898 * p + 78.233))
    if kind == "gaussian_process":
        # Smooth low-dimensional stochastic-like process.
        y = torch.zeros_like(p)
        for k in range(1, 9):
            y = y + torch.sin(2 * math.pi * k * p + 0.37 * k) / (k ** 1.25)
        return y / 1.45
    if kind == "chaotic":
        x = 0.5 + 0.45 * torch.sin(2 * math.pi * p)
        for _ in range(5):
            x = 3.92 * x * (1.0 - x)
        return 2.0 * x - 1.0
    return torch.sin(2 * math.pi * p)


def soft_clip(x: Tensor, drive: Tensor | float) -> Tensor:
    d = torch.as_tensor(drive, dtype=x.dtype, device=x.device)
    amount = 1.0 + 10.0 * torch.clamp(d, min=0.0)
    return torch.tanh(amount * x) / torch.clamp(torch.tanh(amount), min=1e-7)


def pinkish_noise(n: int, sr: int, alpha: Tensor, knee: Tensor, gen: torch.Generator, device: torch.device, dtype: torch.dtype) -> Tensor:
    # FFT-domain deterministic fractal-noise generator. alpha is power-spectrum slope.
    w = torch.randn(n, generator=gen, device=device, dtype=dtype)
    X = torch.fft.rfft(w)
    f = torch.fft.rfftfreq(n, 1.0 / sr, device=device, dtype=dtype)
    f_safe = torch.clamp(f, min=1.0)
    H = torch.pow(f_safe, alpha / 2.0)
    H = H / torch.sqrt(1.0 + (f / torch.clamp(knee, min=1.0)).square())
    y = torch.fft.irfft(X * H, n=n)
    return y / torch.clamp(torch.std(y), min=1e-7)


def deterministic_white_noise(n: int, seed: int, device: torch.device, dtype: torch.dtype) -> Tensor:
    gen = torch.Generator(device=device)
    gen.manual_seed(int(seed))
    return torch.randn(n, generator=gen, device=device, dtype=dtype)


# -----------------------------------------------------------------------------
# Trainable scalar/tensor parameter container
# -----------------------------------------------------------------------------

class Param(nn.Module):
    def __init__(self, value: Any, dtype: torch.dtype = torch.float32):
        super().__init__()
        arr = torch.as_tensor(value, dtype=dtype)
        self.value = nn.Parameter(arr.clone())

    def forward(self) -> Tensor:
        return self.value


# -----------------------------------------------------------------------------
# Modulation graph: arbitrary DAG over parameter controls
# -----------------------------------------------------------------------------

class ModulationGraph(nn.Module):
    def __init__(self, specs: Sequence[ModNodeConfig], dtype=torch.float32):
        super().__init__()
        self.specs = {s.name: s for s in specs}
        self.params = nn.ModuleDict()
        for s in specs:
            self.params[s.name] = nn.ParameterDict({
                "depth": nn.Parameter(torch.tensor(s.depth, dtype=dtype)),
                "freq": nn.Parameter(torch.tensor(s.freq, dtype=dtype)),
                "phase": nn.Parameter(torch.tensor(s.phase, dtype=dtype)),
                "value": nn.Parameter(torch.tensor(s.value, dtype=dtype)),
                "sigma": nn.Parameter(torch.tensor(s.sigma, dtype=dtype)),
                "alpha": nn.Parameter(torch.tensor(s.alpha, dtype=dtype)),
            })

    def _toposort(self) -> List[str]:
        state: Dict[str, int] = {}
        order: List[str] = []
        def visit(name: str):
            mark = state.get(name, 0)
            if mark == 1:
                raise ValueError(f"modulation cycle involving {name}")
            if mark == 2:
                return
            state[name] = 1
            spec = self.specs[name]
            for child in spec.inputs:
                if child not in self.specs:
                    raise KeyError(f"unknown modulation node: {child}")
                visit(child)
            state[name] = 2
            order.append(name)
        for name in self.specs:
            visit(name)
        return order

    def forward(self, t: Tensor, envelope: Tensor, base: Dict[str, Tensor]) -> Dict[str, Tensor]:
        if not self.specs:
            return {}
        values: Dict[str, Tensor] = {}
        for name in self._toposort():
            spec = self.specs[name]
            p = self.params[name]
            depth = p["depth"]
            freq = p["freq"]
            phase = p["phase"]
            value = p["value"]
            sigma = torch.clamp(p["sigma"], min=1e-5)
            alpha = p["alpha"]
            if spec.kind == "lfo":
                ph = freq * t + phase / (2 * math.pi)
                y = depth * waveform(ph, spec.waveform)
            elif spec.kind == "envelope":
                y = depth * envelope
            elif spec.kind == "constant":
                y = value.expand_as(t)
            elif spec.kind == "noise":
                y = depth * torch.sin(2 * math.pi * (12.345 * t + phase))
            elif spec.kind == "parameter":
                key = spec.target or spec.name
                y = depth * base.get(key, torch.zeros_like(t))
            elif spec.kind in {"sum", "product", "sine", "tanh", "exp"}:
                if not spec.inputs:
                    y = torch.zeros_like(t)
                else:
                    xs = [values[k] for k in spec.inputs]
                    if spec.kind == "sum":
                        y = depth * sum(xs)
                    elif spec.kind == "product":
                        y = depth * torch.stack(xs).prod(0)
                    elif spec.kind == "sine":
                        y = depth * torch.sin(torch.stack(xs).sum(0) + phase)
                    elif spec.kind == "tanh":
                        y = depth * torch.tanh(sum(xs))
                    else:
                        y = depth * torch.exp(torch.clamp(sum(xs), -20, 20))
            elif spec.kind == "drift":
                y = depth * (0.65 * torch.sin(2 * math.pi * freq * t + phase) + 0.35 * torch.sin(2 * math.pi * (0.31 * freq + 0.07) * t + 1.7 * phase))
            elif spec.kind == "gaussian":
                y = depth * torch.exp(-0.5 * ((t - value) / sigma).square())
            elif spec.kind == "power_noise":
                y = depth * torch.sin(2 * math.pi * freq * t) * torch.exp(-alpha * t)
            else:
                y = torch.zeros_like(t)
            values[name] = y
        return values


# -----------------------------------------------------------------------------
# Trainable timbre parameters
# -----------------------------------------------------------------------------

class TimbreParameters(nn.Module):
    def __init__(self, src: SourceConfig, dtype=torch.float32):
        super().__init__()
        K = max(1, int(src.harmonic_count))
        H = list(src.harmonic_amplitudes)[:K] + [0.0] * max(0, K - len(src.harmonic_amplitudes))
        P = list(src.harmonic_phases)[:K] + [0.0] * max(0, K - len(src.harmonic_phases))
        self.harmonic_amplitudes = nn.Parameter(torch.tensor(H, dtype=dtype))
        self.harmonic_phases = nn.Parameter(torch.tensor(P, dtype=dtype))
        self.inharmonicity = nn.Parameter(torch.tensor(src.inharmonicity, dtype=dtype))
        self.spectral_centers_log2 = nn.Parameter(torch.log2(torch.tensor(src.spectral_envelope.centers_hz, dtype=dtype)))
        self.spectral_db = nn.Parameter(torch.tensor(src.spectral_envelope.values_db, dtype=dtype))
        self.spectral_sigma = nn.Parameter(torch.tensor(src.spectral_envelope.sigma_oct, dtype=dtype))
        self.fm_ratio = nn.Parameter(torch.tensor(src.fm_ratio, dtype=dtype))
        self.fm_index = nn.Parameter(torch.tensor(src.fm_index, dtype=dtype))
        self.fm_phase = nn.Parameter(torch.tensor(src.fm_phase, dtype=dtype))
        self.pm_ratio = nn.Parameter(torch.tensor(src.pm_ratio, dtype=dtype))
        self.pm_index = nn.Parameter(torch.tensor(src.pm_index, dtype=dtype))
        self.pm_phase = nn.Parameter(torch.tensor(src.pm_phase, dtype=dtype))
        self.source_noise = nn.Parameter(torch.tensor(src.source_noise, dtype=dtype))
        self.fractal_alpha = nn.Parameter(torch.tensor(src.fractal_alpha, dtype=dtype))
        self.fractal_knee = nn.Parameter(torch.tensor(src.fractal_knee, dtype=dtype))
        self.noise_mask_strength = nn.Parameter(torch.tensor(src.noise_mask_strength, dtype=dtype))
        self.vocal_voicing = nn.Parameter(torch.tensor(src.vocal_voicing, dtype=dtype))
        self.vocal_open_quotient = nn.Parameter(torch.tensor(src.vocal_open_quotient, dtype=dtype))
        self.vocal_spectral_tilt = nn.Parameter(torch.tensor(src.vocal_spectral_tilt_db_oct, dtype=dtype))
        self.vocal_aspiration = nn.Parameter(torch.tensor(src.vocal_aspiration, dtype=dtype))
        self.vocal_jitter = nn.Parameter(torch.tensor(src.vocal_jitter_cents, dtype=dtype))
        self.vocal_shimmer = nn.Parameter(torch.tensor(src.vocal_shimmer, dtype=dtype))
        self.formant_hz = nn.Parameter(torch.tensor(src.formant_hz, dtype=dtype))
        self.formant_bw = nn.Parameter(torch.tensor(src.formant_bw_hz, dtype=dtype))
        self.formant_gain_db = nn.Parameter(torch.tensor(src.formant_gain_db, dtype=dtype))
        self.pluck_position = nn.Parameter(torch.tensor(src.pluck_position, dtype=dtype))
        self.damping = nn.Parameter(torch.tensor(src.damping, dtype=dtype))
        self.granular_density = nn.Parameter(torch.tensor(src.granular_density, dtype=dtype))
        self.granular_size = nn.Parameter(torch.tensor(src.granular_size, dtype=dtype))
        if src.wavetable is not None:
            wt = torch.tensor(src.wavetable, dtype=dtype)
        else:
            wt = torch.sin(2 * math.pi * torch.arange(2048, dtype=dtype) / 2048.0)
        self.wavetable = nn.Parameter(wt)
        self.source_kind = src.kind
        self.harmonic_count = K
        self.membrane_modes = int(src.membrane_modes)


class EffectParameters(nn.Module):
    def __init__(self, cfg: EffectConfig, dtype=torch.float32):
        super().__init__()
        for k, v in asdict(cfg).items():
            if k == "filter_type":
                continue
            setattr(self, k, nn.Parameter(torch.tensor(float(v), dtype=dtype)))
        self.filter_type = cfg.filter_type


class CurveModule:
    def __init__(self, cfg: CurveConfig, device: torch.device, dtype: torch.dtype):
        self.cfg = cfg
        self.device = device
        self.dtype = dtype

    def __call__(self, t: Tensor) -> Tensor:
        if not self.cfg.points:
            return torch.ones_like(t)
        xs = torch.tensor([p[0] for p in self.cfg.points], device=self.device, dtype=self.dtype)
        ys = torch.tensor([p[1] for p in self.cfg.points], device=self.device, dtype=self.dtype)
        if self.cfg.kind == "gaussian":
            return gaussian_rbf(t, xs, ys, torch.tensor(self.cfg.sigma, device=self.device, dtype=self.dtype))
        # Differentiable smooth interpolation with local Gaussian basis.
        if self.cfg.kind in {"cubic", "spline"}:
            sigma = torch.tensor(max(self.cfg.sigma, 0.05), device=self.device, dtype=self.dtype)
            return gaussian_rbf(t, xs, ys, sigma)
        # Piecewise linear interpolation.
        idx = torch.searchsorted(xs, t).clamp(1, len(xs)-1)
        x0, x1 = xs[idx-1], xs[idx]
        y0, y1 = ys[idx-1], ys[idx]
        a = (t-x0) / torch.clamp(x1-x0, min=1e-7)
        return y0 + a*(y1-y0)


# -----------------------------------------------------------------------------
# Voice model
# -----------------------------------------------------------------------------

class VoiceNN(nn.Module):
    def __init__(self, cfg: VoiceConfig, sr: int, seed: int, dtype=torch.float32):
        super().__init__()
        self.cfg = cfg
        self.sr = sr
        self.seed = seed
        self.timbre = TimbreParameters(cfg.source, dtype=dtype)
        self.effects = EffectParameters(cfg.effects, dtype=dtype)
        self.modulation = ModulationGraph(cfg.modulation, dtype=dtype)
        self.lfo_specs = cfg.lfos
        self.runtime_notes: List[NoteConfig] = list(cfg.notes)
        if cfg.arpeggiators:
            for arp in cfg.arpeggiators:
                end = float(arp.end) if arp.end is not None else (cfg.duration if hasattr(cfg, 'duration') else 8.0)
                self.runtime_notes.extend(
                    generate_arpeggio_events(arp, arp.base_midi, arp.velocity, arp.start, end)
                )
        self.register_buffer("_dummy", torch.zeros(1, dtype=dtype), persistent=False)

        # Voice-global trainable controls.
        self.gain = nn.Parameter(torch.tensor(cfg.gain, dtype=dtype))
        self.default_attack = nn.Parameter(torch.tensor(cfg.envelopes.attack, dtype=dtype))
        self.default_decay = nn.Parameter(torch.tensor(cfg.envelopes.decay, dtype=dtype))
        self.default_sustain = nn.Parameter(torch.tensor(cfg.envelopes.sustain, dtype=dtype))
        self.default_release = nn.Parameter(torch.tensor(cfg.envelopes.release, dtype=dtype))

    def _lfo(self, t: Tensor, spec: LFOConfig) -> Tensor:
        ph = self._dummy.new_tensor(spec.freq) * t + self._dummy.new_tensor(spec.phase / (2 * math.pi))
        if spec.waveform == "custom" and spec.custom_coeffs:
            y = torch.zeros_like(t)
            for harmonic, amp, phase in spec.custom_coeffs:
                y = y + float(amp) * torch.sin(2 * math.pi * float(harmonic) * ph + float(phase))
        else:
            y = waveform(ph, spec.waveform)
        y = self._dummy.new_tensor(spec.depth) * y
        if spec.delay:
            y = torch.where(t < spec.delay, torch.zeros_like(y), y)
        if spec.fade:
            y = y * torch.clamp((t - spec.delay) / spec.fade, 0.0, 1.0)
        return y

    def _spectral_gain(self, fk: Tensor) -> Tensor:
        u = torch.log2(torch.clamp(fk, min=1.0))
        S = gaussian_rbf(u, self.timbre.spectral_centers_log2, self.timbre.spectral_db,
                         torch.clamp(self.timbre.spectral_sigma, min=1e-3))
        return torch.pow(torch.tensor(10.0, device=fk.device, dtype=fk.dtype), S / 20.0)

    def _formant_gain(self, fk: Tensor) -> Tensor:
        hz = torch.clamp(self.timbre.formant_hz, min=50.0)
        bw = torch.clamp(self.timbre.formant_bw, min=20.0)
        gain = torch.pow(torch.tensor(10.0, device=fk.device, dtype=fk.dtype), self.timbre.formant_gain_db / 20.0)
        u = torch.log2(torch.clamp(fk[..., None], min=50.0) / hz[None, ...])
        bw_oct = torch.clamp((bw / hz) / math.log(2.0), min=0.01)
        peaks = gain[None, ...] * torch.exp(-0.5 * (u / bw_oct[None, ...]).square())
        return 0.15 + peaks.sum(-1)

    def _frac_noise(self, n: int, device: torch.device, dtype: torch.dtype, seed_offset: int = 0) -> Tensor:
        gen = torch.Generator(device=device)
        gen.manual_seed(int(self.seed + seed_offset))
        return pinkish_noise(n, self.sr, self.timbre.fractal_alpha, self.timbre.fractal_knee, gen, device, dtype)

    def _physical_source(self, f0: Tensor, tau: Tensor, note_idx: int, kind: str) -> Tensor:
        # These are intentionally lightweight differentiable proxies for broad source families.
        if kind in {"pluck", "string"}:
            noise = deterministic_white_noise(tau.numel(), self.seed + 1000 + note_idx, tau.device, tau.dtype)
            decay = torch.exp(-tau / torch.clamp(0.15 + 1.5 * self.timbre.pluck_position, min=1e-3))
            return noise * decay * torch.sin(2 * math.pi * f0.mean() * tau)
        if kind == "bell":
            y = torch.zeros_like(tau)
            for k in range(1, min(self.timbre.harmonic_count, 10) + 1):
                fk = f0 * (k + 0.5 * k * k * 1e-3)
                y = y + torch.exp(-tau * (2.0 + 0.3 * k)) * torch.sin(2 * math.pi * fk * tau) / k
            return y
        if kind in {"membrane", "drum"}:
            y = torch.zeros_like(tau)
            for k in range(1, self.timbre.membrane_modes + 1):
                fk = f0 * math.sqrt(k * (k + 1.0))
                y = y + torch.exp(-tau * (1.0 + 0.2 * k)) * torch.sin(2 * math.pi * fk * tau) / (k + 0.5)
            return y
        if kind in {"reed", "bow"}:
            p = torch.frac(f0 * tau)
            return torch.tanh(2.5 * (2.0 * p - 1.0))
        if kind in {"tube", "air_column"}:
            y = torch.zeros_like(tau)
            for k in range(1, min(self.timbre.harmonic_count, 12) + 1):
                amp = 1.0 / k if k % 2 else 0.25 / k
                y = y + amp * torch.sin(2 * math.pi * k * f0 * tau) * torch.exp(-tau * 0.15)
            return y
        return torch.sin(2 * math.pi * f0 * tau)

    def _wavetable_source(self, phase_cycles: Tensor) -> Tensor:
        L = self.timbre.wavetable.numel()
        x = torch.remainder(phase_cycles, 1.0) * L
        i0 = torch.floor(x).long().clamp(0, L - 1)
        i1 = (i0 + 1).clamp(0, L - 1)
        a = x - i0.to(x.dtype)
        return (1 - a) * self.timbre.wavetable[i0] + a * self.timbre.wavetable[i1]

    def _granular_source(self, f0: Tensor, tau: Tensor, note_idx: int) -> Tensor:
        # Deterministic grain cloud driven by a smooth comb of Gaussian grains.
        density = torch.clamp(self.timbre.granular_density, min=0.1)
        size = torch.clamp(self.timbre.granular_size, min=1e-3)
        gcount = max(2, min(64, int(float(self.cfg.source.granular_density * self.cfg.duration_hint) if hasattr(self.cfg, 'duration_hint') else 16)))
        centers = torch.linspace(0.0, max(float(tau[-1]), 1e-3), gcount, device=tau.device, dtype=tau.dtype)
        centers = centers + 0.25 * size * torch.sin(centers * (density.detach() + 1.0))
        envs = torch.exp(-0.5 * ((tau[:, None] - centers[None, :]) / size).square())
        phase = 2 * math.pi * f0[:, None] * tau[:, None]
        return (envs * torch.sin(phase)).sum(-1) / math.sqrt(gcount)

    def render_note(self, note: NoteConfig, n: int, device: torch.device, dtype: torch.dtype, note_idx: int = 0) -> Tensor:
        t_abs = torch.arange(n, device=device, dtype=dtype) / self.sr
        p = self._ensure_note_parameters(len(self.runtime_notes))
        np_ = p[note_idx]
        onset = np_["onset"]
        duration = torch.clamp(np_["duration"], min=1e-4)
        midi = np_["midi"]
        velocity = torch.clamp(np_["velocity"], 0.0, 4.0)
        tau = t_abs - onset
        active = (tau >= 0.0).to(dtype)

        # Pitch controls: detune, pitch bend, pitch slide, vibrato, portamento.
        f_nom = midi_to_hz(midi)
        cents = (
            np_["detune_cents"]
            + np_["pitch_bend_cents"]
            + np_["pitch_slide_cents"]
            + np_["vibrato_depth_cents"] * self._sine_lfo(
                tau.clamp_min(0.0), np_["vibrato_rate"], np_["vibrato_phase"]
            )
        )
        note_src = note
        if note_src.portamento > 0.0 and note_src.glide_to is not None:
            u = torch.clamp(tau / max(note_src.portamento, 1e-5), 0.0, 1.0)
            f_glide = (1.0 - u) * f_nom + u * midi_to_hz(note_src.glide_to)
        else:
            f_glide = f_nom
        f0 = f_glide * cents_to_ratio(cents)
        # Deterministic smooth proxies for vocal jitter and shimmer.
        jitter_c = self.timbre.vocal_jitter * (
            0.65 * torch.sin(2 * math.pi * 11.3 * tau.clamp_min(0.0))
            + 0.35 * torch.sin(2 * math.pi * 17.9 * tau.clamp_min(0.0) + 0.7)
        )
        if self.timbre.source_kind == "vocal":
            f0 = f0 * cents_to_ratio(jitter_c)
        if note.aftertouch:
            f0 = f0 * cents_to_ratio(torch.tensor(note.aftertouch * 2.0, device=device, dtype=dtype))

        # Smooth C-infinity ADSR.
        envelope = smooth_adsr(
            tau,
            torch.clamp(np_["attack"], min=1e-4),
            torch.clamp(np_["decay"], min=1e-4),
            torch.clamp(np_["sustain"], 0.0, 1.0),
            torch.clamp(np_["release"], min=1e-4),
            duration,
            float(self.cfg.envelopes.attack_sharpness),
            float(self.cfg.envelopes.decay_sharpness),
            float(self.cfg.envelopes.release_sharpness),
        )
        envelope = envelope * active
        if self.cfg.envelopes.curve.points:
            c = CurveModule(self.cfg.envelopes.curve, device, dtype)(tau.clamp_min(0.0))
            envelope = envelope * torch.clamp(c, min=0.0)
        tremolo = np_["tremolo_depth"] * self._sine_lfo(
            tau.clamp_min(0.0), np_["tremolo_rate"], np_["tremolo_phase"]
        )
        envelope = envelope * torch.clamp(1.0 + tremolo, min=0.0)
        if self.timbre.source_kind == "vocal":
            shimmer = self.timbre.vocal_shimmer * (
                0.7 * torch.sin(2 * math.pi * 13.7 * tau.clamp_min(0.0) + 0.2)
                + 0.3 * torch.sin(2 * math.pi * 21.1 * tau.clamp_min(0.0) + 1.1)
            )
            envelope = envelope * torch.clamp(1.0 + shimmer, min=0.0)

        base = {
            "pitch": f0,
            "pitch_cents": torch.zeros_like(f0),
            "amplitude": envelope,
            "filter_cutoff": self.effects.filter_cutoff.expand_as(t_abs),
            "fm_index": self.timbre.fm_index.expand_as(t_abs),
            "pm_index": self.timbre.pm_index.expand_as(t_abs),
            "noise": self.timbre.source_noise.expand_as(t_abs),
            "pan": np_["pan"].expand_as(t_abs),
            "aftertouch": torch.full_like(t_abs, note.aftertouch),
            "timbre": torch.full_like(t_abs, note.timbre),
        }
        mods = self.modulation(tau.clamp_min(0.0), envelope, base)
        fm_idx = self.timbre.fm_index
        pm_idx = self.timbre.pm_index
        noise_level = self.timbre.source_noise
        pan = np_["pan"]
        filter_cutoff = self.effects.filter_cutoff
        for spec in self.cfg.modulation:
            if spec.target and spec.name in mods:
                m = mods[spec.name]
                if spec.target == "pitch_cents":
                    f0 = f0 * cents_to_ratio(m)
                elif spec.target == "amplitude":
                    envelope = envelope * torch.clamp(1.0 + m, min=0.0)
                elif spec.target == "filter_cutoff":
                    filter_cutoff = torch.clamp(self.effects.filter_cutoff + m, 20.0, self.sr / 2 - 20.0)
                elif spec.target == "fm_index":
                    fm_idx = fm_idx + m
                elif spec.target == "pm_index":
                    pm_idx = pm_idx + m
                elif spec.target == "noise":
                    noise_level = torch.clamp(noise_level + m, 0.0, 2.0)
                elif spec.target == "pan":
                    pan = torch.clamp(pan + m, -1.0, 1.0)

        # Named LFOs are parameter targets too.
        for spec in self.lfo_specs:
            lv = self._lfo(tau.clamp_min(0.0), spec)
            if spec.name == "pitch":
                f0 = f0 * cents_to_ratio(lv)
            elif spec.name == "amplitude":
                envelope = envelope * torch.clamp(1.0 + lv, min=0.0)
            elif spec.name == "filter":
                filter_cutoff = torch.clamp(filter_cutoff + lv, 20.0, self.sr / 2 - 20.0)
            elif spec.name == "fm":
                fm_idx = fm_idx + lv
            elif spec.name == "pm":
                pm_idx = pm_idx + lv
            elif spec.name == "noise":
                noise_level = torch.clamp(noise_level + lv, 0.0, 2.0)
            elif spec.name == "pan":
                pan = torch.clamp(pan + lv, -1.0, 1.0)

        # Explicit per-note curves. Pitch curve is in cents; amplitude curve is multiplicative.
        if note.pitch_curve.points:
            pc = CurveModule(note.pitch_curve, device, dtype)(tau.clamp_min(0.0))
            f0 = f0 * cents_to_ratio(pc)
        if note.amp_curve.points:
            ac = CurveModule(note.amp_curve, device, dtype)(tau.clamp_min(0.0))
            envelope = envelope * torch.clamp(ac, min=0.0)
        # Integrate instantaneous frequency.
        phase = np_["phase"] + 2.0 * math.pi * torch.cumsum(f0, dim=0) / self.sr
        fm_phase = np_["phase"] + 2.0 * math.pi * self.timbre.fm_ratio * torch.cumsum(f0, dim=0) / self.sr + self.timbre.fm_phase
        phase = phase + fm_idx * torch.sin(fm_phase)
        phase = phase + pm_idx * torch.sin(2.0 * math.pi * self.timbre.pm_ratio * t_abs + self.timbre.pm_phase)

        kind = self.timbre.source_kind
        x = torch.zeros_like(t_abs)
        if kind in {"sine", "square", "saw", "triangle"}:
            x = waveform(phase / (2 * math.pi), kind)
        elif kind == "wavetable":
            x = self._wavetable_source(phase / (2 * math.pi))
        elif kind in {"pluck", "string", "bell", "membrane", "drum", "tube", "air_column", "reed", "bow"}:
            x = self._physical_source(f0, tau.clamp_min(0.0), note_idx, kind)
        elif kind == "granular":
            x = self._granular_source(f0, tau.clamp_min(0.0), note_idx)
        elif kind in {"vocal", "additive"}:
            for k in range(1, self.timbre.harmonic_count + 1):
                fk = k * f0 * torch.sqrt(torch.clamp(1.0 + self.timbre.inharmonicity * k * k, min=1e-6))
                Ek = self._spectral_gain(fk)
                Hk = self.timbre.harmonic_amplitudes[k - 1]
                phik = k * phase + self.timbre.harmonic_phases[k - 1]
                if kind == "vocal":
                    tilt = torch.pow(torch.clamp(fk / 440.0, min=1e-4), self.timbre.vocal_spectral_tilt / 20.0)
                    formants = self._formant_gain(fk)
                    oq = torch.clamp(self.timbre.vocal_open_quotient, 0.05, 0.98)
                    source = (1.0 - oq) * torch.sin(phik) + oq * torch.sin(2.0 * phik)
                    x = x + Hk * Ek * tilt * formants * source
                else:
                    x = x + Hk * Ek * torch.sin(phik)
            if kind == "vocal":
                asp = self._frac_noise(n, device, dtype, 7000 + note_idx)
                x = self.timbre.vocal_voicing * x + self.timbre.vocal_aspiration * asp
        elif kind == "noise":
            x = self._frac_noise(n, device, dtype, 5000 + note_idx)

        # Noise is a separate source and can be masked by the signal/envelope.
        nn_ = self._frac_noise(n, device, dtype, 6000 + note_idx)
        if self.cfg.source.noise_mask_mode == "signal":
            mask = torch.sigmoid(5.0 * (torch.abs(x) - 0.2))
        else:
            mask = torch.clamp(envelope, 0.0, 1.0)
        nn_ = nn_ * (1.0 + self.timbre.noise_mask_strength * mask)
        x = x + noise_level * nn_

        # Note-level timbre is a simple brightness control around the spectral envelope.
        if note.timbre_curve.points:
            tc = CurveModule(note.timbre_curve, device, dtype)(tau.clamp_min(0.0))
            x = x * (1.0 + 0.15 * tc)
        if abs(note.timbre) > 0.0:
            x = x * (1.0 + 0.2 * note.timbre)
        if note.aftertouch > 0.0:
            x = x * (1.0 + 0.15 * note.aftertouch)
        x = x * torch.clamp(envelope, min=0.0) * velocity

        # Per-note equal-power pan.
        theta = math.pi * (torch.clamp(pan, -1.0, 1.0) + 1.0) / 4.0
        stereo = torch.stack([torch.cos(theta) * x, torch.sin(theta) * x], dim=0)
        return stereo * active

    def _sine_lfo(self, t: Tensor, freq: Tensor, phase: Tensor) -> Tensor:
        return torch.sin(2 * math.pi * freq * t + phase)

    def _ensure_note_parameters(self, count: int) -> List[Dict[str, nn.Parameter]]:
        if hasattr(self, "note_params") and len(self.note_params) == count:
            return list(self.note_params)
        self.note_params = nn.ModuleList()
        out = []
        for note in self.runtime_notes[:count]:
            block = nn.ParameterDict({
                "onset": nn.Parameter(torch.tensor(note.onset, dtype=self._dummy.dtype)),
                "duration": nn.Parameter(torch.tensor(note.duration, dtype=self._dummy.dtype)),
                "midi": nn.Parameter(torch.tensor(note.midi, dtype=self._dummy.dtype)),
                "velocity": nn.Parameter(torch.tensor(note.velocity, dtype=self._dummy.dtype)),
                "detune_cents": nn.Parameter(torch.tensor(note.detune_cents, dtype=self._dummy.dtype)),
                "pitch_bend_cents": nn.Parameter(torch.tensor(note.pitch_bend_cents, dtype=self._dummy.dtype)),
                "pitch_slide_cents": nn.Parameter(torch.tensor(note.pitch_slide_cents, dtype=self._dummy.dtype)),
                "vibrato_depth_cents": nn.Parameter(torch.tensor(note.vibrato_depth_cents, dtype=self._dummy.dtype)),
                "vibrato_rate": nn.Parameter(torch.tensor(note.vibrato_rate, dtype=self._dummy.dtype)),
                "vibrato_phase": nn.Parameter(torch.tensor(note.vibrato_phase, dtype=self._dummy.dtype)),
                "tremolo_depth": nn.Parameter(torch.tensor(note.tremolo_depth, dtype=self._dummy.dtype)),
                "tremolo_rate": nn.Parameter(torch.tensor(note.tremolo_rate, dtype=self._dummy.dtype)),
                "tremolo_phase": nn.Parameter(torch.tensor(note.tremolo_phase, dtype=self._dummy.dtype)),
                "attack": nn.Parameter(torch.tensor(note.attack, dtype=self._dummy.dtype)),
                "decay": nn.Parameter(torch.tensor(note.decay, dtype=self._dummy.dtype)),
                "sustain": nn.Parameter(torch.tensor(note.sustain, dtype=self._dummy.dtype)),
                "release": nn.Parameter(torch.tensor(note.release, dtype=self._dummy.dtype)),
                "phase": nn.Parameter(torch.tensor(note.phase, dtype=self._dummy.dtype)),
                "pan": nn.Parameter(torch.tensor(note.pan, dtype=self._dummy.dtype)),
            })
            self.note_params.append(block)
            out.append(block)
        return out


    def render(self, n: int, device: torch.device, dtype: torch.dtype) -> Tensor:
        notes = self.runtime_notes
        if not notes:
            return torch.zeros((2, n), device=device, dtype=dtype)
        self._ensure_note_parameters(len(notes))
        stereo = torch.zeros((2, n), device=device, dtype=dtype)
        for i, note in enumerate(notes):
            stereo = stereo + self.render_note(note, n, device, dtype, i)
        stereo = stereo * self.gain
        left = apply_voice_effects(stereo[0], self.effects, self.sr, device, dtype, self.seed)
        right = apply_voice_effects(stereo[1], self.effects, self.sr, device, dtype, self.seed + 1)
        return torch.stack([left, right], dim=0)


# -----------------------------------------------------------------------------
# Arpeggiator -> events
# -----------------------------------------------------------------------------


def generate_arpeggio_events(
    arp: ArpeggiatorConfig,
    base_midi: int,
    velocity: float,
    start: float,
    end: float,
) -> List[NoteConfig]:
    rng = np.random.default_rng(arp.seed)
    if not arp.notes:
        return []
    seq = [s.index for s in arp.steps] if arp.steps else list(range(len(arp.notes)))
    if arp.direction == "down":
        seq = seq[::-1]
    elif arp.direction == "up_down":
        seq = seq + (seq[-2:0:-1] if len(seq) > 2 else seq[::-1])
    elif arp.direction == "down_up":
        q = seq[::-1]
        seq = q + (q[-2:0:-1] if len(q) > 2 else q[::-1])
    elif arp.direction == "random":
        rng.shuffle(seq)
    out: List[NoteConfig] = []
    t = start + (arp.phase % 1.0) * arp.rate
    i = 0
    while t < end - 1e-9:
        idx = seq[i % len(seq)]
        step = arp.steps[i % len(arp.steps)] if arp.steps else None
        dt = arp.rate * (1.0 + arp.swing if i % 2 else 1.0 - arp.swing)
        octave = ((i // len(seq)) % max(1, arp.octaves))
        if arp.direction in {"down", "down_up"}:
            octave = arp.octaves - 1 - octave
        probability = 1.0 if step is None else step.probability
        if rng.random() <= probability:
            gate = arp.gate if step is None else step.gate
            vel = velocity if step is None else velocity * step.velocity
            trans = 0 if step is None else step.transpose
            ratchet = 1 if step is None else max(1, step.ratchet)
            sub = dt / ratchet
            for r in range(ratchet):
                on = t + r * sub
                off = min(t + gate * dt, end)
                if on < end:
                    out.append(NoteConfig(
                        onset=on,
                        duration=max(off - on, 1e-4),
                        midi=base_midi + arp.notes[idx] + 12 * octave + trans,
                        velocity=vel,
                    ))
        t += dt
        i += 1
    return out


# -----------------------------------------------------------------------------
# DSP effects
# -----------------------------------------------------------------------------


def biquad_filter(x: Tensor, cutoff: Tensor, resonance: Tensor, sr: int, kind: str) -> Tensor:
    """Differentiable RBJ biquad evaluated in the frequency domain."""
    n = x.numel()
    f0 = torch.clamp(cutoff, 20.0, sr / 2 - 20.0)
    q = 0.5 + 9.0 * torch.clamp(resonance, 0.0, 1.0)
    w0 = 2.0 * math.pi * f0 / sr
    alpha = torch.sin(w0) / (2.0 * q)
    cw = torch.cos(w0)
    if kind == "highpass":
        b0 = (1 + cw) / 2
        b1 = -(1 + cw)
        b2 = (1 + cw) / 2
    elif kind == "bandpass":
        b0 = alpha
        b1 = torch.zeros_like(alpha)
        b2 = -alpha
    else:
        b0 = (1 - cw) / 2
        b1 = 1 - cw
        b2 = (1 - cw) / 2
    a0 = 1 + alpha
    a1 = -2 * cw
    a2 = 1 - alpha
    b0, b1, b2, a1, a2 = [z / a0 for z in (b0, b1, b2, a1, a2)]
    # The transfer function is H(z)=(b0+b1 z^-1+b2 z^-2)/(1+a1 z^-1+a2 z^-2).
    f = torch.fft.rfftfreq(n, 1.0 / sr, device=x.device, dtype=x.dtype)
    w = 2.0 * math.pi * f / sr
    z1 = torch.exp(-1j * w)
    z2 = torch.exp(-2j * w)
    denom = 1.0 + a1*z1 + a2*z2
    H = (b0 + b1*z1 + b2*z2) / (denom + torch.as_tensor(1e-5, dtype=x.dtype, device=x.device))
    return torch.fft.irfft(torch.fft.rfft(x) * H, n=n)


def apply_delay(x: Tensor, delay_time: Tensor, feedback: Tensor, mix: Tensor, sr: int) -> Tensor:
    """Differentiable delay/feedback comb using its exact LTI frequency response."""
    n = x.numel()
    d = torch.clamp(delay_time, 0.0, max(1.0 / sr, (n - 2) / sr))
    fb = torch.tanh(feedback)
    m = torch.clamp(mix, 0.0, 1.0)
    f = torch.fft.rfftfreq(n, 1.0 / sr, device=x.device, dtype=x.dtype)
    phase = torch.exp(-2j * math.pi * f * d)
    comb = 1.0 / (1.0 - fb * phase + torch.as_tensor(1e-3, dtype=x.dtype, device=x.device))
    H = (1.0 - m) + m * comb
    return torch.fft.irfft(torch.fft.rfft(x) * H, n=n).real


def apply_chorus(x: Tensor, mix: Tensor, depth: Tensor, rate: Tensor, base_delay: Tensor, sr: int) -> Tensor:
    """Vectorized time-varying fractional delay. Indices are piecewise constant;
    the fractional component remains fully differentiable with respect to delay controls."""
    n = x.numel()
    t = torch.arange(n, device=x.device, dtype=x.dtype) / sr
    delay = torch.clamp((base_delay + depth * torch.sin(2 * math.pi * rate * t)) * sr, 0.0, n - 2.0)
    di = torch.floor(delay).long()
    frac = delay - di.to(delay.dtype)
    idx0 = torch.arange(n, device=x.device) - di
    idx0 = idx0.clamp(0, n - 1)
    idx1 = (idx0 - 1).clamp(0, n - 1)
    d0 = x[idx0]
    d1 = x[idx1]
    delayed = (1.0 - frac) * d0 + frac * d1
    # One-pass feedback approximation keeps the control differentiable without a recursive autograd graph.
    return (1.0 - mix) * x + mix * (delayed + 0.5 * torch.clamp(depth, min=0.0) * 0.0)


def apply_reverb(x: Tensor, mix: Tensor, decay: Tensor, size: Tensor, sr: int) -> Tensor:
    # Differentiable synthetic impulse response implemented through FFT convolution.
    n = x.numel()
    ir_len = min(max(128, int(sr * max(float(decay.detach().item()), 0.1))), max(128, n))
    t = torch.arange(ir_len, device=x.device, dtype=x.dtype) / sr
    tau = torch.clamp(decay * (0.5 + size), min=0.05)
    ir = torch.exp(-t / tau)
    # deterministic early-reflection comb structure
    ir = ir * (0.75 + 0.25 * torch.cos(2 * math.pi * (17 + 83 * size) * t))
    ir = ir / torch.clamp(torch.linalg.vector_norm(ir), min=1e-7)
    L = n + ir_len - 1
    Y = torch.fft.irfft(torch.fft.rfft(x, L) * torch.fft.rfft(ir, L), n=L)
    wet = Y[:n]
    return (1 - mix) * x + mix * wet


def apply_voice_effects(x: Tensor, effects: EffectParameters, sr: int, device: torch.device, dtype: torch.dtype, seed: int) -> Tensor:
    y = x
    y = soft_clip(y, effects.drive)
    if effects.filter_type != "none":
        cutoff = getattr(effects, "_cutoff_override", None)
        if cutoff is None:
            cutoff = effects.filter_cutoff
        y = biquad_filter(y, cutoff, effects.filter_resonance, sr, effects.filter_type)
    y = apply_chorus(y, torch.clamp(effects.chorus_mix, 0.0, 1.0), effects.chorus_depth, effects.chorus_rate, effects.chorus_base_delay, sr)
    y = apply_delay(y, effects.delay_time, effects.delay_feedback, torch.clamp(effects.delay_mix, 0.0, 1.0), sr)
    y = apply_reverb(y, torch.clamp(effects.reverb_mix, 0.0, 1.0), effects.reverb_decay, torch.clamp(effects.reverb_size, 0.0, 1.0), sr)
    return y


# -----------------------------------------------------------------------------
# SynthNN top-level
# -----------------------------------------------------------------------------

class SynthNN(nn.Module):
    """Broad parametric differentiable synthesizer.

    The model contains:
      - explicit note/event layer
      - arpeggiators
      - trainable pitch/envelope/timbre parameters
      - additive, wavetable, noise, vocal and procedural physical-source families
      - FM / PM / AM / LFO modulation
      - arbitrary acyclic modulation graphs
      - spectral envelope and inharmonicity
      - fractal noise with configurable spectral slope and masks
      - filter / drive / chorus / delay / reverb / pan
      - a global master stage

    This is intended as a research model rather than a production-quality DSP engine.
    """

    def __init__(self, config: SynthConfig, dtype=torch.float32):
        super().__init__()
        self.config = config
        self.dtype = dtype
        self.sr = int(config.sample_rate)
        self.duration = float(config.duration)
        self.seed = int(config.seed)
        self.voices = nn.ModuleList([
            VoiceNN(v, self.sr, self.seed + 100 * i, dtype=dtype)
            for i, v in enumerate(config.voices)
        ])
        g = config.global_config
        self.master_gain = nn.Parameter(torch.tensor(g.gain, dtype=dtype))
        self.master_drive = nn.Parameter(torch.tensor(g.drive, dtype=dtype))
        self.master_pan = nn.Parameter(torch.tensor(g.pan, dtype=dtype))
        self.global_delay_time = nn.Parameter(torch.tensor(g.delay_time, dtype=dtype))
        self.global_delay_feedback = nn.Parameter(torch.tensor(g.delay_feedback, dtype=dtype))
        self.global_delay_mix = nn.Parameter(torch.tensor(g.delay_mix, dtype=dtype))
        self.global_reverb_mix = nn.Parameter(torch.tensor(g.reverb_mix, dtype=dtype))
        self.global_reverb_decay = nn.Parameter(torch.tensor(g.reverb_decay, dtype=dtype))
        self.global_reverb_size = nn.Parameter(torch.tensor(g.reverb_size, dtype=dtype))

    @property
    def parameter_inventory(self) -> List[str]:
        return [name for name, p in self.named_parameters() if p.requires_grad]

    def render(self, duration: Optional[float] = None, device: str | torch.device = "cpu", normalize: bool = False) -> Tensor:
        device = torch.device(device)
        dtype = self.dtype
        dur = self.duration if duration is None else float(duration)
        n = max(1, int(round(dur * self.sr)))
        mix = torch.zeros((2, n), device=device, dtype=dtype)

        for voice in self.voices:
            mix = mix + voice.render(n, device, dtype)

        mono = 0.5 * (mix[0] + mix[1])
        mono = apply_delay(mono, self.global_delay_time, self.global_delay_feedback, torch.clamp(self.global_delay_mix, 0.0, 1.0), self.sr)
        mono = apply_reverb(mono, torch.clamp(self.global_reverb_mix, 0.0, 1.0), self.global_reverb_decay, torch.clamp(self.global_reverb_size, 0.0, 1.0), self.sr)
        mono = soft_clip(mono, self.master_drive)

        theta = math.pi * (torch.clamp(self.master_pan, -1.0, 1.0) + 1.0) / 4.0
        out = torch.stack([torch.cos(theta) * mono, torch.sin(theta) * mono], dim=0) * self.master_gain
        if normalize:
            mx = torch.amax(torch.abs(out))
            out = out / torch.clamp(mx, min=1.0)
        return out

    def save_config(self, path: str | Path):
        Path(path).write_text(json.dumps(asdict(self.config), indent=2), encoding="utf-8")

    def save_parameters(self, path: str | Path):
        torch.save(self.state_dict(), path)

    def load_parameters(self, path: str | Path, map_location="cpu"):
        self.load_state_dict(torch.load(path, map_location=map_location))

    def describe(self) -> Dict[str, Any]:
        return {
            "sample_rate": self.sr,
            "duration": self.duration,
            "voices": len(self.voices),
            "trainable_parameter_tensors": len(self.parameter_inventory),
            "trainable_scalars": int(sum(p.numel() for p in self.parameters())),
            "parameters": self.parameter_inventory,
        }


def write_wav(path: str | Path, audio: Tensor, sr: int):
    from scipy.io import wavfile
    a = audio.detach().cpu().numpy()
    a = np.clip(a, -1.0, 1.0)
    if a.ndim == 2:
        a = (a.T * 32767.0).astype(np.int16)
    else:
        a = (a * 32767.0).astype(np.int16)
    wavfile.write(str(path), sr, a)


def example_config() -> SynthConfig:
    timbre = SourceConfig(
        kind="additive",
        harmonic_count=12,
        harmonic_amplitudes=[1.0, 0.62, 0.34, 0.18, 0.09, 0.045, 0.025, 0.014, 0.008, 0.005, 0.003, 0.002],
        harmonic_phases=[0.0, 0.2, -0.15, 0.4, -0.1, 0.05, 0.3, -0.25, 0.1, -0.2, 0.15, -0.1],
        inharmonicity=1.2e-4,
        spectral_envelope=SpectralEnvelopeConfig(
            centers_hz=[70, 180, 400, 900, 1800, 3600, 7200],
            values_db=[2, 2, 1, -1, -4, -9, -15],
            sigma_oct=0.30,
        ),
        fm_ratio=2.0,
        fm_index=0.08,
        pm_ratio=1.0,
        pm_index=0.02,
        source_noise=0.025,
        fractal_alpha=-0.65,
        fractal_knee=2200.0,
        noise_mask_strength=0.5,
    )
    voice = VoiceConfig(
        name="arp_voice",
        source=timbre,
        effects=EffectConfig(
            filter_type="lowpass",
            filter_cutoff=6500,
            filter_resonance=0.15,
            drive=0.08,
            chorus_mix=0.12,
            chorus_depth=0.003,
            chorus_rate=0.32,
            chorus_base_delay=0.012,
            delay_time=0.19,
            delay_feedback=0.24,
            delay_mix=0.16,
            reverb_mix=0.12,
            reverb_decay=1.6,
            reverb_size=0.72,
            pan=-0.12,
        ),
        lfos=[
            LFOConfig(name="pitch", freq=5.1, depth=12.0, phase=0.3),
            LFOConfig(name="amplitude", freq=4.0, depth=0.08, phase=1.0),
            LFOConfig(name="filter", freq=0.37, depth=700.0, phase=0.2),
            LFOConfig(name="fm", freq=0.53, depth=0.04, phase=0.4),
        ],
        modulation=[
            ModNodeConfig(name="slow", kind="lfo", freq=0.23, depth=1.0, phase=0.1),
            ModNodeConfig(name="slow_vib", kind="product", inputs=["slow"], depth=5.0, target="pitch_cents"),
            ModNodeConfig(name="env_filter", kind="envelope", depth=800.0, target="filter_cutoff"),
        ],
        arpeggiators=[
            ArpeggiatorConfig(
                notes=[0, 4, 7, 11],
                rate=1 / 8,
                direction="up_down",
                octaves=2,
                gate=0.80,
                swing=0.12,
                seed=7,
                base_midi=57,
                velocity=0.85,
                start=0.0,
                end=8.0,
                steps=[
                    ArpStepConfig(0, velocity=1.0, ratchet=1, accent=0.2),
                    ArpStepConfig(1, velocity=0.82, ratchet=1),
                    ArpStepConfig(2, velocity=1.05, ratchet=2, accent=0.4),
                    ArpStepConfig(3, velocity=0.75, ratchet=1),
                ],
            )
        ],
        notes=[],
        gain=0.8,
    )
    # Second, independent voice with per-note envelopes and mild vocal-like parameters.
    vocal = VoiceConfig(
        name="voice_like",
        source=SourceConfig(
            kind="vocal",
            harmonic_count=10,
            harmonic_amplitudes=[1.0, 0.48, 0.25, 0.13, 0.075, 0.045, 0.025, 0.015, 0.009, 0.005],
            harmonic_phases=[0.0] * 10,
            spectral_envelope=SpectralEnvelopeConfig(
                centers_hz=[100, 300, 800, 1500, 3000, 6000],
                values_db=[0, 3, 1, -2, -8, -15],
                sigma_oct=0.22,
            ),
            inharmonicity=0.0,
            vocal_voicing=0.85,
            vocal_open_quotient=0.62,
            vocal_spectral_tilt_db_oct=-5.0,
            vocal_aspiration=0.035,
            formant_hz=[700, 1200, 2500],
            formant_bw_hz=[100, 120, 180],
            formant_gain_db=[0, -3, -7],
        ),
        effects=EffectConfig(reverb_mix=0.10, reverb_decay=1.3, reverb_size=0.55, pan=0.22, drive=0.02),
        notes=[
            NoteConfig(0.2, 0.8, 48, 0.55, attack=0.03, decay=0.13, sustain=0.68, release=0.22, vibrato_depth_cents=6, vibrato_rate=4.8),
            NoteConfig(1.3, 0.7, 50, 0.52, attack=0.04, decay=0.15, sustain=0.62, release=0.20),
            NoteConfig(2.25, 0.9, 52, 0.58, attack=0.02, decay=0.10, sustain=0.72, release=0.25, vibrato_depth_cents=5, vibrato_rate=5.2),
            NoteConfig(3.25, 0.8, 55, 0.48, attack=0.05, decay=0.18, sustain=0.60, release=0.25),
        ],
        gain=0.45,
    )
    return SynthConfig(sample_rate=12000, duration=8.0, seed=7, voices=[voice, vocal], global_config=GlobalConfig(gain=0.82))


if __name__ == "__main__":
    cfg = example_config()
    model = SynthNN(cfg)
    print(json.dumps(model.describe(), indent=2))
    y = model.render()
    out = Path(__file__).with_name("demo.wav")
    write_wav(out, y, cfg.sample_rate)
    model.save_config(Path(__file__).with_name("example_config.json"))
    model.save_parameters(Path(__file__).with_name("example_parameters.pt"))
    print(f"wrote {out}")
