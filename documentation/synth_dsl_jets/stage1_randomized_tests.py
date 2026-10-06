#!/usr/bin/env python3
"""
Estágio 1: Frontend Puramente Analítico — Testes Aleatorizados e Verificação Monte Carlo (E1).

Objetivo:
Validar exaustivamente a extração analítica de jatos contínuos temporais
(f_inst, f_dot, f_ddot) a partir da CQT Gaussiana de 60 bins/oitava em banda-base,
sem qualquer aprendizado de máquina.

Critério de Aprovação (v12 Curriculum):
- Erro de afinação: RMSE < 2 cents (ou erro relativo < 0.1%).
- Taxa de sucesso: >= 95% dos ensaios com erro < 2 cents.
- Recuperação de chirp rate (f_dot) e aceleração (f_ddot) com alta precisão.
"""

from __future__ import annotations
from pathlib import Path
import json, math, sys, time
import numpy as np
import matplotlib.pyplot as plt

# =============================================================================
# 1. EXTRATOR CQT GAUSSIANO 60 BINS/OCT EM BANDA-BASE COM DERIVADAS HERMITIANAS
# =============================================================================

class BasebandGaussianCqtJetExtractor:
    """
    Extrator de Jatos Espectrais de Frequência Instantânea e Derivadas Temporais.
    Implementa:
    1. Banco de filtros CQT gaussiano de 60 bins/oitava.
    2. Demodulação em banda-base: s(t) = x(t) * exp(-i 2pi fc t).
    3. Convolução com janelas derivadas Hermite-Gaussianas (ordens 0..3) para obter
       derivadas temporais analíticas sem diferenças finitas.
    4. Recursão Newton-Euler para derivadas logarítmicas de fase.
    5. Inversão analítica da deformação induzida pela janela em chirps rápidos.
    """
    def __init__(self, sr: float = 24000.0, bins_per_octave: int = 60, fmin: float = 40.0, fmax: float = 5500.0):
        self.sr = sr
        self.B = bins_per_octave
        self.fmin = fmin
        self.fmax = fmax
        self.dt = 1.0 / sr

        # Grade geométrica de frequências centrais dos canais CQT
        self.num_octaves = math.log2(self.fmax / self.fmin)
        self.num_channels = int(round(self.B * self.num_octaves))
        self.channels = np.arange(self.num_channels)
        self.freqs = self.fmin * (2.0 ** (self.channels / self.B))

    def find_nearest_bin(self, f: float) -> tuple[int, float]:
        """Localiza o canal CQT mais próximo de uma dada frequência."""
        f_clamped = max(self.fmin, min(self.fmax, f))
        k_bin = int(round(self.B * math.log2(f_clamped / self.fmin)))
        k_bin = max(0, min(self.num_channels - 1, k_bin))
        return k_bin, self.freqs[k_bin]

    def _eval_hermite_moments(self, x: np.ndarray, t0: float, fc: float, sig: float) -> tuple[complex, complex, complex, complex]:
        """
        Convolve o sinal demodulado com as 4 primeiras janelas Hermite-Gaussianas:
        g0(u) = 1/(sqrt(2pi)*sig) * exp(-u^2 / (2*sig^2))
        g1(u) = -(u / sig^2) * g0(u)
        g2(u) = (u^2 / sig^4 - 1 / sig^2) * g0(u)
        g3(u) = (-u^3 / sig^6 + 3u / sig^4) * g0(u)
        """
        n0 = int(round(t0 * self.sr))
        half_w = int(round(5.0 * sig * self.sr))
        start = max(0, n0 - half_w)
        end = min(len(x), n0 + half_w + 1)
        seg = x[start:end]

        t_abs = np.arange(start, end) / self.sr
        u = (np.arange(start, end) - n0) / self.sr

        # Janelas gaussianas analíticas
        g0 = np.exp(-0.5 * (u / sig)**2) / (math.sqrt(2.0 * math.pi) * sig)
        g1 = -(u / (sig**2)) * g0
        g2 = ((u**2) / (sig**4) - (1.0 / (sig**2))) * g0
        g3 = (-(u**3) / (sig**6) + (3.0 * u) / (sig**4)) * g0

        # Demodulação em banda-base na frequência central fc
        carrier = np.exp(-2j * math.pi * fc * t_abs)
        s = seg * carrier

        # Convolução discreta (com alternância de sinais para derivadas de convolução)
        w0 = np.sum(s * g0) * self.dt
        w1 = -np.sum(s * g1) * self.dt
        w2 = np.sum(s * g2) * self.dt
        w3 = -np.sum(s * g3) * self.dt

        return w0, w1, w2, w3

    def _newton_euler_log_derivs(self, w_coeffs: list[complex]) -> np.ndarray:
        """
        Calcula as derivadas temporais de log(W) = log(A) + i*phi
        usando a recursão exata de Taylor Newton-Euler.
        """
        order = len(w_coeffs) - 1
        fact = np.array([math.factorial(m) for m in range(order + 1)], dtype=float)
        W_taylor = np.array(w_coeffs, dtype=complex) / fact

        L_taylor = np.zeros(order + 1, dtype=complex)
        L_taylor[0] = np.log(W_taylor[0])
        for m in range(1, order + 1):
            acc = sum((j / m) * L_taylor[j] * W_taylor[m - j] for j in range(1, m))
            L_taylor[m] = (W_taylor[m] - acc) / W_taylor[0]

        l_derivs = L_taylor * fact
        return l_derivs

    def extract_time_jet(self, x: np.ndarray, t0: float, f_hint: float | None = None) -> dict[str, float]:
        """
        Extrai o jato temporal contínuo (f_inst, f_dot, f_ddot, log_mag) no instante t0.
        Utiliza um refinamento de dois passos (Two-Pass Refinement):
        - Passo 1: análise no canal CQT discreto mais próximo (ou pico espectral).
        - Passo 2: refinamento centralizado em f_est1 para eliminar viés de off-bin (Delta f -> 0).
        - Inversão quadrática analítica do amortecimento de chirp rate pela gaussiana.
        """
        # Identificação da crista (ridge)
        if f_hint is None:
            # Varredura nos canais CQT em torno de t0 para encontrar o canal com máxima energia
            n0 = int(round(t0 * self.sr))
            half_scan = int(round(0.010 * self.sr))
            start_s = max(0, n0 - half_scan)
            end_s = min(len(x), n0 + half_scan + 1)
            X_fft = np.abs(np.fft.rfft(x[start_s:end_s]))
            freqs_fft = np.fft.rfftfreq(len(x[start_s:end_s]), self.dt)
            best_idx = np.argmax(X_fft[1:]) + 1
            f_hint = freqs_fft[best_idx]

        _, fc1 = self.find_nearest_bin(f_hint)

        # Regra de largura de janela adaptativa para chirps dinâmicos
        sig1 = float(np.clip(15.0 / (2.0 * math.pi * fc1), 0.004, 0.022))
        if fc1 < 100.0:
            sig1 = max(sig1, 1.8 / fc1)

        # --- Passo 1: Avaliação na frequência nominal fc1 ---
        w0_1, w1_1, _, _ = self._eval_hermite_moments(x, t0, fc1, sig1)
        delta_f1 = np.imag(w1_1 / w0_1) / (2.0 * math.pi)
        f_est1 = fc1 + delta_f1

        # --- Passo 2: Refinamento na frequência estimada f_est1 ---
        sig2 = float(np.clip(15.0 / (2.0 * math.pi * f_est1), 0.004, 0.022))
        if f_est1 < 100.0:
            sig2 = max(sig2, 1.8 / f_est1)

        w0_2, w1_2, w2_2, w3_2 = self._eval_hermite_moments(x, t0, f_est1, sig2)
        l_derivs = self._newton_euler_log_derivs([w0_2, w1_2, w2_2, w3_2])
        d_phi = np.imag(l_derivs)
        d_logA = np.real(l_derivs)

        f_final = f_est1 + d_phi[1] / (2.0 * math.pi)

        # Inversão quadrática analítica da dispersão de chirp rate:
        # y_obs = fd_true / (1 + (2*pi*sig2^2 * fd_true)^2)
        y_fd = d_phi[2] / (2.0 * math.pi)
        k_sig = 2.0 * math.pi * (sig2**2)
        disc = 1.0 - 4.0 * ((k_sig * y_fd)**2)
        if disc > 0.0 and abs(y_fd) > 1e-4 and abs(k_sig * y_fd) < 0.49:
            fd_final = (1.0 - math.sqrt(disc)) / (2.0 * (k_sig**2) * y_fd)
        else:
            fd_final = y_fd

        fdd_final = d_phi[3] / (2.0 * math.pi)
        log_mag = math.log(max(abs(w0_2), 1e-12))

        return {
            'fc_bin': fc1,
            'f_inst': float(f_final),
            'f_dot': float(fd_final),
            'f_ddot': float(fdd_final),
            'log_mag': float(log_mag),
            'd_logA': [float(v) for v in d_logA],
            'd_phi': [float(v) for v in d_phi]
        }

# =============================================================================
# 2. SUÍTE DE TESTES ALEATORIZADOS MONTE CARLO (N >= 50)
# =============================================================================

def run_stage1_monte_carlo_tests(
    n_trials: int = 100,
    sr: float = 24000.0,
    duration: float = 1.0,
    seed: int = 42,
    output_dir: Path | None = None
) -> dict:
    """
    Executa a suíte de ensaios Monte Carlo aleatorizados no escopo do Estágio 1.
    """
    if output_dir is None:
        output_dir = Path('documentation/synth_dsl_jets')
    output_dir.mkdir(parents=True, exist_ok=True)

    rng = np.random.default_rng(seed)
    extractor = BasebandGaussianCqtJetExtractor(sr=sr)
    t0 = duration / 2.0
    N = int(sr * duration)
    t = np.arange(N) / sr

    results = []
    errors_cents = []
    errors_fdot = []
    errors_fddot = []
    errors_rel_pitch = []

    print("=========================================================================")
    print("🔬 ESTÁGIO 1: TESTES ALEATORIZADOS MONTE CARLO — CQT 60 BINS/OCT & JETS")
    print("=========================================================================")
    print(f"Número de Ensaios (N): {n_trials}")
    print(f"Taxa de Amostragem (sr): {sr} Hz | Duração: {duration} s | Instante t0: {t0} s")
    print("Faixas Paramétricas:")
    print("  * f0     ∈ [60.0, 4000.0] Hz")
    print("  * A      ∈ [0.10, 1.00]")
    print("  * f_dot  ∈ [-500.0, +500.0] Hz/s")
    print("  * f_ddot ∈ [-200.0, +200.0] Hz/s²")
    print("  * phi0   ∈ [0.0, 2π) rad")
    print("-------------------------------------------------------------------------")

    t_start = time.time()

    for i in range(n_trials):
        f0 = float(rng.uniform(60.0, 4000.0))
        A = float(rng.uniform(0.10, 1.00))
        f_dot = float(rng.uniform(-500.0, 500.0))
        f_ddot = float(rng.uniform(-200.0, 200.0))
        phi0 = float(rng.uniform(0.0, 2.0 * math.pi))

        # Síntese analítica exata da senoide com chirp quadrático
        # phi(t) = phi0 + 2pi * [ f0*(t-t0) + 1/2*f_dot*(t-t0)^2 + 1/6*f_ddot*(t-t0)^3 ]
        phi_t = phi0 + 2.0 * np.pi * (f0 * (t - t0) + 0.5 * f_dot * ((t - t0)**2) + (1.0 / 6.0) * f_ddot * ((t - t0)**3))
        x = A * np.sin(phi_t)

        # Extração do jato espectral pela CQT de banda-base
        meas = extractor.extract_time_jet(x, t0=t0, f_hint=f0)

        f_est = meas['f_inst']
        fd_est = meas['f_dot']
        fdd_est = meas['f_ddot']

        # Métricas de erro
        err_cents = abs(1200.0 * math.log2(max(1e-6, f_est) / f0))
        err_rel_pitch = abs(f_est - f0) / f0
        err_fd = abs(fd_est - f_dot)
        err_fdd = abs(fdd_est - f_ddot)

        errors_cents.append(err_cents)
        errors_rel_pitch.append(err_rel_pitch)
        errors_fdot.append(err_fd)
        errors_fddot.append(err_fdd)

        trial_data = {
            'trial': i + 1,
            'f0_true': f0,
            'f0_est': f_est,
            'err_cents': err_cents,
            'err_rel_pitch': err_rel_pitch,
            'A_true': A,
            'f_dot_true': f_dot,
            'f_dot_est': fd_est,
            'err_fdot': err_fd,
            'f_ddot_true': f_ddot,
            'f_ddot_est': fdd_est,
            'err_fddot': err_fdd,
            'phi0': phi0,
            'fc_bin': meas['fc_bin']
        }
        results.append(trial_data)

        if (i + 1) % 25 == 0 or (i + 1) == n_trials:
            print(f"  [{i + 1:3d}/{n_trials:3d}] f0={f0:7.2f} Hz -> est={f_est:7.2f} Hz | Erro: {err_cents:6.4f} cents | Erro f_dot: {err_fd:5.2f} Hz/s")

    elapsed_s = time.time() - t_start

    # Sumário Estatístico
    arr_cents = np.array(errors_cents)
    arr_fd = np.array(errors_fdot)
    arr_fdd = np.array(errors_fddot)
    arr_rel = np.array(errors_rel_pitch)

    rmse_cents = float(np.sqrt(np.mean(arr_cents**2)))
    mae_cents = float(np.mean(arr_cents))
    max_cents = float(np.max(arr_cents))
    p95_cents = float(np.percentile(arr_cents, 95))
    median_cents = float(np.median(arr_cents))

    rmse_fdot = float(np.sqrt(np.mean(arr_fd**2)))
    mae_fdot = float(np.mean(arr_fd))
    rmse_fddot = float(np.sqrt(np.mean(arr_fdd**2)))
    mae_fddot = float(np.mean(arr_fdd))

    success_rate_cents = float(np.mean(arr_cents < 2.0) * 100.0)
    success_rate_rel = float(np.mean(arr_rel < 0.001) * 100.0)

    is_approved = (rmse_cents < 2.0) and (success_rate_cents >= 95.0)

    summary = {
        'n_trials': n_trials,
        'elapsed_seconds': elapsed_s,
        'pitch_rmse_cents': rmse_cents,
        'pitch_mae_cents': mae_cents,
        'pitch_max_cents': max_cents,
        'pitch_p95_cents': p95_cents,
        'pitch_median_cents': median_cents,
        'success_rate_under_2cents_pct': success_rate_cents,
        'success_rate_under_0_1pct_rel': success_rate_rel,
        'f_dot_rmse_hz_s': rmse_fdot,
        'f_dot_mae_hz_s': mae_fdot,
        'f_ddot_rmse_hz_s2': rmse_fddot,
        'f_ddot_mae_hz_s2': mae_fddot,
        'approved': is_approved
    }

    print("-------------------------------------------------------------------------")
    print("📈 SUMÁRIO ESTATÍSTICO DO ESTÁGIO 1 (VERIFICAÇÃO MONTE CARLO):")
    print(f"  -> Pitch RMSE:             {rmse_cents:8.4f} cents  (Critério: < 2.0 cents)")
    print(f"  -> Pitch MAE:              {mae_cents:8.4f} cents")
    print(f"  -> Pitch Mediana:          {median_cents:8.4f} cents")
    print(f"  -> Pitch 95º Percentil:    {p95_cents:8.4f} cents")
    print(f"  -> Pitch Máximo:           {max_cents:8.4f} cents")
    print(f"  -> Taxa de Sucesso (< 2c): {success_rate_cents:8.2f}%    (Meta: >= 95.0%)")
    print(f"  -> Taxa de Sucesso (<0.1%):{success_rate_rel:8.2f}%")
    print(f"  -> Chirp Rate f_dot MAE:   {mae_fdot:8.2f} Hz/s")
    print(f"  -> Chirp Accel f_ddot MAE: {mae_fddot:8.2f} Hz/s²")
    print(f"  -> Status de Aprovação:    {'✅ APROVADO' if is_approved else '❌ REPROVADO'}")
    print("-------------------------------------------------------------------------")

    # 1. Salvar JSON com Métricas
    json_path = output_dir / 'stage1_metrics.json'
    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump({'summary': summary, 'trials': results}, f, indent=2)
    print(f"Métricas salvas em: {json_path}")

    # 2. Salvar CSV com Ensaios
    csv_path = output_dir / 'stage1_metrics.csv'
    with open(csv_path, 'w', encoding='utf-8') as f:
        f.write("trial,f0_true,f0_est,err_cents,err_rel_pitch,A_true,f_dot_true,f_dot_est,err_fdot,f_ddot_true,f_ddot_est,err_fddot,phi0,fc_bin\n")
        for r in results:
            f.write(f"{r['trial']},{r['f0_true']:.6f},{r['f0_est']:.6f},{r['err_cents']:.6f},{r['err_rel_pitch']:.8f},"
                    f"{r['A_true']:.6f},{r['f_dot_true']:.6f},{r['f_dot_est']:.6f},{r['err_fdot']:.6f},"
                    f"{r['f_ddot_true']:.6f},{r['f_ddot_est']:.6f},{r['err_fddot']:.6f},{r['phi0']:.6f},{r['fc_bin']:.6f}\n")
    print(f"Tabela CSV salva em: {csv_path}")

    # 3. Gerar Gráficos de Dispersão e Resíduos
    plot_path = output_dir / '08_stage1_randomized_errors.png'
    _generate_stage1_plots(results, summary, plot_path)
    print(f"Gráfico de dispersão salvo em: {plot_path}")

    return summary

def _generate_stage1_plots(results: list[dict], summary: dict, output_path: Path):
    """Gera visualização de auditoria com 4 subplots de diagnóstico."""
    f0_vals = np.array([r['f0_true'] for r in results])
    cents_errs = np.array([r['err_cents'] for r in results])
    fdot_vals = np.array([r['f_dot_true'] for r in results])
    fdot_errs = np.array([r['err_fdot'] for r in results])
    fddot_vals = np.array([r['f_ddot_true'] for r in results])
    fddot_errs = np.array([r['err_fddot'] for r in results])
    amps = np.array([r['A_true'] for r in results])

    fig, axes = plt.subplots(2, 2, figsize=(14, 10))
    fig.suptitle(f"Estágio 1: Auditoria Analítica Monte Carlo (CQT 60 bins/oct) — N={len(results)}", fontsize=15, fontweight='bold')

    # Subplot 1: Dispersão do Erro de Pitch (cents) vs Frequência Fundamental
    ax1 = axes[0, 0]
    sc1 = ax1.scatter(f0_vals, cents_errs, c=amps, cmap='viridis', edgecolors='k', alpha=0.85, s=45)
    ax1.axhline(2.0, color='crimson', linestyle='--', linewidth=1.5, label='Limiar de Aprovação (2 cents)')
    ax1.axhline(summary['pitch_rmse_cents'], color='dodgerblue', linestyle='-', linewidth=1.5, label=f"RMSE = {summary['pitch_rmse_cents']:.3f} cents")
    ax1.set_xscale('log')
    ax1.set_xlabel("Frequência Fundamental f0 (Hz) [escala log]")
    ax1.set_ylabel("Erro Absoluto de Pitch (cents)")
    ax1.set_title("Erro de Afinação vs Frequência Fundamental")
    ax1.grid(True, which='both', alpha=0.3)
    ax1.legend(loc='upper right')
    cbar1 = plt.colorbar(sc1, ax=ax1)
    cbar1.set_label("Amplitude A")

    # Subplot 2: Dispersão do Erro de Chirp Rate (Hz/s) vs Chirp Rate Verdadeiro
    ax2 = axes[0, 1]
    ax2.scatter(fdot_vals, fdot_errs, color='darkorange', edgecolors='k', alpha=0.85, s=45)
    ax2.axhline(summary['f_dot_mae_hz_s'], color='red', linestyle='--', label=f"MAE = {summary['f_dot_mae_hz_s']:.2f} Hz/s")
    ax2.set_xlabel("Taxa de Chirp Verdadeira f_dot (Hz/s)")
    ax2.set_ylabel("Erro Absoluto em f_dot (Hz/s)")
    ax2.set_title("Recuperação do Chirp Linear: Erro em f_dot")
    ax2.grid(True, alpha=0.3)
    ax2.legend(loc='upper right')

    # Subplot 3: Dispersão do Erro de Chirp Accel (Hz/s^2) vs Chirp Accel Verdadeiro
    ax3 = axes[1, 0]
    ax3.scatter(fddot_vals, fddot_errs, color='purple', edgecolors='k', alpha=0.85, s=45)
    ax3.axhline(summary['f_ddot_mae_hz_s2'], color='firebrick', linestyle='--', label=f"MAE = {summary['f_ddot_mae_hz_s2']:.2f} Hz/s²")
    ax3.set_xlabel("Aceleração de Chirp Verdadeira f_ddot (Hz/s²)")
    ax3.set_ylabel("Erro Absoluto em f_ddot (Hz/s²)")
    ax3.set_title("Recuperação do Chirp Quadrático: Erro em f_ddot")
    ax3.grid(True, alpha=0.3)
    ax3.legend(loc='upper right')

    # Subplot 4: Histograma do Erro de Pitch
    ax4 = axes[1, 1]
    ax4.hist(cents_errs, bins=25, color='teal', edgecolor='black', alpha=0.75, density=True)
    ax4.axvline(2.0, color='crimson', linestyle='--', linewidth=2.0, label='Limiar 2 cents')
    ax4.axvline(summary['pitch_p95_cents'], color='gold', linestyle='-', linewidth=2.0, label=f"P95 = {summary['pitch_p95_cents']:.3f} c")
    ax4.set_xlabel("Erro de Pitch (cents)")
    ax4.set_ylabel("Densidade de Probabilidade")
    ax4.set_title(f"Distribuição do Erro de Pitch (Sucesso = {summary['success_rate_under_2cents_pct']:.1f}%)")
    ax4.grid(True, alpha=0.3)
    ax4.legend(loc='upper right')

    plt.tight_layout()
    plt.savefig(output_path, dpi=200)
    plt.close()

if __name__ == '__main__':
    run_stage1_monte_carlo_tests(n_trials=100)
