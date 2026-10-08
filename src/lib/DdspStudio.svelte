<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import {
    wasm_ddsp_synthesize_wav,
    wasm_ddsp_synthesize_pcm,
    wasm_ddsp_analyze_audio,
    wasm_ddsp_get_preset,
    wasm_ddsp_list_presets
  } from '../wasm/core_wasm.js';

  let {
    originalBytes = null,
    initialConfig = null,
    onNavigate = () => {},
    onTransportToEditor = () => {},
  }: {
    originalBytes?: Uint8Array | null,
    initialConfig?: any | null,
    onNavigate?: (view: 'converter' | 'editor' | 'synth') => void,
    onTransportToEditor?: (wavBytes: Uint8Array) => void,
  } = $props();

  // ----------------------------------------------------
  // ESTADO REATIVO DO STUDIO DDSP & TREENN
  // ----------------------------------------------------
  let activeTab = $state<'harmonics' | 'adsr' | 'spectral' | 'modulation' | 'noise' | 'effects'>('harmonics');
  let selectedPresetId = $state<string>('vocal_formant');
  let isSynthesizing = $state(false);
  let isPlaying = $state(false);
  let statusBanner = $state<string | null>(null);
  let audioPlayer = $state<HTMLAudioElement | null>(null);
  let currentWavBytes = $state<Uint8Array | null>(null);
  let currentPcm = $state<Float32Array | null>(null);

  // Canvases para visualização de osciloscópio e espectro
  let oscCanvas: HTMLCanvasElement;
  let specCanvas: HTMLCanvasElement;

  // Lista de Presets curados disponíveis no kernel WASM
  let presetList = $state<Array<{ id: string; name: string; description: string }>>([
    { id: 'vocal_formant', name: '🗣️ Vocal Formant (A3)', description: 'Síntese vocal com formantes acústicos F1-F3 e vibrato natural' },
    { id: 'fm_bell', name: '🔔 FM Bell & Inharmonicity', description: 'Sino FM com modulação angular β=1.8 e rigidez inarmônica' },
    { id: 'stiff_piano', name: '🎹 Stiff Piano String', description: 'Corda de piano com dispersão física de parciais e decaimento' },
    { id: 'vibrato_strings', name: '🎻 Vibrato Strings Ensemble', description: 'Cordas com envelope suave, vibrato de 32c, filtro SVF e reverb' },
    { id: 'analog_bass', name: '🎸 Analog Resonant Bass', description: 'Baixo analógico com harmônicos ricos e filtro passa-baixas' },
    { id: 'treenn_3node', name: '🌳 TreeNN 3-Node Topology', description: 'Grafo causal hierárquico com portadora e 2 nós moduladores' },
  ]);

  // Configuração mestre do sintetizador DDSP
  let config = $state({
    f0: 220.0,
    duration_s: 2.0,
    sample_rate: 44100.0,
    amplitude: 0.85,
    adsr: {
      attack_s: 0.03,
      decay_s: 0.15,
      sustain: 0.70,
      release_s: 0.25,
      curve: 'cubic_smooth' as 'linear' | 'exponential' | 'cubic_smooth',
    },
    harmonics: {
      amplitudes: [1.0, 0.55, 0.30, 0.18, 0.10, 0.06, 0.03, 0.02, 0.015, 0.01, 0.008, 0.005],
      phases: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
      inharmonicity_b: 0.0,
      roll_off_alpha: 1.1,
    },
    spectral_envelope: {
      gauge_weights: [0.0, 0.0, 0.0, 0.0] as [number, number, number, number],
      formants: [
        { center_hz: 700.0, bandwidth_hz: 110.0, gain_db: 3.5 },
        { center_hz: 1220.0, bandwidth_hz: 140.0, gain_db: 0.5 },
        { center_hz: 2600.0, bandwidth_hz: 180.0, gain_db: -3.0 },
      ],
    },
    lfo: {
      enabled: true,
      rate_hz: 5.5,
      depth_cents: 25.0,
      phase_rad: 0.0,
      waveform: 'sine' as 'sine' | 'triangle' | 'saw' | 'square',
    },
    fm: {
      enabled: false,
      ratio: 1.414,
      index: 0.6,
      phase_rad: 0.0,
    },
    am: {
      enabled: false,
      rate_hz: 4.5,
      depth: 0.35,
      phase_rad: 0.0,
    },
    noise: {
      enabled: false,
      level_db: -36.0,
      alpha: 1.0,
      knee_hz: 1500.0,
      mix: 0.05,
    },
    effects: {
      filter_enabled: false,
      filter_type: 'lowpass' as 'lowpass' | 'highpass' | 'bandpass' | 'notch',
      filter_cutoff_hz: 3500.0,
      filter_resonance_q: 1.0,
      delay_enabled: false,
      delay_time_ms: 150.0,
      delay_feedback: 0.35,
      delay_mix: 0.20,
      reverb_enabled: false,
      reverb_decay_s: 1.2,
      reverb_mix: 0.15,
    },
    tree: {
      enabled: true,
      nodes: [
        { id: 'node-root', label: 'Raiz Portadora (F0)', freq_hz: 220.0, amplitude: 1.0, phase_rad: 0.0, color: '#00f2fe' },
        { id: 'node-mod1', label: 'Modulador 1 (Sub)', freq_hz: 110.0, amplitude: 0.5, phase_rad: 0.0, color: '#38ef7d' },
        { id: 'node-vib', label: 'Vibrato LFO', freq_hz: 5.5, amplitude: 0.2, phase_rad: 0.0, color: '#f59e0b' },
      ],
      edges: [
        { parent_id: 'node-root', child_id: 'node-mod1', beta: 0.6 },
        { parent_id: 'node-root', child_id: 'node-vib', beta: 0.35 },
      ],
    },
  });

  // Converte Hz em nome de nota musical (ex: 440 Hz -> A4, 261.63 Hz -> C4)
  function freqToNoteName(freq: number): string {
    if (freq <= 0) return '---';
    const noteNames = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B'];
    const midi = Math.round(69 + 12 * Math.log2(freq / 440.0));
    const octave = Math.floor(midi / 12) - 1;
    const note = noteNames[((midi % 12) + 12) % 12];
    return `${note}${octave}`;
  }

  // ----------------------------------------------------
  // SÍNTESE E REPRODUÇÃO
  // ----------------------------------------------------
  function synthesizeAudio(autoPlay = false) {
    try {
      isSynthesizing = true;
      const jsonStr = JSON.stringify(config);
      const wav = wasm_ddsp_synthesize_wav(jsonStr);
      const pcm = wasm_ddsp_synthesize_pcm(jsonStr);
      currentWavBytes = wav;
      currentPcm = pcm;

      renderOscilloscope();
      renderSpectrum();

      if (autoPlay) {
        playAudio(wav);
      } else {
        statusBanner = `⚡ Áudio sintetizado: ${pcm.length} amostras (${(wav.length / 1024).toFixed(1)} kB)`;
        setTimeout(() => { if (statusBanner?.startsWith('⚡')) statusBanner = null; }, 3000);
      }
    } catch (err: any) {
      console.error('Erro ao sintetizar áudio:', err);
      statusBanner = `❌ Erro de síntese: ${err.message || String(err)}`;
    } finally {
      isSynthesizing = false;
    }
  }

  function playAudio(wavBytes?: Uint8Array) {
    const bytes = wavBytes || currentWavBytes;
    if (!bytes) {
      synthesizeAudio(true);
      return;
    }
    if (audioPlayer) {
      audioPlayer.pause();
    }
    const blob = new Blob([bytes as any], { type: 'audio/wav' });
    const url = URL.createObjectURL(blob);
    audioPlayer = new Audio(url);
    audioPlayer.play().catch(e => console.warn('Autoplay bloqueado pelo navegador:', e));
    isPlaying = true;
    audioPlayer.onended = () => { isPlaying = false; };
    statusBanner = `🔊 Reproduzindo som sintetizado...`;
  }

  function stopAudio() {
    if (audioPlayer) {
      audioPlayer.pause();
      isPlaying = false;
      statusBanner = `⏹️ Reprodução pausada`;
      setTimeout(() => { if (statusBanner?.startsWith('⏹️')) statusBanner = null; }, 1500);
    }
  }

  function togglePlay() {
    if (isPlaying) {
      stopAudio();
    } else {
      playAudio();
    }
  }

  // ----------------------------------------------------
  // PRESETS E TRANSPORTE INTER-UI
  // ----------------------------------------------------
  function loadPreset(name: string) {
    try {
      selectedPresetId = name;
      const jsonStr = wasm_ddsp_get_preset(name);
      config = JSON.parse(jsonStr);
      synthesizeAudio(false);
      statusBanner = `✅ Preset [${name}] carregado com sucesso!`;
      setTimeout(() => { if (statusBanner?.startsWith('✅')) statusBanner = null; }, 3000);
    } catch (err: any) {
      console.error('Erro ao carregar preset:', err);
      statusBanner = `❌ Erro preset: ${err.message || String(err)}`;
    }
  }

  function transportToEditor() {
    try {
      const jsonStr = JSON.stringify(config);
      const wav = wasm_ddsp_synthesize_wav(jsonStr);
      onTransportToEditor(wav);
      onNavigate('editor');
    } catch (err: any) {
      console.error('Erro ao transportar para o WebGL Editor:', err);
      statusBanner = `❌ Erro no transporte: ${err.message || String(err)}`;
    }
  }

  function extractFromLoadedAudio() {
    if (!originalBytes) {
      statusBanner = `❌ Nenhum arquivo de áudio carregado no projeto`;
      return;
    }
    try {
      statusBanner = `🪄 Executando inversão analítica E00-E18 no áudio...`;
      const jsonStr = wasm_ddsp_analyze_audio(originalBytes, config.f0);
      config = JSON.parse(jsonStr);
      synthesizeAudio(false);
      statusBanner = `✅ Extraído com sucesso! F0=${config.f0.toFixed(1)}Hz (${freqToNoteName(config.f0)})`;
      setTimeout(() => { if (statusBanner?.startsWith('✅')) statusBanner = null; }, 4000);
    } catch (err: any) {
      console.error('Erro ao extrair parâmetros do áudio:', err);
      statusBanner = `❌ Erro na inversão analítica: ${err.message || String(err)}`;
    }
  }

  function exportWav() {
    try {
      const jsonStr = JSON.stringify(config);
      const wav = wasm_ddsp_synthesize_wav(jsonStr);
      const blob = new Blob([wav as any], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `ddsp-${selectedPresetId}-${Math.round(config.f0)}hz.wav`;
      a.click();
      URL.revokeObjectURL(url);
      statusBanner = `💾 Arquivo WAV exportado com sucesso!`;
      setTimeout(() => { if (statusBanner?.startsWith('💾')) statusBanner = null; }, 2500);
    } catch (err: any) {
      console.error('Erro ao exportar WAV:', err);
    }
  }

  function exportJson() {
    const jsonStr = JSON.stringify(config, null, 2);
    const blob = new Blob([jsonStr], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `ddsp-config-${selectedPresetId}.json`;
    a.click();
    URL.revokeObjectURL(url);
  }

  function importJson(e: Event) {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files[0]) {
      const reader = new FileReader();
      reader.onload = () => {
        try {
          const parsed = JSON.parse(reader.result as string);
          config = parsed;
          synthesizeAudio(false);
          statusBanner = `📁 Configuração importada com sucesso!`;
          setTimeout(() => { if (statusBanner?.startsWith('📁')) statusBanner = null; }, 3000);
        } catch (err: any) {
          statusBanner = `❌ Arquivo JSON inválido`;
        }
      };
      reader.readAsText(target.files[0]);
    }
  }

  // ----------------------------------------------------
  // RENDERIZAÇÃO DE OSCILOSCÓPIO E ESPECTRO NO CANVAS
  // ----------------------------------------------------
  function renderOscilloscope() {
    if (!oscCanvas || !currentPcm) return;
    const ctx = oscCanvas.getContext('2d');
    if (!ctx) return;
    const w = oscCanvas.width;
    const h = oscCanvas.height;

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    // Linha central de grade
    ctx.strokeStyle = 'rgba(255, 255, 255, 0.08)';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, h / 2);
    ctx.lineTo(w, h / 2);
    ctx.stroke();

    ctx.strokeStyle = '#00f2fe';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const pcm = currentPcm;
    const step = Math.max(1, Math.floor(pcm.length / w));
    for (let x = 0; x < w; x++) {
      const idx = x * step;
      if (idx < pcm.length) {
        const sample = pcm[idx];
        const y = h / 2 - sample * (h * 0.42);
        if (x === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
    }
    ctx.stroke();
  }

  function renderSpectrum() {
    if (!specCanvas || !config) return;
    const ctx = specCanvas.getContext('2d');
    if (!ctx) return;
    const w = specCanvas.width;
    const h = specCanvas.height;

    ctx.fillStyle = '#0f172a';
    ctx.fillRect(0, 0, w, h);

    // Barras dos harmônicos
    const amps = config.harmonics.amplitudes;
    const barWidth = Math.max(4, Math.floor((w - 20) / amps.length) - 3);

    for (let i = 0; i < amps.length; i++) {
      const amp = amps[i];
      const barH = amp * (h * 0.85);
      const x = 10 + i * (barWidth + 3);
      const y = h - barH - 4;

      // Gradiente ciano para roxo
      const grad = ctx.createLinearGradient(0, y, 0, h);
      grad.addColorStop(0, '#00f2fe');
      grad.addColorStop(1, '#6366f1');

      ctx.fillStyle = grad;
      ctx.fillRect(x, y, barWidth, barH);
    }
  }

  // ----------------------------------------------------
  // CURVAS SVG DE ENVELOPE E ESPECTRO
  // ----------------------------------------------------
  function computeAdsrSvg(adsr: typeof config.adsr, width = 360, height = 70): string {
    const total = adsr.attack_s + adsr.decay_s + 0.4 + adsr.release_s;
    const aX = (adsr.attack_s / total) * width;
    const dX = ((adsr.attack_s + adsr.decay_s) / total) * width;
    const sX = ((adsr.attack_s + adsr.decay_s + 0.4) / total) * width;
    const rX = width;
    const sY = height * (1.0 - adsr.sustain);

    return `M 0 ${height} C ${aX * 0.4} ${height * 0.1}, ${aX * 0.8} 0, ${aX} 0 C ${aX + (dX - aX) * 0.5} ${sY * 0.5}, ${dX * 0.9} ${sY}, ${dX} ${sY} L ${sX} ${sY} C ${sX + (rX - sX) * 0.5} ${sY + (height - sY) * 0.5}, ${rX * 0.9} ${height}, ${rX} ${height}`;
  }

  function computeSpectralSvg(spectral: typeof config.spectral_envelope, width = 360, height = 70): string {
    const pts: string[] = [];
    const steps = 40;
    const w = spectral.gauge_weights;
    for (let i = 0; i <= steps; i++) {
      const u = -2.0 + (i / steps) * 4.0;
      const u2 = u * u;
      const u3 = u2 * u;
      const denom = 1.0 + 0.5 * u2;
      const db = w[0] * u2 + w[1] * u3 + w[2] * (u2 / denom) + w[3] * (u3 / denom);
      const px = (i / steps) * width;
      const py = Math.max(0, Math.min(height, height * 0.5 - (db / 20.0) * (height * 0.45)));
      pts.push(`${i === 0 ? 'M' : 'L'} ${px.toFixed(1)} ${py.toFixed(1)}`);
    }
    return pts.join(' ');
  }

  onMount(() => {
    if (initialConfig) {
      config = initialConfig;
    }
    synthesizeAudio(false);
  });

  onDestroy(() => {
    if (audioPlayer) {
      audioPlayer.pause();
    }
  });
</script>

<div class="ddsp-studio-container">
  <!-- TOP STUDIO HEADER -->
  <header class="studio-header">
    <div class="brand-group">
      <span class="studio-icon">🌳🎛️</span>
      <div class="title-meta">
        <h2>DDSP & TreeNN Studio</h2>
        <span class="sub-badge font-mono">Differentiable Neural Synthesizer (E00–E18)</span>
      </div>
    </div>

    <!-- PRESET SELECTOR & MAIN CONTROLS -->
    <div class="preset-action-bar">
      <div class="preset-group">
        <label for="studio-preset-select" class="preset-lbl">Preset:</label>
        <select 
          id="studio-preset-select" 
          class="preset-select" 
          bind:value={selectedPresetId} 
          onchange={() => loadPreset(selectedPresetId)}
        >
          {#each presetList as p}
            <option value={p.id}>{p.name}</option>
          {/each}
        </select>
      </div>

      <div class="transport-btn-group">
        <button class="action-btn play-btn" class:active={isPlaying} onclick={togglePlay} title="Reproduzir áudio sintetizado">
          {#if isPlaying}⏹️ Parar{:else}▶️ Play DDSP{/if}
        </button>

        <button class="action-btn synth-btn" onclick={() => synthesizeAudio(false)} disabled={isSynthesizing} title="Re-sintetizar no kernel Rust WASM">
          🔄 Re-sintetizar
        </button>

        <button class="action-btn transport-btn" onclick={transportToEditor} title="Sintetiza e transporta o som gerado diretamente para o espectrograma no WebGL Editor">
          🚀 Para o WebGL Editor
        </button>

        {#if originalBytes}
          <button class="action-btn extract-btn" onclick={extractFromLoadedAudio} title="Extrai os parâmetros analíticos a partir do áudio carregado no editor">
            🪄 Extrair do Áudio
          </button>
        {/if}

        <button class="action-btn util-btn" onclick={exportWav} title="Exporta WAV 16-bit">
          📥 WAV
        </button>

        <button class="action-btn util-btn" onclick={exportJson} title="Exporta configuração JSON">
          💾 JSON
        </button>

        <label class="action-btn util-btn import-label" title="Importa arquivo JSON">
          📁 Importar
          <input type="file" accept=".json" onchange={importJson} style="display: none;" />
        </label>
      </div>
    </div>

    <!-- NAVEGAÇÃO DE ROTAS GLOBAIS -->
    <div class="nav-toggles">
      <button class="nav-btn" onclick={() => onNavigate('converter')}>1. Converter</button>
      <button class="nav-btn" onclick={() => onNavigate('editor')}>2. WebGL Editor</button>
      <button class="nav-btn active-view" onclick={() => onNavigate('synth')}>3. DDSP Studio</button>
    </div>
  </header>

  {#if statusBanner}
    <div class="status-banner font-mono">{statusBanner}</div>
  {/if}

  <!-- MAIN STUDIO WORKSPACE -->
  <main class="studio-workspace">
    <!-- LEFT PANEL: TREENN GRAPH & MASTER VOICE CONTROLS -->
    <aside class="left-panel">
      <div class="panel-card">
        <div class="card-title">
          <span>🌳 Grafo de Modulação Causal (TreeNN)</span>
          <label class="toggle-control font-mono">
            <input type="checkbox" bind:checked={config.tree.enabled} onchange={() => synthesizeAudio(false)} />
            Ativo
          </label>
        </div>

        <div class="tree-nodes-list">
          {#each config.tree.nodes as node, nIdx}
            <div class="tree-node-item" style="border-left-color: {node.color};">
              <div class="node-info">
                <span class="node-name" style="color: {node.color};">{node.label}</span>
                <span class="node-hz font-mono">{node.freq_hz.toFixed(1)} Hz</span>
              </div>
              <input 
                type="range" 
                min="20" 
                max="2000" 
                step="5" 
                bind:value={node.freq_hz} 
                oninput={() => synthesizeAudio(false)}
                class="studio-slider" 
              />
            </div>
          {/each}
        </div>

        <div class="tree-edges-section">
          <span class="section-subtitle">Conexões de Acoplamento Direcionadas (β):</span>
          {#each config.tree.edges as edge}
            <div class="edge-item">
              <span class="edge-relation font-mono">{edge.child_id} → {edge.parent_id}</span>
              <input 
                type="range" 
                min="0.0" 
                max="2.0" 
                step="0.05" 
                bind:value={edge.beta} 
                oninput={() => synthesizeAudio(false)}
                class="studio-slider compact" 
              />
              <span class="edge-val font-mono">{edge.beta.toFixed(2)}</span>
            </div>
          {/each}
        </div>
      </div>

      <!-- MASTER CONTROLS -->
      <div class="panel-card">
        <div class="card-title">
          <span>🎚️ Parâmetros Mestre</span>
          <span class="note-badge font-mono">{freqToNoteName(config.f0)}</span>
        </div>

        <div class="control-row">
          <span>Frequência Base (f₀):</span>
          <span class="val font-mono">{config.f0.toFixed(1)} Hz</span>
        </div>
        <input 
          type="range" 
          min="40" 
          max="2000" 
          step="1" 
          bind:value={config.f0} 
          oninput={() => synthesizeAudio(false)}
          class="studio-slider" 
        />

        <div class="control-row">
          <span>Ganho Mestre:</span>
          <span class="val font-mono">{(config.amplitude * 100).toFixed(0)}%</span>
        </div>
        <input 
          type="range" 
          min="0.0" 
          max="1.0" 
          step="0.02" 
          bind:value={config.amplitude} 
          oninput={() => synthesizeAudio(false)}
          class="studio-slider" 
        />

        <div class="control-row">
          <span>Duração (s):</span>
          <span class="val font-mono">{config.duration_s.toFixed(2)} s</span>
        </div>
        <input 
          type="range" 
          min="0.2" 
          max="5.0" 
          step="0.1" 
          bind:value={config.duration_s} 
          oninput={() => synthesizeAudio(false)}
          class="studio-slider" 
        />
      </div>
    </aside>

    <!-- CENTER PANEL: MULTI-TAB DEEP PARAMETER EDITOR -->
    <section class="center-panel">
      <!-- SUBTABS BAR -->
      <nav class="studio-subtabs">
        <button class="tab-btn" class:active={activeTab === 'harmonics'} onclick={() => activeTab = 'harmonics'}>
          🎼 Harmônicos & Rigidez B
        </button>
        <button class="tab-btn" class:active={activeTab === 'adsr'} onclick={() => activeTab = 'adsr'}>
          📈 Envelope ADSR C¹
        </button>
        <button class="tab-btn" class:active={activeTab === 'spectral'} onclick={() => activeTab = 'spectral'}>
          🗣️ Formantes & Gauge
        </button>
        <button class="tab-btn" class:active={activeTab === 'modulation'} onclick={() => activeTab = 'modulation'}>
          📻 FM & AM Modulação
        </button>
        <button class="tab-btn" class:active={activeTab === 'noise'} onclick={() => activeTab = 'noise'}>
          🌪️ Ruído Fractal
        </button>
        <button class="tab-btn" class:active={activeTab === 'effects'} onclick={() => activeTab = 'effects'}>
          🎛️ Efeitos Acústicos
        </button>
      </nav>

      <div class="tab-container">
        <!-- TAB 1: Harmônicos & Inarmonicidade B (E06, E08) -->
        {#if activeTab === 'harmonics'}
          <div class="tab-pane">
            <div class="gauge-badge-row">
              <span class="gauge-tag">Condição de Gauge E06:</span>
              <span class="font-mono text-cyan">H₁ ≡ 1.0 (Normalização Estrita)</span>
              <span class="gauge-tag" style="margin-left: auto;">Dispersão E08:</span>
              <span class="font-mono text-amber">f_k = k · f₀ · √(1 + B · k²)</span>
            </div>

            <div class="control-grid">
              <div class="control-block">
                <div class="control-row">
                  <span>Rigidez Inarmônica (B):</span>
                  <span class="val font-mono">{config.harmonics.inharmonicity_b.toExponential(2)}</span>
                </div>
                <input 
                  type="range" 
                  min="0.0" 
                  max="0.005" 
                  step="0.0001" 
                  bind:value={config.harmonics.inharmonicity_b} 
                  oninput={() => synthesizeAudio(false)}
                  class="studio-slider" 
                />
              </div>

              <div class="control-block">
                <div class="control-row">
                  <span>Roll-off Espectral (α):</span>
                  <span class="val font-mono">{config.harmonics.roll_off_alpha.toFixed(2)}</span>
                </div>
                <input 
                  type="range" 
                  min="0.4" 
                  max="3.0" 
                  step="0.05" 
                  bind:value={config.harmonics.roll_off_alpha} 
                  oninput={() => {
                    for (let k = 1; k < config.harmonics.amplitudes.length; k++) {
                      config.harmonics.amplitudes[k] = Math.pow(k + 1, -config.harmonics.roll_off_alpha);
                    }
                    synthesizeAudio(false);
                  }}
                  class="studio-slider" 
                />
              </div>
            </div>

            <!-- FADER BANK DOS 12 HARMÔNICOS -->
            <div class="harmonics-fader-bank">
              <span class="section-subtitle">Banco de Parciais Harmônicos (H₁ a H₁₂):</span>
              <div class="faders-row">
                {#each config.harmonics.amplitudes.slice(0, 12) as amp, hIdx}
                  <div class="fader-col">
                    <span class="h-tag font-mono">H{hIdx + 1}</span>
                    <input 
                      type="range" 
                      min="0.0" 
                      max="1.0" 
                      step="0.01" 
                      disabled={hIdx === 0}
                      bind:value={config.harmonics.amplitudes[hIdx]} 
                      oninput={() => synthesizeAudio(false)}
                      class="vertical-fader"
                      title={`H${hIdx + 1}: ${amp.toFixed(2)}`}
                    />
                    <span class="h-val font-mono">{amp.toFixed(2)}</span>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 2: ADSR C1 Contínuo (E05) -->
        {#if activeTab === 'adsr'}
          <div class="tab-pane">
            <div class="svg-preview-container">
              <span class="svg-label font-mono">Curva de Envelope C¹ Cubic Hermite (3u² - 2u³)</span>
              <svg class="adsr-svg" viewBox="0 0 360 70">
                <path 
                  d={computeAdsrSvg(config.adsr)} 
                  fill="rgba(0, 242, 254, 0.12)" 
                  stroke="#00f2fe" 
                  stroke-width="2.5" 
                />
              </svg>
            </div>

            <div class="control-grid-2x2">
              <div class="control-block">
                <div class="control-row">
                  <span>Ataque (τ_A):</span>
                  <span class="val font-mono">{(config.adsr.attack_s * 1000).toFixed(0)} ms</span>
                </div>
                <input 
                  type="range" 
                  min="0.005" 
                  max="0.5" 
                  step="0.005" 
                  bind:value={config.adsr.attack_s} 
                  oninput={() => synthesizeAudio(false)}
                  class="studio-slider" 
                />
              </div>

              <div class="control-block">
                <div class="control-row">
                  <span>Decay (τ_D):</span>
                  <span class="val font-mono">{(config.adsr.decay_s * 1000).toFixed(0)} ms</span>
                </div>
                <input 
                  type="range" 
                  min="0.01" 
                  max="0.8" 
                  step="0.01" 
                  bind:value={config.adsr.decay_s} 
                  oninput={() => synthesizeAudio(false)}
                  class="studio-slider" 
                />
              </div>

              <div class="control-block">
                <div class="control-row">
                  <span>Sustain (S):</span>
                  <span class="val font-mono">{config.adsr.sustain.toFixed(2)}</span>
                </div>
                <input 
                  type="range" 
                  min="0.0" 
                  max="1.0" 
                  step="0.02" 
                  bind:value={config.adsr.sustain} 
                  oninput={() => synthesizeAudio(false)}
                  class="studio-slider" 
                />
              </div>

              <div class="control-block">
                <div class="control-row">
                  <span>Release (τ_R):</span>
                  <span class="val font-mono">{(config.adsr.release_s * 1000).toFixed(0)} ms</span>
                </div>
                <input 
                  type="range" 
                  min="0.02" 
                  max="1.2" 
                  step="0.02" 
                  bind:value={config.adsr.release_s} 
                  oninput={() => synthesizeAudio(false)}
                  class="studio-slider" 
                />
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 3: Formantes & Envelope Espectral sob Gauge (E07) -->
        {#if activeTab === 'spectral'}
          <div class="tab-pane">
            <div class="svg-preview-container">
              <span class="svg-label font-mono">Base Espectral sob Gauge S(u) com S(440)=0 dB, S'(440)=0 dB/oct</span>
              <svg class="spectral-svg" viewBox="0 0 360 70">
                <line x1="180" y1="0" x2="180" y2="70" stroke="#ef4444" stroke-dasharray="3 3" stroke-width="1.5" />
                <text x="184" y="14" fill="#ef4444" font-size="9" font-family="monospace">440 Hz (0 dB)</text>
                <path 
                  d={computeSpectralSvg(config.spectral_envelope)} 
                  fill="none" 
                  stroke="#c084fc" 
                  stroke-width="2.5" 
                />
              </svg>
            </div>

            <div class="control-block">
              <div class="control-row">
                <span>Curvatura sob Gauge (w₀ - u²):</span>
                <span class="val font-mono">{config.spectral_envelope.gauge_weights[0].toFixed(2)}</span>
              </div>
              <input 
                type="range" 
                min="-4.0" 
                max="4.0" 
                step="0.1" 
                bind:value={config.spectral_envelope.gauge_weights[0]} 
                oninput={() => synthesizeAudio(false)}
                class="studio-slider" 
              />
            </div>

            <div class="formants-card-grid">
              <span class="section-subtitle">Filtros de Formantes Acústicos (F₁ a F₃):</span>
              {#each config.spectral_envelope.formants as formant, fIdx}
                <div class="formant-box">
                  <span class="formant-header font-mono">Formante F{fIdx + 1}</span>
                  <div class="formant-fields">
                    <label class="control-row">
                      <span>Centro:</span>
                      <span class="val font-mono">{formant.center_hz.toFixed(0)} Hz</span>
                    </label>
                    <input 
                      type="range" 
                      min="150" 
                      max="4500" 
                      step="25" 
                      bind:value={formant.center_hz} 
                      oninput={() => synthesizeAudio(false)}
                      class="studio-slider" 
                    />

                    <label class="control-row">
                      <span>Ganho:</span>
                      <span class="val font-mono">{formant.gain_db.toFixed(1)} dB</span>
                    </label>
                    <input 
                      type="range" 
                      min="-18" 
                      max="12" 
                      step="0.5" 
                      bind:value={formant.gain_db} 
                      oninput={() => synthesizeAudio(false)}
                      class="studio-slider" 
                    />
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}

        <!-- TAB 4: FM, PM e AM Modulação (E09, E10, E12) -->
        {#if activeTab === 'modulation'}
          <div class="tab-pane">
            <div class="modules-dual-grid">
              <!-- LFO Vibrato -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Vibrato LFO (E09)</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.lfo.enabled} onchange={() => synthesizeAudio(false)} />
                    {config.lfo.enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.lfo.enabled}
                  <div class="control-row">
                    <span>Taxa (f_m):</span>
                    <span class="val font-mono">{config.lfo.rate_hz.toFixed(1)} Hz</span>
                  </div>
                  <input type="range" min="0.5" max="15.0" step="0.2" bind:value={config.lfo.rate_hz} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Profundidade (cents):</span>
                    <span class="val font-mono">{config.lfo.depth_cents.toFixed(1)}c</span>
                  </div>
                  <input type="range" min="1.0" max="80.0" step="1.0" bind:value={config.lfo.depth_cents} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>

              <!-- Modulação FM -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Modulação de Frequência (FM - E10)</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.fm.enabled} onchange={() => synthesizeAudio(false)} />
                    {config.fm.enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.fm.enabled}
                  <div class="control-row">
                    <span>Razão Portadora/Moduladora:</span>
                    <span class="val font-mono">{config.fm.ratio.toFixed(3)}</span>
                  </div>
                  <input type="range" min="0.25" max="6.0" step="0.05" bind:value={config.fm.ratio} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Índice de Modulação (β):</span>
                    <span class="val font-mono">{config.fm.index.toFixed(2)}</span>
                  </div>
                  <input type="range" min="0.0" max="4.0" step="0.05" bind:value={config.fm.index} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>

              <!-- Modulação AM (Tremolo) -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Modulação de Amplitude (AM - E12)</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.am.enabled} onchange={() => synthesizeAudio(false)} />
                    {config.am.enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.am.enabled}
                  <div class="control-row">
                    <span>Taxa de Tremolo (fam):</span>
                    <span class="val font-mono">{config.am.rate_hz.toFixed(1)} Hz</span>
                  </div>
                  <input type="range" min="0.5" max="25.0" step="0.5" bind:value={config.am.rate_hz} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Profundidade (m):</span>
                    <span class="val font-mono">{(config.am.depth * 100).toFixed(0)}%</span>
                  </div>
                  <input type="range" min="0.0" max="1.0" step="0.02" bind:value={config.am.depth} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>
            </div>
          </div>
        {/if}

        <!-- TAB 5: Ruído Fractal (E13) -->
        {#if activeTab === 'noise'}
          <div class="tab-pane">
            <div class="sub-module-card">
              <div class="module-title">
                <span>Ruído Estocástico Fractal (Tilt α e Knee - E13)</span>
                <label class="toggle-control font-mono">
                  <input type="checkbox" bind:checked={config.noise.enabled} onchange={() => synthesizeAudio(false)} />
                  {config.noise.enabled ? 'ON' : 'OFF'}
                </label>
              </div>

              {#if config.noise.enabled}
                <div class="control-row">
                  <span>Tilt Espectral (α): {config.noise.alpha < 0.3 ? 'Ruído Branco' : config.noise.alpha < 1.3 ? 'Ruído Rosa (1/f)' : 'Ruído Marrom (1/f²)'}</span>
                  <span class="val font-mono">{config.noise.alpha.toFixed(2)}</span>
                </div>
                <input type="range" min="0.0" max="2.0" step="0.05" bind:value={config.noise.alpha} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                <div class="control-row">
                  <span>Nível (dB):</span>
                  <span class="val font-mono">{config.noise.level_db.toFixed(1)} dB</span>
                </div>
                <input type="range" min="-60" max="-12" step="1" bind:value={config.noise.level_db} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                <div class="control-row">
                  <span>Mix de Injeção:</span>
                  <span class="val font-mono">{(config.noise.mix * 100).toFixed(0)}%</span>
                </div>
                <input type="range" min="0.0" max="0.5" step="0.01" bind:value={config.noise.mix} oninput={() => synthesizeAudio(false)} class="studio-slider" />
              {/if}
            </div>
          </div>
        {/if}

        <!-- TAB 6: Efeitos Acústicos (E14) -->
        {#if activeTab === 'effects'}
          <div class="tab-pane">
            <div class="modules-dual-grid">
              <!-- Filtro SVF -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Filtro SVF Zero-Delay</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.effects.filter_enabled} onchange={() => synthesizeAudio(false)} />
                    {config.effects.filter_enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.effects.filter_enabled}
                  <div class="control-row">
                    <span>Frequência de Corte (fc):</span>
                    <span class="val font-mono">{config.effects.filter_cutoff_hz.toFixed(0)} Hz</span>
                  </div>
                  <input type="range" min="100" max="12000" step="50" bind:value={config.effects.filter_cutoff_hz} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Ressonância (Q):</span>
                    <span class="val font-mono">{config.effects.filter_resonance_q.toFixed(1)}</span>
                  </div>
                  <input type="range" min="0.5" max="15.0" step="0.2" bind:value={config.effects.filter_resonance_q} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>

              <!-- Delay -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Delay Fracionário</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.effects.delay_enabled} onchange={() => synthesizeAudio(false)} />
                    {config.effects.delay_enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.effects.delay_enabled}
                  <div class="control-row">
                    <span>Tempo de Atraso (τ):</span>
                    <span class="val font-mono">{config.effects.delay_time_ms.toFixed(0)} ms</span>
                  </div>
                  <input type="range" min="20" max="800" step="10" bind:value={config.effects.delay_time_ms} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Realimentação (Feedback):</span>
                    <span class="val font-mono">{(config.effects.delay_feedback * 100).toFixed(0)}%</span>
                  </div>
                  <input type="range" min="0.0" max="0.85" step="0.02" bind:value={config.effects.delay_feedback} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>

              <!-- Reverb -->
              <div class="sub-module-card">
                <div class="module-title">
                  <span>Reverb de Schroeder</span>
                  <label class="toggle-control font-mono">
                    <input type="checkbox" bind:checked={config.effects.reverb_enabled} onchange={() => synthesizeAudio(false)} />
                    {config.effects.reverb_enabled ? 'ON' : 'OFF'}
                  </label>
                </div>
                {#if config.effects.reverb_enabled}
                  <div class="control-row">
                    <span>Decaimento T60:</span>
                    <span class="val font-mono">{config.effects.reverb_decay_s.toFixed(1)} s</span>
                  </div>
                  <input type="range" min="0.2" max="4.0" step="0.1" bind:value={config.effects.reverb_decay_s} oninput={() => synthesizeAudio(false)} class="studio-slider" />

                  <div class="control-row">
                    <span>Mix Reverb:</span>
                    <span class="val font-mono">{(config.effects.reverb_mix * 100).toFixed(0)}%</span>
                  </div>
                  <input type="range" min="0.0" max="0.6" step="0.02" bind:value={config.effects.reverb_mix} oninput={() => synthesizeAudio(false)} class="studio-slider" />
                {/if}
              </div>
            </div>
          </div>
        {/if}
      </div>
    </section>

    <!-- RIGHT PANEL: OSCILLOSCOPE, SPECTRUM & TRANSPORT METRICS -->
    <aside class="right-panel">
      <!-- OSCILLOSCOPE -->
      <div class="panel-card">
        <div class="card-title">
          <span>📊 Osciloscópio (Tempo)</span>
          <span class="fps-badge font-mono">{currentPcm ? `${currentPcm.length} pts` : 'Inativo'}</span>
        </div>
        <canvas bind:this={oscCanvas} width="320" height="110" class="display-canvas"></canvas>
      </div>

      <!-- SPECTRUM BARS -->
      <div class="panel-card">
        <div class="card-title">
          <span>🌈 Espectro Harmônico (Frequência)</span>
          <span class="fps-badge font-mono">12 Harmônicos</span>
        </div>
        <canvas bind:this={specCanvas} width="320" height="110" class="display-canvas"></canvas>
      </div>

      <!-- TRANSPORT & DIAGNOSTICS CARD -->
      <div class="panel-card">
        <div class="card-title">
          <span>🧭 Protocolo de Transporte Inter-UI</span>
        </div>
        <div class="transport-info-box">
          <p class="transport-desc">
            Transporte contínuo entre a <strong>representação espectral de alta resolução</strong> e o <strong>modelo paramétrico DDSP</strong>.
          </p>
          <button class="transport-main-btn" onclick={transportToEditor}>
            🚀 Injetar no WebGL Editor & Espectrograma
          </button>
        </div>
      </div>
    </aside>
  </main>
</div>

<style>
  .ddsp-studio-container {
    width: 100%;
    min-height: 100vh;
    background: #0b0f19;
    color: #e2e8f0;
    display: flex;
    flex-direction: column;
    overflow-x: hidden;
  }

  .studio-header {
    background: rgba(15, 23, 42, 0.95);
    border-bottom: 1px solid rgba(0, 242, 254, 0.25);
    padding: 0.75rem 1.25rem;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    flex-wrap: wrap;
    backdrop-filter: blur(12px);
    z-index: 10;
  }

  .brand-group {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .studio-icon {
    font-size: 1.75rem;
  }

  .title-meta h2 {
    margin: 0;
    font-size: 1.15rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .sub-badge {
    font-size: 0.68rem;
    color: #38bdf8;
    background: rgba(56, 189, 248, 0.12);
    border: 1px solid rgba(56, 189, 248, 0.3);
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
  }

  .preset-action-bar {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    flex-wrap: wrap;
  }

  .preset-group {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    background: rgba(0, 0, 0, 0.35);
    padding: 0.25rem 0.5rem;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
  }

  .preset-lbl {
    font-size: 0.75rem;
    font-weight: 700;
    color: #94a3b8;
  }

  .preset-select {
    background: #0f172a;
    border: 1px solid rgba(56, 189, 248, 0.4);
    color: #f8fafc;
    font-size: 0.75rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .transport-btn-group {
    display: flex;
    gap: 0.4rem;
    align-items: center;
  }

  .action-btn {
    border: 1px solid rgba(255, 255, 255, 0.15);
    padding: 0.35rem 0.65rem;
    border-radius: 5px;
    font-size: 0.72rem;
    font-weight: 700;
    cursor: pointer;
    transition: all 0.15s ease;
    display: flex;
    align-items: center;
    gap: 0.25rem;
    white-space: nowrap;
  }

  .action-btn.play-btn {
    background: linear-gradient(135deg, rgba(16, 185, 129, 0.3), rgba(5, 150, 105, 0.3));
    border-color: #10b981;
    color: #34d399;
  }

  .action-btn.play-btn.active {
    background: #ef4444;
    border-color: #f87171;
    color: #ffffff;
  }

  .action-btn.synth-btn {
    background: rgba(56, 189, 248, 0.15);
    border-color: #38bdf8;
    color: #38bdf8;
  }

  .action-btn.transport-btn {
    background: linear-gradient(135deg, #0ea5e9, #6366f1);
    color: #ffffff;
    border-color: #38bdf8;
    box-shadow: 0 0 10px rgba(14, 165, 233, 0.4);
  }

  .action-btn.extract-btn {
    background: linear-gradient(135deg, rgba(168, 85, 247, 0.3), rgba(126, 34, 206, 0.3));
    border-color: #a855f7;
    color: #c084fc;
  }

  .action-btn.util-btn {
    background: rgba(255, 255, 255, 0.05);
    color: #cbd5e1;
  }

  .action-btn:hover {
    filter: brightness(1.2);
    transform: translateY(-1px);
  }

  .import-label {
    cursor: pointer;
  }

  .nav-toggles {
    display: flex;
    gap: 0.25rem;
    background: rgba(0, 0, 0, 0.35);
    padding: 0.2rem;
    border-radius: 6px;
  }

  .nav-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 0.72rem;
    font-weight: 600;
    padding: 0.3rem 0.6rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .nav-btn.active-view {
    background: rgba(0, 242, 254, 0.2);
    color: #00f2fe;
    font-weight: 700;
  }

  .status-banner {
    background: rgba(0, 0, 0, 0.6);
    border-bottom: 1px solid rgba(0, 242, 254, 0.3);
    color: #38bdf8;
    font-size: 0.72rem;
    padding: 0.3rem 1.25rem;
  }

  .studio-workspace {
    flex: 1;
    display: grid;
    grid-template-columns: 280px 1fr 340px;
    gap: 1rem;
    padding: 1rem;
    overflow-y: auto;
  }

  @media (max-width: 1200px) {
    .studio-workspace {
      grid-template-columns: 1fr;
    }
  }

  .panel-card {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
    margin-bottom: 1rem;
  }

  .card-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.82rem;
    font-weight: 700;
    color: #f8fafc;
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    padding-bottom: 0.4rem;
  }

  .note-badge {
    background: rgba(56, 189, 248, 0.2);
    color: #38bdf8;
    padding: 0.1rem 0.4rem;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 800;
  }

  .tree-node-item {
    border-left: 3px solid;
    background: rgba(0, 0, 0, 0.25);
    padding: 0.4rem 0.5rem;
    border-radius: 0 4px 4px 0;
    margin-bottom: 0.35rem;
  }

  .node-info {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
    font-weight: 600;
  }

  .edge-item {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.68rem;
    margin-bottom: 0.25rem;
  }

  .edge-relation {
    color: #94a3b8;
    width: 120px;
  }

  .edge-val {
    color: #38bdf8;
    width: 30px;
    text-align: right;
  }

  .control-row {
    display: flex;
    justify-content: space-between;
    font-size: 0.72rem;
    color: #94a3b8;
  }

  .studio-slider {
    width: 100%;
    height: 4px;
    accent-color: #00f2fe;
    cursor: pointer;
  }

  .studio-slider.compact {
    height: 3px;
  }

  .section-subtitle {
    font-size: 0.7rem;
    font-weight: 700;
    color: #cbd5e1;
  }

  .center-panel {
    background: rgba(15, 23, 42, 0.7);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .studio-subtabs {
    display: flex;
    background: rgba(0, 0, 0, 0.35);
    border-bottom: 1px solid rgba(255, 255, 255, 0.08);
    overflow-x: auto;
  }

  .tab-btn {
    background: transparent;
    border: none;
    color: #94a3b8;
    font-size: 0.75rem;
    font-weight: 600;
    padding: 0.65rem 0.9rem;
    cursor: pointer;
    white-space: nowrap;
    border-bottom: 2px solid transparent;
    transition: all 0.15s;
  }

  .tab-btn:hover {
    color: #f8fafc;
  }

  .tab-btn.active {
    color: #00f2fe;
    border-bottom-color: #00f2fe;
    background: rgba(0, 242, 254, 0.08);
  }

  .tab-container {
    padding: 1rem;
    flex: 1;
    overflow-y: auto;
  }

  .tab-pane {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .gauge-badge-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: rgba(0, 0, 0, 0.3);
    padding: 0.4rem 0.6rem;
    border-radius: 6px;
    font-size: 0.72rem;
  }

  .gauge-tag {
    color: #94a3b8;
    font-weight: 600;
  }

  .control-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
  }

  .control-grid-2x2 {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .control-block {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .harmonics-fader-bank {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    background: rgba(0, 0, 0, 0.25);
    padding: 0.75rem;
    border-radius: 8px;
  }

  .faders-row {
    display: flex;
    justify-content: space-between;
    height: 110px;
    align-items: flex-end;
  }

  .fader-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }

  .h-tag {
    font-size: 0.62rem;
    color: #94a3b8;
  }

  .h-val {
    font-size: 0.6rem;
    color: #38bdf8;
  }

  .vertical-fader {
    writing-mode: vertical-lr;
    direction: rtl;
    width: 16px;
    height: 70px;
    accent-color: #00f2fe;
    cursor: pointer;
  }

  .svg-preview-container {
    width: 100%;
    background: rgba(0, 0, 0, 0.45);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 8px;
    padding: 0.5rem;
    position: relative;
  }

  .svg-label {
    position: absolute;
    top: 6px;
    left: 10px;
    font-size: 0.62rem;
    color: #94a3b8;
  }

  .adsr-svg, .spectral-svg {
    width: 100%;
    height: 70px;
    display: block;
  }

  .formants-card-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.75rem;
  }

  .formant-box {
    background: rgba(0, 0, 0, 0.35);
    border: 1px solid rgba(192, 132, 252, 0.3);
    border-radius: 6px;
    padding: 0.5rem;
  }

  .formant-header {
    font-size: 0.72rem;
    font-weight: 700;
    color: #c084fc;
    display: block;
    margin-bottom: 0.35rem;
  }

  .modules-dual-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.75rem;
  }

  .sub-module-card {
    background: rgba(0, 0, 0, 0.3);
    border: 1px solid rgba(255, 255, 255, 0.08);
    border-radius: 6px;
    padding: 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .module-title {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.75rem;
    font-weight: 700;
    color: #f8fafc;
  }

  .toggle-control {
    font-size: 0.68rem;
    color: #38bdf8;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }

  .right-panel {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .display-canvas {
    width: 100%;
    height: 110px;
    background: #0f172a;
    border-radius: 4px;
    border: 1px solid rgba(255, 255, 255, 0.06);
  }

  .fps-badge {
    font-size: 0.62rem;
    color: #38bdf8;
  }

  .transport-info-box {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .transport-desc {
    font-size: 0.72rem;
    color: #94a3b8;
    line-height: 1.4;
    margin: 0;
  }

  .transport-main-btn {
    background: linear-gradient(135deg, #0ea5e9, #6366f1);
    color: #ffffff;
    border: none;
    padding: 0.55rem 0.8rem;
    border-radius: 6px;
    font-size: 0.78rem;
    font-weight: 700;
    cursor: pointer;
    box-shadow: 0 4px 14px rgba(14, 165, 233, 0.35);
    transition: transform 0.15s ease;
  }

  .transport-main-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 6px 18px rgba(14, 165, 233, 0.5);
  }

  .text-cyan { color: #00f2fe; }
  .text-amber { color: #f59e0b; }
</style>
