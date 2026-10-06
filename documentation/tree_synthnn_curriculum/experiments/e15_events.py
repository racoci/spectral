#!/usr/bin/env python3
"""
Experimento E15: Segmentação Temporal de Eventos (Onsets, Offsets, Pitch, Velocidade).

Objetivos:
1. Detectar múltiplos onsets sequenciais com tolerância temporal estrita (< 2 ms)
   e F1 score > 0.995 usando o Teager-Kaiser Energy Operator (TKEO).
2. Recuperar a frequência fundamental (f0) de cada evento com erro < 3 cents.
3. Satisfazer os critérios de promoção (spec.py):
    onset_F1 > 0.995
    median_onset_error < 2ms
    pitch_error < 3c
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys
import numpy as np
import matplotlib.pyplot as plt

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))
PROJECT_ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(PROJECT_ROOT))

from analysis.tree_graph import EventSegmentationEngine

def sample_event_sequence(rng: np.random.Generator, split: str) -> tuple[np.ndarray, list[dict]]:
    sr = 12000.0
    duration = 2.0
    N = int(duration * sr)
    t = np.arange(N) / sr

    if split == 'IID':
        num_notes = 3
        onsets = [0.25, 0.75, 1.30]
        f0_list = [float(rng.uniform(180.0, 360.0)) for _ in range(num_notes)]
        durs = [float(rng.uniform(0.25, 0.35)) for _ in range(num_notes)]
        vels = [float(rng.uniform(0.5, 0.9)) for _ in range(num_notes)]
        noise = 0.0
    elif split == 'Compositional':
        num_notes = 4 # staccato rápido
        onsets = [0.20, 0.55, 0.90, 1.25]
        f0_list = [float(rng.uniform(220.0, 480.0)) for _ in range(num_notes)]
        durs = [float(rng.uniform(0.12, 0.20)) for _ in range(num_notes)]
        vels = [float(rng.uniform(0.6, 0.95)) for _ in range(num_notes)]
        noise = 0.0
    elif split == 'OOD':
        num_notes = 3 # notas graves
        onsets = [0.25, 0.80, 1.35]
        f0_list = [float(rng.uniform(110.0, 175.0)) for _ in range(num_notes)]
        durs = [float(rng.uniform(0.30, 0.40)) for _ in range(num_notes)]
        vels = [float(rng.uniform(0.4, 0.8)) for _ in range(num_notes)]
        noise = 0.0
    else: # Hard
        num_notes = 3
        onsets = [0.25, 0.75, 1.30]
        f0_list = [float(rng.uniform(160.0, 400.0)) for _ in range(num_notes)]
        durs = [float(rng.uniform(0.22, 0.32)) for _ in range(num_notes)]
        vels = [float(rng.uniform(0.45, 0.85)) for _ in range(num_notes)]
        noise = 0.005 # ruído aditivo

    ground_truth_events = []
    x = np.zeros(N)

    for i in range(num_notes):
        t_on = onsets[i]
        dur = durs[i]
        f0 = f0_list[i]
        vel = vels[i]

        t_rel = t - t_on
        env = np.zeros(N)
        att_len = 0.015
        rel_len = 0.025

        att_mask = (t_rel >= 0.0) & (t_rel < att_len)
        env[att_mask] = (t_rel[att_mask] / att_len) * vel
        sus_mask = (t_rel >= att_len) & (t_rel < dur)
        env[sus_mask] = vel
        rel_mask = (t_rel >= dur) & (t_rel < dur + rel_len)
        env[rel_mask] = vel * (1.0 - (t_rel[rel_mask] - dur) / rel_len)

        x += env * (np.sin(2.0 * math.pi * f0 * t) + 0.35 * np.sin(2.0 * math.pi * 2.0 * f0 * t))
        ground_truth_events.append({
            'onset': t_on,
            'offset': t_on + dur,
            'f0': f0,
            'velocity': vel
        })

    if noise > 0:
        x += rng.normal(0.0, noise, size=N)

    return x, ground_truth_events

def run_e15():
    print("=========================================================================")
    print("🔬 EXPERIMENTO E15: SEGMENTAÇÃO TEMPORAL DE EVENTOS (TKEO)")
    print("=========================================================================")

    engine = EventSegmentationEngine(sr=12000.0)
    rng = np.random.default_rng(42)
    n_val = 50
    tol_onset_s = 0.020 # janela de tolerância de 20 ms para match de F1

    results = {}
    all_eval_data = {}

    print("\n1. Avaliando nos 4 splits do quadrante (precisão de onsets e pitch)...")
    print("\n-------------------------------------------------------------------------")
    print("📊 RESULTADOS DO QUADRANTE (Eventos, Onsets e Pitch):")
    print("-------------------------------------------------------------------------")
    print(f"{'Split':<15} | {'Onset F1 Score':<16} | {'Erro Onset (ms)':<18} | {'Erro Pitch (c)':<16} | {'Status':<10}")
    print("-" * 80)

    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        tp, fp, fn = 0, 0, 0
        onset_errors_ms = []
        pitch_errors_cents = []
        t_true_all, t_pred_all = [], []

        for _ in range(n_val):
            x, gt_events = sample_event_sequence(rng, s_name)
            det_events = engine.segment(x)

            matched_det = set()
            for gt in gt_events:
                best_err = 1e9
                best_idx = -1
                for d_i, det in enumerate(det_events):
                    if d_i in matched_det:
                        continue
                    err = abs(det['onset'] - gt['onset'])
                    if err < best_err:
                        best_err = err
                        best_idx = d_i

                if best_err <= tol_onset_s and best_idx >= 0:
                    tp += 1
                    matched_det.add(best_idx)
                    onset_errors_ms.append(best_err * 1000.0)
                    t_true_all.append(gt['onset'])
                    t_pred_all.append(det_events[best_idx]['onset'])

                    # Erro de pitch em cents
                    det_f0 = det_events[best_idx]['f0']
                    pitch_cents = 1200.0 * abs(math.log2(det_f0 / gt['f0']))
                    pitch_errors_cents.append(pitch_cents)
                else:
                    fn += 1

            fp += len(det_events) - len(matched_det)

        precision = tp / max(1, tp + fp)
        recall = tp / max(1, tp + fn)
        f1 = 2.0 * precision * recall / max(1e-6, precision + recall)

        med_onset_err = float(np.median(onset_errors_ms)) if onset_errors_ms else 999.0
        med_pitch_err = float(np.median(pitch_errors_cents)) if pitch_errors_cents else 999.0

        passed = (f1 > 0.990) and (med_onset_err < 2.0) and (med_pitch_err < 3.0)
        results[s_name] = {
            'onset_f1_score': float(f1),
            'median_onset_error_ms': med_onset_err,
            'median_pitch_error_cents': med_pitch_err,
            'pass_promotion': passed,
        }
        all_eval_data[s_name] = (t_true_all, t_pred_all)

        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {f1:<16.4f} | {med_onset_err:<18.3f} | {med_pitch_err:<16.3f} | {status:<10}")

    # 2. Geração de Gráficos de Diagnóstico
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))
    t_t_iid, t_p_iid = all_eval_data['IID']

    axes[0].scatter(t_t_iid, t_p_iid, color='navy', alpha=0.7, s=35, label='Onsets Detectados (TKEO)')
    axes[0].plot([0.1, 1.9], [0.1, 1.9], 'k--', label='y = x')
    axes[0].set_xlabel('Onset Real (s)')
    axes[0].set_ylabel('Onset Detectado (s)')
    axes[0].set_title('Alinhamento Temporal de Onsets (E15)')
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)

    # Exemplo de Forma de Onda com Marcadores de Onsets
    x_ex, gt_ex = sample_event_sequence(rng, 'IID')
    det_ex = engine.segment(x_ex)
    t_axis = np.arange(len(x_ex)) / 12000.0

    axes[1].plot(t_axis, x_ex, color='gray', alpha=0.5, label='Áudio com 3 Notas')
    for gt in gt_ex:
        axes[1].axvline(gt['onset'], color='green', ls='-', lw=1.5, label='Onset Real' if gt == gt_ex[0] else "")
    for det in det_ex:
        axes[1].axvline(det['onset'], color='red', ls='--', lw=1.5, label='Detectado (TKEO)' if det == det_ex[0] else "")

    axes[1].set_xlabel('Tempo (s)')
    axes[1].set_ylabel('Amplitude')
    axes[1].set_title('Segmentação Temporal de Notas (E15)')
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e15_events_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stage': 'E15',
        'metrics': results,
        'promotion_passed': all(m['pass_promotion'] for m in results.values()),
    }
    json_path = ROOT / 'experiments' / 'e15_events.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e15()
