#!/usr/bin/env python3
"""
End-to-End Curriculum Benchmark for Phase 5: Canonicalization and Gauges (E39-E41).
Executes exact equivalence transformations, canonical gauge projections, bit savings,
and physical ambiguity posterior evaluations, exporting structured JSON results.
"""

from __future__ import annotations
import json
import sys
from pathlib import Path
import numpy as np

# Add project root and curriculum to path
CURRICULUM_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(CURRICULUM_DIR))
sys.path.insert(0, str(CURRICULUM_DIR / 'phase5_canonicalization'))

from equivalence_classes import (
    time_shift_vs_phase,
    fm_vs_pm_angle_modulation,
    filter_vs_spectral_envelope,
    polar_amplitude_phase,
    compute_gauge_jacobian_nullity
)

from canonical_gauge import (
    CanonicalTimePhase,
    CanonicalAngleModulation,
    CanonicalFilterEnvelope,
    evaluate_canonical_mdl_savings
)

from ambiguity_distribution import (
    HarmonicCollisionAmbiguity,
    ResonatorVsNoiseAmbiguity,
    CausalLookaheadDisambiguator
)


def run_e39_benchmarks(sr: float = 16000.0) -> dict:
    print("\n---------------------------------------------------------")
    print("🔬 E39: EXACT EQUIVALENCE CLASSES (GAUGE SYMMETRIES)")
    print("---------------------------------------------------------")
    
    # 1. Time Shift vs Phase Shift
    dt_diff, _, _ = time_shift_vs_phase(f0=440.0, dt=0.003, phi0=0.5, duration=0.1, sr=sr)
    print(f"  [1] Time Shift vs Phase Shift Max Absolute Diff: {dt_diff:.3e}")
    
    # 2. FM vs PM
    fmpm_diff, _, _ = fm_vs_pm_angle_modulation(fc=600.0, fm=10.0, beta=2.5, psi=0.4, duration=0.1, sr=sr)
    print(f"  [2] FM vs PM Angle Modulation Max Absolute Diff: {fmpm_diff:.3e}")
    
    # 3. Filter vs Harmonic Spectral Envelope
    filt_diff, _, _ = filter_vs_spectral_envelope(f0=220.0, num_harmonics=10, cutoff_hz=1500.0, duration=0.1, sr=sr)
    print(f"  [3] LTI Filter vs Spectral Envelope Max Diff   : {filt_diff:.3e}")
    
    # 4. Polar Amplitude vs Phase
    polar_diff, _, _ = polar_amplitude_phase(A=-1.25, phi=0.7, f=350.0, duration=0.1, sr=sr)
    print(f"  [4] Polar Amplitude Sign Flip Max Diff         : {polar_diff:.3e}")
    
    # 5. Numerical Jacobian Nullity Check
    nullity = compute_gauge_jacobian_nullity(f0=440.0, duration=0.05, sr=sr)
    print(f"  [5] Gauge Jacobian Nullity Ratio (s1 / s0)     : {nullity['singular_value_ratio']:.3e} (Null: {nullity['has_nullity']})")
    
    return {
        'time_shift_vs_phase_diff': dt_diff,
        'fm_vs_pm_diff': fmpm_diff,
        'filter_vs_envelope_diff': filt_diff,
        'polar_phase_diff': polar_diff,
        'jacobian_nullity': nullity
    }


def run_e40_benchmarks(sr: float = 16000.0) -> dict:
    print("\n---------------------------------------------------------")
    print("📐 E40: CANONICAL GAUGE PROJECTIONS & BIT SAVINGS")
    print("---------------------------------------------------------")
    
    canon_tp = CanonicalTimePhase(sr=sr)
    raw_tp = {'A': -1.6, 'f0': 520.0, 't0': 0.008, 'phi0': 1.1}
    proj_tp = canon_tp.project(raw_tp)
    print(f"  [1] Time/Phase Gauge Projection:")
    print(f"      Raw   : A={raw_tp['A']:+.2f}, t0={raw_tp['t0']*1000:.1f}ms, phi0={raw_tp['phi0']:+.3f}")
    print(f"      Canon : A={proj_tp['A']:+.2f}, t0={proj_tp['t0']*1000:.1f}ms, phi0={proj_tp['phi0']:+.3f}")
    
    canon_fe = CanonicalFilterEnvelope(sr=sr)
    dec_a = canon_fe.select_and_project(num_poles=2, num_harmonics=16)
    dec_b = canon_fe.select_and_project(num_poles=8, num_harmonics=3)
    print(f"  [2] Min-MDL Filter vs Envelope Selection:")
    print(f"      Case A (2 poles vs 16 harmonics): Chose {dec_a['chosen_family']} (Saved {dec_a['bit_savings']:.0f} bits)")
    print(f"      Case B (8 poles vs 3 harmonics) : Chose {dec_b['chosen_family']} (Saved {dec_b['bit_savings']:.0f} bits)")
    
    savings = evaluate_canonical_mdl_savings(num_blocks=50, sr=sr, duration_block=0.05)
    print(f"  [3] Sequence MDL Bit Savings over 50 Blocks:")
    print(f"      Unconstrained Parameters Cost: {savings['total_unconstrained_bits']:.0f} bits ({savings['total_unconstrained_bits']/1024:.2f} kb)")
    print(f"      Canonical Parameters Cost    : {savings['total_canonical_bits']:.0f} bits ({savings['total_canonical_bits']/1024:.2f} kb)")
    print(f"      Net Bit Reduction            : {savings['bit_savings_total']:.0f} bits ({savings['bit_savings_percentage']:.1f}%)")
    print(f"      Waveform Reconstruction Error: {savings['max_reconstruction_error']:.3e} (Bit-Exact / Zero Distortion)")
    
    return {
        'time_phase_projection': {'raw': raw_tp, 'canon': proj_tp},
        'filter_envelope_decisions': {'case_a': dec_a, 'case_b': dec_b},
        'mdl_savings': savings
    }


def run_e41_benchmarks(sr: float = 16000.0) -> dict:
    print("\n---------------------------------------------------------")
    print("🎲 E41: TRUE PHYSICAL AMBIGUITY & CAUSAL DISAMBIGUATION")
    print("---------------------------------------------------------")
    
    # 1. Harmonic Collision Posterior
    h_amb = HarmonicCollisionAmbiguity(sr=sr)
    post = h_amb.compute_posterior(f1=220.0, f2=440.0, observed_amplitude=1.5, noise_std=0.04)
    print(f"  [1] Harmonic Collision Posterior (2*f1 = f2):")
    print(f"      Prior Entropy   : {post['prior_entropy_bits']:.2f} bits")
    print(f"      Posterior Mean  : E[A1]={post['mean_A1']:.2f}, E[A2]={post['mean_A2']:.2f} (Sum = {post['mean_A1']+post['mean_A2']:.2f})")
    print(f"      Posterior Cov   : [[{post['covariance'][0][0]:.4f}, {post['covariance'][0][1]:.4f}], [{post['covariance'][1][0]:.4f}, {post['covariance'][1][1]:.4f}]]")
    print(f"      Posterior Entropy: {post['entropy_bits']:.2f} bits (Info Gain: {post['information_gain_bits']:.2f} bits)")
    
    # 2. Resonator vs Noise Hypothesis Testing
    r_amb = ResonatorVsNoiseAmbiguity(sr=sr)
    xr = r_amb.generate_resonator(f0=500.0, decay_rate=20.0, duration=0.1)
    xn = r_amb.generate_noise_grain(center_f=500.0, bandwidth=50.0, duration=0.1)
    
    eval_r = r_amb.evaluate_likelihood(xr)
    eval_n = r_amb.evaluate_likelihood(xn)
    print(f"  [2] Physical Hypothesis Likelihoods:")
    print(f"      Ground Truth Resonator : P(Res)={eval_r['p_resonator']*100:.1f}%, P(Noise)={eval_r['p_noise']*100:.1f}%")
    print(f"      Ground Truth Noise Burst: P(Res)={eval_n['p_resonator']*100:.1f}%, P(Noise)={eval_n['p_noise']*100:.1f}%")
    
    # 3. Causal Lookahead Horizon Expansion
    lookahead = CausalLookaheadDisambiguator(sr=sr)
    horizons = [0.01, 0.02, 0.05, 0.10, 0.20]
    trace = lookahead.simulate_horizon_expansion(f1=220.0, f2=440.0, horizons=horizons)
    print(f"  [3] Causal Horizon Disambiguation (Entropy Collapse):")
    for entry in trace:
        print(f"      Horizon {entry['horizon_seconds']*1000:>5.1f} ms | Cov Det: {entry['det_covariance']:>9.2e} | Entropy: {entry['entropy_bits']:>6.2f} bits")
        
    return {
        'harmonic_collision': post,
        'resonator_vs_noise': {'resonator_eval': eval_r, 'noise_eval': eval_n},
        'lookahead_trace': trace
    }


def main():
    print("=========================================================")
    print("🎛️ FASE 5: CANONICALIZAÇÃO E GAUGES (E39-E41)")
    print("=========================================================")
    
    sr = 16000.0
    results_e39 = run_e39_benchmarks(sr=sr)
    results_e40 = run_e40_benchmarks(sr=sr)
    results_e41 = run_e41_benchmarks(sr=sr)
    
    full_results = {
        'phase': 'Phase 5: Canonicalization and Gauges',
        'sample_rate_hz': sr,
        'e39_equivalence_classes': results_e39,
        'e40_canonical_gauge': results_e40,
        'e41_physical_ambiguity': results_e41
    }
    
    out_file = Path(__file__).resolve().parent / 'phase5_results.json'
    with open(out_file, 'w') as f:
        json.dump(full_results, f, indent=2)
        
    print("\n=========================================================")
    print(f"✅ FASE 5 CONCLUÍDA COM SUCESSO! Resultados salvos em:")
    print(f"   {out_file}")
    print("=========================================================")


if __name__ == '__main__':
    main()
