"""
Módulo de Rastreamento Probabilístico de Cristas Múltiplas (E46 a E50).
Implementa EKF 4D, propagação de covariância, associação de Mahalanobis e inpainting via GP.
"""

from typing import List, Tuple, Dict, Optional
import numpy as np
from scipy.optimize import linear_sum_assignment

class RidgeState4D:
    def __init__(self, f: float, f_dot: float, ln_a: float, a_dot_rel: float):
        self.f = float(f)
        self.f_dot = float(f_dot)
        self.ln_a = float(ln_a)
        self.a_dot_rel = float(a_dot_rel)

    def to_array(self) -> np.ndarray:
        return np.array([self.f, self.f_dot, self.ln_a, self.a_dot_rel], dtype=np.float64)


class KalmanRidgeFilter:
    """Filtro de Kalman Estendido / GP-SDE de 4 Dimensões para dinâmica de cristas espectrais."""
    def __init__(
        self,
        initial_f: float,
        initial_f_dot: float = 0.0,
        initial_ln_a: float = 0.0,
        initial_a_dot_rel: float = 0.0,
        dt: float = 0.005,
        sigma_f: float = 2.0,
        sigma_lna: float = 0.05,
        sigma_ddot_f: float = 60.0,
        sigma_ddot_a: float = 1.5
    ):
        self.dt = float(dt)
        self.state = np.array([initial_f, initial_f_dot, initial_ln_a, initial_a_dot_rel], dtype=np.float64)

        # Matriz de transição F
        self.F = np.array([
            [1.0, self.dt, 0.0, 0.0],
            [0.0, 1.0,     0.0, 0.0],
            [0.0, 0.0,     1.0, self.dt],
            [0.0, 0.0,     0.0, 1.0]
        ], dtype=np.float64)

        # Matriz de observação H (mede f e ln_a)
        self.H = np.array([
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0]
        ], dtype=np.float64)

        # Matriz de ruído de medição R
        self.R = np.array([
            [sigma_f ** 2, 0.0],
            [0.0, sigma_lna ** 2]
        ], dtype=np.float64)

        # Matriz de ruído de processo Q
        dt2 = self.dt ** 2
        dt3 = self.dt ** 3
        q_f = sigma_ddot_f ** 2
        q_a = sigma_ddot_a ** 2

        self.Q = np.array([
            [dt3 / 3.0 * q_f, dt2 / 2.0 * q_f, 0.0, 0.0],
            [dt2 / 2.0 * q_f, self.dt * q_f,   0.0, 0.0],
            [0.0, 0.0, dt3 / 3.0 * q_a, dt2 / 2.0 * q_a],
            [0.0, 0.0, dt2 / 2.0 * q_a, self.dt * q_a]
        ], dtype=np.float64)

        # Matriz de covariância inicial P
        self.P = np.diag([sigma_f ** 2, (sigma_f / self.dt) ** 2, sigma_lna ** 2, 1.0]).astype(np.float64)

    def predict(self):
        """Passo de predição temporal a priori."""
        self.state = self.F @ self.state
        self.P = self.F @ self.P @ self.F.T + self.Q

    def update(self, z: np.ndarray):
        """Passo de atualização com observação z = [f, ln_a]."""
        y = z - (self.H @ self.state)  # Inovação
        S = self.H @ self.P @ self.H.T + self.R + np.eye(2) * 1e-6
        K = self.P @ self.H.T @ np.linalg.inv(S)

        self.state = self.state + K @ y
        I = np.eye(4)
        self.P = (I - K @ self.H) @ self.P

    def get_predicted_measurement(self) -> np.ndarray:
        return self.H @ self.state

    def get_innovation_covariance(self) -> np.ndarray:
        return self.H @ self.P @ self.H.T + self.R + np.eye(2) * 1e-6


class MahalanobisAssociator:
    """Associador ótimo com distância de Mahalanobis e Algoritmo Húngaro."""
    def __init__(self, gate_threshold_chi2: float = 9.21):
        self.gate_thresh = float(gate_threshold_chi2)

    def compute_distance(self, filter_obj: KalmanRidgeFilter, peak: np.ndarray) -> float:
        pred_z = filter_obj.get_predicted_measurement()
        diff = peak - pred_z
        S = filter_obj.get_innovation_covariance()
        inv_S = np.linalg.inv(S)
        return float(diff.T @ inv_S @ diff)

    def associate(
        self,
        tracks: List[KalmanRidgeFilter],
        peaks: List[np.ndarray]
    ) -> Tuple[Dict[int, int], List[int], List[int]]:
        num_tracks = len(tracks)
        num_peaks = len(peaks)

        if num_tracks == 0 or num_peaks == 0:
            return {}, list(range(num_tracks)), list(range(num_peaks))

        cost_matrix = np.full((num_tracks, num_peaks), 1e6)

        for i, trk in enumerate(tracks):
            for j, pk in enumerate(peaks):
                d2 = self.compute_distance(trk, pk)
                if d2 <= self.gate_thresh:
                    cost_matrix[i, j] = d2

        row_ind, col_ind = linear_sum_assignment(cost_matrix)

        assignments = {}
        assigned_tracks = set()
        assigned_peaks = set()

        for r, c in zip(row_ind, col_ind):
            if cost_matrix[r, c] < self.gate_thresh:
                assignments[r] = c
                assigned_tracks.add(r)
                assigned_peaks.add(c)

        unassigned_tracks = [i for i in range(num_tracks) if i not in assigned_tracks]
        unassigned_peaks = [j for j in range(num_peaks) if j not in assigned_peaks]

        return assignments, unassigned_tracks, unassigned_peaks


class Trajectory:
    """Representação persistente de uma crista ao longo do tempo."""
    def __init__(self, track_id: int, start_frame: int, initial_peak: np.ndarray, dt: float):
        self.id = track_id
        self.start_frame = start_frame
        self.end_frame = start_frame
        self.frequencies = [float(initial_peak[0])]
        self.amplitudes = [float(np.exp(initial_peak[1]))]
        self.filter = KalmanRidgeFilter(
            initial_f=initial_peak[0],
            initial_f_dot=0.0,
            initial_ln_a=initial_peak[1],
            initial_a_dot_rel=0.0,
            dt=dt
        )
        self.state = "tentative"
        self.hits = 1
        self.consecutive_misses = 0


class ProbabilisticMultiRidgeTracker:
    """Rastreador probabilístico multi-cristas com ciclo de vida e EKF."""
    def __init__(
        self,
        dt: float = 0.005,
        min_hits_confirm: int = 3,
        max_coasting_frames: int = 4,
        gate_threshold_chi2: float = 9.21
    ):
        self.dt = dt
        self.min_hits_confirm = min_hits_confirm
        self.max_coasting_frames = max_coasting_frames
        self.associator = MahalanobisAssociator(gate_threshold_chi2)

        self.current_frame = 0
        self.next_track_id = 1
        self.active_tracks: List[Trajectory] = []
        self.completed_tracks: List[Trajectory] = []

    def process_frame(self, peaks: List[np.ndarray]):
        self.current_frame += 1

        # 1. Predição de todos os filtros ativos
        for trk in self.active_tracks:
            trk.filter.predict()

        filters = [trk.filter for trk in self.active_tracks]

        # 2. Associação Húngara por distância de Mahalanobis
        assignments, unassigned_tracks, unassigned_peaks = self.associator.associate(filters, peaks)

        # 3. Atualização das cristas associadas
        for trk_idx, pk_idx in assignments.items():
            trk = self.active_tracks[trk_idx]
            peak = peaks[pk_idx]
            trk.filter.update(peak)
            trk.frequencies.append(float(trk.filter.state[0]))
            trk.amplitudes.append(float(np.exp(trk.filter.state[2])))
            trk.end_frame = self.current_frame
            trk.hits += 1
            trk.consecutive_misses = 0
            if trk.hits >= self.min_hits_confirm:
                trk.state = "active"

        # 4. Tratamento de cristas ocluídas (coasting)
        surviving_tracks = []
        for trk_idx in unassigned_tracks:
            trk = self.active_tracks[trk_idx]
            trk.consecutive_misses += 1
            trk.frequencies.append(float(trk.filter.state[0]))
            trk.amplitudes.append(float(np.exp(trk.filter.state[2])))
            trk.end_frame = self.current_frame

            if trk.consecutive_misses <= self.max_coasting_frames and trk.state == "active":
                trk.state = "coasting"
                surviving_tracks.append(trk)
            else:
                trk.state = "terminated"
                self.completed_tracks.append(trk)

        # 5. Inicialização de novas cristas potenciais a partir de picos não associados
        new_tracks = []
        for pk_idx in unassigned_peaks:
            new_trk = Trajectory(self.next_track_id, self.current_frame, peaks[pk_idx], self.dt)
            self.next_track_id += 1
            new_tracks.append(new_trk)

        # Atualiza lista de ativas
        assigned_survivors = [self.active_tracks[i] for i in assignments.keys()]
        self.active_tracks = assigned_survivors + surviving_tracks + new_tracks

    def get_active_trajectories(self) -> List[Trajectory]:
        # Ordena por comprimento decrescente
        return sorted(self.active_tracks, key=lambda t: len(t.frequencies), reverse=True)

    def get_completed_trajectories(self) -> List[Trajectory]:
        return sorted(self.completed_tracks, key=lambda t: len(t.frequencies), reverse=True)


def gp_spectral_inpainting(
    t_obs: np.ndarray,
    f_obs: np.ndarray,
    t_gap: np.ndarray,
    length_scale: float = 0.08,
    sigma_n: float = 0.01
) -> np.ndarray:
    """Interpolação espectral sobre lacunas de oclusão via regressão por Processos Gaussianos."""
    diff_obs = t_obs[:, None] - t_obs[None, :]
    K_obs = np.exp(-0.5 * (diff_obs / length_scale) ** 2) + np.eye(len(t_obs)) * (sigma_n ** 2)

    diff_star = t_gap[:, None] - t_obs[None, :]
    K_star = np.exp(-0.5 * (diff_star / length_scale) ** 2)

    mean_f = np.mean(f_obs)
    y = f_obs - mean_f

    pred_mean = K_star @ np.linalg.solve(K_obs, y) + mean_f
    return pred_mean
