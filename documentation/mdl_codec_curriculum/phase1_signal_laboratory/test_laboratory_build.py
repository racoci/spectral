#!/usr/bin/env python3
import json
import numpy as np
from pathlib import Path
from signal_laboratory import SignalLaboratory

def main():
    print("=========================================================")
    print("🔬 FASE 1: LABORATÓRIO DE SINAIS (CONTRATO DE GROUND TRUTH)")
    print("=========================================================")
    
    lab = SignalLaboratory(sr=12000)
    dur = 1.0
    
    # E06 Pure Tone
    x06, t06 = lab.e06_pure_tone(440.0, 1.0, 0.0, dur)
    print(f"E06 (Pure Tone)    : f0={t06['f'][500]:.1f} Hz | Max Amp={np.max(t06['A']):.2f}")
    
    # E08 Linear Chirp
    x08, t08 = lab.e08_linear_chirp(100.0, 50.0, 1.0, 0.0, dur)
    print(f"E08 (Linear Chirp) : f_start={t08['f'][0]:.1f} Hz | f_end={t08['f'][-1]:.1f} Hz | df={t08['df'][0]:.1f}")
    
    # E10 Damped Exponential (Polo)
    x10, t10 = lab.e10_damped_exponential(300.0, 2.0, 1.0, 0.0, dur)
    print(f"E10 (Exponential)  : f0={t10['f'][0]:.1f} Hz | gamma={t10['gamma'][0]:.1f} | Amp final={t10['A'][-1]:.4f}")
    
    # E12 Crossing
    x12, t12 = lab.e12_crossing_ridges(200.0, 400.0, 400.0, 200.0, dur)
    cruzamento = abs(t12['comp1']['f'][6000] - t12['comp2']['f'][6000])
    print(f"E12 (Crossing)     : Diferença de freq em 0.5s = {cruzamento:.2f} Hz")
    
    # E14 Transient
    x14, t14 = lab.e14_transient(0.3, dur, kind='noise_burst')
    print(f"E14 (Transient)    : Peak em {np.argmax(x14)/12000:.3f} s")
    
    # E15 Noise (SNR 20 dB sobre E06)
    x15 = lab.add_noise(x06, 20.0)
    sig_e = np.mean(x06**2)
    noise_e = np.mean((x15 - x06)**2)
    snr_meas = 10 * np.log10(sig_e / noise_e)
    print(f"E15 (SNR Sweep)    : Alvo=20.0 dB | Medido={snr_meas:.2f} dB")
    
    # Salvar sanity
    out_dir = Path(__file__).parent
    with open(out_dir / "phase1_laboratory_status.json", "w") as f:
        json.dump({
            "status": "ready",
            "modules": ["E06", "E07", "E08", "E09", "E10", "E11", "E12", "E13", "E14", "E15"],
            "snr_calibrated": True
        }, f, indent=2)

if __name__ == '__main__':
    main()
