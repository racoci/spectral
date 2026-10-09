#!/usr/bin/env python3
"""
End-to-End Curriculum Benchmark Runner for Phase 6: Evolution and Uncertainty (E42-E45).
Executes SDE anchor distance variance propagation, cubic phase uncertainty analysis,
rigorous statistical calibration (exact 95% +/- 1.96 sigma envelope), and GP vs Taylor Jets comparison.
Exports structured benchmark metrics to phase6_results.json.
"""

from __future__ import annotations
import json
import sys
from pathlib import Path
import numpy as np

# Add project root and phase directory to sys.path
PHASE_DIR = Path(__file__).resolve().parent
CURRICULUM_DIR = PHASE_DIR.parent
sys.path.insert(0, str(CURRICULUM_DIR))
sys.path.insert(0, str(PHASE_DIR))

from gp_anchor_evolution import evaluate_anchor_evolution_benchmarks
from phase_uncertainty import evaluate_phase_uncertainty_benchmarks
from probabilistic_calibration import evaluate_calibration_benchmarks
from gp_vs_jet_benchmark import evaluate_gp_vs_jet_benchmarks


def run_e42_benchmarks() -> dict:
    print("\n---------------------------------------------------------")
    print("📈 E42: GP / SDE DISTANCE FROM ANCHOR (VARIANCE PROPAGATION)")
    print("---------------------------------------------------------")
    res = evaluate_anchor_evolution_benchmarks()
    
    print("  [1] Amplitude Ornstein-Uhlenbeck (OU) Evolution:")
    print("      dt (ms) | Ana Mean | Ana Std  | Emp Std  | Rel Var Error")
    print("      --------+----------+----------+----------+--------------")
    for r in res['amplitude_ou']:
        dt_ms = r['delta_t'] * 1000.0
        emp_std = np.sqrt(r['empirical_var'])
        print(f"      {dt_ms:7.1f} | {r['analytical_mu']:8.4f} | {r['analytical_std']:8.4f} | {emp_std:8.4f} | {r['relative_error']*100:6.2f}%")
        
    print("\n  [2] Frequency Kinematic SDE Evolution [f, dot_f]:")
    print("      dt (ms) | Mean f (Hz) | Std f (Hz) | Std dot_f (Hz/s)")
    print("      --------+-------------+------------+-----------------")
    for r in res['frequency_kinematic']:
        dt_ms = r['delta_t'] * 1000.0
        std_fdot = np.sqrt(r['var_fdot'])
        print(f"      {dt_ms:7.1f} | {r['f_mean']:11.2f} | {r['std_f']:10.4f} | {std_fdot:17.4f}")
        
    return res


def run_e43_benchmarks() -> dict:
    print("\n---------------------------------------------------------")
    print("🌊 E43: PHASE UNCERTAINTY & CUBIC VARIANCE EXPLOSION")
    print("---------------------------------------------------------")
    res = evaluate_phase_uncertainty_benchmarks()
    
    print("  [1] Continuous Phase Variance & Coherent Signal Attenuation:")
    print("      dt (ms) | Phase Std (rad) | Phase Std (deg) | Coherence E[cos(phi)]")
    print("      --------+-----------------+-----------------+----------------------")
    for r in res['scaling_table']:
        dt_ms = r['delta_t'] * 1000.0
        print(f"      {dt_ms:7.1f} | {r['phase_std_rad']:15.4f} | {r['phase_std_deg']:15.2f} | {r['coherence_attenuation']:22.4f}")
        
    print(f"\n  [2] Power-Law Log-Log Scaling Exponent: p = {res['power_law_exponent']:.4f} (Theoretical: 3.0000)")
    mc = res['mc_verification']
    print(f"  [3] Monte Carlo vs Analytical Variance at {mc['delta_t']*1000:.0f}ms:")
    print(f"      Analytical: {mc['analytical_var']:.5f} rad^2, Empirical: {mc['empirical_var']:.5f} rad^2 (Error: {mc['relative_error']*100:.2f}%)")
    
    dh = res['decoherence_horizons']
    print(f"  [4] Phase Decoherence Horizons:")
    print(f"      tau(sigma_phi = 1.0 rad) : {dh['tau_1_radian_ms']:.2f} ms")
    print(f"      tau(sigma_phi = pi rad)  : {dh['tau_pi_ms']:.2f} ms")
    
    return res


def run_e44_benchmarks() -> dict:
    print("\n---------------------------------------------------------")
    print("🎯 E44: PROBABILISTIC CALIBRATION & STATISTICAL HONESTY")
    print("---------------------------------------------------------")
    res = evaluate_calibration_benchmarks()
    
    amp = res['amplitude_ou_calibration']
    phase = res['phase_stochastic_calibration']
    
    print("  [1] Multi-Quantile Empirical Coverage vs Theoretical Normal:")
    print("      Confidence Interval | Theoretical | Amplitude OU | Phase Stochastic")
    print("      --------------------+-------------+--------------+-----------------")
    print(f"      +/- 0.674 sigma     |      50.00% |      {amp['coverage_50_percent']*100:6.2f}% |          {phase['coverage_50_percent']*100:6.2f}%")
    print(f"      +/- 1.000 sigma     |      68.27% |      {amp['coverage_68_sigma']*100:6.2f}% |          {phase['coverage_68_sigma']*100:6.2f}%")
    print(f"      +/- 1.645 sigma     |      90.00% |      {amp['coverage_90_percent']*100:6.2f}% |          {phase['coverage_90_percent']*100:6.2f}%")
    print(f"      +/- 1.960 sigma [GATE] 95.00% |      {amp['coverage_1_96_sigma']*100:6.2f}% |          {phase['coverage_1_96_sigma']*100:6.2f}%")
    print(f"      +/- 2.576 sigma     |      99.00% |      {amp['coverage_99_percent']*100:6.2f}% |          {phase['coverage_99_percent']*100:6.2f}%")
    
    print(f"\n  [2] Innovation Standard Normality & Uniform PIT:")
    print(f"      Amplitude: Z_mean={amp['z_mean']:+.4f}, Z_std={amp['z_std']:.4f}, KS p-val={amp['pit_ks_pvalue']:.4f} ({amp['diagnosis']})")
    print(f"      Phase    : Z_mean={phase['z_mean']:+.4f}, Z_std={phase['z_std']:.4f}, KS p-val={phase['pit_ks_pvalue']:.4f} ({phase['diagnosis']})")
    
    ctrl = res['control_experiments']
    print(f"\n  [3] Control Experiments (Detection of Miscalibration):")
    print(f"      Overconfident Model  (+/-1.96 cov): {ctrl['overconfident']['coverage_1_96_sigma']*100:6.2f}% -> {ctrl['overconfident']['diagnosis']}")
    print(f"      Underconfident Model (+/-1.96 cov): {ctrl['underconfident']['coverage_1_96_sigma']*100:6.2f}% -> {ctrl['underconfident']['diagnosis']}")
    
    return res


def run_e45_benchmarks() -> dict:
    print("\n---------------------------------------------------------")
    print("⚔️  E45: GP VERSUS HIGH-ORDER TAYLOR JETS HORIZON BENCHMARK")
    print("---------------------------------------------------------")
    res = evaluate_gp_vs_jet_benchmarks()
    
    print("  [1] Frequency Prediction Error Comparison across Horizons:")
    print("      dt (ms) | GT (Hz) | Err J0 (Hz) | Err J1 (Hz) | Err J2 (Hz) | Err GP (Hz) | GP Std (Hz)")
    print("      --------+---------+-------------+-------------+-------------+-------------+------------")
    for r in res['horizon_table']:
        print(f"      {r['delta_t_ms']:7.1f} | {r['ground_truth_hz']:7.2f} | {r['err_j0_hz']:11.2f} | {r['err_j1_hz']:11.2f} | {r['err_j2_hz']:11.2f} | {r['err_gp_hz']:11.2f} | {r['gp_predicted_std_hz']:10.2f}")
        
    print("\n  [2] Total Predictive Description Length (MDL Cost over 300ms window):")
    mdl = res['mdl_costs_bits']
    print(f"      J0 Constant    : {mdl['J0_constant']:10.1f} bits")
    print(f"      J1 Linear      : {mdl['J1_linear']:10.1f} bits")
    print(f"      J2 Parabolic   : {mdl['J2_parabolic']:10.1f} bits")
    print(f"      GP Stochastic  : {mdl['GP_stochastic']:10.1f} bits (Savings vs J1: {mdl['gp_savings_vs_j1_percent']:.1f}%)")
    
    print("\n  [3] Maximum Reliable Prediction Horizon (Tolerance = 15 Hz):")
    hor = res['valid_prediction_horizons_ms']
    print(f"      tau(J0) : {hor['tau_J0_ms']:.1f} ms")
    print(f"      tau(J1) : {hor['tau_J1_ms']:.1f} ms")
    print(f"      tau(J2) : {hor['tau_J2_ms']:.1f} ms")
    print(f"      tau(GP) : {hor['tau_GP_ms']:.1f} ms")
    
    return res


def main() -> None:
    print("=========================================================")
    print("🚀 RUNNING PHASE 6: EVOLUTION AND UNCERTAINTY (E42-E45)")
    print("=========================================================")
    
    e42_results = run_e42_benchmarks()
    e43_results = run_e43_benchmarks()
    e44_results = run_e44_benchmarks()
    e45_results = run_e45_benchmarks()
    
    summary = {
        'phase': 6,
        'title': 'Phase 6: Evolution and Uncertainty (E42-E45)',
        'E42_gp_anchor_evolution': e42_results,
        'E43_phase_uncertainty': e43_results,
        'E44_probabilistic_calibration': e44_results,
        'E45_gp_vs_jet_benchmark': e45_results
    }
    
    output_path = PHASE_DIR / 'phase6_results.json'
    with open(output_path, 'w', encoding='utf-8') as f:
        json.dump(summary, f, indent=2)
        
    print("\n=========================================================")
    print(f"✅ PHASE 6 BENCHMARKS COMPLETED SUCCESSFULLY!")
    print(f"📁 Results exported to: {output_path}")
    print("=========================================================\n")


if __name__ == '__main__':
    main()
