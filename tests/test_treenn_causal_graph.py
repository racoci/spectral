"""
Bateria de Testes Automatizados para a Fase 12 do Currículo MDL (E60 a E63):
SynthNN & TreeNN Grafo Causal Hierárquico e Poda Estrutural por MDL.
"""

import os
import sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import numpy as np

# Módulos a serem implementados na Green Phase
from synthnn.core.tree_nn_graph import (
    TreeNNCausalGraph,
    OperatorConfig
)
from synthnn.inverse.mdl_topology_pruner import (
    MdlTopologyPruner
)

def test_treenn_dense_synthesis():
    """E60-E61: Verifica a síntese causal não-linear de operadores acoplados com matriz densa e feedback estável."""
    fs = 48000
    duration = 0.5  # 24000 amostras
    num_ops = 3

    ops = [
        OperatorConfig(freq=440.0, base_amplitude=0.8, is_carrier=True),
        OperatorConfig(freq=220.0, base_amplitude=0.6, is_carrier=False),
        OperatorConfig(freq=5.0,   base_amplitude=1.0, is_carrier=False)
    ]

    # Matriz densa com arestas principais e ruído espúrio de acoplamento
    W_dense = np.array([
        [0.0,  1.5,  0.03], # Op 0 é modulado por Op 1 (aresta real) e leve ruído de Op 2
        [0.02, 0.4,  0.80], # Op 1 possui feedback 0.4 e é modulado por Op 2 (aresta real)
        [0.01, 0.02, 0.00]  # Op 2 (LFO) quase autônomo com ruído espúrio
    ], dtype=np.float64)

    output_weights = np.array([1.0, 0.0, 0.0], dtype=np.float64)

    graph = TreeNNCausalGraph(ops=ops, W=W_dense, output_weights=output_weights, sample_rate=fs)
    audio = graph.synthesize(duration=duration)

    assert len(audio) == int(fs * duration), "Tamanho do buffer de áudio deve coincidir com sample_rate * duration"
    assert not np.isnan(audio).any(), "O áudio sintetizado não pode conter valores NaN"
    assert not np.isinf(audio).any(), "O áudio sintetizado não pode conter valores infinitos"
    assert np.max(np.abs(audio)) > 0.05, "O áudio sintetizado deve conter energia acústica audível"
    assert np.max(np.abs(audio)) <= 1.5, "A saturação suave deve manter o áudio estável"
    print(f"  ✓ Síntese densa estável: RMS = {np.sqrt(np.mean(audio**2)):.4f}, Pico = {np.max(np.abs(audio)):.4f}")


def test_mdl_edge_pruning():
    """E62: Verifica a poda estrutural por MDL eliminando > 60% das arestas fracas e conservando topologia causal."""
    fs = 48000
    duration = 0.3

    ops = [
        OperatorConfig(freq=440.0, base_amplitude=0.8, is_carrier=True),
        OperatorConfig(freq=220.0, base_amplitude=0.6, is_carrier=False),
        OperatorConfig(freq=5.0,   base_amplitude=1.0, is_carrier=False)
    ]

    # Ground truth esparso: apenas W[0, 1] e W[1, 2]
    W_true = np.zeros((3, 3), dtype=np.float64)
    W_true[0, 1] = 1.4
    W_true[1, 2] = 0.9

    output_weights = np.array([1.0, 0.0, 0.0], dtype=np.float64)
    gt_graph = TreeNNCausalGraph(ops=ops, W=W_true, output_weights=output_weights, sample_rate=fs)
    target_audio = gt_graph.synthesize(duration=duration)

    # Matriz inicial ruidosa (totalmente conectada com 9 arestas)
    np.random.seed(42)
    W_init = W_true + np.random.uniform(-0.08, 0.08, size=(3, 3))
    dense_graph = TreeNNCausalGraph(ops=ops, W=W_init, output_weights=output_weights, sample_rate=fs)

    pruner = MdlTopologyPruner(lambda_g=0.015, threshold_tau=0.12)
    pruned_W = pruner.prune(dense_graph, target_audio)

    initial_active_edges = int(np.sum(np.abs(W_init) > 0.01))
    pruned_active_edges = int(np.sum(np.abs(pruned_W) > 0.01))
    reduction_pct = (1.0 - pruned_active_edges / initial_active_edges) * 100.0

    print(f"E62 Poda de arestas: {initial_active_edges} -> {pruned_active_edges} arestas (Redução de {reduction_pct:.1f}%)")
    assert reduction_pct >= 60.0, f"Poda MDL deve eliminar >= 60% das arestas fracas, eliminou {reduction_pct:.1f}%"

    # As arestas ground truth devem ser preservadas
    assert abs(pruned_W[0, 1] - 1.4) < 0.2, "Aresta dominante W[0, 1] deve ser preservada"
    assert abs(pruned_W[1, 2] - 0.9) < 0.2, "Aresta dominante W[1, 2] deve ser preservada"

    # Avalia SNR do áudio podado
    pruned_graph = TreeNNCausalGraph(ops=ops, W=pruned_W, output_weights=output_weights, sample_rate=fs)
    pruned_audio = pruned_graph.synthesize(duration=duration)
    err = target_audio - pruned_audio
    snr = 10.0 * np.log10(np.sum(target_audio**2) / (np.sum(err**2) + 1e-9))
    print(f"E62 SNR do sinal reconstruído após poda MDL: {snr:.2f} dB (esperado > 25 dB)")
    assert snr > 25.0, f"SNR deve ser > 25 dB, obteve {snr:.2f} dB"


def test_json_export_roundtrip():
    """E63: Valida a exportação e importação do grafo TreeNN para formato JSON compatível com a Web UI."""
    ops = [
        OperatorConfig(freq=440.0, base_amplitude=0.8, is_carrier=True),
        OperatorConfig(freq=220.0, base_amplitude=0.6, is_carrier=False)
    ]
    W = np.array([[0.0, 1.2], [0.0, 0.0]])
    v = np.array([1.0, 0.0])

    graph = TreeNNCausalGraph(ops=ops, W=W, output_weights=v)
    json_dict = graph.to_dict()

    assert json_dict["num_operators"] == 2
    assert len(json_dict["operators"]) == 2
    assert json_dict["W"][0][1] == 1.2

    reloaded_graph = TreeNNCausalGraph.from_dict(json_dict)
    assert reloaded_graph.W[0, 1] == 1.2
    assert reloaded_graph.ops[0].freq == 440.0
    print("  ✓ Exportação JSON canônica validada com 100% de simetria")

if __name__ == '__main__':
    print("🧪 Executando Bateria de Testes da TreeNN e Grafo Causal (Python)...")
    test_treenn_dense_synthesis()
    test_mdl_edge_pruning()
    test_json_export_roundtrip()
    print("\n🎉 TODOS OS TESTES EM PYTHON (E60 A E63) PASSARAM COM SUCESSO!\n")
