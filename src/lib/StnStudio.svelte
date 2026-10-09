<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  interface Props {
    originalBytes?: Uint8Array | null;
    onNavigate?: (view: 'converter' | 'editor' | 'synth' | 'stn') => void;
    onTransportToEditor?: (wavBytes: Uint8Array) => void;
  }

  let { 
    originalBytes = null, 
    onNavigate, 
    onTransportToEditor 
  }: Props = $props();

  // --------------------------------------------------------------------------
  // TIPOS E ESTRUTURAS DE DADOS STN (Sines + Transients + Noise)
  // --------------------------------------------------------------------------
  interface SinePartial {
    id: number;
    enabled: boolean;
    f: number;        // Frequência (Hz)
    db: number;       // Amplitude (dB)
    phase: number;    // Fase inicial (rad)
    start: number;    // Início (s)
    end: number;      // Fim (s)
    interp: 'Linear' | 'Cúbica' | 'Hermite' | 'Exponencial';
    fadeInMs: number;
    fadeOutMs: number;
    vibratoHz: number;
    vibRateHz: number;
    inharmonicity: number;
    bandwidthHz: number;
  }

  interface TransientEvent {
    id: number;
    enabled: boolean;
    start: number;     // Início (s)
    dur: number;       // Duração (ms)
    db: number;        // Ganho (dB)
    type: 'pcm' | 'dct' | 'mdct' | 'noise';
    window: 'Hann' | 'Blackman' | 'Retangular' | 'Kaiser';
    decay: 'Exponencial' | 'Duplo exponencial' | 'Impulso curto' | 'Ressonante';
  }

  // --------------------------------------------------------------------------
  // ESTADO REATIVO DO MODELO STN
  // --------------------------------------------------------------------------
  let statusText = $state('PRONTO');
  let isRendering = $state(false);
  let isPlaying = $state(false);
  let currentTime = $state(0.0);
  let duration = $state(5.0);
  let sampleRate = $state(48000);
  let bpm = $state(120);
  let frameMs = $state(10);

  // Navegação do Topbar
  let activeNavTab = $state<'synthesis' | 'analysis' | 'explore' | 'presets' | 'export'>('synthesis');

  // Visão Geral e Ganhos Globais dos Componentes
  let enableS = $state(true);
  let sGain = $state(0.0);

  let enableT = $state(true);
  let tGain = $state(0.0);

  let enableN = $state(true);
  let nGain = $state(-12.0);

  // Camadas e Overlays
  let showTracks = $state(true);
  let showIDs = $state(true);
  let showEvents = $state(true);
  let eventSnap = $state(true);
  let showNoiseEnv = $state(true);
  let showMasks = $state(false);

  // Parciais Senoidais (S)
  let sines = $state<SinePartial[]>([
    { id: 1, enabled: true, f: 110.0, db: -6.2, phase: 0.0, start: 0.0, end: 5.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 2, enabled: true, f: 220.1, db: -9.1, phase: 0.3, start: 0.0, end: 5.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 3, enabled: true, f: 329.8, db: -12.5, phase: 1.1, start: 0.12, end: 4.8, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 4, enabled: true, f: 440.2, db: -15.0, phase: -0.8, start: 0.0, end: 5.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 5, enabled: true, f: 554.3, db: -18.1, phase: 2.4, start: 0.23, end: 4.5, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 6, enabled: true, f: 660.1, db: -20.3, phase: -1.7, start: 0.0, end: 3.8, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 7, enabled: true, f: 880.5, db: -24.0, phase: 0.6, start: 0.5, end: 3.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 }
  ]);
  let selectedSineId = $state(3);
  let sineSubTab = $state<'amplitude' | 'frequency' | 'phase'>('amplitude');

  // Eventos Transitórios (T)
  let events = $state<TransientEvent[]>([
    { id: 1, enabled: true, start: 0.12, dur: 12.5, db: -3.0, type: 'pcm', window: 'Hann', decay: 'Exponencial' },
    { id: 2, enabled: true, start: 0.85, dur: 8.0, db: -6.0, type: 'dct', window: 'Hann', decay: 'Exponencial' },
    { id: 3, enabled: true, start: 1.43, dur: 15.0, db: -1.0, type: 'pcm', window: 'Hann', decay: 'Duplo exponencial' },
    { id: 4, enabled: true, start: 2.10, dur: 9.0, db: -4.5, type: 'dct', window: 'Blackman', decay: 'Impulso curto' }
  ]);
  let selectedEventId = $state(1);
  let eventSubTab = $state<'wave' | 'dct'>('wave');

  // Ruído Estocástico (N)
  let noiseModel = $state<'lpc' | 'bands' | 'points'>('lpc');
  let lpcOrder = $state(24);
  let noiseEnvGain = $state(-12.0);
  let excitation = $state<'Ruído branco' | 'Ruído gaussiano' | 'Ruído colorido'>('Ruído branco');
  let noiseSeed = $state(12345);
  let noiseBands = $state([-4, -2, 0, -3, -8, -12, -18, -24]); // 80Hz, 250Hz, 500Hz, 1k, 2k, 5k, 10k, 16kHz
  let noiseCorr = $state(0.15);
  let noiseUpdateMs = $state(10);
  let noiseSubTab = $state<'envelope' | 'lpc' | 'bands'>('envelope');

  // Saída e Canais
  let masterGain = $state(0.0);
  let normalize = $state(true);
  let peakLimit = $state(-1.0);
  let channelMode = $state<'mono' | 'stereo' | 'multi'>('stereo');
  let channelMap = $state('Pan por componente');

  // Espacial e Binaural
  let itdMs = $state(0.0);
  let ildDb = $state(0.0);
  let pannerMode = $state('Estéreo simples');
  let panS = $state(0.0);
  let panT = $state(0.0);
  let panN = $state(0.0);

  // Reverberação (Convolução)
  let reverbOn = $state(false);
  let irPreset = $state('Sala média');
  let rt60 = $state(1.2);
  let reverbWet = $state(0.25);

  // Visualização e Espectrograma
  let activeCenterTab = $state<'spec' | 'components' | 'tracks' | 'masks' | 'wave' | 'real'>('spec');
  let viewMode = $state<'reassignment' | 'stft' | 'components' | 'phase'>('reassignment');
  let freqScale = $state<'log' | 'linear'>('log');
  let magScale = $state<'db' | 'linear' | 'power'>('db');
  let fMin = $state(50);
  let fMax = $state(20000);
  let dynamicRangeDb = $state(120);
  let palette = $state<'inferno' | 'magma' | 'viridis' | 'mono'>('inferno');

  // Parâmetros STFT
  let windowType = $state<'Hann' | 'Hamming' | 'Blackman' | 'Retangular'>('Hann');
  let fftSize = $state(4096);
  let hopSize = $state(1024);
  let reassignmentType = $state<'reassign_all' | 'freq_only' | 'off'>('reassign_all');
  let energyThresholdDb = $state(-65);
  let phaseLock = $state(true);
  let antiAlias = $state(true);

  // Estado do Renderizador
  let frameCount = $state(0);
  let resolutionStr = $state('11.7 Hz / 21.33 ms');
  let peakReadout = $state('-1.0 dBFS');
  let computeTimeMs = $state(12);

  // Áudio e Canvases
  let specCanvas = $state<HTMLCanvasElement | null>(null);
  let overviewCanvas = $state<HTMLCanvasElement | null>(null);
  let ampChartCanvas = $state<HTMLCanvasElement | null>(null);
  let eventWaveCanvas = $state<HTMLCanvasElement | null>(null);
  let noiseChartCanvas = $state<HTMLCanvasElement | null>(null);

  let cursorInfo = $state('Passe o ponteiro sobre o espectrograma');
  let audioCtx: AudioContext | null = null;
  let audioSource: AudioBufferSourceNode | null = null;
  let audioBufferL: Float32Array | null = null;
  let audioBufferR: Float32Array | null = null;
  let audioStartTime = 0;
  let animTimer: number | null = null;

  // Parcial e Evento selecionados derivados
  let selectedSine = $derived(sines.find(s => s.id === selectedSineId) || sines[0]);
  let selectedEvent = $derived(events.find(e => e.id === selectedEventId) || events[0]);

  // --------------------------------------------------------------------------
  // SÍNTESE E GERAÇÃO DE SINAL DE ÁUDIO
  // --------------------------------------------------------------------------
  function dbToAmp(db: number): number {
    return Math.pow(10, db / 20.0);
  }

  function synthesizeStnAudio() {
    const fs = sampleRate;
    const totalSamples = Math.floor(fs * duration);
    const L = new Float32Array(totalSamples);
    const R = new Float32Array(totalSamples);

    // 1. Senoides (S)
    if (enableS) {
      const sMasterGain = dbToAmp(sGain);
      const panGl = Math.sqrt((1.0 - panS) * 0.5);
      const panGr = Math.sqrt((1.0 + panS) * 0.5);

      for (const s of sines) {
        if (!s.enabled) continue;
        const partGain = dbToAmp(s.db) * sMasterGain;
        const startIdx = Math.max(0, Math.floor(s.start * fs));
        const endIdx = Math.min(totalSamples, Math.floor(s.end * fs));
        const fadeInSamples = Math.max(1, Math.floor((s.fadeInMs / 1000.0) * fs));
        const fadeOutSamples = Math.max(1, Math.floor((s.fadeOutMs / 1000.0) * fs));

        for (let i = startIdx; i < endIdx; i++) {
          const t = (i - startIdx) / fs;
          const remaining = (endIdx - i) / fs;
          const elapsed = (i - startIdx) / fs;

          // Envelope trapezoidal com fades
          let env = 1.0;
          if (i - startIdx < fadeInSamples) {
            env = Math.min(env, (i - startIdx) / fadeInSamples);
          }
          if (endIdx - i < fadeOutSamples) {
            env = Math.min(env, (endIdx - i) / fadeOutSamples);
          }

          // Inarmonicidade e modulação de vibrato
          const fInst = s.f * (1.0 + s.inharmonicity * s.id * s.id * 0.001) + 
                        s.vibratoHz * Math.sin(2.0 * Math.PI * s.vibRateHz * t);
          const phase = 2.0 * Math.PI * fInst * t + s.phase;
          const val = Math.sin(phase) * partGain * env;

          L[i] += val * panGl;
          R[i] += val * panGr;
        }
      }
    }

    // 2. Transientes (T)
    if (enableT) {
      const tMasterGain = dbToAmp(tGain);
      const panGl = Math.sqrt((1.0 - panT) * 0.5);
      const panGr = Math.sqrt((1.0 + panT) * 0.5);

      for (const e of events) {
        if (!e.enabled) continue;
        const evGain = dbToAmp(e.db) * tMasterGain;
        const startIdx = Math.max(0, Math.floor(e.start * fs));
        const durSamples = Math.min(Math.floor((e.dur / 1000.0) * fs), totalSamples - startIdx);

        for (let j = 0; j < durSamples; j++) {
          const env = Math.exp(-j / Math.max(1, durSamples * 0.18));
          const tone = Math.sin(2.0 * Math.PI * (1200 + 400 * Math.sin(j * 0.012)) * (j / fs));
          let pulse = tone * env;

          if (e.decay === 'Impulso curto') pulse *= Math.exp(-j / 8);
          if (e.decay === 'Duplo exponencial') pulse = (Math.exp(-j / 40) - Math.exp(-j / 8)) * tone * 1.5;

          const val = pulse * evGain;
          L[startIdx + j] += val * panGl;
          R[startIdx + j] += val * panGr;
        }
      }
    }

    // 3. Ruído Estocástico (N)
    if (enableN) {
      const nMasterGain = dbToAmp(nGain + noiseEnvGain);
      const panGl = Math.sqrt((1.0 - panN) * 0.5);
      const panGr = Math.sqrt((1.0 + panN) * 0.5);

      let seed = noiseSeed;
      const rnd = () => {
        seed = (seed * 1664525 + 1013904223) >>> 0;
        return (seed / 4294967296.0) * 2.0 - 1.0;
      };

      const bandFreqs = [80, 250, 500, 1000, 2000, 5000, 10000, 16000];
      const bandGains = noiseBands.map(b => dbToAmp(b));
      const lpCoeffs = bandFreqs.map(f => Math.exp(-2.0 * Math.PI * Math.min(f, fs * 0.48) / fs));
      const bandStates = new Float64Array(bandFreqs.length);

      for (let i = 0; i < totalSamples; i++) {
        const white = rnd();
        let noiseSample = 0.0;
        let prevBand = 0.0;

        for (let b = 0; b < bandFreqs.length; b++) {
          const a = lpCoeffs[b];
          bandStates[b] = (1.0 - a) * white + a * bandStates[b];
          const bandSignal = bandStates[b] - prevBand;
          prevBand = bandStates[b];
          noiseSample += bandSignal * bandGains[b];
        }

        const val = noiseSample * nMasterGain * 0.35;
        L[i] += val * panGl;
        R[i] += val * panGr;
      }
    }

    // 4. ITD e ILD (Binaural)
    const itdSamples = Math.round((itdMs / 1000.0) * fs);
    const ildFactor = dbToAmp(ildDb * 0.5);

    if (itdSamples > 0) {
      for (let i = totalSamples - 1; i >= itdSamples; i--) R[i] = R[i - itdSamples];
      R.fill(0, 0, Math.min(itdSamples, totalSamples));
    } else if (itdSamples < 0) {
      const delay = -itdSamples;
      for (let i = totalSamples - 1; i >= delay; i--) L[i] = L[i - delay];
      L.fill(0, 0, Math.min(delay, totalSamples));
    }

    for (let i = 0; i < totalSamples; i++) {
      L[i] *= ildFactor * dbToAmp(masterGain);
      R[i] *= (1.0 / ildFactor) * dbToAmp(masterGain);
    }

    // 5. Normalização de Pico
    let peak = 1e-9;
    for (let i = 0; i < totalSamples; i++) {
      peak = Math.max(peak, Math.abs(L[i]), Math.abs(R[i]));
    }
    peakReadout = `${(20.0 * Math.log10(peak)).toFixed(1)} dBFS`;

    if (normalize && peak > 0.0) {
      const targetAmp = dbToAmp(peakLimit);
      const normGain = targetAmp / peak;
      for (let i = 0; i < totalSamples; i++) {
        L[i] = Math.max(-1.0, Math.min(1.0, L[i] * normGain));
        R[i] = Math.max(-1.0, Math.min(1.0, R[i] * normGain));
      }
    }

    audioBufferL = L;
    audioBufferR = R;
    frameCount = Math.ceil(totalSamples / hopSize);
    resolutionStr = `${(fs / fftSize).toFixed(1)} Hz / ${((hopSize / fs) * 1000).toFixed(2)} ms`;
  }

  // --------------------------------------------------------------------------
  // RENDERIZAÇÃO DO ESPECTROGRAMA E GRÁFICOS
  // --------------------------------------------------------------------------
  function drawSpectrogram() {
    if (!specCanvas || !audioBufferL) return;
    const ctx = specCanvas.getContext('2d');
    if (!ctx) return;

    const w = specCanvas.width;
    const h = specCanvas.height;
    ctx.fillStyle = '#02060b';
    ctx.fillRect(0, 0, w, h);

    const left = 54;
    const right = 20;
    const top = 18;
    const bottom = 32;
    const gw = w - left - right;
    const gh = h - top - bottom;

    // Fundo do gráfico
    ctx.fillStyle = '#050c14';
    ctx.fillRect(left, top, gw, gh);

    // Eixos e ticks
    ctx.strokeStyle = '#1b3248';
    ctx.lineWidth = 1;
    ctx.strokeRect(left, top, gw, gh);

    const tickFreqs = [50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
    const mapY = (f: number) => {
      if (freqScale === 'log') {
        const minL = Math.log(fMin);
        const maxL = Math.log(fMax);
        const norm = (Math.log(Math.max(fMin, Math.min(fMax, f))) - minL) / (maxL - minL);
        return top + gh * (1.0 - norm);
      }
      return top + gh * (1.0 - (f - fMin) / (fMax - fMin));
    };

    ctx.font = '10px Inter, system-ui, sans-serif';
    ctx.fillStyle = '#8ca5ba';
    ctx.textAlign = 'right';

    for (const f of tickFreqs) {
      if (f < fMin || f > fMax) continue;
      const y = mapY(f);
      ctx.beginPath();
      ctx.moveTo(left, y);
      ctx.lineTo(left + gw, y);
      ctx.strokeStyle = '#0e2235';
      ctx.stroke();

      const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
      ctx.fillText(label, left - 6, y + 3);
    }

    // Ticks de tempo
    ctx.textAlign = 'center';
    for (let t = 0; t <= duration; t += duration / 10) {
      const x = left + (t / duration) * gw;
      ctx.beginPath();
      ctx.moveTo(x, top);
      ctx.lineTo(x, top + gh);
      ctx.strokeStyle = '#0e2235';
      ctx.stroke();
      ctx.fillText(t.toFixed(1), x, top + gh + 15);
    }

    ctx.fillStyle = '#e5f1fb';
    ctx.fillText('Tempo (s)', left + gw / 2, h - 6);

    ctx.save();
    ctx.translate(14, top + gh / 2);
    ctx.rotate(-Math.PI / 2);
    ctx.fillText('Frequência (Hz)', 0, 0);
    ctx.restore();

    // Renderiza dados simulados ou calculados do espectrograma S/T/N
    ctx.save();
    ctx.beginPath();
    ctx.rect(left, top, gw, gh);
    ctx.clip();

    // 1. Simulação visual de espectro reassigned rico conforme o paleta
    const cols = 220;
    const rows = 120;
    for (let ci = 0; ci < cols; ci++) {
      const tNorm = ci / cols;
      const tSec = tNorm * duration;
      const px = left + tNorm * gw;

      for (const s of sines) {
        if (!s.enabled || tSec < s.start || tSec > s.end) continue;
        const fInst = s.f * (1.0 + s.inharmonicity * s.id * s.id * 0.001) + 
                      s.vibratoHz * Math.sin(2.0 * Math.PI * s.vibRateHz * tSec);
        const py = mapY(fInst);

        // Ponto de reatribuição concentrado
        const u = Math.min(1.0, Math.max(0.0, (s.db + 60) / 60));
        ctx.fillStyle = palette === 'inferno' 
          ? `hsl(${280 - u * 280}, 95%, ${15 + u * 65}%)` 
          : `hsl(${200 + u * 60}, 90%, ${20 + u * 60}%)`;

        ctx.fillRect(px, py - 1.5, Math.max(2, gw / cols), 3);
      }

      // Eventos de transientes
      for (const e of events) {
        if (!e.enabled) continue;
        const durSec = e.dur / 1000.0;
        if (tSec >= e.start && tSec <= e.start + durSec) {
          ctx.fillStyle = 'rgba(255, 83, 103, 0.45)';
          ctx.fillRect(px, top, 2, gh);
        }
      }
    }

    // Trajetórias senoidais sobrepostas
    if (showTracks) {
      const trackColors = ['#56c1ff', '#62e6cc', '#ffbf64', '#e58cff', '#96ee75', '#38bdf8', '#fbbf24'];
      for (let si = 0; si < sines.length; si++) {
        const s = sines[si];
        if (!s.enabled) continue;
        ctx.strokeStyle = trackColors[si % trackColors.length];
        ctx.lineWidth = s.id === selectedSineId ? 2.5 : 1.2;

        ctx.beginPath();
        let started = false;
        for (let step = 0; step <= 100; step++) {
          const t = s.start + (step / 100) * (s.end - s.start);
          const f = s.f * (1.0 + s.inharmonicity * s.id * s.id * 0.001) + 
                    s.vibratoHz * Math.sin(2.0 * Math.PI * s.vibRateHz * t);
          const px = left + (t / duration) * gw;
          const py = mapY(f);

          if (!started) {
            ctx.moveTo(px, py);
            started = true;
          } else {
            ctx.lineTo(px, py);
          }
        }
        ctx.stroke();

        if (showIDs) {
          const px = left + (s.start / duration) * gw + 4;
          const py = mapY(s.f) - 6;
          ctx.fillStyle = trackColors[si % trackColors.length];
          ctx.font = 'bold 9px Inter, monospace';
          ctx.fillText(`ID ${s.id}`, px, py);
        }
      }
    }

    // Marcadores de transientes
    if (showEvents) {
      for (const e of events) {
        if (!e.enabled) continue;
        const px = left + (e.start / duration) * gw;
        ctx.strokeStyle = '#ff5367';
        ctx.lineWidth = 1.2;
        ctx.setLineDash([4, 3]);
        ctx.beginPath();
        ctx.moveTo(px, top);
        ctx.lineTo(px, top + gh);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#ff5367';
        ctx.font = 'bold 9px Inter, monospace';
        ctx.fillText(`T${e.id}`, px + 3, top + 12);
      }
    }

    // Playhead indicador
    if (isPlaying || currentTime > 0) {
      const playX = left + (currentTime / duration) * gw;
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(playX, top);
      ctx.lineTo(playX, top + gh);
      ctx.stroke();
    }

    ctx.restore();
  }

  function drawOverviewMinimap() {
    if (!overviewCanvas) return;
    const ctx = overviewCanvas.getContext('2d');
    if (!ctx) return;
    const w = overviewCanvas.width;
    const h = overviewCanvas.height;

    ctx.fillStyle = '#050c14';
    ctx.fillRect(0, 0, w, h);

    // Miniatura de espectrograma
    ctx.strokeStyle = '#203a50';
    ctx.strokeRect(0, 0, w, h);

    for (const s of sines) {
      if (!s.enabled) continue;
      const x1 = (s.start / duration) * w;
      const x2 = (s.end / duration) * w;
      const y = h * (1.0 - (s.f / fMax));
      ctx.fillStyle = '#35a8ff';
      ctx.fillRect(x1, y - 1, x2 - x1, 2);
    }

    // Frustum box de visualização
    ctx.strokeStyle = '#e5f1fb';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(4, 2, w - 8, h - 4);
  }

  function drawSineEnvelopeChart() {
    if (!ampChartCanvas) return;
    const ctx = ampChartCanvas.getContext('2d');
    if (!ctx) return;
    const w = ampChartCanvas.width;
    const h = ampChartCanvas.height;

    ctx.fillStyle = '#06111d';
    ctx.fillRect(0, 0, w, h);

    ctx.strokeStyle = '#162b3d';
    for (let i = 0; i < 4; i++) {
      const y = (i / 4) * h;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    const s = selectedSine;
    if (!s) return;

    ctx.strokeStyle = '#35a8ff';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const fadeInRatio = (s.fadeInMs / 1000.0) / (s.end - s.start);
    const fadeOutRatio = (s.fadeOutMs / 1000.0) / (s.end - s.start);
    const x0 = (s.start / duration) * w;
    const x1 = (s.end / duration) * w;
    const baseH = h - 16;
    const ampY = baseH * (1.0 - dbToAmp(s.db));

    ctx.moveTo(x0, baseH);
    ctx.lineTo(x0 + (x1 - x0) * fadeInRatio, ampY);
    ctx.lineTo(x1 - (x1 - x0) * fadeOutRatio, ampY);
    ctx.lineTo(x1, baseH);
    ctx.stroke();

    // Pontos de controle
    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc(x0 + (x1 - x0) * fadeInRatio, ampY, 3.5, 0, Math.PI * 2);
    ctx.arc(x1 - (x1 - x0) * fadeOutRatio, ampY, 3.5, 0, Math.PI * 2);
    ctx.fill();
  }

  function drawTransientWaveformChart() {
    if (!eventWaveCanvas) return;
    const ctx = eventWaveCanvas.getContext('2d');
    if (!ctx) return;
    const w = eventWaveCanvas.width;
    const h = eventWaveCanvas.height;

    ctx.fillStyle = '#050d18';
    ctx.fillRect(0, 0, w, h);

    ctx.strokeStyle = '#18334b';
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    const e = selectedEvent;
    if (!e) return;

    ctx.strokeStyle = '#ff5367';
    ctx.lineWidth = 1.5;
    ctx.beginPath();

    for (let x = 0; x < w; x++) {
      const t = x / w;
      const env = Math.exp(-t * 9.0);
      const osc = Math.sin(t * 80.0 * Math.PI) * 0.65 + Math.sin(t * 180.0 * Math.PI) * 0.35;
      const y = h / 2 - osc * env * (h * 0.42);

      if (x === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();
  }

  function drawNoiseSpectralChart() {
    if (!noiseChartCanvas) return;
    const ctx = noiseChartCanvas.getContext('2d');
    if (!ctx) return;
    const w = noiseChartCanvas.width;
    const h = noiseChartCanvas.height;

    ctx.fillStyle = '#041013';
    ctx.fillRect(0, 0, w, h);

    ctx.strokeStyle = '#153835';
    for (let i = 0; i < 4; i++) {
      const y = (i / 4) * h;
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();
    }

    const minDb = -48;
    const maxDb = 12;
    const vals = noiseBands.map(b => b + noiseEnvGain);

    ctx.strokeStyle = '#22d69a';
    ctx.lineWidth = 2;
    ctx.beginPath();

    vals.forEach((v, i) => {
      const x = 12 + (i / (vals.length - 1)) * (w - 24);
      const y = h - 10 - ((v - minDb) / (maxDb - minDb)) * (h - 20);
      if (i === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    });
    ctx.stroke();

    vals.forEach((v, i) => {
      const x = 12 + (i / (vals.length - 1)) * (w - 24);
      const y = h - 10 - ((v - minDb) / (maxDb - minDb)) * (h - 20);
      ctx.fillStyle = '#22d69a';
      ctx.beginPath();
      ctx.arc(x, y, 3.5, 0, Math.PI * 2);
      ctx.fill();
    });
  }

  function renderAll() {
    isRendering = true;
    statusText = 'RENDERIZANDO';
    const t0 = performance.now();

    try {
      synthesizeStnAudio();
      drawSpectrogram();
      drawOverviewMinimap();
      drawSineEnvelopeChart();
      drawTransientWaveformChart();
      drawNoiseSpectralChart();
      computeTimeMs = Math.round(performance.now() - t0);
      statusText = 'PRONTO';
    } catch (err) {
      console.error('Erro na renderização STN:', err);
      statusText = 'ERRO';
    } finally {
      isRendering = false;
    }
  }

  // --------------------------------------------------------------------------
  // TRANSPORTE E REPRODUÇÃO WEB AUDIO API
  // --------------------------------------------------------------------------
  async function togglePlay() {
    if (isPlaying) {
      stopPlayback();
      return;
    }

    if (!audioBufferL || !audioBufferR) {
      renderAll();
    }

    try {
      const AC = window.AudioContext || (window as any).webkitAudioContext;
      if (!audioCtx) audioCtx = new AC({ sampleRate });
      if (audioCtx.state === 'suspended') await audioCtx.resume();

      const buffer = audioCtx.createBuffer(2, audioBufferL!.length, sampleRate);
      buffer.getChannelData(0).set(audioBufferL!);
      buffer.getChannelData(1).set(audioBufferR!);

      audioSource = audioCtx.createBufferSource();
      audioSource.buffer = buffer;
      audioSource.connect(audioCtx.destination);

      audioSource.onended = () => {
        stopPlayback();
      };

      audioStartTime = audioCtx.currentTime - currentTime;
      audioSource.start(0, currentTime);
      isPlaying = true;
      statusText = 'REPRODUZINDO';

      const updateProgress = () => {
        if (!isPlaying || !audioCtx) return;
        currentTime = Math.min(duration, audioCtx.currentTime - audioStartTime);
        drawSpectrogram();
        if (currentTime < duration) {
          animTimer = requestAnimationFrame(updateProgress);
        } else {
          stopPlayback();
        }
      };
      animTimer = requestAnimationFrame(updateProgress);
    } catch (err) {
      console.error('Erro ao reproduzir áudio:', err);
      stopPlayback();
    }
  }

  function stopPlayback() {
    isPlaying = false;
    statusText = 'PRONTO';
    if (animTimer) cancelAnimationFrame(animTimer);
    if (audioSource) {
      try { audioSource.stop(); } catch {}
      audioSource.disconnect();
      audioSource = null;
    }
    drawSpectrogram();
  }

  // --------------------------------------------------------------------------
  // EXPORTAÇÃO WAV E TRANSPORTE PARA WEBGL EDITOR
  // --------------------------------------------------------------------------
  function generateWavBytes(): Uint8Array {
    if (!audioBufferL || !audioBufferR) {
      synthesizeStnAudio();
    }
    const L = audioBufferL!;
    const R = audioBufferR!;
    const n = L.length;
    const wavBytes = new Uint8Array(44 + n * 4);
    const view = new DataView(wavBytes.buffer);

    const writeStr = (offset: number, str: string) => {
      for (let i = 0; i < str.length; i++) {
        view.setUint8(offset + i, str.charCodeAt(i));
      }
    };

    writeStr(0, 'RIFF');
    view.setUint32(4, 36 + n * 4, true);
    writeStr(8, 'WAVE');
    writeStr(12, 'fmt ');
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true); // PCM
    view.setUint16(22, 2, true); // 2 canais
    view.setUint32(24, sampleRate, true);
    view.setUint32(28, sampleRate * 4, true);
    view.setUint16(32, 4, true);
    view.setUint16(34, 16, true); // 16-bit
    writeStr(36, 'data');
    view.setUint32(40, n * 4, true);

    let offset = 44;
    for (let i = 0; i < n; i++) {
      const sL = Math.max(-1.0, Math.min(1.0, L[i]));
      const sR = Math.max(-1.0, Math.min(1.0, R[i]));
      view.setInt16(offset, sL < 0 ? sL * 32768 : sL * 32767, true);
      offset += 2;
      view.setInt16(offset, sR < 0 ? sR * 32768 : sR * 32767, true);
      offset += 2;
    }

    return wavBytes;
  }

  function exportWavFile() {
    const bytes = generateWavBytes();
    const blob = new Blob([bytes as any], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'stn-synth-render.wav';
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1500);
  }

  function transportToEditor() {
    const bytes = generateWavBytes();
    if (onTransportToEditor) {
      onTransportToEditor(bytes);
      if (onNavigate) onNavigate('editor');
    }
  }

  // Presets curados
  function applyPreset(name: string) {
    if (name === 'voice') {
      sines = [
        { id: 1, enabled: true, f: 145, db: -6, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 10, fadeOutMs: 20, vibratoHz: 0.8, vibRateHz: 5.5, inharmonicity: 0.0, bandwidthHz: 0 },
        { id: 2, enabled: true, f: 290, db: -11, phase: 0.2, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 10, fadeOutMs: 20, vibratoHz: 1.6, vibRateHz: 5.5, inharmonicity: 0.0, bandwidthHz: 0 },
        { id: 3, enabled: true, f: 435, db: -15, phase: 1.0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 10, fadeOutMs: 20, vibratoHz: 2.4, vibRateHz: 5.5, inharmonicity: 0.0, bandwidthHz: 0 },
        { id: 4, enabled: true, f: 580, db: -19, phase: -0.4, start: 0.1, end: duration - 0.2, interp: 'Cúbica', fadeInMs: 10, fadeOutMs: 20, vibratoHz: 3.2, vibRateHz: 5.5, inharmonicity: 0.0, bandwidthHz: 0 }
      ];
      events = [
        { id: 1, enabled: true, start: 0.22, dur: 25, db: -9, type: 'noise', window: 'Hann', decay: 'Exponencial' }
      ];
      nGain = -18;
      noiseBands = [-12, -8, -6, -3, -1, -4, -12, -24];
    } else if (name === 'pluck') {
      sines = [
        { id: 1, enabled: true, f: 220, db: -4, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 2, fadeOutMs: 300, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0.0012, bandwidthHz: 0 },
        { id: 2, enabled: true, f: 441, db: -9, phase: 0.4, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 2, fadeOutMs: 250, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0.0012, bandwidthHz: 0 },
        { id: 3, enabled: true, f: 663, db: -13, phase: 1.2, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 2, fadeOutMs: 200, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0.0012, bandwidthHz: 0 }
      ];
      events = [
        { id: 1, enabled: true, start: 0.02, dur: 45, db: 0, type: 'pcm', window: 'Hann', decay: 'Impulso curto' }
      ];
      nGain = -32;
    } else if (name === 'drums') {
      sines = [];
      events = [
        { id: 1, enabled: true, start: 0.1, dur: 60, db: 0, type: 'pcm', window: 'Hann', decay: 'Exponencial' },
        { id: 2, enabled: true, start: 0.6, dur: 35, db: -3, type: 'pcm', window: 'Hann', decay: 'Impulso curto' },
        { id: 3, enabled: true, start: 1.1, dur: 90, db: -5, type: 'noise', window: 'Hann', decay: 'Exponencial' }
      ];
      nGain = -9;
      noiseBands = [0, -3, -6, -8, -12, -18, -24, -30];
    } else if (name === 'tone') {
      sines = [
        { id: 1, enabled: true, f: 440, db: -3, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0, bandwidthHz: 0 },
        { id: 2, enabled: true, f: 880, db: -9, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0, bandwidthHz: 0 },
        { id: 3, enabled: true, f: 1320, db: -15, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0, bandwidthHz: 0 }
      ];
      events = [];
      nGain = -45;
    }
    selectedSineId = sines[0]?.id || 1;
    selectedEventId = events[0]?.id || 1;
    renderAll();
  }

  function handleMouseMoveSpec(e: MouseEvent) {
    if (!specCanvas) return;
    const rect = specCanvas.getBoundingClientRect();
    const left = 54;
    const right = 20;
    const top = 18;
    const bottom = 32;
    const gw = specCanvas.width - left - right;
    const gh = specCanvas.height - top - bottom;

    const xRatio = (e.clientX - rect.left - (left / specCanvas.width) * rect.width) / ((gw / specCanvas.width) * rect.width);
    const yRatio = (e.clientY - rect.top - (top / specCanvas.height) * rect.height) / ((gh / specCanvas.height) * rect.height);

    if (xRatio >= 0 && xRatio <= 1 && yRatio >= 0 && yRatio <= 1) {
      const t = xRatio * duration;
      const f = freqScale === 'log'
        ? fMin * Math.pow(fMax / fMin, 1.0 - yRatio)
        : fMin + (1.0 - yRatio) * (fMax - fMin);
      cursorInfo = `t = ${t.toFixed(3)} s · f ≈ ${f >= 1000 ? (f / 1000).toFixed(2) + ' kHz' : f.toFixed(1) + ' Hz'}`;
    }
  }

  onMount(() => {
    renderAll();
  });

  onDestroy(() => {
    stopPlayback();
  });
</script>

<div class="stn-app">
  <!-- Topbar Header -->
  <header class="topbar">
    <div class="brand">
      <div class="logo">∿</div>
      <div>
        <strong>STN Synth</strong>
        <small>Sines + Transients + Noise</small>
      </div>
    </div>

    <nav class="nav">
      <button class:active={activeNavTab === 'synthesis'} onclick={() => activeNavTab = 'synthesis'}>Síntese</button>
      <button class:active={activeNavTab === 'analysis'} onclick={() => activeNavTab = 'analysis'}>Análise</button>
      <button class:active={activeNavTab === 'explore'} onclick={() => activeNavTab = 'explore'}>Explorar</button>
      <button class:active={activeNavTab === 'presets'} onclick={() => activeNavTab = 'presets'}>Presets</button>
      <button onclick={transportToEditor} title="Transportar áudio para o WebGL Editor">Editor 2D ↗</button>
    </nav>

    <div class="grow"></div>

    <span class="status-badge" class:busy={isRendering}>{isRendering ? '◌ ' : '● '}{statusText}</span>

    <div class="header-tools">
      <select id="preset-selector" onchange={(e) => applyPreset((e.target as HTMLSelectElement).value)}>
        <option value="custom">Preset atual · Personalizado</option>
        <option value="voice">Voz + fricativas</option>
        <option value="pluck">Cordas percutidas</option>
        <option value="drums">Impactos e ruído</option>
        <option value="tone">Tons harmônicos</option>
      </select>

      <button class="action-btn" onclick={renderAll} title="Renderizar modelo STN">Renderizar</button>
      <button class="export-btn" onclick={exportWavFile} title="Exportar arquivo WAV 16-bit">Exportar WAV</button>
      <button class="icon-tool" onclick={transportToEditor} title="Enviar ao Editor WebGL">⤴</button>
    </div>
  </header>

  <!-- 3-Column Studio Layout -->
  <div class="studio-layout">
    
    <!-- LEFT COLUMN -->
    <aside class="col left-col">
      <!-- Visão Geral -->
      <section class="panel">
        <div class="panel-head">
          <span>Visão geral</span>
          <span class="badge">3 componentes</span>
        </div>
        <div class="panel-body">
          <div class="component-box">
            <div class="comp-title">
              <span class="dot s">S</span>
              <strong>Senoides</strong>
              <input type="checkbox" bind:checked={enableS} onchange={renderAll} />
            </div>
            <div class="field">
              <div class="lbl-row">
                <label for="s-gain-slider">Ganho global</label>
                <output>{sGain.toFixed(1)} dB</output>
              </div>
              <input id="s-gain-slider" type="range" min="-36" max="12" step="0.5" bind:value={sGain} oninput={renderAll} />
            </div>
          </div>

          <div class="component-box">
            <div class="comp-title">
              <span class="dot t">T</span>
              <strong>Transientes</strong>
              <input type="checkbox" bind:checked={enableT} onchange={renderAll} />
            </div>
            <div class="field">
              <div class="lbl-row">
                <label for="t-gain-slider">Ganho global</label>
                <output>{tGain.toFixed(1)} dB</output>
              </div>
              <input id="t-gain-slider" type="range" min="-36" max="12" step="0.5" bind:value={tGain} oninput={renderAll} />
            </div>
          </div>

          <div class="component-box">
            <div class="comp-title">
              <span class="dot n">N</span>
              <strong>Ruído</strong>
              <input type="checkbox" bind:checked={enableN} onchange={renderAll} />
            </div>
            <div class="field">
              <div class="lbl-row">
                <label for="n-gain-slider">Ganho global</label>
                <output>{nGain.toFixed(1)} dB</output>
              </div>
              <input id="n-gain-slider" type="range" min="-60" max="6" step="0.5" bind:value={nGain} oninput={renderAll} />
            </div>
          </div>
        </div>
      </section>

      <!-- Camadas -->
      <section class="panel">
        <div class="panel-head">Camadas</div>
        <div class="panel-body">
          <div class="layer-item">
            <div class="comp-title"><span class="dot s">S</span><strong>Trajetórias senoidais</strong></div>
            <small class="muted-text">IDs, amplitude, fase e frequência</small>
            <label class="check-line"><input type="checkbox" bind:checked={showTracks} onchange={renderAll} /> Exibir trajetórias no espectrograma</label>
            <label class="check-line"><input type="checkbox" bind:checked={showIDs} onchange={renderAll} /> Exibir IDs e pontos de controle</label>
          </div>

          <div class="layer-item">
            <div class="comp-title"><span class="dot t">T</span><strong>Eventos transitórios</strong></div>
            <label class="check-line"><input type="checkbox" bind:checked={showEvents} onchange={renderAll} /> Marcar inícios e durações</label>
            <label class="check-line"><input type="checkbox" bind:checked={eventSnap} onchange={renderAll} /> Alinhar ao sample clock</label>
          </div>

          <div class="layer-item">
            <div class="comp-title"><span class="dot n">N</span><strong>Envelope estocástico</strong></div>
            <label class="check-line"><input type="checkbox" bind:checked={showNoiseEnv} onchange={renderAll} /> Exibir envelope espectral</label>
            <label class="check-line"><input type="checkbox" bind:checked={showMasks} onchange={renderAll} /> Sobrepor máscaras S/T/N</label>
          </div>
        </div>
      </section>

      <!-- Análise e Visualização -->
      <section class="panel">
        <div class="panel-head">Análise e visualização</div>
        <div class="panel-body">
          <div class="field">
            <label for="stn-view-mode">Visualização principal</label>
            <select id="stn-view-mode" bind:value={viewMode} onchange={renderAll}>
              <option value="reassignment">Espectrograma (Reassignment)</option>
              <option value="stft">STFT convencional</option>
              <option value="components">Componentes S/T/N</option>
              <option value="phase">Fase / frequência instantânea</option>
            </select>
          </div>

          <div class="field">
            <label>Escala de frequência</label>
            <div class="pill-set">
              <button class:active={freqScale === 'log'} onclick={() => { freqScale = 'log'; renderAll(); }}>Logarítmica</button>
              <button class:active={freqScale === 'linear'} onclick={() => { freqScale = 'linear'; renderAll(); }}>Linear</button>
            </div>
          </div>

          <div class="field">
            <label for="stn-mag-scale">Escala de magnitude</label>
            <select id="stn-mag-scale" bind:value={magScale} onchange={renderAll}>
              <option value="db">Decibéis (dB)</option>
              <option value="linear">Linear</option>
              <option value="power">Potência</option>
            </select>
          </div>

          <div class="field">
            <div class="lbl-row">
              <label>Faixa de frequência</label>
              <output>{fMin} Hz – {fMax >= 1000 ? (fMax/1000) + ' kHz' : fMax + ' Hz'}</output>
            </div>
            <div class="two-inputs">
              <input type="number" min="1" max="20000" bind:value={fMin} onchange={renderAll} />
              <input type="number" min="100" max="24000" bind:value={fMax} onchange={renderAll} />
            </div>
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-dyn-range">Faixa dinâmica</label>
              <output>{dynamicRangeDb} dB</output>
            </div>
            <input id="stn-dyn-range" type="range" min="40" max="160" step="5" bind:value={dynamicRangeDb} oninput={renderAll} />
          </div>

          <div class="field">
            <label for="stn-pal-select">Paleta</label>
            <select id="stn-pal-select" bind:value={palette} onchange={renderAll}>
              <option value="inferno">Inferno</option>
              <option value="magma">Magma</option>
              <option value="viridis">Viridis</option>
              <option value="mono">Monocromática</option>
            </select>
          </div>
        </div>
      </section>
    </aside>

    <!-- CENTER COLUMN -->
    <main class="col main-col">
      <!-- Upper Main Spectrogram Panel -->
      <section class="panel main-spectrogram-panel">
        <!-- Transport Bar -->
        <div class="main-topbar">
          <button class="transport-btn play" class:active={isPlaying} onclick={togglePlay} title={isPlaying ? 'Pausar' : 'Reproduzir'}>
            {isPlaying ? '⏸' : '▶'}
          </button>
          <button class="transport-btn stop" onclick={stopPlayback} title="Parar">■</button>
          <button class="transport-btn record" title="Gravar">●</button>

          <span class="badge time-badge">
            00:0{currentTime.toFixed(2).padStart(5, '0')} / 00:0{duration.toFixed(2).padStart(5, '0')}
          </span>

          <input 
            type="range" 
            class="timeline-slider" 
            min="0" 
            max={duration} 
            step="0.001" 
            bind:value={currentTime} 
            oninput={() => { stopPlayback(); drawSpectrogram(); }}
          />

          <span class="meta-tag">Fs</span>
          <select class="mini-select" bind:value={sampleRate} onchange={renderAll}>
            <option value={24000}>24 kHz</option>
            <option value={44100}>44.1 kHz</option>
            <option value={48000}>48 kHz</option>
          </select>

          <span class="meta-tag">Duração</span>
          <select class="mini-select" bind:value={duration} onchange={renderAll}>
            <option value={1}>1 s</option>
            <option value={2}>2 s</option>
            <option value={3}>3 s</option>
            <option value={5}>5 s</option>
          </select>

          <span class="meta-tag">BPM {bpm}</span>
          <span class="meta-tag">Frame {frameMs} ms</span>
        </div>

        <!-- Center Tabs -->
        <div class="center-tabs">
          <button class:active={activeCenterTab === 'spec'} onclick={() => { activeCenterTab = 'spec'; drawSpectrogram(); }}>Espectrograma (Reassignment)</button>
          <button class:active={activeCenterTab === 'components'} onclick={() => { activeCenterTab = 'components'; drawSpectrogram(); }}>Componentes</button>
          <button class:active={activeCenterTab === 'tracks'} onclick={() => { activeCenterTab = 'tracks'; drawSpectrogram(); }}>Trajetórias parciais</button>
          <button class:active={activeCenterTab === 'masks'} onclick={() => { activeCenterTab = 'masks'; drawSpectrogram(); }}>Máscaras S/T/N</button>
          <button class:active={activeCenterTab === 'wave'} onclick={() => { activeCenterTab = 'wave'; drawSpectrogram(); }}>Forma de onda</button>
          <button class:active={activeCenterTab === 'real'} onclick={() => { activeCenterTab = 'real'; drawSpectrogram(); }}>Espectro em tempo real</button>
        </div>

        <!-- Canvas Header -->
        <div class="spec-header">
          <strong>Espectrograma com Reassignment</strong>
          <div class="spec-tools">
            <span class="color-legend">
              <span class="legend-bar"></span>
              Magnitude (dB) 0 .. -120
            </span>
            <button class="mini-tool-btn" onclick={renderAll}>Ajustar</button>
            <button class="mini-tool-btn" onclick={renderAll}>PNG</button>
          </div>
        </div>

        <!-- Canvas Container -->
        <div class="canvas-wrap">
          <canvas 
            bind:this={specCanvas} 
            class="spec-canvas" 
            width={1100} 
            height={390}
            onmousemove={handleMouseMoveSpec}
          ></canvas>

          <canvas 
            bind:this={overviewCanvas} 
            class="overview-canvas" 
            width={1100} 
            height={60}
          ></canvas>

          <div class="canvas-footerline">
            <span class="spec-status">Reassignment ligado · STFT sintética exata</span>
            <span class="cursor-info font-mono">{cursorInfo}</span>
          </div>
        </div>
      </section>

      <!-- Lower 3 Panels (Senoides, Transientes, Ruído) -->
      <section class="lower-grid">
        <!-- 1. SENOIDES (S) -->
        <div class="panel lower-card">
          <div class="accent-head s">
            <span>● SENOIDES (S)</span>
            <button class="mini-add-btn" onclick={() => {
              const newId = Math.max(0, ...sines.map(s => s.id)) + 1;
              sines.push({ id: newId, enabled: true, f: 440, db: -18, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0, bandwidthHz: 0 });
              selectedSineId = newId;
              renderAll();
            }}>+ Adicionar parcial</button>
          </div>

          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th></th>
                  <th>ID</th>
                  <th>Frequência (Hz)</th>
                  <th>Amplitude (dB)</th>
                  <th>Fase (rad)</th>
                  <th>Início (s)</th>
                  <th>Fim (s)</th>
                </tr>
              </thead>
              <tbody>
                {#each sines as s}
                  <tr class:selected={s.id === selectedSineId} onclick={() => { selectedSineId = s.id; drawSineEnvelopeChart(); }}>
                    <td><input type="checkbox" bind:checked={s.enabled} onchange={renderAll} /></td>
                    <td><span class="dot s">{s.id}</span></td>
                    <td>{s.f.toFixed(1)}</td>
                    <td>{s.db.toFixed(1)}</td>
                    <td>{s.phase.toFixed(2)}</td>
                    <td>{s.start.toFixed(3)}</td>
                    <td>{s.end.toFixed(3)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          {#if selectedSine}
            <div class="sub-editor">
              <div class="row-between">
                <strong>Parcial selecionada (ID {selectedSine.id})</strong>
                <button class="danger-btn" onclick={() => {
                  sines = sines.filter(s => s.id !== selectedSineId);
                  if (sines.length > 0) selectedSineId = sines[0].id;
                  renderAll();
                }}>Remover</button>
              </div>

              <div class="mini-tabs">
                <button class:active={sineSubTab === 'amplitude'} onclick={() => sineSubTab = 'amplitude'}>Amplitude</button>
                <button class:active={sineSubTab === 'frequency'} onclick={() => sineSubTab = 'frequency'}>Frequência</button>
                <button class:active={sineSubTab === 'phase'} onclick={() => sineSubTab = 'phase'}>Fase</button>
              </div>

              <div class="form-grid">
                <div class="field">
                  <label>Frequência (Hz)</label>
                  <input type="number" min="1" max="24000" bind:value={selectedSine.f} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Amplitude (dB)</label>
                  <input type="number" min="-120" max="12" step="0.1" bind:value={selectedSine.db} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Fase inicial (rad)</label>
                  <input type="number" min="-1000" max="1000" step="0.1" bind:value={selectedSine.phase} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Interpolação</label>
                  <select bind:value={selectedSine.interp} onchange={renderAll}>
                    <option value="Linear">Linear</option>
                    <option value="Cúbica">Cúbica</option>
                    <option value="Hermite">Hermite</option>
                    <option value="Exponencial">Exponencial</option>
                  </select>
                </div>
              </div>

              <div class="form-grid">
                <div class="field">
                  <label>Fade-in (ms)</label>
                  <input type="number" min="0" max="500" bind:value={selectedSine.fadeInMs} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Fade-out (ms)</label>
                  <input type="number" min="0" max="500" bind:value={selectedSine.fadeOutMs} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Inarmonicidade B</label>
                  <input type="number" min="0" max="1" step="0.0001" bind:value={selectedSine.inharmonicity} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Largura de banda</label>
                  <input type="number" min="0" max="500" bind:value={selectedSine.bandwidthHz} onchange={renderAll} />
                </div>
              </div>

              <div class="chart-box">
                <div class="chart-title">Envelope de amplitude · tempo (s)</div>
                <canvas bind:this={ampChartCanvas} width={340} height={85}></canvas>
              </div>
            </div>
          {/if}
        </div>

        <!-- 2. TRANSIENTES (T) -->
        <div class="panel lower-card">
          <div class="accent-head t">
            <span>● TRANSIENTES (T)</span>
            <button class="mini-add-btn" onclick={() => {
              const newId = Math.max(0, ...events.map(e => e.id)) + 1;
              events.push({ id: newId, enabled: true, start: Math.min(duration * 0.8, newId * 0.15), dur: 12.5, db: -6, type: 'pcm', window: 'Hann', decay: 'Exponencial' });
              selectedEventId = newId;
              renderAll();
            }}>+ Adicionar evento</button>
          </div>

          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th></th>
                  <th>ID</th>
                  <th>Início (s)</th>
                  <th>Duração (ms)</th>
                  <th>Tipo</th>
                  <th>Ganho (dB)</th>
                </tr>
              </thead>
              <tbody>
                {#each events as e}
                  <tr class:selected={e.id === selectedEventId} onclick={() => { selectedEventId = e.id; drawTransientWaveformChart(); }}>
                    <td><input type="checkbox" bind:checked={e.enabled} onchange={renderAll} /></td>
                    <td><span class="dot t">{e.id}</span></td>
                    <td>{e.start.toFixed(3)}</td>
                    <td>{e.dur.toFixed(1)}</td>
                    <td>{e.type.toUpperCase()}</td>
                    <td>{e.db.toFixed(1)}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          {#if selectedEvent}
            <div class="sub-editor">
              <div class="row-between">
                <strong>Evento selecionado (ID {selectedEvent.id})</strong>
                <button class="danger-btn" onclick={() => {
                  events = events.filter(e => e.id !== selectedEventId);
                  if (events.length > 0) selectedEventId = events[0].id;
                  renderAll();
                }}>Remover</button>
              </div>

              <div class="mini-tabs">
                <button class:active={eventSubTab === 'wave'} onclick={() => eventSubTab = 'wave'}>Forma de onda</button>
                <button class:active={eventSubTab === 'dct'} onclick={() => eventSubTab = 'dct'}>Espectro (DCT)</button>
              </div>

              <div class="form-grid">
                <div class="field">
                  <label>Início (s)</label>
                  <input type="number" min="0" step="0.001" bind:value={selectedEvent.start} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Duração (ms)</label>
                  <input type="number" min="1" max="500" bind:value={selectedEvent.dur} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Ganho (dB)</label>
                  <input type="number" min="-90" max="12" step="0.5" bind:value={selectedEvent.db} onchange={renderAll} />
                </div>
                <div class="field">
                  <label>Representação</label>
                  <select bind:value={selectedEvent.type} onchange={renderAll}>
                    <option value="pcm">PCM (tempo)</option>
                    <option value="dct">DCT</option>
                    <option value="mdct">MDCT</option>
                    <option value="noise">Excitação curta</option>
                  </select>
                </div>
              </div>

              <div class="form-grid">
                <div class="field">
                  <label>Janela</label>
                  <select bind:value={selectedEvent.window} onchange={renderAll}>
                    <option value="Hann">Hann</option>
                    <option value="Blackman">Blackman</option>
                    <option value="Retangular">Retangular</option>
                    <option value="Kaiser">Kaiser</option>
                  </select>
                </div>
                <div class="field">
                  <label>Modelo de decaimento</label>
                  <select bind:value={selectedEvent.decay} onchange={renderAll}>
                    <option value="Exponencial">Exponencial</option>
                    <option value="Duplo exponencial">Duplo exponencial</option>
                    <option value="Impulso curto">Impulso curto</option>
                    <option value="Ressonante">Ressonante</option>
                  </select>
                </div>
              </div>

              <div class="chart-box">
                <div class="chart-title">Forma do transiente</div>
                <canvas bind:this={eventWaveCanvas} width={340} height={75}></canvas>
              </div>
            </div>
          {/if}
        </div>

        <!-- 3. RUÍDO (N) -->
        <div class="panel lower-card">
          <div class="accent-head n">
            <span>● RUÍDO (N)</span>
            <span class="badge">estado estocástico</span>
          </div>

          <div class="sub-editor">
            <div class="form-grid">
              <div class="field">
                <label>Modelo espectral</label>
                <select bind:value={noiseModel} onchange={renderAll}>
                  <option value="bands">Envelope por bandas</option>
                  <option value="lpc">LPC (Linear Predictive)</option>
                  <option value="points">Pontos espectrais</option>
                </select>
              </div>
              <div class="field">
                <label>Ordem (coeficientes)</label>
                <input type="number" min="2" max="64" bind:value={lpcOrder} onchange={renderAll} />
              </div>
            </div>

            <div class="field">
              <div class="lbl-row">
                <label>Ganho global (dB)</label>
                <output>{noiseEnvGain.toFixed(1)} dB</output>
              </div>
              <input type="range" min="-60" max="12" step="0.5" bind:value={noiseEnvGain} oninput={renderAll} />
            </div>

            <div class="form-grid">
              <div class="field">
                <label>Excitação</label>
                <select bind:value={excitation} onchange={renderAll}>
                  <option value="Ruído branco">Ruído branco</option>
                  <option value="Ruído gaussiano">Ruído gaussiano</option>
                  <option value="Ruído colorido">Ruído colorido</option>
                </select>
              </div>
              <div class="field">
                <label>Semente aleatória</label>
                <div class="input-with-btn">
                  <input type="number" bind:value={noiseSeed} onchange={renderAll} />
                  <button class="shuffle-btn" onclick={() => { noiseSeed = Math.floor(Math.random() * 99999); renderAll(); }}>🔀</button>
                </div>
              </div>
            </div>

            <div class="mini-tabs">
              <button class:active={noiseSubTab === 'envelope'} onclick={() => noiseSubTab = 'envelope'}>Envelope espectral</button>
              <button class:active={noiseSubTab === 'lpc'} onclick={() => noiseSubTab = 'lpc'}>Coeficientes LPC</button>
              <button class:active={noiseSubTab === 'bands'} onclick={() => noiseSubTab = 'bands'}>Bandas</button>
            </div>

            <div class="chart-box">
              <div class="chart-title">Envelope espectral resultante</div>
              <canvas bind:this={noiseChartCanvas} width={340} height={75}></canvas>
            </div>

            <div class="bands-slider-grid">
              {#each [80, 250, 500, 1000, 2000, 5000, 10000, 16000] as bFreq, bi}
                <div class="band-col">
                  <span class="band-hz">{bFreq >= 1000 ? (bFreq / 1000) + 'k' : bFreq}</span>
                  <input 
                    type="range" 
                    min="-48" 
                    max="12" 
                    bind:value={noiseBands[bi]} 
                    oninput={renderAll} 
                  />
                  <span class="band-val">{noiseBands[bi]}</span>
                </div>
              {/each}
            </div>
          </div>
        </div>
      </section>
    </main>

    <!-- RIGHT COLUMN -->
    <aside class="col right-col">
      <!-- Saída -->
      <section class="panel">
        <div class="panel-head">Saída</div>
        <div class="panel-body">
          <div class="field">
            <div class="lbl-row">
              <label for="stn-master-gain">Volume global</label>
              <output>{masterGain.toFixed(1)} dB</output>
            </div>
            <input id="stn-master-gain" type="range" min="-36" max="12" step="0.5" bind:value={masterGain} oninput={renderAll} />
          </div>

          <label class="check-line">
            <input type="checkbox" bind:checked={normalize} onchange={renderAll} />
            Normalização de pico
          </label>

          <div class="field">
            <label for="stn-peak-limit">Limite de pico (dBFS)</label>
            <input id="stn-peak-limit" type="number" min="-12" max="0" step="0.5" bind:value={peakLimit} onchange={renderAll} />
          </div>
        </div>
      </section>

      <!-- Canais -->
      <section class="panel">
        <div class="panel-head">Canais</div>
        <div class="panel-body">
          <div class="field">
            <label>Modo de canais</label>
            <div class="pill-set">
              <button class:active={channelMode === 'mono'} onclick={() => { channelMode = 'mono'; renderAll(); }}>Mono</button>
              <button class:active={channelMode === 'stereo'} onclick={() => { channelMode = 'stereo'; renderAll(); }}>Estéreo</button>
              <button class:active={channelMode === 'multi'} onclick={() => { channelMode = 'multi'; renderAll(); }}>Multicanal</button>
            </div>
          </div>

          <div class="field">
            <label for="stn-chan-map">Mapeamento de canais</label>
            <select id="stn-chan-map" bind:value={channelMap} onchange={renderAll}>
              <option value="Duplicar mono">Duplicar mono</option>
              <option value="Pan por componente">Pan por componente</option>
              <option value="Independente por canal">Independente por canal</option>
            </select>
          </div>
        </div>
      </section>

      <!-- Espacial (Binaural) -->
      <section class="panel">
        <div class="panel-head">Espacial (Binaural)</div>
        <div class="panel-body">
          <div class="field">
            <div class="lbl-row">
              <label for="stn-itd-slider">ITD (ms)</label>
              <output>{itdMs.toFixed(2)} ms</output>
            </div>
            <input id="stn-itd-slider" type="range" min="-1" max="1" step="0.01" bind:value={itdMs} oninput={renderAll} />
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-ild-slider">ILD (dB)</label>
              <output>{ildDb.toFixed(1)} dB</output>
            </div>
            <input id="stn-ild-slider" type="range" min="-24" max="24" step="0.5" bind:value={ildDb} oninput={renderAll} />
          </div>

          <div class="field">
            <label for="stn-panner-mode">Panner</label>
            <select id="stn-panner-mode" bind:value={pannerMode} onchange={renderAll}>
              <option value="Estéreo simples">Estéreo simples</option>
              <option value="Pan por componente">Pan por componente</option>
            </select>
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-pan-s">Pan Senoides</label>
              <output>{panS < 0 ? `L ${Math.round(-panS * 100)}%` : panS > 0 ? `R ${Math.round(panS * 100)}%` : 'Centro'}</output>
            </div>
            <input id="stn-pan-s" type="range" min="-1" max="1" step="0.01" bind:value={panS} oninput={renderAll} />
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-pan-t">Pan Transientes</label>
              <output>{panT < 0 ? `L ${Math.round(-panT * 100)}%` : panT > 0 ? `R ${Math.round(panT * 100)}%` : 'Centro'}</output>
            </div>
            <input id="stn-pan-t" type="range" min="-1" max="1" step="0.01" bind:value={panT} oninput={renderAll} />
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-pan-n">Pan Ruído</label>
              <output>{panN < 0 ? `L ${Math.round(-panN * 100)}%` : panN > 0 ? `R ${Math.round(panN * 100)}%` : 'Centro'}</output>
            </div>
            <input id="stn-pan-n" type="range" min="-1" max="1" step="0.01" bind:value={panN} oninput={renderAll} />
          </div>
        </div>
      </section>

      <!-- Reverberação (Convolução) -->
      <section class="panel">
        <div class="panel-head">
          <span>Reverberação (Convolução)</span>
          <input type="checkbox" bind:checked={reverbOn} onchange={renderAll} />
        </div>
        <div class="panel-body">
          <div class="field">
            <label for="stn-ir-preset">Resposta Impulsiva (IR)</label>
            <select id="stn-ir-preset" bind:value={irPreset} onchange={renderAll}>
              <option value="Pequena sala">Pequena sala</option>
              <option value="Sala média">Sala média</option>
              <option value="Hall">Hall</option>
              <option value="Placa">Placa</option>
            </select>
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-rt60-slider">Tempo (s)</label>
              <output>{rt60.toFixed(1)} s</output>
            </div>
            <input id="stn-rt60-slider" type="range" min="0.1" max="8" step="0.1" bind:value={rt60} oninput={renderAll} />
          </div>

          <div class="field">
            <div class="lbl-row">
              <label for="stn-wet-slider">Dry / Wet</label>
              <output>{reverbWet.toFixed(2)}</output>
            </div>
            <input id="stn-wet-slider" type="range" min="0" max="1" step="0.01" bind:value={reverbWet} oninput={renderAll} />
          </div>
        </div>
      </section>

      <!-- Parâmetros STFT -->
      <section class="panel">
        <div class="panel-head">Parâmetros STFT</div>
        <div class="panel-body">
          <div class="field">
            <label for="stn-win-type">Janela</label>
            <select id="stn-win-type" bind:value={windowType} onchange={renderAll}>
              <option value="Hann">Hann</option>
              <option value="Hamming">Hamming</option>
              <option value="Blackman">Blackman</option>
              <option value="Retangular">Retangular</option>
            </select>
          </div>

          <div class="two-inputs">
            <div class="field">
              <label for="stn-fft-size">Tamanho FFT</label>
              <select id="stn-fft-size" bind:value={fftSize} onchange={renderAll}>
                <option value={512}>512</option>
                <option value={1024}>1024</option>
                <option value={2048}>2048</option>
                <option value={4096}>4096</option>
              </select>
            </div>

            <div class="field">
              <label for="stn-hop-size">Hop size</label>
              <select id="stn-hop-size" bind:value={hopSize} onchange={renderAll}>
                <option value={128}>128</option>
                <option value={256}>256</option>
                <option value={512}>512</option>
                <option value={1024}>1024</option>
              </select>
            </div>
          </div>

          <div class="field">
            <label for="stn-reassign-type">Tipo Reassignment</label>
            <select id="stn-reassign-type" bind:value={reassignmentType} onchange={renderAll}>
              <option value="reassign_all">Tempo + Frequência</option>
              <option value="freq_only">Somente Frequência</option>
              <option value="off">Desativado (STFT convencional)</option>
            </select>
          </div>
        </div>
      </section>

      <!-- Estado do Renderizador -->
      <section class="panel">
        <div class="panel-head">Estado do renderizador</div>
        <div class="panel-body">
          <div class="row-between">
            <span class="small-lbl">Quadros analisados:</span>
            <strong>{frameCount}</strong>
          </div>
          <div class="row-between">
            <span class="small-lbl">Bin / resolução:</span>
            <strong>{resolutionStr}</strong>
          </div>
          <div class="row-between">
            <span class="small-lbl">Pico do sinal:</span>
            <strong>{peakReadout}</strong>
          </div>
          <div class="row-between">
            <span class="small-lbl">Tempo de cálculo:</span>
            <strong>{computeTimeMs} ms</strong>
          </div>
        </div>
      </section>
    </aside>

  </div>
</div>

<style>
  :root {
    --bg: #07111e;
    --panel: #0b1928;
    --panel2: #0e2032;
    --line: #203a50;
    --muted: #8ca5ba;
    --text: #e5f1fb;
    --blue: #35a8ff;
    --red: #ff5367;
    --green: #22d69a;
    --amber: #ffc166;
    --radius: 8px;
  }

  .stn-app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: var(--bg);
    color: var(--text);
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    font-size: 12px;
    line-height: 1.35;
    user-select: none;
  }

  /* Topbar */
  .topbar {
    display: flex;
    align-items: center;
    gap: 16px;
    height: 54px;
    padding: 0 16px;
    background: #081522;
    border-bottom: 1px solid var(--line);
    position: sticky;
    top: 0;
    z-index: 100;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 190px;
  }

  .logo {
    width: 28px;
    height: 28px;
    border: 1px solid #2284c8;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: var(--blue);
    font-weight: 800;
    font-size: 15px;
    background: rgba(34, 132, 200, 0.1);
  }

  .brand strong {
    font-size: 14px;
    letter-spacing: 0.3px;
    color: #fff;
  }

  .brand small {
    display: block;
    color: var(--muted);
    font-size: 10px;
  }

  .nav {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .nav button {
    background: transparent;
    border: 1px solid transparent;
    color: #a9c0d4;
    padding: 6px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
    transition: all 0.15s;
  }

  .nav button:hover {
    color: #fff;
    background: rgba(255, 255, 255, 0.05);
  }

  .nav button.active {
    background: #12314a;
    color: #fff;
    border-color: #214e70;
  }

  .grow {
    flex: 1;
  }

  .status-badge {
    font-size: 10px;
    color: var(--green);
    padding: 3px 8px;
    border: 1px solid #1e664f;
    border-radius: 99px;
    font-weight: 700;
    letter-spacing: 0.03em;
  }

  .status-badge.busy {
    color: var(--amber);
    border-color: #7c5c1e;
  }

  .header-tools {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  select, input[type="text"], input[type="number"] {
    background: #071522;
    border: 1px solid #29445a;
    border-radius: 5px;
    color: var(--text);
    padding: 5px 7px;
    font-size: 11px;
    outline: none;
  }

  select:focus, input:focus {
    border-color: var(--blue);
  }

  input[type="range"] {
    width: 100%;
    accent-color: var(--blue);
  }

  input[type="checkbox"] {
    accent-color: var(--blue);
    cursor: pointer;
  }

  .action-btn {
    background: #133a5b;
    border: 1px solid #285d88;
    color: #fff;
    padding: 6px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-weight: 600;
    font-size: 11px;
  }

  .action-btn:hover {
    background: #194b75;
    border-color: var(--blue);
  }

  .export-btn {
    background: #10263a;
    border: 1px solid #2a465e;
    color: var(--text);
    padding: 6px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 11px;
  }

  .export-btn:hover {
    border-color: var(--blue);
  }

  .icon-tool {
    background: #10263a;
    border: 1px solid #2a465e;
    color: var(--blue);
    padding: 5px 8px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 12px;
  }

  /* 3-Column Studio Layout */
  .studio-layout {
    display: grid;
    grid-template-columns: 220px minmax(460px, 1fr) 250px;
    gap: 8px;
    padding: 8px;
    align-items: start;
    flex: 1;
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  /* Panels */
  .panel {
    background: linear-gradient(180deg, #0d1d2d, #091725);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
  }

  .panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 9px 12px;
    border-bottom: 1px solid #1b3348;
    font-weight: 700;
    font-size: 11px;
    color: #d8e8f6;
  }

  .panel-body {
    padding: 9px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .badge {
    background: #132a3e;
    color: var(--muted);
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 99px;
    font-weight: 500;
  }

  .dot {
    display: inline-grid;
    place-items: center;
    width: 17px;
    height: 17px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 800;
    color: #fff;
  }

  .dot.s { background: var(--blue); }
  .dot.t { background: var(--red); }
  .dot.n { background: var(--green); color: #000; }

  .component-box, .layer-item {
    padding: 6px 0;
    border-bottom: 1px solid #14283b;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .component-box:last-child, .layer-item:last-child {
    border-bottom: none;
  }

  .comp-title {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
  }

  .comp-title strong {
    flex: 1;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .lbl-row {
    display: flex;
    justify-content: space-between;
    color: var(--muted);
    font-size: 10px;
  }

  .muted-text {
    color: var(--muted);
    font-size: 9px;
    margin-left: 23px;
  }

  .check-line {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: #cbd8e4;
    cursor: pointer;
  }

  .pill-set {
    display: flex;
    background: #081726;
    border: 1px solid #203a50;
    border-radius: 5px;
    overflow: hidden;
  }

  .pill-set button {
    flex: 1;
    background: transparent;
    border: none;
    color: #9cb1c4;
    padding: 5px;
    font-size: 10px;
    cursor: pointer;
  }

  .pill-set button.active {
    background: #1b3d5c;
    color: #fff;
    font-weight: 600;
  }

  .two-inputs {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  /* Center Main Spectrogram Panel */
  .main-spectrogram-panel {
    display: flex;
    flex-direction: column;
  }

  .main-topbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 12px;
    background: #081624;
    border-bottom: 1px solid var(--line);
  }

  .transport-btn {
    width: 28px;
    height: 28px;
    border-radius: 5px;
    border: 1px solid #26435c;
    background: #10263a;
    display: grid;
    place-items: center;
    cursor: pointer;
    font-size: 12px;
  }

  .transport-btn.play {
    color: var(--green);
  }

  .transport-btn.play.active {
    background: #1e5a43;
    border-color: var(--green);
  }

  .transport-btn.stop {
    color: var(--muted);
  }

  .transport-btn.record {
    color: var(--red);
  }

  .time-badge {
    font-family: monospace;
    font-size: 11px;
    background: #06121e;
    border: 1px solid #1a354c;
    padding: 4px 8px;
    color: #e5f1fb;
  }

  .timeline-slider {
    flex: 1;
  }

  .meta-tag {
    color: var(--muted);
    font-size: 10px;
  }

  .mini-select {
    padding: 3px 5px;
    font-size: 10px;
  }

  .center-tabs {
    display: flex;
    background: #081624;
    border-bottom: 1px solid var(--line);
    overflow-x: auto;
  }

  .center-tabs button {
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    color: #9cb1c4;
    padding: 7px 12px;
    font-size: 11px;
    cursor: pointer;
    white-space: nowrap;
  }

  .center-tabs button:hover {
    color: #fff;
  }

  .center-tabs button.active {
    color: #fff;
    border-bottom-color: var(--blue);
    background: rgba(53, 168, 255, 0.08);
    font-weight: 600;
  }

  .spec-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 12px;
    background: #091929;
    font-size: 11px;
  }

  .spec-tools {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .color-legend {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 10px;
    color: var(--muted);
  }

  .legend-bar {
    width: 50px;
    height: 8px;
    border-radius: 2px;
    background: linear-gradient(90deg, #000, #ff5367, #ffc166, #fff);
  }

  .mini-tool-btn {
    background: #0e2235;
    border: 1px solid #203c54;
    color: var(--muted);
    padding: 3px 6px;
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
  }

  .mini-tool-btn:hover {
    color: #fff;
    border-color: var(--blue);
  }

  .canvas-wrap {
    display: flex;
    flex-direction: column;
    background: #02060b;
    border-top: 1px solid #14283b;
    position: relative;
  }

  .spec-canvas {
    width: 100%;
    height: auto;
    display: block;
    cursor: crosshair;
  }

  .overview-canvas {
    width: 100%;
    height: 50px;
    display: block;
    border-top: 1px solid #162c40;
    cursor: pointer;
  }

  .canvas-footerline {
    display: flex;
    justify-content: space-between;
    padding: 4px 10px;
    background: #071421;
    border-top: 1px solid #172d42;
    font-size: 10px;
    color: var(--muted);
  }

  .cursor-info {
    color: #35a8ff;
    font-weight: 600;
  }

  /* Lower Grid (S, T, N) */
  .lower-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin-top: 8px;
  }

  .lower-card {
    display: flex;
    flex-direction: column;
  }

  .accent-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 8px 10px;
    border-bottom: 1px solid #1b3348;
    font-size: 11px;
    font-weight: 700;
  }

  .accent-head.s { background: rgba(53, 168, 255, 0.12); color: #83caff; }
  .accent-head.t { background: rgba(255, 83, 103, 0.12); color: #ff8594; }
  .accent-head.n { background: rgba(34, 214, 154, 0.12); color: #64e3b7; }

  .mini-add-btn {
    background: rgba(255, 255, 255, 0.08);
    border: 1px solid rgba(255, 255, 255, 0.15);
    color: #fff;
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .mini-add-btn:hover {
    background: rgba(255, 255, 255, 0.15);
  }

  .table-wrap {
    max-height: 140px;
    overflow-y: auto;
    border-bottom: 1px solid #172d42;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 10px;
  }

  th {
    background: #081827;
    padding: 4px 6px;
    text-align: left;
    color: var(--muted);
    font-weight: 600;
    border-bottom: 1px solid #172d42;
  }

  td {
    padding: 4px 6px;
    border-bottom: 1px solid #0f2233;
    color: #d1e2f1;
  }

  tr:hover {
    background: rgba(53, 168, 255, 0.06);
    cursor: pointer;
  }

  tr.selected {
    background: rgba(53, 168, 255, 0.15);
  }

  .sub-editor {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row-between {
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 10px;
  }

  .danger-btn {
    background: rgba(255, 83, 103, 0.15);
    border: 1px solid rgba(255, 83, 103, 0.4);
    color: var(--red);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 4px;
    cursor: pointer;
  }

  .mini-tabs {
    display: flex;
    gap: 4px;
    border-bottom: 1px solid #1a3248;
    padding-bottom: 4px;
  }

  .mini-tabs button {
    background: transparent;
    border: none;
    color: var(--muted);
    font-size: 10px;
    padding: 2px 6px;
    cursor: pointer;
  }

  .mini-tabs button.active {
    color: #fff;
    background: #143048;
    border-radius: 3px;
  }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .chart-box {
    background: #040d16;
    border: 1px solid #14283b;
    border-radius: 4px;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .chart-title {
    font-size: 9px;
    color: var(--muted);
  }

  .chart-box canvas {
    width: 100%;
    height: 70px;
    display: block;
  }

  .bands-slider-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
    background: #05101a;
    padding: 6px;
    border-radius: 4px;
    border: 1px solid #14283b;
  }

  .band-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
  }

  .band-hz {
    font-size: 8px;
    color: var(--muted);
  }

  .band-col input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    height: 60px;
    width: 14px;
  }

  .band-val {
    font-size: 8px;
    color: #22d69a;
    font-family: monospace;
  }

  .input-with-btn {
    display: flex;
    gap: 4px;
  }

  .shuffle-btn {
    background: #0f2438;
    border: 1px solid #23435e;
    border-radius: 4px;
    padding: 0 6px;
    cursor: pointer;
  }

  .small-lbl {
    color: var(--muted);
    font-size: 10px;
  }

  @media (max-width: 1200px) {
    .studio-layout {
      grid-template-columns: 200px 1fr;
    }
    .right-col {
      grid-column: 1 / -1;
      display: grid;
      grid-template-columns: repeat(3, 1fr);
    }
    .lower-grid {
      grid-template-columns: 1fr;
    }
  }

  @media (max-width: 800px) {
    .studio-layout {
      display: flex;
      flex-direction: column;
    }
    .left-col, .main-col, .right-col {
      width: 100%;
    }
    .lower-grid {
      grid-template-columns: 1fr;
    }
    .topbar {
      flex-wrap: wrap;
      height: auto;
      padding: 8px;
    }
  }
</style>
