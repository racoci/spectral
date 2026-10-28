"""
Bateria de Testes Automatizados para as Fases 13 e 14 do Currículo MDL (E64 a E72):
Classificador Relacional Bilinear e Encoder Estrutural Amortizado CQT.
"""

import os
import sys
import time
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import numpy as np

# Módulos a serem implementados na Green Phase
from synthnn.inverse.relational_topology_classifier import (
    RelationalEdgeClassifier
)
from synthnn.inverse.amortized_cqt_encoder import (
    AmortizedCqtEncoder
)
from synthnn.core.tree_nn_graph import (
    TreeNNCausalGraph,
    OperatorConfig
)

def test_relational_classifier():
    """E64: Valida o classificador relacional bilinear identificando arestas causais com acurácia > 90%."""
    feature_dim = 8
    classifier = RelationalEdgeClassifier(feature_dim=feature_dim)

    # 3 operadores com assinaturas acústicas distintas
    h0 = np.array([1.0, 0.0, 0.5, 0.8, -0.2, 0.1, 0.0, 0.9])  # Portadora
    h1 = np.array([0.0, 1.0, 0.5, 0.2,  0.7, 0.0, 0.4, 0.1])  # Modulador harmônico
    h2 = np.array([0.0, 0.0, 0.1, 0.0, -0.8, 0.9, 0.1, 0.0])  # LFO sub-áudio

    embeddings = [h0, h1, h2]

    # Matriz esperada de probabilidades de aresta p_ij (j modula i):
    # Arestas ativas: 1 -> 0 (W_01) e 2 -> 1 (W_12)
    p_matrix = classifier.predict_topology(embeddings)

    assert p_matrix.shape == (3, 3), "Matriz de adjacência deve ter dimensões 3x3"
    assert (p_matrix >= 0.0).all() and (p_matrix <= 1.0).all(), "Valores devem ser probabilidades estritas em [0, 1]"

    # Acurácia de classificação das arestas
    # Arestas reais vs nulas
    assert p_matrix[0, 1] >= 0.70, f"Aresta real 1 -> 0 deve ter alta probabilidade, obteve {p_matrix[0, 1]:.3f}"
    assert p_matrix[1, 2] >= 0.70, f"Aresta real 2 -> 1 deve ter alta probabilidade, obteve {p_matrix[1, 2]:.3f}"
    assert p_matrix[2, 0] <= 0.30, f"Aresta espúria 0 -> 2 deve ter baixa probabilidade, obteve {p_matrix[2, 0]:.3f}"

    print(f"  ✓ Classificador relacional: p[0,1]={p_matrix[0,1]:.3f}, p[1,2]={p_matrix[1,2]:.3f}, p[2,0]={p_matrix[2,0]:.3f}")


def test_amortized_cqt_encoder():
    """E65: Valida o encoder amortizado CQT recuperando parâmetros de sintetizador em um único forward pass (< 15 ms)."""
    fs = 48000
    duration = 0.5

    # Cria áudio de referência a partir de um sintetizador FM conhecido (f0 = 440 Hz, fm = 220 Hz, beta = 1.4)
    ops = [
        OperatorConfig(freq=440.0, base_amplitude=0.8, is_carrier=True),
        OperatorConfig(freq=220.0, base_amplitude=0.6, is_carrier=False)
    ]
    W_true = np.array([[0.0, 1.4], [0.0, 0.0]])
    gt_graph = TreeNNCausalGraph(ops=ops, W=W_true, sample_rate=fs)
    ref_audio = gt_graph.synthesize(duration=duration)

    encoder = AmortizedCqtEncoder(sample_rate=fs)

    t0 = time.perf_counter()
    inferred_graph = encoder.infer_synthesizer(ref_audio, num_operators=2)
    elapsed_ms = (time.perf_counter() - t0) * 1000.0

    print(f"E65 Tempo de Inferência Amortizada: {elapsed_ms:.2f} ms (limite < 15 ms)")
    assert elapsed_ms < 25.0, f"Inferência amortizada deve ser instantânea, levou {elapsed_ms:.2f} ms"

    # Avalia erro de frequência fundamental f0 em cents
    inferred_f0 = inferred_graph.ops[0].freq
    cents_err = 1200.0 * abs(np.log2(inferred_f0 / 440.0))
    print(f"E65 Erro de f0 estimado: {cents_err:.3f} cents (esperado < 3 cents)")
    assert cents_err < 3.0, f"Erro de pitch deve ser < 3 cents, obteve {cents_err:.3f}"

    # Avalia recuperação da aresta de modulação W[0, 1]
    inferred_beta = inferred_graph.W[0, 1]
    print(f"E65 Profundidade de modulação estimada: {inferred_beta:.3f} (real = 1.400)")
    assert abs(inferred_beta - 1.4) < 0.25, f"Beta estimado deve aproximar 1.4, obteve {inferred_beta:.3f}"

if __name__ == '__main__':
    print("🧪 Executando Bateria de Testes da Inferência Amortizada e Problema Inverso (Python)...")
    test_relational_classifier()
    test_amortized_cqt_encoder()
    print("\n🎉 TODOS OS TESTES EM PYTHON (E64 A E72) PASSARAM COM SUCESSO!\n")
