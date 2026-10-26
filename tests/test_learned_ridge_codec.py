"""
Bateria de Testes Automatizados para a Fase 8 do Currículo MDL (E51 a E59):
Dicionário Aprendido de Trajetórias de Cristas, Prior Estocástico e Surprise Encoding.
"""

import os
import sys
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

import numpy as np

# Módulos a serem implementados na Green Phase
from synthnn.analysis.learned_ridge_codec import (
    LearnedRidgeDictionary,
    RidgeInnovationQuantizer,
    SurpriseRidgeCodec
)

def test_learned_dictionary():
    """E51: Verifica se o dicionário de bases aprendido reduz o MSE em > 40% em relação ao DPCM linear."""
    np.random.seed(42)
    block_len = 16
    num_blocks = 200

    # Gera segmentos sintéticos realistas de cristas (vibrato de 5-6 Hz + glissando + ruído)
    t = np.linspace(0, 0.08, block_len)  # 80 ms por bloco
    training_data = []
    for _ in range(num_blocks):
        f0 = np.random.uniform(200, 800)
        gliss = np.random.uniform(-100, 100)
        vib_depth = np.random.uniform(5, 25)
        vib_rate = np.random.uniform(4.5, 6.5)
        phase0 = np.random.uniform(0, 2 * np.pi)

        # Trajetória contínua
        f_traj = f0 + gliss * t + vib_depth * np.sin(2 * np.pi * vib_rate * t + phase0)
        f_traj += np.random.normal(0, 0.5, size=block_len)
        training_data.append(f_traj)

    training_data = np.array(training_data)

    # 1. Ajusta o dicionário com K=4 componentes
    dict_model = LearnedRidgeDictionary(block_len=block_len, num_components=4)
    dict_model.fit(training_data)

    # Verifica ortonormalidade da base aprendida: Phi^T @ Phi ≈ I
    basis = dict_model.basis
    gram = basis.T @ basis
    identity = np.eye(4)
    assert np.allclose(gram, identity, atol=1e-3), "A base do dicionário aprendido deve ser ortonormal"

    # 2. Avalia em blocos de teste não vistos
    test_blocks = []
    for _ in range(50):
        f0 = np.random.uniform(250, 750)
        gliss = np.random.uniform(-80, 80)
        vib_depth = np.random.uniform(8, 20)
        vib_rate = 5.5
        phase0 = np.random.uniform(0, 2 * np.pi)
        f_traj = f0 + gliss * t + vib_depth * np.sin(2 * np.pi * vib_rate * t + phase0)
        test_blocks.append(f_traj)
    test_blocks = np.array(test_blocks)

    # Reconstrução via dicionário
    coeffs = dict_model.transform(test_blocks)
    reconstructed = dict_model.inverse_transform(coeffs)
    mse_learned = float(np.mean((test_blocks - reconstructed) ** 2))

    # Baseline DPCM 1D: constante = f_0 ou reta linear
    baseline_pred = np.repeat(test_blocks[:, [0]], block_len, axis=1)
    mse_baseline = float(np.mean((test_blocks - baseline_pred) ** 2))

    reduction = (1.0 - mse_learned / mse_baseline) * 100.0
    print(f"E51 Redução de Erro do Dicionário Aprendido: {reduction:.2f}% (esperado > 40%)")
    assert reduction > 40.0, f"O dicionário aprendido deve superar baseline em > 40%, obteve {reduction:.1f}%"


def test_rans_ridge_coding():
    """E52-E54: Verifica quantização e codificação de coeficientes latentes."""
    quantizer = RidgeInnovationQuantizer(step_size=0.1)

    raw_coeffs = np.array([2.34, -1.82, 0.45, -0.08, 0.02], dtype=np.float64)
    q_integers = quantizer.quantize(raw_coeffs)
    deq_coeffs = quantizer.dequantize(q_integers)

    # O erro deve ser limitado a meio passo de quantização
    max_err = np.max(np.abs(raw_coeffs - deq_coeffs))
    assert max_err <= 0.05 + 1e-5, f"Erro de quantização deve ser <= delta/2, obteve {max_err}"
    assert np.all(np.isreal(q_integers)), "Quantização deve produzir inteiros"


def test_surprise_encoding_gating():
    """E58-E59: Verifica que o Surprise Encoding suprime transmissões em trechos estáveis e detecta surpresas."""
    num_frames = 60
    # Quadros 0 a 40: Tom estável com vibrato suave regular
    # Quadros 41 a 60: Transição abrupta (pitch bend repentino de +100 Hz)
    dt = 0.005
    t = np.arange(num_frames) * dt

    f_true = np.full(num_frames, 440.0)
    # Suave vibrato de 5 Hz
    f_true[:40] += 5.0 * np.sin(2 * np.pi * 5.0 * t[:40])
    # Salto abrupto
    f_true[40:] += 100.0 * (t[40:] - t[40]) * 10.0

    codec = SurpriseRidgeCodec(threshold_lambda=0.08, heartbeat_interval=15)

    transmitted_count = 0
    skipped_count = 0
    reconstructed_f = []

    for k in range(num_frames):
        is_transmitted, f_hat = codec.process_frame(f_true[k])
        reconstructed_f.append(f_hat)
        if is_transmitted:
            transmitted_count += 1
        else:
            skipped_count += 1

    skip_ratio = skipped_count / num_frames * 100.0
    print(f"E58 Taxa de Quadros Suprimidos por Surprise Encoding: {skip_ratio:.1f}% (esperado > 50%)")
    assert skip_ratio >= 50.0, f"Surprise encoding deve suprimir > 50% das transmissões, obteve {skip_ratio:.1f}%"

    # Erro de frequência médio em cents
    reconstructed_f = np.array(reconstructed_f)
    cents_error = 1200.0 * np.abs(np.log2(reconstructed_f / f_true))
    mean_cents = np.mean(cents_error)
    print(f"E58 Erro médio em cents com transmissão reduzida: {mean_cents:.2f} cents")
    assert mean_cents < 3.0, f"Erro médio de frequência deve ser < 3 cents, obteve {mean_cents:.2f}"

if __name__ == '__main__':
    print("🧪 Executando Bateria de Testes do Codec de Cristas Aprendido (Python)...")
    test_learned_dictionary()
    test_rans_ridge_coding()
    test_surprise_encoding_gating()
    print("\n🎉 TODOS OS TESTES EM PYTHON (E51 A E59) PASSARAM COM SUCESSO!\n")
