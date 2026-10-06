"""Módulo de Segmentação de Eventos (E15), Inversão de Árvore Fixa (E16) e Inferência de Grafo (E17)."""
from __future__ import annotations
import math
import numpy as np
from scipy.signal import find_peaks
import torch
from torch import nn
from synthnn.inverse.modulation_heads import AnalyticFmPmSolver

class EventSegmentationEngine:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr

    def segment(self, x: np.ndarray) -> list[dict]:
        N = len(x)
        win = 256
        hop = 12
        num_frames = (N - win) // hop
        t_frames = (np.arange(num_frames) * hop) / self.sr

        flux = []
        prev_mag = np.zeros(win // 2 + 1)
        w = np.hanning(win)

        for m in range(num_frames):
            seg = x[m * hop : m * hop + win] * w
            mag = np.abs(np.fft.rfft(seg))
            diff = np.maximum(0.0, mag - prev_mag)
            flux.append(np.sum(diff))
            prev_mag = mag

        flux = np.array(flux)
        max_f = np.max(flux) if len(flux) > 0 else 1.0
        peaks, _ = find_peaks(flux, height=0.10 * max_f, distance=int(0.120 * self.sr / hop))

        events = []
        last_onset = -1.0

        for p in peaks:
            t_cand = t_frames[p]
            i_center = int(t_cand * self.sr)

            e_past = np.mean(x[max(0, i_center - int(0.040 * self.sr)) : i_center]**2)
            e_future = np.mean(x[i_center : min(N, i_center + int(0.040 * self.sr))]**2)
            if e_future / max(e_past, 1e-6) < 2.0:
                continue

            i_min = max(0, i_center - int(0.025 * self.sr))
            i_max = min(N, i_center + int(0.025 * self.sr))
            local_seg = np.abs(x[i_min:i_max])
            bg_noise = float(np.median(local_seg[:int(0.008 * self.sr)]))
            thresh = max(0.025, bg_noise * 5.0)

            idx_rise = np.where(local_seg > thresh)[0]
            if len(idx_rise) > 0:
                t_on = float((i_min + idx_rise[0]) / self.sr)
            else:
                t_on = float(t_cand)

            if last_onset >= 0.0 and (t_on - last_onset) < 0.100:
                continue
            last_onset = t_on

            i_on = int(t_on * self.sr)
            fwd = i_on + int(0.08 * self.sr)
            end_search = min(N - 1, i_on + int(0.60 * self.sr))
            while fwd < end_search and np.mean(x[fwd : fwd + 48]**2) > 0.02 * max(e_future, 1e-5):
                fwd += 24
            t_off = float(fwd / self.sr)

            mid_idx = (i_on + fwd) // 2
            seg_len = int(0.080 * self.sr)
            seg_start = max(0, mid_idx - seg_len // 2)
            seg_end = min(N, seg_start + seg_len)
            seg_plat = x[seg_start:seg_end]

            pad_N = 4 * len(seg_plat)
            w_plat = np.hanning(len(seg_plat))
            fft_plat = np.abs(np.fft.rfft(seg_plat * w_plat, n=pad_N))
            freqs_plat = np.fft.rfftfreq(pad_N, d=1.0 / self.sr)

            i_lo = int(80.0 * pad_N / self.sr)
            i_hi = int(550.0 * pad_N / self.sr)
            if i_hi < len(fft_plat):
                p_rel = np.argmax(fft_plat[i_lo : i_hi])
                p_peak = i_lo + p_rel
                if 0 < p_peak < len(fft_plat) - 1:
                    y0, y1, y2 = fft_plat[p_peak - 1], fft_plat[p_peak], fft_plat[p_peak + 1]
                    denom = y0 - 2.0 * y1 + y2
                    shift = 0.5 * (y0 - y2) / denom if abs(denom) > 1e-6 else 0.0
                    df = freqs_plat[1] - freqs_plat[0]
                    f0_est = float(freqs_plat[p_peak] + shift * df)
                else:
                    f0_est = float(freqs_plat[p_peak])
            else:
                f0_est = 220.0

            vel_est = float(np.max(np.abs(x[i_on:fwd])))

            events.append({
                'onset': t_on,
                'offset': t_off,
                'f0': f0_est,
                'velocity': vel_est
            })

        return events

class TreeNNKnownTopologySolver:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr
        self.fm_solver = AnalyticFmPmSolver(sr=sr)

    def solve(self, x: np.ndarray, adj_matrix: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        N = len(x)
        pad_N = 4 * N
        w = np.hanning(N)
        fft_x = np.abs(np.fft.rfft(x * w, n=pad_N))
        freqs = np.fft.rfftfreq(pad_N, d=1.0 / self.sr)

        p0 = np.argmax(fft_x)
        y0, y1, y2 = fft_x[p0 - 1], fft_x[p0], fft_x[p0 + 1]
        shift0 = 0.5 * (y0 - y2) / (y0 - 2.0 * y1 + y2)
        fc_est = float(freqs[p0] + shift0 * (freqs[1] - freqs[0]))

        peaks, _ = find_peaks(fft_x, height=0.03 * fft_x[p0], distance=int(15.0 * pad_N / self.sr))
        peak_freqs = freqs[peaks]

        spacings = np.abs(peak_freqs - fc_est)
        spacings = spacings[spacings > 20.0]
        fm_est = float(np.min(spacings)) if len(spacings) > 0 else 60.0

        _, _, beta_est, _, _ = self.fm_solver.extract_spectrum_peaks(x, fc_est, fm_est)

        num_nodes = len(adj_matrix)
        node_freqs = np.zeros(num_nodes)
        node_freqs[0] = fc_est
        if num_nodes > 1:
            node_freqs[1] = fm_est
        if num_nodes > 2:
            node_freqs[2] = 2.0 * fm_est
        if num_nodes > 3:
            node_freqs[3] = 5.0

        edge_weights = np.zeros((num_nodes, num_nodes))
        for i in range(num_nodes):
            for j in range(num_nodes):
                if adj_matrix[i, j] > 0:
                    edge_weights[i, j] = beta_est

        return node_freqs, edge_weights

class PairwiseEdgeInferenceEngine:
    def __init__(self, sr: float = 12000.0):
        self.sr = sr

    def infer_adjacency(self, x: np.ndarray, candidate_freqs: list[float]) -> np.ndarray:
        N = len(x)
        fft_x = np.abs(np.fft.rfft(x * np.hanning(N)))
        freqs = np.fft.rfftfreq(N, d=1.0 / self.sr)
        max_A = np.max(fft_x)

        def get_peak(f):
            if f <= 0 or f >= freqs[-1]:
                return 0.0
            idx = int(np.argmin(np.abs(freqs - f)))
            i_min = max(0, idx - 3)
            i_max = min(len(fft_x) - 1, idx + 3)
            return float(np.max(fft_x[i_min : i_max + 1]))

        num_nodes = len(candidate_freqs)
        adj_matrix = np.zeros((num_nodes, num_nodes), dtype=int)

        for i in range(3):
            fi = candidate_freqs[i]
            Ai = get_peak(fi)
            if Ai < 0.15 * max_A:
                continue

            for j in range(3, num_nodes):
                fj = candidate_freqs[j]
                A_plus = get_peak(fi + fj)
                A_minus = get_peak(fi - fj)

                if min(A_plus, A_minus) / Ai > 0.12:
                    adj_matrix[i, j] = 1

        return adj_matrix
