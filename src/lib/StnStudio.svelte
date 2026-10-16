<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { 
    wasm_stn_synthesize_wav,
    wasm_stn_synthesize_pcm,
    wasm_stn_render_spectrogram,
    wasm_stn_get_preset
  } from '../wasm/core_wasm.js';

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
  // MODELO DE DADOS STN (Sines + Transients + Noise)
  // --------------------------------------------------------------------------
  interface SinePartial {
    id: number;
    enabled: boolean;
    f: number;        // Hz
    db: number;       // dB
    phase: number;    // rad
    start: number;    // s
    end: number;      // s
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
    start: number;     // s
    dur: number;       // ms
    db: number;        // dB
    type: 'pcm' | 'dct' | 'mdct' | 'noise';
    window: 'Hann' | 'Blackman' | 'Retangular' | 'Kaiser';
    decay: 'Exponencial' | 'Duplo exponencial' | 'Impulso curto' | 'Ressonante';
  }

  // --------------------------------------------------------------------------
  // ESTADOS GERAIS DO SINTETIZADOR
  // --------------------------------------------------------------------------
  let statusText = $state('PRONTO');
  let isRendering = $state(false);
  let isPlaying = $state(false);
  let currentTime = $state(0.0);
  let duration = $state(2.0);
  let sampleRate = $state(48000);
  let bpm = $state(120);
  let frameMs = $state(10);

  // Topbar Navigation
  let activeNavTab = $state<'synthesis' | 'analysis' | 'explore' | 'presets' | 'export'>('synthesis');

  // Ganhos e Habilitação dos Componentes
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

  // Senoides (S)
  let sines = $state<SinePartial[]>([
    { id: 1, enabled: true, f: 110.0, db: -6.2, phase: 0.0, start: 0.0, end: 2.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 2, enabled: true, f: 220.1, db: -9.1, phase: 0.3, start: 0.0, end: 2.0, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 3, enabled: true, f: 329.8, db: -12.5, phase: 1.1, start: 0.12, end: 1.85, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
    { id: 4, enabled: true, f: 440.2, db: -15.0, phase: -0.8, start: 0.0, end: 1.7, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0.0, vibRateHz: 5.0, inharmonicity: 0.0, bandwidthHz: 0.0 },
  ]);
  let selectedSineId = $state(3);
  let sineSubTab = $state<'amplitude' | 'frequency' | 'phase'>('amplitude');

  // Transientes (T)
  let events = $state<TransientEvent[]>([
    { id: 1, enabled: true, start: 0.12, dur: 12.5, db: -3.0, type: 'pcm', window: 'Hann', decay: 'Exponencial' },
    { id: 2, enabled: true, start: 0.85, dur: 8.0, db: -6.0, type: 'dct', window: 'Hann', decay: 'Exponencial' },
  ]);
  let selectedEventId = $state(1);
  let eventSubTab = $state<'wave' | 'dct'>('wave');

  // Ruído (N)
  let noiseModel = $state<'lpc' | 'bands' | 'points'>('lpc');
  let lpcOrder = $state(24);
  let noiseEnvGain = $state(-12.0);
  let excitation = $state<'Ruído branco' | 'Ruído gaussiano' | 'Ruído colorido'>('Ruído branco');
  let noiseSeed = $state(12345);
  let noiseBands = $state([-4, -2, 0, -3, -8, -12, -18, -24]);
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

  // Reverberação
  let reverbOn = $state(false);
  let irPreset = $state('Sala média');
  let rt60 = $state(1.2);
  let reverbWet = $state(0.25);

  // Visualização e Espectrograma
  let activeCenterTab = $state<'spec' | 'components' | 'tracks' | 'masks' | 'wave' | 'real'>('spec');
  let viewMode = $state<'reassignment' | 'stft' | 'components' | 'phase'>('reassignment');
  let freqScale = $state<'log' | 'linear'>('log');
  let magScale = $state<'db' | 'linear' | 'power'>('db');
  let fMin = $state(40);
  let fMax = $state(20000);
  let dynamicRangeDb = $state(100);
  let palette = $state<'inferno' | 'magma' | 'viridis' | 'mono'>('inferno');

  // Parâmetros STFT
  let windowType = $state<'Hann' | 'Hamming' | 'Blackman' | 'Retangular'>('Hann');
  let fftSize = $state(2048);
  let hopSize = $state(512);
  let energyThresholdDb = $state(-65);
  let phaseLock = $state(true);
  let antiAlias = $state(true);

  // Métricas do Renderizador
  let frameCount = $state(0);
  let resolutionStr = $state('23.4 Hz / 10.67 ms');
  let peakReadout = $state('-1.0 dBFS');
  let computeTimeMs = $state(8);

  // Canvases
  let specCanvas = $state<HTMLCanvasElement | null>(null);
  let overviewCanvas = $state<HTMLCanvasElement | null>(null);
  let ampChartCanvas = $state<HTMLCanvasElement | null>(null);
  let eventWaveCanvas = $state<HTMLCanvasElement | null>(null);
  let noiseChartCanvas = $state<HTMLCanvasElement | null>(null);
  let cursorInfo = $state('Passe o ponteiro sobre o espectrograma');

  // Web Audio Context e Buffers
  let audioCtx: AudioContext | null = null;
  let audioSource: AudioBufferSourceNode | null = null;
  let pcmDataL: Float32Array | null = null;
  let pcmDataR: Float32Array | null = null;
  let audioStartTime = 0;
  let animFrameId: number | null = null;

  // Derivados
  let selectedSine = $derived(sines.find(s => s.id === selectedSineId) || sines[0]);
  let selectedEvent = $derived(events.find(e => e.id === selectedEventId) || events[0]);

  // --------------------------------------------------------------------------
  // SISTEMA DE LAYOUT REORGANIZÁVEL E AJUSTÁVEL (DOCKING / SPLITTERS / DRAG-AND-DROP)
  // --------------------------------------------------------------------------
  let leftColWidth = $state(230);
  let rightColWidth = $state(254);
  let specHeight = $state(380);
  let lowerLayoutMode = $state<'grid3' | 'tabs' | 'stacked'>('grid3');
  let activeLowerTab = $state<'sines' | 'transients' | 'noise'>('sines');

  // Painéis colapsáveis (true = expandido, false = colapsado)
  let panelExpanded = $state<Record<string, boolean>>({
    overview: true,
    layers: true,
    analysis: true,
    sines: true,
    transients: true,
    noise: true,
    output: true,
    channels: true,
    spatial: true,
    reverb: true,
    stft: true,
    status: true,
  });

  // Lista dinâmica de painéis por coluna (permitindo reorganização e reordenação livre)
  let leftPanels = $state<string[]>(['overview', 'layers', 'analysis']);
  let rightPanels = $state<string[]>(['output', 'channels', 'spatial', 'reverb', 'stft', 'status']);
  let lowerPanels = $state<string[]>(['sines', 'transients', 'noise']);

  let draggedPanelId = $state<string | null>(null);
  let dragSourceCol = $state<'left' | 'right' | 'lower' | null>(null);

  function handleDragStart(panelId: string, col: 'left' | 'right' | 'lower') {
    draggedPanelId = panelId;
    dragSourceCol = col;
  }

  function handleDragOver(e: DragEvent) {
    e.preventDefault();
  }

  function handleDrop(targetPanelId: string, targetCol: 'left' | 'right' | 'lower') {
    if (!draggedPanelId || !dragSourceCol) return;

    // Remove do container de origem
    if (dragSourceCol === 'left') {
      leftPanels = leftPanels.filter(p => p !== draggedPanelId);
    } else if (dragSourceCol === 'right') {
      rightPanels = rightPanels.filter(p => p !== draggedPanelId);
    } else if (dragSourceCol === 'lower') {
      lowerPanels = lowerPanels.filter(p => p !== draggedPanelId);
    }

    // Insere no container de destino antes do alvo
    const insertInList = (list: string[], target: string, item: string) => {
      const idx = list.indexOf(target);
      if (idx === -1) {
        return [...list, item];
      }
      const copy = [...list];
      copy.splice(idx, 0, item);
      return copy;
    };

    if (targetCol === 'left') {
      leftPanels = insertInList(leftPanels, targetPanelId, draggedPanelId);
    } else if (targetCol === 'right') {
      rightPanels = insertInList(rightPanels, targetPanelId, draggedPanelId);
    } else if (targetCol === 'lower') {
      lowerPanels = insertInList(lowerPanels, targetPanelId, draggedPanelId);
    }

    draggedPanelId = null;
    dragSourceCol = null;
  }

  // Redimensionamento de colunas por splitter
  let isResizingLeft = false;
  let isResizingRight = false;
  let isResizingSpec = false;

  function startResizeLeft(e: MouseEvent) {
    e.preventDefault();
    isResizingLeft = true;
    window.addEventListener('mousemove', onMouseMoveSplitter);
    window.addEventListener('mouseup', onMouseUpSplitter);
  }

  function startResizeRight(e: MouseEvent) {
    e.preventDefault();
    isResizingRight = true;
    window.addEventListener('mousemove', onMouseMoveSplitter);
    window.addEventListener('mouseup', onMouseUpSplitter);
  }

  function startResizeSpec(e: MouseEvent) {
    e.preventDefault();
    isResizingSpec = true;
    window.addEventListener('mousemove', onMouseMoveSplitter);
    window.addEventListener('mouseup', onMouseUpSplitter);
  }

  function onMouseMoveSplitter(e: MouseEvent) {
    if (isResizingLeft) {
      leftColWidth = Math.max(160, Math.min(480, e.clientX - 10));
    } else if (isResizingRight) {
      rightColWidth = Math.max(180, Math.min(500, window.innerWidth - e.clientX - 10));
    } else if (isResizingSpec) {
      if (specCanvas) {
        const rect = specCanvas.getBoundingClientRect();
        specHeight = Math.max(200, Math.min(800, e.clientY - rect.top));
      }
    }
  }

  function onMouseUpSplitter() {
    if (isResizingLeft || isResizingRight || isResizingSpec) {
      isResizingLeft = false;
      isResizingRight = false;
      isResizingSpec = false;
      window.removeEventListener('mousemove', onMouseMoveSplitter);
      window.removeEventListener('mouseup', onMouseUpSplitter);
      renderAll();
    }
  }

  function resetDefaultLayout() {
    leftColWidth = 230;
    rightColWidth = 254;
    specHeight = 380;
    lowerLayoutMode = 'grid3';
    leftPanels = ['overview', 'layers', 'analysis'];
    rightPanels = ['output', 'channels', 'spatial', 'reverb', 'stft', 'status'];
    lowerPanels = ['sines', 'transients', 'noise'];
    Object.keys(panelExpanded).forEach(k => panelExpanded[k] = true);
    renderAll();
  }

  // --------------------------------------------------------------------------
  // SÍNTESE E RENDERIZAÇÃO REAL VIA WASM
  // --------------------------------------------------------------------------
  function buildStnConfigJson(): string {
    return JSON.stringify({
      sample_rate: sampleRate,
      duration_s: duration,
      enable_s: enableS,
      s_gain_db: sGain,
      enable_t: enableT,
      t_gain_db: tGain,
      enable_n: enableN,
      n_gain_db: nGain,
      master_gain_db: masterGain,
      normalize: normalize,
      peak_limit_db: peakLimit,
      itd_ms: itdMs,
      ild_db: ildDb,
      pan_s: panS,
      pan_t: panT,
      pan_n: panN,
      reverb_enabled: reverbOn,
      reverb_rt60: rt60,
      reverb_wet: reverbWet,
      sines: sines.map(s => ({
        id: s.id,
        enabled: s.enabled,
        f_hz: s.f,
        db: s.db,
        phase_rad: s.phase,
        start_s: s.start,
        end_s: s.end,
        interp: s.interp,
        fade_in_ms: s.fadeInMs,
        fade_out_ms: s.fadeOutMs,
        vibrato_hz: s.vibratoHz,
        vib_rate_hz: s.vibRateHz,
        inharmonicity: s.inharmonicity,
        bandwidth_hz: s.bandwidthHz,
      })),
      events: events.map(e => ({
        id: e.id,
        enabled: e.enabled,
        start_s: e.start,
        dur_ms: e.dur,
        db: e.db,
        type_repr: e.type,
        window: e.window,
        decay: e.decay,
      })),
      noise: {
        model: noiseModel,
        order: lpcOrder,
        env_gain_db: noiseEnvGain,
        excitation: excitation,
        seed: noiseSeed,
        bands_db: noiseBands,
        correlation: noiseCorr,
        update_ms: noiseUpdateMs,
      }
    });
  }

  let renderDebounceTimer: any = null;
  function scheduleRender() {
    if (renderDebounceTimer) clearTimeout(renderDebounceTimer);
    renderDebounceTimer = setTimeout(() => {
      renderAll();
    }, 40);
  }

  function renderAll() {
    isRendering = true;
    statusText = 'RENDERIZANDO';
    const t0 = performance.now();

    try {
      const configJson = buildStnConfigJson();

      // 1. Síntese de áudio PCM de alta performance em WASM
      const interleaved = wasm_stn_synthesize_pcm(configJson);
      const totalPairs = interleaved.length / 2;

      pcmDataL = new Float32Array(totalPairs);
      pcmDataR = new Float32Array(totalPairs);
      let peak = 1e-9;

      for (let i = 0; i < totalPairs; i++) {
        const sl = interleaved[i * 2];
        const sr = interleaved[i * 2 + 1];
        pcmDataL[i] = sl;
        pcmDataR[i] = sr;
        const absVal = Math.max(Math.abs(sl), Math.abs(sr));
        if (absVal > peak) peak = absVal;
      }
      peakReadout = `${(20.0 * Math.log10(peak)).toFixed(1)} dBFS`;

      // 2. Renderização rápida do espectrograma real com reatribuição em WASM
      if (specCanvas) {
        const w = specCanvas.width;
        const h = specCanvas.height;
        const rgbaBytes = wasm_stn_render_spectrogram(
          configJson,
          w,
          h,
          fMin,
          fMax,
          viewMode,
          palette
        );

        const ctx = specCanvas.getContext('2d');
        if (ctx && rgbaBytes.length === w * h * 4) {
          const imgData = ctx.createImageData(w, h);
          imgData.data.set(rgbaBytes);
          ctx.putImageData(imgData, 0, 0);

          // Eixos, grids e overlays sobrepostos
          drawSpectrogramOverlays(ctx, w, h);
        }
      }

      drawOverviewMinimap();
      drawSineEnvelopeChart();
      drawTransientWaveformChart();
      drawNoiseSpectralChart();

      computeTimeMs = Math.round(performance.now() - t0);
      frameCount = Math.ceil(totalPairs / hopSize);
      resolutionStr = `${(sampleRate / fftSize).toFixed(1)} Hz / ${((hopSize / sampleRate) * 1000).toFixed(2)} ms`;
      statusText = 'PRONTO';
    } catch (err) {
      console.error('Erro na síntese WASM do STN:', err);
      statusText = 'ERRO';
    } finally {
      isRendering = false;
    }
  }

  function drawSpectrogramOverlays(ctx: CanvasRenderingContext2D, w: number, h: number) {
    const mapY = (f: number) => {
      if (freqScale === 'log') {
        const minL = Math.log(fMin);
        const maxL = Math.log(fMax);
        const norm = (Math.log(Math.max(fMin, Math.min(fMax, f))) - minL) / (maxL - minL);
        return h * (1.0 - norm);
      }
      return h * (1.0 - (f - fMin) / (fMax - fMin));
    };

    // Ticks de frequência no eixo esquerdo
    ctx.font = '10px Inter, monospace';
    ctx.textAlign = 'left';
    ctx.fillStyle = 'rgba(255, 255, 255, 0.7)';
    const ticks = [50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000];
    for (const f of ticks) {
      if (f < fMin || f > fMax) continue;
      const y = mapY(f);
      ctx.strokeStyle = 'rgba(255, 255, 255, 0.12)';
      ctx.beginPath();
      ctx.moveTo(0, y);
      ctx.lineTo(w, y);
      ctx.stroke();

      const label = f >= 1000 ? `${f / 1000}k` : `${f}`;
      ctx.fillText(label, 6, y - 2);
    }

    // Trajetórias das parciais senoidais
    if (showTracks) {
      const colors = ['#56c1ff', '#62e6cc', '#ffbf64', '#e58cff', '#96ee75', '#38bdf8'];
      for (let si = 0; si < sines.length; si++) {
        const s = sines[si];
        if (!s.enabled) continue;
        ctx.strokeStyle = colors[si % colors.length];
        ctx.lineWidth = s.id === selectedSineId ? 2.5 : 1.2;

        ctx.beginPath();
        let started = false;
        for (let step = 0; step <= 80; step++) {
          const t = s.start + (step / 80) * (s.end - s.start);
          const f = s.f * (1.0 + s.inharmonicity * s.id * s.id * 0.001) + 
                    s.vibratoHz * Math.sin(2.0 * Math.PI * s.vibRateHz * t);
          const px = (t / duration) * w;
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
          ctx.fillStyle = colors[si % colors.length];
          ctx.font = 'bold 9px monospace';
          ctx.fillText(`ID ${s.id}`, (s.start / duration) * w + 4, mapY(s.f) - 4);
        }
      }
    }

    // Marcadores verticais de eventos transitórios
    if (showEvents) {
      for (const e of events) {
        if (!e.enabled) continue;
        const px = (e.start / duration) * w;
        ctx.strokeStyle = '#ff5367';
        ctx.lineWidth = 1.2;
        ctx.setLineDash([4, 3]);
        ctx.beginPath();
        ctx.moveTo(px, 0);
        ctx.lineTo(px, h);
        ctx.stroke();
        ctx.setLineDash([]);

        ctx.fillStyle = '#ff5367';
        ctx.font = 'bold 9px monospace';
        ctx.fillText(`T${e.id}`, px + 3, 14);
      }
    }

    // Playhead indicador
    if (isPlaying || currentTime > 0) {
      const playX = (currentTime / duration) * w;
      ctx.strokeStyle = '#ffffff';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(playX, 0);
      ctx.lineTo(playX, h);
      ctx.stroke();
    }
  }

  function drawOverviewMinimap() {
    if (!overviewCanvas || !pcmDataL) return;
    const ctx = overviewCanvas.getContext('2d');
    if (!ctx) return;
    const w = overviewCanvas.width;
    const h = overviewCanvas.height;

    ctx.fillStyle = '#050c14';
    ctx.fillRect(0, 0, w, h);

    // Miniatura da forma de onda sintetizada
    ctx.strokeStyle = '#203a50';
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    ctx.strokeStyle = '#35a8ff';
    ctx.lineWidth = 1;
    ctx.beginPath();
    const step = Math.max(1, Math.floor(pcmDataL.length / w));
    for (let x = 0; x < w; x++) {
      const idx = x * step;
      const v = pcmDataL[idx] || 0.0;
      const y = h / 2 - v * (h * 0.45);
      if (x === 0) ctx.moveTo(x, y);
      else ctx.lineTo(x, y);
    }
    ctx.stroke();

    // Frustum box de visualização
    ctx.strokeStyle = '#e5f1fb';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(2, 2, w - 4, h - 4);
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

    const fadeInRatio = (s.fadeInMs / 1000.0) / Math.max(0.01, s.end - s.start);
    const fadeOutRatio = (s.fadeOutMs / 1000.0) / Math.max(0.01, s.end - s.start);
    const x0 = (s.start / duration) * w;
    const x1 = (s.end / duration) * w;
    const baseH = h - 14;
    const ampRatio = Math.min(1.0, Math.max(0.0, (s.db + 60.0) / 60.0));
    const ampY = baseH * (1.0 - ampRatio);

    ctx.moveTo(x0, baseH);
    ctx.lineTo(x0 + (x1 - x0) * fadeInRatio, ampY);
    ctx.lineTo(x1 - (x1 - x0) * fadeOutRatio, ampY);
    ctx.lineTo(x1, baseH);
    ctx.stroke();

    ctx.fillStyle = '#ffffff';
    ctx.beginPath();
    ctx.arc(x0 + (x1 - x0) * fadeInRatio, ampY, 3, 0, Math.PI * 2);
    ctx.arc(x1 - (x1 - x0) * fadeOutRatio, ampY, 3, 0, Math.PI * 2);
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
      const norm = x / w;
      const decay = Math.exp(-norm * 7.0);
      const carrier = Math.sin(norm * 30.0 * Math.PI);
      const y = h / 2 - carrier * decay * (h * 0.42);

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
      ctx.arc(x, y, 3, 0, Math.PI * 2);
      ctx.fill();
    });
  }

  // --------------------------------------------------------------------------
  // REPRODUÇÃO WEB AUDIO API
  // --------------------------------------------------------------------------
  async function togglePlay() {
    if (isPlaying) {
      stopPlayback();
      return;
    }

    if (!pcmDataL || !pcmDataR) {
      renderAll();
    }

    try {
      const AC = window.AudioContext || (window as any).webkitAudioContext;
      if (!audioCtx) audioCtx = new AC({ sampleRate });
      if (audioCtx.state === 'suspended') await audioCtx.resume();

      const buffer = audioCtx.createBuffer(2, pcmDataL!.length, sampleRate);
      buffer.getChannelData(0).set(pcmDataL!);
      buffer.getChannelData(1).set(pcmDataR!);

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
        if (specCanvas) {
          const ctx = specCanvas.getContext('2d');
          if (ctx) drawSpectrogramOverlays(ctx, specCanvas.width, specCanvas.height);
        }
        if (currentTime < duration) {
          animFrameId = requestAnimationFrame(updateProgress);
        } else {
          stopPlayback();
        }
      };
      animFrameId = requestAnimationFrame(updateProgress);
    } catch (err) {
      console.error('Erro ao reproduzir áudio:', err);
      stopPlayback();
    }
  }

  function stopPlayback() {
    isPlaying = false;
    statusText = 'PRONTO';
    if (animFrameId) cancelAnimationFrame(animFrameId);
    if (audioSource) {
      try { audioSource.stop(); } catch {}
      audioSource.disconnect();
      audioSource = null;
    }
    if (specCanvas) {
      const ctx = specCanvas.getContext('2d');
      if (ctx) drawSpectrogramOverlays(ctx, specCanvas.width, specCanvas.height);
    }
  }

  // --------------------------------------------------------------------------
  // PRESETS E EXPORTAÇÃO
  // --------------------------------------------------------------------------
  function applyPreset(name: string) {
    try {
      const jsonStr = wasm_stn_get_preset(name);
      const parsed = JSON.parse(jsonStr);

      if (parsed.sines) {
        sines = parsed.sines.map((s: any) => ({
          id: s.id,
          enabled: s.enabled,
          f: s.f_hz,
          db: s.db,
          phase: s.phase_rad,
          start: s.start_s,
          end: s.end_s,
          interp: s.interp,
          fadeInMs: s.fade_in_ms,
          fadeOutMs: s.fade_out_ms,
          vibratoHz: s.vibrato_hz,
          vibRateHz: s.vib_rate_hz,
          inharmonicity: s.inharmonicity,
          bandwidthHz: s.bandwidth_hz,
        }));
      }
      if (parsed.events) {
        events = parsed.events.map((e: any) => ({
          id: e.id,
          enabled: e.enabled,
          start: e.start_s,
          dur: e.dur_ms,
          db: e.db,
          type: e.type_repr,
          window: e.window,
          decay: e.decay,
        }));
      }
      if (parsed.n_gain_db !== undefined) nGain = parsed.n_gain_db;
      if (parsed.noise?.bands_db) noiseBands = [...parsed.noise.bands_db];

      selectedSineId = sines[0]?.id || 1;
      selectedEventId = events[0]?.id || 1;
      renderAll();
    } catch (err) {
      console.warn('Erro ao carregar preset STN:', err);
    }
  }

  function exportWavFile() {
    try {
      const configJson = buildStnConfigJson();
      const wavBytes = wasm_stn_synthesize_wav(configJson);
      const blob = new Blob([wavBytes as any], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = 'stn-synth-render.wav';
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1500);
    } catch (err) {
      console.error('Erro ao exportar WAV:', err);
    }
  }

  function transportToEditor() {
    try {
      const configJson = buildStnConfigJson();
      const wavBytes = wasm_stn_synthesize_wav(configJson);
      if (onTransportToEditor) {
        onTransportToEditor(wavBytes);
        if (onNavigate) onNavigate('editor');
      }
    } catch (err) {
      console.error('Erro ao transportar para o Editor WebGL:', err);
    }
  }

  function handleMouseMoveSpec(e: MouseEvent) {
    if (!specCanvas) return;
    const rect = specCanvas.getBoundingClientRect();
    const xRatio = (e.clientX - rect.left) / rect.width;
    const yRatio = (e.clientY - rect.top) / rect.height;

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
      <button onclick={transportToEditor} title="Transportar áudio sintetizado para o WebGL Editor">Editor 2D ↗</button>
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

      <button class="action-btn" onclick={renderAll} title="Renderizar modelo STN em Rust/WASM">Renderizar</button>
      <button class="export-btn" onclick={exportWavFile} title="Exportar arquivo WAV 16-bit">Exportar WAV</button>
      <button class="layout-reset-btn" onclick={resetDefaultLayout} title="Restaurar layout padrão e posições originais">↺ Layout</button>
    </div>
  </header>

  <!-- 3-Column Studio Layout com Splitters Horizontais Redimensionáveis -->
  <div 
    class="studio-layout" 
    style="grid-template-columns: {leftColWidth}px 6px minmax(400px, 1fr) 6px {rightColWidth}px;"
  >
    <!-- LEFT COLUMN (Reorganizável por Drag-and-Drop) -->
    <aside class="col left-col">
      {#each leftPanels as panelId (panelId)}
        {#if panelId === 'overview'}
          <!-- 1. Visão Geral -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('overview', 'left')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('overview', 'left')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Visão geral</span>
              <span class="badge">3 componentes</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.overview = !panelExpanded.overview}>
                {panelExpanded.overview ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.overview}
              <div class="panel-body">
                <div class="component-box">
                  <div class="comp-title">
                    <span class="dot s">S</span>
                    <strong>Senoides</strong>
                    <input type="checkbox" bind:checked={enableS} onchange={scheduleRender} />
                  </div>
                  <div class="field">
                    <div class="lbl-row">
                      <label for="s-gain-slider">Ganho global</label>
                      <output>{sGain.toFixed(1)} dB</output>
                    </div>
                    <input id="s-gain-slider" type="range" min="-36" max="12" step="0.5" bind:value={sGain} oninput={scheduleRender} />
                  </div>
                </div>

                <div class="component-box">
                  <div class="comp-title">
                    <span class="dot t">T</span>
                    <strong>Transientes</strong>
                    <input type="checkbox" bind:checked={enableT} onchange={scheduleRender} />
                  </div>
                  <div class="field">
                    <div class="lbl-row">
                      <label for="t-gain-slider">Ganho global</label>
                      <output>{tGain.toFixed(1)} dB</output>
                    </div>
                    <input id="t-gain-slider" type="range" min="-36" max="12" step="0.5" bind:value={tGain} oninput={scheduleRender} />
                  </div>
                </div>

                <div class="component-box">
                  <div class="comp-title">
                    <span class="dot n">N</span>
                    <strong>Ruído</strong>
                    <input type="checkbox" bind:checked={enableN} onchange={scheduleRender} />
                  </div>
                  <div class="field">
                    <div class="lbl-row">
                      <label for="n-gain-slider">Ganho global</label>
                      <output>{nGain.toFixed(1)} dB</output>
                    </div>
                    <input id="n-gain-slider" type="range" min="-60" max="6" step="0.5" bind:value={nGain} oninput={scheduleRender} />
                  </div>
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'layers'}
          <!-- 2. Camadas -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('layers', 'left')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('layers', 'left')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Camadas</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.layers = !panelExpanded.layers}>
                {panelExpanded.layers ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.layers}
              <div class="panel-body">
                <div class="layer-item">
                  <div class="comp-title"><span class="dot s">S</span><strong>Trajetórias senoidais</strong></div>
                  <small class="muted-text">IDs, amplitude, fase e frequência</small>
                  <label class="check-line"><input type="checkbox" bind:checked={showTracks} onchange={scheduleRender} /> Exibir trajetórias no espectrograma</label>
                  <label class="check-line"><input type="checkbox" bind:checked={showIDs} onchange={scheduleRender} /> Exibir IDs e pontos de controle</label>
                </div>

                <div class="layer-item">
                  <div class="comp-title"><span class="dot t">T</span><strong>Eventos transitórios</strong></div>
                  <label class="check-line"><input type="checkbox" bind:checked={showEvents} onchange={scheduleRender} /> Marcar inícios e durações</label>
                  <label class="check-line"><input type="checkbox" bind:checked={eventSnap} onchange={scheduleRender} /> Alinhar ao sample clock</label>
                </div>

                <div class="layer-item">
                  <div class="comp-title"><span class="dot n">N</span><strong>Envelope estocástico</strong></div>
                  <label class="check-line"><input type="checkbox" bind:checked={showNoiseEnv} onchange={scheduleRender} /> Exibir envelope espectral</label>
                  <label class="check-line"><input type="checkbox" bind:checked={showMasks} onchange={scheduleRender} /> Sobrepor máscaras S/T/N</label>
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'analysis'}
          <!-- 3. Análise e Visualização -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('analysis', 'left')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('analysis', 'left')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Análise e visualização</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.analysis = !panelExpanded.analysis}>
                {panelExpanded.analysis ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.analysis}
              <div class="panel-body">
                <div class="field">
                  <label for="stn-view-mode">Visualização principal</label>
                  <select id="stn-view-mode" bind:value={viewMode} onchange={scheduleRender}>
                    <option value="reassignment">Espectrograma (Reassignment)</option>
                    <option value="stft">STFT convencional</option>
                    <option value="components">Componentes S/T/N</option>
                    <option value="phase">Fase / frequência instantânea</option>
                  </select>
                </div>

                <div class="field">
                  <label>Escala de frequência</label>
                  <div class="pill-set">
                    <button class:active={freqScale === 'log'} onclick={() => { freqScale = 'log'; scheduleRender(); }}>Logarítmica</button>
                    <button class:active={freqScale === 'linear'} onclick={() => { freqScale = 'linear'; scheduleRender(); }}>Linear</button>
                  </div>
                </div>

                <div class="field">
                  <label for="stn-mag-scale">Escala de magnitude</label>
                  <select id="stn-mag-scale" bind:value={magScale} onchange={scheduleRender}>
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
                    <input type="number" min="1" max="20000" bind:value={fMin} onchange={scheduleRender} />
                    <input type="number" min="100" max="24000" bind:value={fMax} onchange={scheduleRender} />
                  </div>
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-dyn-range">Faixa dinâmica</label>
                    <output>{dynamicRangeDb} dB</output>
                  </div>
                  <input id="stn-dyn-range" type="range" min="40" max="160" step="5" bind:value={dynamicRangeDb} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <label for="stn-pal-select">Paleta</label>
                  <select id="stn-pal-select" bind:value={palette} onchange={scheduleRender}>
                    <option value="inferno">Inferno</option>
                    <option value="magma">Magma</option>
                    <option value="viridis">Viridis</option>
                    <option value="mono">Monocromática</option>
                  </select>
                </div>
              </div>
            {/if}
          </section>
        {/if}
      {/each}
    </aside>

    <!-- Splitter Vertical Esquerdo -->
    <div class="splitter-v" onmousedown={startResizeLeft} title="Arrastar para ajustar largura da coluna esquerda"></div>

    <!-- CENTER COLUMN -->
    <main class="col main-col">
      <!-- Painel Principal de Espectrograma e Minimap -->
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
            oninput={() => { stopPlayback(); renderAll(); }}
          />

          <span class="meta-tag">Fs</span>
          <select class="mini-select" bind:value={sampleRate} onchange={scheduleRender}>
            <option value={24000}>24 kHz</option>
            <option value={44100}>44.1 kHz</option>
            <option value={48000}>48 kHz</option>
          </select>

          <span class="meta-tag">Duração</span>
          <select class="mini-select" bind:value={duration} onchange={scheduleRender}>
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
          <button class:active={activeCenterTab === 'spec'} onclick={() => { activeCenterTab = 'spec'; scheduleRender(); }}>Espectrograma (Reassignment)</button>
          <button class:active={activeCenterTab === 'components'} onclick={() => { activeCenterTab = 'components'; scheduleRender(); }}>Componentes</button>
          <button class:active={activeCenterTab === 'tracks'} onclick={() => { activeCenterTab = 'tracks'; scheduleRender(); }}>Trajetórias parciais</button>
          <button class:active={activeCenterTab === 'masks'} onclick={() => { activeCenterTab = 'masks'; scheduleRender(); }}>Máscaras S/T/N</button>
          <button class:active={activeCenterTab === 'wave'} onclick={() => { activeCenterTab = 'wave'; scheduleRender(); }}>Forma de onda</button>
          <button class:active={activeCenterTab === 'real'} onclick={() => { activeCenterTab = 'real'; scheduleRender(); }}>Espectro em tempo real</button>
        </div>

        <!-- Canvas Header -->
        <div class="spec-header">
          <strong>Espectrograma com Reassignment</strong>
          <div class="spec-tools">
            <span class="color-legend">
              <span class="legend-bar"></span>
              Magnitude (dB) 0 .. -100
            </span>
            <button class="mini-tool-btn" onclick={renderAll}>Ajustar</button>
            <button class="mini-tool-btn" onclick={exportWavFile}>WAV</button>
          </div>
        </div>

        <!-- Canvas Container -->
        <div class="canvas-wrap" style="height: {specHeight}px;">
          <canvas 
            bind:this={specCanvas} 
            class="spec-canvas" 
            width={1100} 
            height={specHeight - 60}
            onmousemove={handleMouseMoveSpec}
          ></canvas>

          <canvas 
            bind:this={overviewCanvas} 
            class="overview-canvas" 
            width={1100} 
            height={48}
          ></canvas>

          <div class="canvas-footerline">
            <span class="spec-status">Reassignment ligado · STFT em Rust/WASM</span>
            <span class="cursor-info font-mono">{cursorInfo}</span>
          </div>
        </div>

        <!-- Splitter Horizontal para Ajustar Altura do Espectrograma -->
        <div class="splitter-h" onmousedown={startResizeSpec} title="Arrastar verticalmente para ajustar a altura do espectrograma">
          <span class="splitter-handle-bar"></span>
        </div>
      </section>

      <!-- Barra de Controle de Layout dos Painéis Inferiores S/T/N -->
      <div class="lower-layout-control">
        <span class="layout-lbl">Layout Inferior:</span>
        <div class="pill-set mini">
          <button class:active={lowerLayoutMode === 'grid3'} onclick={() => lowerLayoutMode = 'grid3'}>⊞ Grade 3x1</button>
          <button class:active={lowerLayoutMode === 'tabs'} onclick={() => lowerLayoutMode = 'tabs'}>📑 Abas</button>
          <button class:active={lowerLayoutMode === 'stacked'} onclick={() => lowerLayoutMode = 'stacked'}>☰ Empilhado</button>
        </div>

        {#if lowerLayoutMode === 'tabs'}
          <div class="lower-tab-selector">
            <button class:active={activeLowerTab === 'sines'} onclick={() => activeLowerTab = 'sines'}>● Senoides (S)</button>
            <button class:active={activeLowerTab === 'transients'} onclick={() => activeLowerTab = 'transients'}>● Transientes (T)</button>
            <button class:active={activeLowerTab === 'noise'} onclick={() => activeLowerTab = 'noise'}>● Ruído (N)</button>
          </div>
        {/if}
      </div>

      <!-- Lower Panels (S, T, N) com Layout Reconfigurável -->
      <section 
        class="lower-container" 
        class:grid-mode={lowerLayoutMode === 'grid3'} 
        class:stacked-mode={lowerLayoutMode === 'stacked'}
      >
        {#each lowerPanels as panelId (panelId)}
          {#if panelId === 'sines' && (lowerLayoutMode !== 'tabs' || activeLowerTab === 'sines')}
            <!-- 1. SENOIDES (S) -->
            <div 
              class="panel lower-card"
              draggable="true" 
              ondragstart={() => handleDragStart('sines', 'lower')} 
              ondragover={handleDragOver} 
              ondrop={() => handleDrop('sines', 'lower')}
            >
              <div class="accent-head s">
                <span class="drag-grip" title="Arrastar para reordenar componente">⠿</span>
                <span>● SENOIDES (S)</span>
                <button class="mini-add-btn" onclick={() => {
                  const newId = Math.max(0, ...sines.map(s => s.id)) + 1;
                  sines.push({ id: newId, enabled: true, f: 440, db: -18, phase: 0, start: 0, end: duration, interp: 'Cúbica', fadeInMs: 5, fadeOutMs: 5, vibratoHz: 0, vibRateHz: 5, inharmonicity: 0, bandwidthHz: 0 });
                  selectedSineId = newId;
                  scheduleRender();
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
                        <td><input type="checkbox" bind:checked={s.enabled} onchange={scheduleRender} /></td>
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
                      scheduleRender();
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
                      <input type="number" min="1" max="24000" bind:value={selectedSine.f} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Amplitude (dB)</label>
                      <input type="number" min="-120" max="12" step="0.1" bind:value={selectedSine.db} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Fase inicial (rad)</label>
                      <input type="number" min="-1000" max="1000" step="0.1" bind:value={selectedSine.phase} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Interpolação</label>
                      <select bind:value={selectedSine.interp} onchange={scheduleRender}>
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
                      <input type="number" min="0" max="500" bind:value={selectedSine.fadeInMs} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Fade-out (ms)</label>
                      <input type="number" min="0" max="500" bind:value={selectedSine.fadeOutMs} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Inarmonicidade B</label>
                      <input type="number" min="0" max="1" step="0.0001" bind:value={selectedSine.inharmonicity} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Largura de banda</label>
                      <input type="number" min="0" max="500" bind:value={selectedSine.bandwidthHz} onchange={scheduleRender} />
                    </div>
                  </div>

                  <div class="chart-box">
                    <div class="chart-title">Envelope de amplitude · tempo (s)</div>
                    <canvas bind:this={ampChartCanvas} width={340} height={70}></canvas>
                  </div>
                </div>
              {/if}
            </div>
          {:else if panelId === 'transients' && (lowerLayoutMode !== 'tabs' || activeLowerTab === 'transients')}
            <!-- 2. TRANSIENTES (T) -->
            <div 
              class="panel lower-card"
              draggable="true" 
              ondragstart={() => handleDragStart('transients', 'lower')} 
              ondragover={handleDragOver} 
              ondrop={() => handleDrop('transients', 'lower')}
            >
              <div class="accent-head t">
                <span class="drag-grip" title="Arrastar para reordenar componente">⠿</span>
                <span>● TRANSIENTES (T)</span>
                <button class="mini-add-btn" onclick={() => {
                  const newId = Math.max(0, ...events.map(e => e.id)) + 1;
                  events.push({ id: newId, enabled: true, start: Math.min(duration * 0.8, newId * 0.15), dur: 12.5, db: -6, type: 'pcm', window: 'Hann', decay: 'Exponencial' });
                  selectedEventId = newId;
                  scheduleRender();
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
                        <td><input type="checkbox" bind:checked={e.enabled} onchange={scheduleRender} /></td>
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
                      scheduleRender();
                    }}>Remover</button>
                  </div>

                  <div class="mini-tabs">
                    <button class:active={eventSubTab === 'wave'} onclick={() => eventSubTab = 'wave'}>Forma de onda</button>
                    <button class:active={eventSubTab === 'dct'} onclick={() => eventSubTab = 'dct'}>Espectro (DCT)</button>
                  </div>

                  <div class="form-grid">
                    <div class="field">
                      <label>Início (s)</label>
                      <input type="number" min="0" step="0.001" bind:value={selectedEvent.start} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Duração (ms)</label>
                      <input type="number" min="1" max="500" bind:value={selectedEvent.dur} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Ganho (dB)</label>
                      <input type="number" min="-90" max="12" step="0.5" bind:value={selectedEvent.db} onchange={scheduleRender} />
                    </div>
                    <div class="field">
                      <label>Representação</label>
                      <select bind:value={selectedEvent.type} onchange={scheduleRender}>
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
                      <select bind:value={selectedEvent.window} onchange={scheduleRender}>
                        <option value="Hann">Hann</option>
                        <option value="Blackman">Blackman</option>
                        <option value="Retangular">Retangular</option>
                        <option value="Kaiser">Kaiser</option>
                      </select>
                    </div>
                    <div class="field">
                      <label>Modelo de decaimento</label>
                      <select bind:value={selectedEvent.decay} onchange={scheduleRender}>
                        <option value="Exponencial">Exponencial</option>
                        <option value="Duplo exponencial">Duplo exponencial</option>
                        <option value="Impulso curto">Impulso curto</option>
                        <option value="Ressonante">Ressonante</option>
                      </select>
                    </div>
                  </div>

                  <div class="chart-box">
                    <div class="chart-title">Forma do transiente</div>
                    <canvas bind:this={eventWaveCanvas} width={340} height={70}></canvas>
                  </div>
                </div>
              {/if}
            </div>
          {:else if panelId === 'noise' && (lowerLayoutMode !== 'tabs' || activeLowerTab === 'noise')}
            <!-- 3. RUÍDO (N) -->
            <div 
              class="panel lower-card"
              draggable="true" 
              ondragstart={() => handleDragStart('noise', 'lower')} 
              ondragover={handleDragOver} 
              ondrop={() => handleDrop('noise', 'lower')}
            >
              <div class="accent-head n">
                <span class="drag-grip" title="Arrastar para reordenar componente">⠿</span>
                <span>● RUÍDO (N)</span>
                <span class="badge">estado estocástico</span>
              </div>

              <div class="sub-editor">
                <div class="form-grid">
                  <div class="field">
                    <label>Modelo espectral</label>
                    <select bind:value={noiseModel} onchange={scheduleRender}>
                      <option value="bands">Envelope por bandas</option>
                      <option value="lpc">LPC (Linear Predictive)</option>
                      <option value="points">Pontos espectrais</option>
                    </select>
                  </div>
                  <div class="field">
                    <label>Ordem (coeficientes)</label>
                    <input type="number" min="2" max="64" bind:value={lpcOrder} onchange={scheduleRender} />
                  </div>
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label>Ganho global (dB)</label>
                    <output>{noiseEnvGain.toFixed(1)} dB</output>
                  </div>
                  <input type="range" min="-60" max="12" step="0.5" bind:value={noiseEnvGain} oninput={scheduleRender} />
                </div>

                <div class="form-grid">
                  <div class="field">
                    <label>Excitação</label>
                    <select bind:value={excitation} onchange={scheduleRender}>
                      <option value="Ruído branco">Ruído branco</option>
                      <option value="Ruído gaussiano">Ruído gaussiano</option>
                      <option value="Ruído colorido">Ruído colorido</option>
                    </select>
                  </div>
                  <div class="field">
                    <label>Semente aleatória</label>
                    <div class="input-with-btn">
                      <input type="number" bind:value={noiseSeed} onchange={scheduleRender} />
                      <button class="shuffle-btn" onclick={() => { noiseSeed = Math.floor(Math.random() * 99999); scheduleRender(); }}>🔀</button>
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
                  <canvas bind:this={noiseChartCanvas} width={340} height={70}></canvas>
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
                        oninput={scheduleRender} 
                      />
                      <span class="band-val">{noiseBands[bi]}</span>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          {/if}
        {/each}
      </section>
    </main>

    <!-- Splitter Vertical Direito -->
    <div class="splitter-v" onmousedown={startResizeRight} title="Arrastar para ajustar largura da coluna direita"></div>

    <!-- RIGHT COLUMN (Reorganizável por Drag-and-Drop) -->
    <aside class="col right-col">
      {#each rightPanels as panelId (panelId)}
        {#if panelId === 'output'}
          <!-- 1. Saída -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('output', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('output', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Saída</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.output = !panelExpanded.output}>
                {panelExpanded.output ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.output}
              <div class="panel-body">
                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-master-gain">Volume global</label>
                    <output>{masterGain.toFixed(1)} dB</output>
                  </div>
                  <input id="stn-master-gain" type="range" min="-36" max="12" step="0.5" bind:value={masterGain} oninput={scheduleRender} />
                </div>

                <label class="check-line">
                  <input type="checkbox" bind:checked={normalize} onchange={scheduleRender} />
                  Normalização de pico
                </label>

                <div class="field">
                  <label for="stn-peak-limit">Limite de pico (dBFS)</label>
                  <input id="stn-peak-limit" type="number" min="-12" max="0" step="0.5" bind:value={peakLimit} onchange={scheduleRender} />
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'channels'}
          <!-- 2. Canais -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('channels', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('channels', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Canais</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.channels = !panelExpanded.channels}>
                {panelExpanded.channels ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.channels}
              <div class="panel-body">
                <div class="field">
                  <label>Modo de canais</label>
                  <div class="pill-set">
                    <button class:active={channelMode === 'mono'} onclick={() => { channelMode = 'mono'; scheduleRender(); }}>Mono</button>
                    <button class:active={channelMode === 'stereo'} onclick={() => { channelMode = 'stereo'; scheduleRender(); }}>Estéreo</button>
                    <button class:active={channelMode === 'multi'} onclick={() => { channelMode = 'multi'; scheduleRender(); }}>Multicanal</button>
                  </div>
                </div>

                <div class="field">
                  <label for="stn-chan-map">Mapeamento de canais</label>
                  <select id="stn-chan-map" bind:value={channelMap} onchange={scheduleRender}>
                    <option value="Duplicar mono">Duplicar mono</option>
                    <option value="Pan por componente">Pan por componente</option>
                    <option value="Independente por canal">Independente por canal</option>
                  </select>
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'spatial'}
          <!-- 3. Espacial (Binaural) -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('spatial', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('spatial', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Espacial (Binaural)</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.spatial = !panelExpanded.spatial}>
                {panelExpanded.spatial ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.spatial}
              <div class="panel-body">
                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-itd-slider">ITD (ms)</label>
                    <output>{itdMs.toFixed(2)} ms</output>
                  </div>
                  <input id="stn-itd-slider" type="range" min="-1" max="1" step="0.01" bind:value={itdMs} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-ild-slider">ILD (dB)</label>
                    <output>{ildDb.toFixed(1)} dB</output>
                  </div>
                  <input id="stn-ild-slider" type="range" min="-24" max="24" step="0.5" bind:value={ildDb} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <label for="stn-panner-mode">Panner</label>
                  <select id="stn-panner-mode" bind:value={pannerMode} onchange={scheduleRender}>
                    <option value="Estéreo simples">Estéreo simples</option>
                    <option value="Pan por componente">Pan por componente</option>
                  </select>
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-pan-s">Pan Senoides</label>
                    <output>{panS < 0 ? `L ${Math.round(-panS * 100)}%` : panS > 0 ? `R ${Math.round(panS * 100)}%` : 'Centro'}</output>
                  </div>
                  <input id="stn-pan-s" type="range" min="-1" max="1" step="0.01" bind:value={panS} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-pan-t">Pan Transientes</label>
                    <output>{panT < 0 ? `L ${Math.round(-panT * 100)}%` : panT > 0 ? `R ${Math.round(panT * 100)}%` : 'Centro'}</output>
                  </div>
                  <input id="stn-pan-t" type="range" min="-1" max="1" step="0.01" bind:value={panT} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-pan-n">Pan Ruído</label>
                    <output>{panN < 0 ? `L ${Math.round(-panN * 100)}%` : panN > 0 ? `R ${Math.round(panN * 100)}%` : 'Centro'}</output>
                  </div>
                  <input id="stn-pan-n" type="range" min="-1" max="1" step="0.01" bind:value={panN} oninput={scheduleRender} />
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'reverb'}
          <!-- 4. Reverberação (Convolução) -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('reverb', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('reverb', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Reverberação (Convolução)</span>
              <input type="checkbox" bind:checked={reverbOn} onchange={scheduleRender} />
              <button class="collapse-toggle" onclick={() => panelExpanded.reverb = !panelExpanded.reverb}>
                {panelExpanded.reverb ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.reverb}
              <div class="panel-body">
                <div class="field">
                  <label for="stn-ir-preset">Resposta Impulsiva (IR)</label>
                  <select id="stn-ir-preset" bind:value={irPreset} onchange={scheduleRender}>
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
                  <input id="stn-rt60-slider" type="range" min="0.1" max="8" step="0.1" bind:value={rt60} oninput={scheduleRender} />
                </div>

                <div class="field">
                  <div class="lbl-row">
                    <label for="stn-wet-slider">Dry / Wet</label>
                    <output>{reverbWet.toFixed(2)}</output>
                  </div>
                  <input id="stn-wet-slider" type="range" min="0" max="1" step="0.01" bind:value={reverbWet} oninput={scheduleRender} />
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'stft'}
          <!-- 5. Parâmetros STFT -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('stft', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('stft', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Parâmetros STFT</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.stft = !panelExpanded.stft}>
                {panelExpanded.stft ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.stft}
              <div class="panel-body">
                <div class="field">
                  <label for="stn-win-type">Janela</label>
                  <select id="stn-win-type" bind:value={windowType} onchange={scheduleRender}>
                    <option value="Hann">Hann</option>
                    <option value="Hamming">Hamming</option>
                    <option value="Blackman">Blackman</option>
                    <option value="Retangular">Retangular</option>
                  </select>
                </div>

                <div class="two-inputs">
                  <div class="field">
                    <label for="stn-fft-size">Tamanho FFT</label>
                    <select id="stn-fft-size" bind:value={fftSize} onchange={scheduleRender}>
                      <option value={512}>512</option>
                      <option value={1024}>1024</option>
                      <option value={2048}>2048</option>
                      <option value={4096}>4096</option>
                    </select>
                  </div>

                  <div class="field">
                    <label for="stn-hop-size">Hop size</label>
                    <select id="stn-hop-size" bind:value={hopSize} onchange={scheduleRender}>
                      <option value={128}>128</option>
                      <option value={256}>256</option>
                      <option value={512}>512</option>
                      <option value={1024}>1024</option>
                    </select>
                  </div>
                </div>

                <div class="field">
                  <label for="stn-reassign-select">Reassignment</label>
                  <select id="stn-reassign-select" bind:value={viewMode} onchange={scheduleRender}>
                    <option value="reassignment">Ativado · tempo + frequência</option>
                    <option value="stft">Desativado · STFT convencional</option>
                  </select>
                </div>
              </div>
            {/if}
          </section>
        {:else if panelId === 'status'}
          <!-- 6. Estado do Renderizador -->
          <section 
            class="panel" 
            draggable="true" 
            ondragstart={() => handleDragStart('status', 'right')} 
            ondragover={handleDragOver} 
            ondrop={() => handleDrop('status', 'right')}
          >
            <div class="panel-head">
              <span class="drag-grip" title="Arrastar para reordenar painel">⠿</span>
              <span>Estado do renderizador</span>
              <button class="collapse-toggle" onclick={() => panelExpanded.status = !panelExpanded.status}>
                {panelExpanded.status ? '▼' : '▲'}
              </button>
            </div>
            {#if panelExpanded.status}
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
            {/if}
          </section>
        {/if}
      {/each}
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

  select, input[type="number"] {
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

  .layout-reset-btn {
    background: #0e2032;
    border: 1px solid #24425e;
    color: var(--muted);
    padding: 5px 9px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 11px;
  }

  .layout-reset-btn:hover {
    color: #fff;
    border-color: var(--blue);
  }

  /* 3-Column Studio Layout com Splitters */
  .studio-layout {
    display: grid;
    gap: 0;
    padding: 8px;
    align-items: start;
    flex: 1;
    overflow-x: auto;
  }

  /* Splitters verticais redimensionáveis */
  .splitter-v {
    width: 6px;
    cursor: col-resize;
    background: transparent;
    transition: background 0.2s;
    height: 100%;
    border-left: 1px solid rgba(255, 255, 255, 0.04);
    border-right: 1px solid rgba(255, 255, 255, 0.04);
  }

  .splitter-v:hover {
    background: rgba(53, 168, 255, 0.35);
  }

  /* Splitter horizontal para altura do espectrograma */
  .splitter-h {
    height: 10px;
    cursor: row-resize;
    display: flex;
    justify-content: center;
    align-items: center;
    background: #081624;
    border-top: 1px solid #162c40;
    border-bottom: 1px solid #162c40;
  }

  .splitter-handle-bar {
    width: 48px;
    height: 3px;
    background: #2a4c6a;
    border-radius: 2px;
  }

  .splitter-h:hover .splitter-handle-bar {
    background: var(--blue);
  }

  .col {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  /* Panels e Drag-Grips */
  .panel {
    background: linear-gradient(180deg, #0d1d2d, #091725);
    border: 1px solid var(--line);
    border-radius: var(--radius);
    overflow: hidden;
    transition: border-color 0.2s, box-shadow 0.2s;
  }

  .panel-head {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-bottom: 1px solid #1b3348;
    font-weight: 700;
    font-size: 11px;
    color: #d8e8f6;
  }

  .drag-grip {
    cursor: grab;
    color: var(--muted);
    font-size: 13px;
    user-select: none;
    letter-spacing: -1px;
  }

  .drag-grip:hover {
    color: var(--blue);
  }

  .collapse-toggle {
    margin-left: auto;
    background: transparent;
    border: none;
    color: var(--muted);
    font-size: 9px;
    cursor: pointer;
    padding: 2px 4px;
  }

  .collapse-toggle:hover {
    color: #fff;
  }

  .panel-body {
    padding: 8px 10px;
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

  .pill-set.mini button {
    padding: 3px 8px;
    font-size: 9px;
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
    padding: 7px 10px;
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
    width: 48px;
    height: 7px;
    border-radius: 2px;
    background: linear-gradient(90deg, #000, #ff5367, #ffc166, #fff);
  }

  .mini-tool-btn {
    background: #0e2235;
    border: 1px solid #203c54;
    color: var(--muted);
    padding: 2px 6px;
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
    overflow: hidden;
  }

  .spec-canvas {
    width: 100%;
    flex: 1;
    display: block;
    cursor: crosshair;
  }

  .overview-canvas {
    width: 100%;
    height: 48px;
    display: block;
    border-top: 1px solid #162c40;
    cursor: pointer;
  }

  .canvas-footerline {
    display: flex;
    justify-content: space-between;
    padding: 3px 8px;
    background: #071421;
    border-top: 1px solid #172d42;
    font-size: 10px;
    color: var(--muted);
  }

  .cursor-info {
    color: #35a8ff;
    font-weight: 600;
  }

  /* Lower Panels Layout Controls */
  .lower-layout-control {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 6px;
    background: #081624;
    border-radius: 6px;
    border: 1px solid var(--line);
    margin-top: 6px;
    font-size: 10px;
  }

  .layout-lbl {
    color: var(--muted);
    font-weight: 600;
  }

  .lower-tab-selector {
    display: flex;
    gap: 4px;
    margin-left: auto;
  }

  .lower-tab-selector button {
    background: transparent;
    border: 1px solid #1e3a54;
    color: var(--muted);
    padding: 2px 7px;
    border-radius: 4px;
    font-size: 10px;
    cursor: pointer;
  }

  .lower-tab-selector button.active {
    background: #183e60;
    color: #fff;
    border-color: var(--blue);
  }

  /* Lower Container */
  .lower-container {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-top: 6px;
  }

  .lower-container.grid-mode {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
  }

  .lower-container.stacked-mode {
    display: flex;
    flex-direction: column;
  }

  .lower-card {
    display: flex;
    flex-direction: column;
  }

  .accent-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 8px;
    border-bottom: 1px solid #1b3348;
    font-size: 11px;
    font-weight: 700;
  }

  .accent-head.s { background: rgba(53, 168, 255, 0.12); color: #83caff; }
  .accent-head.t { background: rgba(255, 83, 103, 0.12); color: #ff8594; }
  .accent-head.n { background: rgba(34, 214, 154, 0.12); color: #64e3b7; }

  .mini-add-btn {
    margin-left: auto;
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
    max-height: 120px;
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
    padding: 3px 6px;
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
    padding: 6px 8px;
    display: flex;
    flex-direction: column;
    gap: 5px;
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
    padding-bottom: 3px;
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
    gap: 5px;
  }

  .chart-box {
    background: #040d16;
    border: 1px solid #14283b;
    border-radius: 4px;
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }

  .chart-title {
    font-size: 9px;
    color: var(--muted);
  }

  .chart-box canvas {
    width: 100%;
    height: 60px;
    display: block;
  }

  .bands-slider-grid {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 2px;
    background: #05101a;
    padding: 4px;
    border-radius: 4px;
    border: 1px solid #14283b;
  }

  .band-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
  }

  .band-hz {
    font-size: 8px;
    color: var(--muted);
  }

  .band-col input[type="range"] {
    writing-mode: vertical-lr;
    direction: rtl;
    height: 52px;
    width: 12px;
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
    padding: 0 5px;
    cursor: pointer;
  }

  .small-lbl {
    color: var(--muted);
    font-size: 10px;
  }

  @media (max-width: 1100px) {
    .studio-layout {
      grid-template-columns: 1fr !important;
    }
    .splitter-v {
      display: none;
    }
    .lower-container.grid-mode {
      grid-template-columns: 1fr;
    }
  }
</style>
