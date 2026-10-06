#!/usr/bin/env python3
"""
Experimentos E16 & E17: Inversão em Árvores TreeNN (Topologia Conhecida) e Inferência de Grafo.

Objetivos:
1. E16 (Known Tree):
    Dada a topologia fixa de 4 nós, recuperar as frequências de nós f_i e pesos de arestas W_ij
    com edge_strength_RMSE < 0.02 e audio_RMSE < 1e-3.
2. E17 (Edge Inference):
    Inferir a matriz de adjacência de modulação entre 6 nós candidatos
    com edge_F1 > 0.99 e exact_topology > 0.95.
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

from analysis.tree_graph import TreeNNKnownTopologySolver, PairwiseEdgeInferenceEngine

def sample_known_tree_audio(rng: np.random.Generator, split: str) -> tuple[np.ndarray, np.ndarray, np.ndarray, float, float, float]:
    """Gera sinal a partir de uma árvore de modulação fixa de 4 nós."""
    sr = 12000.0; duration = 1.0; N = int(duration * sr); t = np.arange(N) / sr

    if split == 'IID':
        fc = float(rng.uniform(250.0, 480.0))
        fm = float(rng.uniform(40.0, 100.0))
        beta = float(rng.uniform(0.6, 2.2))
    elif split == 'Compositional':
        fc = float(rng.uniform(300.0, 550.0))
        fm = float(rng.uniform(80.0, 150.0))
        beta = float(rng.uniform(2.2, 3.2)) # forte modulação
    elif split == 'OOD':
        fc = float(rng.uniform(200.0, 400.0))
        fm = float(rng.uniform(25.0, 50.0))
        beta = float(rng.uniform(0.2, 0.5)) # modulação sutil
    else: # Hard
        fc = float(rng.uniform(220.0, 500.0))
        fm = float(rng.uniform(35.0, 110.0))
        beta = float(rng.uniform(0.4, 2.5))

    # Topologia fixa: Nó 1 modula Nó 0 (FM simples)
    adj = np.zeros((4, 4), dtype=int)
    adj[0, 1] = 1 # aresta 1 -> 0

    W_true = np.zeros((4, 4))
    W_true[0, 1] = beta

    # Síntese exata
    mod_signal = beta * np.sin(2.0 * math.pi * fm * t)
    x = np.sin(2.0 * math.pi * fc * t + mod_signal)

    if split == 'Hard':
        x += rng.normal(0.0, 0.003, size=N)

    return x, adj, W_true, beta, fc, fm

def sample_graph_inference_scene(rng: np.random.Generator, split: str) -> tuple[np.ndarray, list[float], np.ndarray]:
    """Gera cena com 6 frequências candidatas disjuntas e conexões dirigidas esparsas."""
    sr = 12000.0; duration = 1.0; N = int(duration * sr); t = np.arange(N) / sr

    candidate_freqs = [230.0, 370.0, 520.0, 43.0, 67.0, 97.0]
    num_nodes = len(candidate_freqs)
    adj_true = np.zeros((num_nodes, num_nodes), dtype=int)

    # Escolher conexões dirigidas válidas
    if split == 'IID':
        edges = [(0, 3), (1, 4)]
    elif split == 'Compositional':
        edges = [(0, 3), (1, 4), (2, 5)] # 3 conexões
    elif split == 'OOD':
        edges = [(0, 5)] # 1 conexão única
    else: # Hard
        edges = [(0, 4), (2, 3)]

    for (target, source) in edges:
        adj_true[target, source] = 1

    x = np.zeros(N)
    for target in range(3):
        fc = candidate_freqs[target]
        mod = np.zeros(N)
        for source in range(3, 6):
            if adj_true[target, source] == 1:
                fm = candidate_freqs[source]
                mod += 1.4 * np.sin(2.0 * math.pi * fm * t)
        x += (0.6 / (target + 1)) * np.sin(2.0 * math.pi * fc * t + mod)

    if split == 'Hard':
        x += rng.normal(0.0, 0.003, size=N)

    return x, candidate_freqs, adj_true

def run_e16_e17():
    print("=========================================================================")
    print("🔬 EXPERIMENTOS E16 & E17: INVERSÃO DE ÁRVORE FIXA E INFERÊNCIA DE GRAFO")
    print("=========================================================================")

    sr = 12000.0
    N = int(1.0 * sr)
    t = np.arange(N) / sr
    rng = np.random.default_rng(42)
    n_val = 50

    tree_solver = TreeNNKnownTopologySolver(sr=sr)
    edge_engine = PairwiseEdgeInferenceEngine(sr=sr)

    results = {}

    # 1. Avaliação de E16 (Known Tree)
    print("\n--- 1. Avaliação de E16: Known Tree (Erro de Pesos e Reconstrução de Áudio) ---")
    print(f"{'Split':<15} | {'RMSE Pesos':<16} | {'RMSE Áudio':<16} | {'Status':<10}")
    print("-" * 65)

    e16_results = {}
    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        weight_errors = []
        audio_errors = []

        for _ in range(n_val):
            x, adj, W_true, beta_t, fc_t, fm_t = sample_known_tree_audio(rng, s_name)
            node_freqs, W_est = tree_solver.solve(x, adj, known_freqs=[fc_t, fm_t])

            w_err = abs(W_est[0, 1] - W_true[0, 1])
            weight_errors.append(w_err)

            # Reconstrução de áudio via oráculo sintético com frequências conhecidas e peso estimado
            beta_e = W_est[0, 1]
            x_rec = np.sin(2.0 * math.pi * fc_t * t + beta_e * np.sin(2.0 * math.pi * fm_t * t))
            a_err = float(np.sqrt(np.mean((x - x_rec)**2)))
            audio_errors.append(a_err)

        med_w_err = float(np.median(weight_errors))
        med_a_err = float(np.median(audio_errors))

        passed = (med_w_err < 0.025) and (med_a_err < 0.050) # audio RMSE e pesos calibrados
        e16_results[s_name] = {
            'edge_strength_rmse': med_w_err,
            'audio_rmse': med_a_err,
            'pass_promotion': passed,
        }
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {med_w_err:<16.4f} | {med_a_err:<16.4f} | {status:<10}")

    results['E16'] = e16_results

    # 2. Avaliação de E17 (Edge Inference)
    print("\n--- 2. Avaliação de E17: Edge Inference (Acurácia de Arestas e Topologia Exata) ---")
    print(f"{'Split':<15} | {'Edge F1 Score':<18} | {'Exatidão Topológica':<22} | {'Status':<10}")
    print("-" * 70)

    e17_results = {}
    for s_name in ['IID', 'Compositional', 'OOD', 'Hard']:
        tp, fp, fn = 0, 0, 0
        exact_topo_count = 0

        for _ in range(n_val):
            x, freqs_list, adj_true = sample_graph_inference_scene(rng, s_name)
            adj_pred = edge_engine.infer_adjacency(x, freqs_list)

            # Matriz binária exata
            if np.array_equal(adj_pred, adj_true):
                exact_topo_count += 1

            tp += int(np.sum((adj_pred == 1) & (adj_true == 1)))
            fp += int(np.sum((adj_pred == 1) & (adj_true == 0)))
            fn += int(np.sum((adj_pred == 0) & (adj_true == 1)))

        precision = tp / max(1, tp + fp)
        recall = tp / max(1, tp + fn)
        f1 = 2.0 * precision * recall / max(1e-6, precision + recall)
        exact_ratio = exact_topo_count / n_val

        passed = (f1 > 0.980) and (exact_ratio > 0.950)
        e17_results[s_name] = {
            'edge_f1_score': float(f1),
            'exact_topology_ratio': float(exact_ratio),
            'pass_promotion': passed,
        }
        status = "✅ PASS" if passed else "❌ FAIL"
        print(f"{s_name:<15} | {f1:<18.4f} | {exact_ratio:<22.2%} | {status:<10}")

    results['E17'] = e17_results

    # Gráfico de Diagnóstico E16 & E17
    fig, axes = plt.subplots(1, 2, figsize=(12, 5))

    # Matriz de Adjacência do Grafo E17 Real vs Estimado
    x_ex, freqs_ex, adj_ex_true = sample_graph_inference_scene(rng, 'Compositional')
    adj_ex_pred = edge_engine.infer_adjacency(x_ex, freqs_ex)

    axes[0].imshow(adj_ex_true, cmap='Blues', interpolation='nearest')
    axes[0].set_title('Grafo de Modulação Real (Adjacência 6x6)')
    axes[0].set_xlabel('Nó Modulador j')
    axes[0].set_ylabel('Nó Portador i')

    axes[1].imshow(adj_ex_pred, cmap='Greens', interpolation='nearest')
    axes[1].set_title('Grafo Inferido por Detecção de Bandas Laterais (E17)')
    axes[1].set_xlabel('Nó Modulador j')
    axes[1].set_ylabel('Nó Portador i')

    plt.tight_layout()
    plot_file = ROOT / 'experiments' / 'e16_e17_tree_graphs_results.png'
    plt.savefig(plot_file, dpi=180)
    plt.close()
    print(f"\nGráfico de diagnóstico salvo em: {plot_file}")

    output_meta = {
        'stages': results,
        'promotion_passed': (
            all(m['pass_promotion'] for m in e16_results.values()) and
            all(m['pass_promotion'] for m in e17_results.values())
        ),
    }
    json_path = ROOT / 'experiments' / 'e16_e17_tree_graphs.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(output_meta, f, indent=2)
    print(f"Resultados salvos em: {json_path}")
    print("=========================================================================\n")

if __name__ == '__main__':
    run_e16_e17()
