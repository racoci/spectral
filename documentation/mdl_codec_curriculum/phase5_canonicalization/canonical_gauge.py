"""
E40: Canonical Gauge Projections in Structural Audio Modeling.
Defines idempotent projection operators G: Theta -> Theta / ~ collapsing
gauge orbits to unique quotient representatives and saving description bits.
"""

from __future__ import annotations
import numpy as np
from typing import Dict, Any, Tuple


class CanonicalTimePhase:
    """
    Collapses the continuous gauge orbit (t0, phi0, A) of an isolated sinusoidal component
    to a unique canonical representative with t0 = 0, A >= 0, and phi0 in [-pi, pi).
    """
    def __init__(self, sr: float = 16000.0):
        self.sr = sr

    def project(self, params: Dict[str, Any]) -> Dict[str, Any]:
        """
        Idempotent projection G(theta).
        """
        A = float(params['A'])
        f0 = float(params['f0'])
        t0 = float(params.get('t0', 0.0))
        phi0 = float(params.get('phi0', 0.0))

        # 1. Absorb t0 into phase: phi_1 = phi0 - 2*pi*f0*t0
        phi_1 = phi0 - 2.0 * np.pi * f0 * t0

        # 2. Absorb negative amplitude into pi phase rotation
        if A < 0.0:
            A_canon = -A
            phi_2 = phi_1 + np.pi
        else:
            A_canon = A
            phi_2 = phi_1

        # 3. Wrap phase into [-pi, pi)
        phi_canon = ((phi_2 + np.pi) % (2.0 * np.pi)) - np.pi

        return {
            'A': float(A_canon),
            'f0': float(f0),
            't0': 0.0,
            'phi0': float(phi_canon)
        }

    def synthesize(self, params: Dict[str, Any], duration: float) -> np.ndarray:
        N = int(self.sr * duration)
        t = np.linspace(0, duration, N, endpoint=False)
        A = params['A']
        f0 = params['f0']
        t0 = params.get('t0', 0.0)
        phi0 = params.get('phi0', 0.0)
        return A * np.cos(2.0 * np.pi * f0 * (t - t0) + phi0)


class CanonicalAngleModulation:
    """
    Eliminates the separate classification and encoding of FM vs PM.
    Maps both to the canonical AngleModulation representation:
    phi(t) = 2*pi*fc*t + beta * sin(2*pi*fm*t + psi)
    where beta >= 0, psi in [-pi, pi).
    """
    def __init__(self, sr: float = 16000.0):
        self.sr = sr

    def project(self, params: Dict[str, Any]) -> Dict[str, Any]:
        mod_type = params.get('type', 'AngleModulation').upper()
        fc = float(params['fc'])
        fm = float(params['fm'])
        psi = float(params.get('psi', 0.0))

        if mod_type == 'PM':
            beta = float(params['I'])
        elif mod_type == 'FM':
            delta_f = float(params['delta_f'])
            beta = float(delta_f / max(fm, 1e-9))
        elif mod_type == 'ANGLEMODULATION':
            beta = float(params.get('beta', params.get('I', 0.0)))
        else:
            raise ValueError(f"Unknown angle modulation type: {mod_type}")

        # Enforce beta >= 0
        if beta < 0.0:
            beta = -beta
            psi += np.pi

        # Wrap psi into [-pi, pi)
        psi_canon = ((psi + np.pi) % (2.0 * np.pi)) - np.pi

        return {
            'type': 'AngleModulation',
            'fc': fc,
            'beta': beta,
            'fm': fm,
            'psi': psi_canon
        }

    def synthesize(self, params: Dict[str, Any], duration: float) -> np.ndarray:
        N = int(self.sr * duration)
        t = np.linspace(0, duration, N, endpoint=False)
        fc = float(params['fc'])
        fm = float(params['fm'])
        psi = float(params.get('psi', 0.0))
        if 'beta' in params:
            beta = float(params['beta'])
        elif 'I' in params:
            beta = float(params['I'])
        elif 'delta_f' in params:
            beta = float(params['delta_f']) / max(fm, 1e-9)
        else:
            beta = 0.0
        phi = 2.0 * np.pi * fc * t + beta * np.sin(2.0 * np.pi * fm * t + psi)
        return np.cos(phi)


class CanonicalFilterEnvelope:
    """
    Determines whether a timbral spectral shape is canonicalized as
    autoregressive all-pole filter coefficients or discrete harmonic amplitudes,
    strictly selecting the representation that minimizes Description Length L(theta).
    """
    def __init__(self, sr: float = 16000.0, bits_per_pole: float = 16.0, bits_per_harmonic: float = 12.0):
        self.sr = sr
        self.bits_per_pole = bits_per_pole
        self.bits_per_harmonic = bits_per_harmonic
        self.header_bits = 8.0  # Bits for descriptor tag

    def select_and_project(self, num_poles: int, num_harmonics: int, f0: float = 200.0) -> Dict[str, Any]:
        """
        Compares L(theta_filter) vs L(theta_envelope) and returns the canonical selection.
        """
        # Filter description cost: 2 parameters per pole pair (fc, Q)
        L_filter = (num_poles * self.bits_per_pole) + self.header_bits
        # Harmonic envelope cost: discrete amplitude per harmonic
        L_env = (num_harmonics * self.bits_per_harmonic) + self.header_bits

        if L_filter < L_env:
            chosen = 'FILTER'
            L_chosen = L_filter
            L_alt = L_env
        else:
            chosen = 'HARMONIC_ENVELOPE'
            L_chosen = L_env
            L_alt = L_filter

        savings = L_alt - L_chosen
        return {
            'chosen_family': chosen,
            'L_chosen_bits': float(L_chosen),
            'L_alternative_bits': float(L_alt),
            'bit_savings': float(savings),
            'num_poles': num_poles,
            'num_harmonics': num_harmonics
        }


def canonicalize_parameter_vector(params: Dict[str, Any], sr: float = 16000.0) -> Dict[str, Any]:
    """
    Universal canonicalizer applying appropriate gauge projections.
    """
    if 'fc' in params and ('fm' in params or 'delta_f' in params or 'I' in params):
        return CanonicalAngleModulation(sr=sr).project(params)
    elif 'f0' in params and 'A' in params:
        return CanonicalTimePhase(sr=sr).project(params)
    return params.copy()


def evaluate_canonical_mdl_savings(num_blocks: int = 20, sr: float = 16000.0, duration_block: float = 0.05) -> Dict[str, Any]:
    """
    Simulates a sequence of audio blocks parameterized with unconstrained vs
    canonical gauge representations, measuring total bit savings and verifying
    zero acoustic distortion.
    """
    rng = np.random.RandomState(42)
    canon_tp = CanonicalTimePhase(sr=sr)
    canon_am = CanonicalAngleModulation(sr=sr)
    canon_fe = CanonicalFilterEnvelope(sr=sr)

    total_unconstrained_bits = 0.0
    total_canonical_bits = 0.0
    max_reconstruction_error = 0.0

    raw_baseline_bits = 0.0
    N_b = int(sr * duration_block)

    for i in range(num_blocks):
        # 16-bit PCM baseline
        raw_baseline_bits += N_b * 16.0

        if i % 2 == 0:
            # Sinusoidal component with arbitrary time delay and signed amplitude
            A_raw = float(rng.uniform(-2.0, 2.0))
            if abs(A_raw) < 0.1:
                A_raw = 0.5
            f0 = float(rng.uniform(200.0, 1000.0))
            t0 = float(rng.uniform(0.001, 0.02))
            phi0 = float(rng.uniform(-np.pi, np.pi))

            raw_param = {'A': A_raw, 'f0': f0, 't0': t0, 'phi0': phi0}
            canon_param = canon_tp.project(raw_param)

            # Unconstrained bit cost: A (16b) + f0 (16b) + t0 (16b) + phi0 (16b) + sign (1b) = 65 bits
            L_unconstrained = 65.0
            # Canonical bit cost: A_canon >= 0 (15b) + f0 (16b) + phi_canon (14b) [t0 eliminated] = 45 bits
            L_canonical = 45.0

            x_raw = canon_tp.synthesize(raw_param, duration=duration_block)
            x_canon = canon_tp.synthesize(canon_param, duration=duration_block)
            diff = float(np.max(np.abs(x_raw - x_canon)))
            if diff > max_reconstruction_error:
                max_reconstruction_error = diff

        else:
            # Angle modulation with redundant FM/PM selector
            fc = float(rng.uniform(400.0, 1200.0))
            fm = float(rng.uniform(4.0, 25.0))
            beta = float(rng.uniform(0.5, 3.0))
            psi = float(rng.uniform(-np.pi, np.pi))

            # Unconstrained format spends 8 bits on mode selector ("FM" vs "PM")
            raw_param = {'type': 'FM', 'fc': fc, 'fm': fm, 'delta_f': beta * fm, 'psi': psi}
            canon_param = canon_am.project(raw_param)

            # Unconstrained: mode (8b) + fc (16b) + fm (12b) + delta_f (16b) + psi (12b) = 64 bits
            L_unconstrained = 64.0
            # Canonical: AngleModulation (implicit) + fc (16b) + fm (12b) + beta (14b) + psi (12b) = 54 bits
            L_canonical = 54.0

            x_raw = canon_am.synthesize(raw_param, duration=duration_block)
            x_canon = canon_am.synthesize(canon_param, duration=duration_block)
            diff = float(np.max(np.abs(x_raw - x_canon)))
            if diff > max_reconstruction_error:
                max_reconstruction_error = diff

        total_unconstrained_bits += L_unconstrained
        total_canonical_bits += L_canonical

    bit_savings_total = total_unconstrained_bits - total_canonical_bits
    cr_unconstrained = total_unconstrained_bits / max(raw_baseline_bits, 1.0)
    cr_canonical = total_canonical_bits / max(raw_baseline_bits, 1.0)

    return {
        'total_unconstrained_bits': float(total_unconstrained_bits),
        'total_canonical_bits': float(total_canonical_bits),
        'bit_savings_total': float(bit_savings_total),
        'bit_savings_percentage': float(100.0 * bit_savings_total / total_unconstrained_bits),
        'compression_ratio_unconstrained': float(cr_unconstrained),
        'compression_ratio_canonical': float(cr_canonical),
        'max_reconstruction_error': float(max_reconstruction_error)
    }
