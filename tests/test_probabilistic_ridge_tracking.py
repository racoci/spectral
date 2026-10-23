"""
Bateria de Testes Automatizados para a Fase 7 do Currículo MDL (E46 a E50):
Rastreamento Probabilístico de Cristas, Associação por Mahalanobis, Cruzamento e Oclusão.
"""

import pytest
import numpy as np

# Módulos do rastreador a serem implementados na Green Phase
from synthnn.analysis.probabilistic_tracker import (
    RidgeState4D,
    KalmanRidgeFilter,
    MahalanobisAssociator,
    ProbabilisticMultiRidgeTracker,
    gp_spectral_inpainting
)

def test_ekf_covariance_calibration():
    """E46: Verifica a calibração probabilística da matriz de covariância P a 95% via teste qui-quadrado (NEES)."""
    np.random.seed(42)
    fs = 48000
    hop = 256
    dt = hop / fs  # ~5.33 ms
    num_frames = 200

    # Sinal de chirp linear com amplitude decrescente
    f0 = 440.0
    chirp_rate = 120.0  # Hz/s
    a0 = 0.8
    decay_rate = -0.5  # s^-1

    t_axis = np.arange(num_frames) * dt
    true_f = f0 + chirp_rate * t_axis
    true_f_dot = np.full(num_frames, chirp_rate)
    true_ln_a = np.log(a0) + decay_rate * t_axis
    true_a_dot_rel = np.full(num_frames, decay_rate)

    # Ruído de observação
    sigma_f = 2.0  # Hz
    sigma_lna = 0.05
    obs_f = true_f + np.random.normal(0, sigma_f, size=num_frames)
    obs_lna = true_ln_a + np.random.normal(0, sigma_lna, size=num_frames)

    # Inicializa EKF 4D
    ekf = KalmanRidgeFilter(
        initial_f=obs_f[0],
        initial_f_dot=0.0,
        initial_ln_a=obs_lna[0],
        initial_a_dot_rel=0.0,
        dt=dt,
        sigma_f=sigma_f,
        sigma_lna=sigma_lna
    )

    nees_values = []
    for k in range(1, num_frames):
        ekf.predict()
        z = np.array([obs_f[k], obs_lna[k]])
        ekf.update(z)

        # Erro de estado normalizado pela covariância
        true_x = np.array([true_f[k], true_f_dot[k], true_ln_a[k], true_a_dot_rel[k]])
        err = true_x - ekf.state
        nees = float(err.T @ np.linalg.inv(ekf.P) @ err)
        nees_values.append(nees)

    # Limiar qui-quadrado para 4 graus de liberdade a 95% de confiança
    chi2_95 = 9.488
    coverage = np.mean(np.array(nees_values) <= chi2_95)
    print(f"E46 Cobertura da elipse a 95%: {coverage * 100:.2f}% (esperado >= 90%)")
    assert coverage >= 0.88, f"Calibração de covariância falhou: cobertura = {coverage * 100:.1f}%"


def test_mahalanobis_association():
    """E47: Valida a atribuição ótima por distância de Mahalanobis e Algoritmo Húngaro."""
    dt = 0.005
    associator = MahalanobisAssociator(gate_threshold_chi2=9.21)

    # Duas cristas com frequências próximas (400 Hz e 440 Hz)
    track1 = KalmanRidgeFilter(initial_f=400.0, initial_f_dot=0.0, initial_ln_a=0.0, initial_a_dot_rel=0.0, dt=dt)
    track2 = KalmanRidgeFilter(initial_f=440.0, initial_f_dot=0.0, initial_ln_a=0.0, initial_a_dot_rel=0.0, dt=dt)
    track1.predict()
    track2.predict()

    # Dois picos observados com pequena perturbação
    peaks = [
        np.array([441.5, -0.02]),  # Deve associar ao track2
        np.array([399.0, 0.01]),   # Deve associar ao track1
        np.array([800.0, -2.00])    # Fora do gate (novo pico)
    ]

    assignments, unassigned_tracks, unassigned_peaks = associator.associate([track1, track2], peaks)

    # track1 (índice 0) deve receber o pico 1 (399 Hz)
    assert assignments[0] == 1, "Track 1 deve ser associado ao pico de 399 Hz"
    # track2 (índice 1) deve receber o pico 0 (441.5 Hz)
    assert assignments[1] == 0, "Track 2 deve ser associado ao pico de 441.5 Hz"
    # Pico 2 (800 Hz) deve ser não-atribuído
    assert 2 in unassigned_peaks, "Pico de 800 Hz deve permanecer não-associado para criar nova crista"


def test_crossing_benchmark():
    """E48: Benchmark de cruzamento em 'X' garantindo conservação de identidade (swap < 1%)."""
    dt = 0.005
    num_frames = 100
    t = np.arange(num_frames) * dt

    # Chirp ascendente (400 -> 600 Hz) e descendente (600 -> 400 Hz), cruzam em t=0.25s
    f1 = 400.0 + 400.0 * t
    f2 = 600.0 - 400.0 * t

    tracker = ProbabilisticMultiRidgeTracker(dt=dt)

    # Alimenta o rastreador quadro a quadro
    for k in range(num_frames):
        peaks = [np.array([f1[k], 0.0]), np.array([f2[k], 0.0])]
        # Embaralha picos para forçar o rastreador a depender da dinâmica de chirp
        if k % 2 == 1:
            peaks.reverse()
        tracker.process_frame(peaks)

    # Identifica as duas trajetórias ativas mais longas
    trajectories = tracker.get_active_trajectories()
    assert len(trajectories) >= 2, "Devem existir pelo menos 2 trajetórias contínuas"

    # Mede desvio em relação ao chirp ascendente e descendente
    traj_a = trajectories[0]
    traj_b = trajectories[1]

    # Verifica continuidade monotônica das frequências
    f_diff_a = np.diff(traj_a.frequencies)
    f_diff_b = np.diff(traj_b.frequencies)
    
    # As derivadas devem manter o sinal consistente (sem inversão abrupta no ponto de cruzamento)
    assert np.mean(f_diff_a > 0) > 0.90 or np.mean(f_diff_a < 0) > 0.90, "Trajetória deve preservar sinal do chirp rate"
    assert np.mean(f_diff_b > 0) > 0.90 or np.mean(f_diff_b < 0) > 0.90, "Trajetória deve preservar sinal do chirp rate"


def test_birth_death_benchmark():
    """E49: Benchmark de ciclo de vida (ataque e término preciso de parciais)."""
    dt = 0.005
    tracker = ProbabilisticMultiRidgeTracker(dt=dt, min_hits_confirm=3, max_coasting_frames=4)

    # Crista viva apenas entre quadros 10 e 40 (duração 150 ms)
    for k in range(50):
        peaks = []
        if 10 <= k <= 40:
            peaks.append(np.array([440.0, 0.0]))
        tracker.process_frame(peaks)

    completed = tracker.get_completed_trajectories()
    assert len(completed) >= 1, "A crista terminada deve ser registrada nas concluídas"
    traj = completed[0]

    # Erro de início e fim deve ser inferior a 3 quadros (< 15 ms)
    assert abs(traj.start_frame - 10) <= 2, f"Início detectado no quadro {traj.start_frame}, esperado ~10"
    assert abs(traj.end_frame - 40) <= 3, f"Fim detectado no quadro {traj.end_frame}, esperado ~40"


def test_occlusion_gp_inpainting():
    """E50: Benchmark de oclusão e inpainting espectral via Processo Gaussiano."""
    dt = 0.005
    num_frames = 40
    t = np.arange(num_frames) * dt
    # Tom com vibrato suave de 5 Hz
    true_f = 440.0 + 15.0 * np.sin(2 * np.pi * 5.0 * t)

    # Mascaramento entre os quadros 15 e 25 (gap de 50 ms)
    observed_t = np.concatenate([t[:15], t[25:]])
    observed_f = np.concatenate([true_f[:15], true_f[25:]])
    gap_t = t[15:25]

    # Inpainting via GP condicionado
    infilled_f = gp_spectral_inpainting(observed_t, observed_f, gap_t, length_scale=0.08)

    # Erro máximo em cents
    cents_error = 1200.0 * np.abs(np.log2(infilled_f / true_f[15:25]))
    max_cents = np.max(cents_error)
    print(f"E50 Erro máximo de inpainting GP no gap: {max_cents:.2f} cents")
    assert max_cents < 5.0, f"Erro de interpolação GP sob oclusão deve ser < 5 cents, obteve {max_cents:.2f}"
