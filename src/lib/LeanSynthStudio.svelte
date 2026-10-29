<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, lineNumbers, keymap, highlightActiveLineGutter, highlightActiveLine } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';

  import type { Dx7Patch, Dx7Operator } from './synth/dx7_parser';
  import { parseDx7Syx } from './synth/dx7_parser';
  import { 
    DX7_TUB_BELLS, 
    DX7_E_PIANO_1, 
    DX7_SLAP_BASS, 
    CANONICAL_DX7_PRESETS 
  } from './synth/dx7_presets';
  import { 
    emitLeanCode, 
    parseLeanCode, 
    locateLeanParameterRange 
  } from './synth/lean_ast_codec';
  import Splitter from './ui/Splitter.svelte';
  import { 
    synthesizeDx7Patch, 
    synthesizePolyphonicChord, 
    midiToFrequency, 
    encodeWav16Mono,
    type SynthOptions,
    DX7_ALGORITHMS
  } from './synth/dx7_audio_engine';
  import { exportDx7SingleVoiceSyx } from './export/dx7_sysex_exporter';
  import { exportMultisampleSoundFont, triggerBinaryDownload } from './export/soundfont2_exporter';

  import OperatorMatrixWidget from './widgets/OperatorMatrixWidget.svelte';
  import EnvelopeStageWidget from './widgets/EnvelopeStageWidget.svelte';
  import ParameterKnobWidget from './widgets/ParameterKnobWidget.svelte';
  import type { WidgetDescriptor } from './widgets/types';

  interface Props {
    onNavigate?: (view: 'converter' | 'editor' | 'synth' | 'stn' | 'lean') => void;
    onTransportToEditor?: (wavBytes: Uint8Array) => void;
  }

  let { onNavigate, onTransportToEditor }: Props = $props();

  // --------------------------------------------------------------------------
  // ESTADO REATIVO DO STUDIO (SVELTE 5 RUNES)
  // --------------------------------------------------------------------------
  let currentPatch = $state<Dx7Patch>(JSON.parse(JSON.stringify(DX7_TUB_BELLS)));
  let leanSource = $state<string>(emitLeanCode(currentPatch));
  let activeOpId = $state<number>(1);
  let isPlaying = $state<boolean>(false);
  let activeMidiNotes = $state<number[]>([]);
  let copyFeedback = $state<string>('📋 Copiar Texto Puro');
  let leftPaneWidth = $state<number>(540);
  let statusText = $state<string>('PRONTO');

  // Referência ao elemento DOM do CodeMirror
  let editorContainer = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let fileInput = $state<HTMLInputElement | null>(null);

  // Web Audio Context para audição interativa imediata
  let audioCtx: AudioContext | null = null;
  let activeAudioSource: AudioBufferSourceNode | null = null;

  // Derivados
  let activeOperator = $derived(
    currentPatch.operators.find((op) => op.id === activeOpId) || currentPatch.operators[0]
  );
  let routing = $derived(
    DX7_ALGORITHMS[currentPatch.algorithm] || DX7_ALGORITHMS[1]
  );
  let isCarrier = $derived(routing.carriers.includes(activeOpId));

  // --------------------------------------------------------------------------
  // SINCRONIZAÇÃO CIRÚRGICA BIDIRECIONAL
  // --------------------------------------------------------------------------

  /**
   * Chamado quando qualquer controle/widget gráfico é alterado pelo usuário.
   * Realiza um despacho cirúrgico no CodeMirror [from, to] sem reformatar o documento.
   */
  function handleParamChange(paramPath: string, newValue: number | string | boolean) {
    if (!editorView) return;

    const currentDoc = editorView.state.doc.toString();
    const range = locateLeanParameterRange(currentDoc, paramPath);

    if (range) {
      editorView.dispatch({
        changes: {
          from: range.from,
          to: range.to,
          insert: String(newValue)
        }
      });
      leanSource = editorView.state.doc.toString();
    }

    // Atualiza o modelo de dados interno do patch
    applyParamToPatch(paramPath, newValue);
    // Toca uma prévia sutil ao alterar parâmetros
    triggerLivePreviewNote();
  }

  function applyParamToPatch(paramPath: string, value: any) {
    if (paramPath === 'algorithm') {
      currentPatch.algorithm = Number(value);
    } else if (paramPath === 'feedback') {
      currentPatch.feedback = Number(value);
    } else if (paramPath === 'transpose') {
      currentPatch.transpose = Number(value);
    } else {
      const opMatch = paramPath.match(/^op\.(\d+)\.(.+)$/);
      if (opMatch) {
        const opId = parseInt(opMatch[1], 10);
        const subParam = opMatch[2];
        const op = currentPatch.operators.find((o) => o.id === opId);
        if (!op) return;

        if (subParam === 'freqCoarse') op.freqCoarse = Number(value);
        else if (subParam === 'freqFine') op.freqFine = Number(value);
        else if (subParam === 'detune') op.detune = Number(value);
        else if (subParam === 'outputLevel') op.outputLevel = Number(value);
        else if (subParam === 'eg.r1') op.rates[0] = Number(value);
        else if (subParam === 'eg.r2') op.rates[1] = Number(value);
        else if (subParam === 'eg.r3') op.rates[2] = Number(value);
        else if (subParam === 'eg.r4') op.rates[3] = Number(value);
        else if (subParam === 'eg.l1') op.levels[0] = Number(value);
        else if (subParam === 'eg.l2') op.levels[1] = Number(value);
        else if (subParam === 'eg.l3') op.levels[2] = Number(value);
        else if (subParam === 'eg.l4') op.levels[3] = Number(value);
      }
    }
  }

  /**
   * Chamado quando o usuário digita diretamente no buffer de texto do CodeMirror.
   */
  function handleEditorDocumentChange(newCode: string) {
    leanSource = newCode;
    try {
      const parsed = parseLeanCode(newCode);
      currentPatch = parsed;
      statusText = 'SINCRONIZADO';
    } catch (e) {
      statusText = 'SINTAXE INCOMPLETA';
    }
  }

  // --------------------------------------------------------------------------
  // INICIALIZAÇÃO DO CODEMIRROR 6
  // --------------------------------------------------------------------------
  function initCodeMirror() {
    if (!editorContainer) return;

    const startState = EditorState.create({
      doc: leanSource,
      extensions: [
        lineNumbers(),
        highlightActiveLineGutter(),
        highlightActiveLine(),
        history(),
        keymap.of([...defaultKeymap, ...historyKeymap]),
        EditorView.updateListener.of((update) => {
          if (update.docChanged) {
            handleEditorDocumentChange(update.state.doc.toString());
          }
        }),
        EditorView.theme({
          '&': {
            height: '100%',
            fontSize: '12px',
            backgroundColor: '#050c15',
            color: '#d4e4f7',
            fontFamily: 'JetBrains Mono, Menlo, Monaco, "Courier New", monospace'
          },
          '.cm-content': {
            padding: '12px 4px'
          },
          '.cm-line': {
            lineHeight: '1.6'
          },
          '.cm-gutters': {
            backgroundColor: '#03080e',
            color: '#415a77',
            borderRight: '1px solid #13273c'
          },
          '.cm-activeLine': {
            backgroundColor: 'rgba(56, 189, 248, 0.06)'
          },
          '.cm-activeLineGutter': {
            backgroundColor: '#0a1d30',
            color: '#38bdf8'
          }
        })
      ]
    });

    editorView = new EditorView({
      state: startState,
      parent: editorContainer
    });
  }

  // --------------------------------------------------------------------------
  // SÍNTESE DE ÁUDIO E AUDIÇÃO IMEDIATA
  // --------------------------------------------------------------------------
  async function ensureAudioContext(): Promise<AudioContext> {
    const AC = window.AudioContext || (window as any).webkitAudioContext;
    if (!audioCtx) {
      audioCtx = new AC({ sampleRate: 48000 });
    }
    if (audioCtx.state === 'suspended') {
      await audioCtx.resume();
    }
    return audioCtx;
  }

  async function playMidiNote(midiNote: number, durSeconds = 1.8) {
    try {
      const ctx = await ensureAudioContext();
      const samples = synthesizeDx7Patch(currentPatch, {
        sampleRate: 48000,
        duration: durSeconds,
        fundamentalHz: midiToFrequency(midiNote),
        velocity: 0.9
      });

      const buffer = ctx.createBuffer(1, samples.length, 48000);
      buffer.getChannelData(0).set(samples);

      const src = ctx.createBufferSource();
      src.buffer = buffer;
      src.connect(ctx.destination);
      src.start();

      activeMidiNotes = [...activeMidiNotes, midiNote];
      src.onended = () => {
        activeMidiNotes = activeMidiNotes.filter((n) => n !== midiNote);
      };
    } catch (e) {
      console.warn('Erro ao reproduzir nota MIDI:', e);
    }
  }

  let previewDebounceTimer: any = null;
  function triggerLivePreviewNote() {
    if (previewDebounceTimer) clearTimeout(previewDebounceTimer);
    previewDebounceTimer = setTimeout(() => {
      playMidiNote(60, 0.5); // C4 rápido para preview
    }, 60);
  }

  // --------------------------------------------------------------------------
  // PRESETS E IMPORTAÇÃO/EXPORTAÇÃO
  // --------------------------------------------------------------------------
  function selectPreset(patch: Dx7Patch) {
    currentPatch = JSON.parse(JSON.stringify(patch));
    const newCode = emitLeanCode(currentPatch);
    leanSource = newCode;
    activeOpId = 1;

    if (editorView) {
      editorView.dispatch({
        changes: {
          from: 0,
          to: editorView.state.doc.length,
          insert: newCode
        }
      });
    }

    playMidiNote(60, 1.2);
  }

  async function handleFileUpload(e: Event) {
    const target = e.target as HTMLInputElement;
    if (!target.files || target.files.length === 0) return;
    const file = target.files[0];

    try {
      const arrayBuffer = await file.arrayBuffer();
      const bytes = new Uint8Array(arrayBuffer);
      const parsedPatches = parseDx7Syx(bytes);

      if (parsedPatches.length > 0) {
        selectPreset(parsedPatches[0]);
        statusText = `CARREGADO: ${parsedPatches.length} PATCHES`;
      }
    } catch (err: any) {
      console.error('Erro ao ler arquivo .syx:', err);
      alert(`Falha ao ler arquivo .syx: ${err.message}`);
    }
  }

  function copyPlainText() {
    if (!navigator?.clipboard) return;
    navigator.clipboard.writeText(leanSource);
    copyFeedback = '✓ Copiado com Sucesso!';
    setTimeout(() => {
      copyFeedback = '📋 Copiar Texto Puro';
    }, 2000);
  }

  function exportWav() {
    try {
      const samples = synthesizeDx7Patch(currentPatch, {
        sampleRate: 48000,
        duration: 2.5,
        fundamentalHz: 261.63,
        velocity: 0.9
      });
      const wavBytes = encodeWav16Mono(samples, 48000);
      const blob = new Blob([wavBytes as any], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `${currentPatch.name.trim().toLowerCase().replace(/[^a-z0-9]/g, '_')}.wav`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      console.error('Erro ao exportar WAV:', e);
    }
  }

  function downloadSyx() {
    try {
      const syxBytes = exportDx7SingleVoiceSyx(currentPatch);
      const filename = `${currentPatch.name.trim().toLowerCase().replace(/[^a-z0-9]/g, '_') || 'dx7_voice'}.syx`;
      triggerBinaryDownload(syxBytes, filename, 'application/x-syx');
    } catch (e) {
      console.error('Erro ao exportar SysEx DX7:', e);
    }
  }

  function downloadSoundFont() {
    try {
      const sf2Bytes = exportMultisampleSoundFont(currentPatch.name.trim() || 'DX7 Synth', (freqHz, durationSec) => {
        const samples = synthesizeDx7Patch(currentPatch, {
          sampleRate: 48000,
          duration: durationSec,
          fundamentalHz: freqHz,
          velocity: 0.85
        });
        const pcm16 = new Int16Array(samples.length);
        for (let i = 0; i < samples.length; i++) {
          pcm16[i] = Math.max(-32768, Math.min(32767, Math.round(samples[i] * 32767)));
        }
        return pcm16;
      });
      const filename = `${currentPatch.name.trim().toLowerCase().replace(/[^a-z0-9]/g, '_') || 'dx7_instrument'}.sf2`;
      triggerBinaryDownload(sf2Bytes, filename, 'audio/x-soundfont');
    } catch (e) {
      console.error('Erro ao exportar SoundFont 2:', e);
    }
  }

  function transportToEditor() {
    try {
      const samples = synthesizeDx7Patch(currentPatch, {
        sampleRate: 48000,
        duration: 2.0,
        fundamentalHz: 261.63
      });
      const wavBytes = encodeWav16Mono(samples, 48000);
      if (onTransportToEditor) onTransportToEditor(wavBytes);
      if (onNavigate) onNavigate('editor');
    } catch (e) {
      console.error('Erro ao transportar para WebGL Editor:', e);
    }
  }

  // Splitter resizer
  let isResizing = false;
  function startResize(e: MouseEvent) {
    e.preventDefault();
    isResizing = true;
    window.addEventListener('mousemove', onMouseMoveResize);
    window.addEventListener('mouseup', onMouseUpResize);
  }
  function onMouseMoveResize(e: MouseEvent) {
    if (!isResizing) return;
    leftPaneWidth = Math.max(300, Math.min(window.innerWidth - 350, e.clientX));
  }
  function onMouseUpResize() {
    isResizing = false;
    window.removeEventListener('mousemove', onMouseMoveResize);
    window.removeEventListener('mouseup', onMouseUpResize);
  }

  onMount(() => {
    initCodeMirror();
  });

  onDestroy(() => {
    if (editorView) editorView.destroy();
    if (audioCtx) audioCtx.close();
  });
</script>

<div class="lean-ide">
  <!-- Topbar Principal -->
  <header class="topbar">
    <div class="brand">
      <div class="logo">λ</div>
      <div>
        <strong>Lean 4 Synth IDE</strong>
        <small>Arquitetura Dual Código ↔ Widgets</small>
      </div>
    </div>

    <div class="divider"></div>

    <div class="preset-selector">
      <span class="lbl">Preset:</span>
      <select onchange={(e) => {
        const val = (e.target as HTMLSelectElement).value;
        const p = CANONICAL_DX7_PRESETS.find(x => x.name === val) || DX7_TUB_BELLS;
        selectPreset(p);
      }}>
        {#each CANONICAL_DX7_PRESETS as p}
          <option value={p.name} selected={p.name === currentPatch.name}>{p.name}</option>
        {/each}
      </select>

      <button class="upload-btn" onclick={() => fileInput?.click()} title="Importar banco ou patch DX7 (.syx)">
        📁 Subir .syx
      </button>
      <input 
        bind:this={fileInput} 
        type="file" 
        accept=".syx,.bin" 
        style="display: none;" 
        onchange={handleFileUpload} 
      />
    </div>

    <div class="grow"></div>

    <span class="status-pill">{statusText}</span>

    <div class="actions">
      <button class="copy-btn" onclick={copyPlainText} title="Copia o código Lean 4 desformatado">{copyFeedback}</button>
      <button class="export-btn" onclick={exportWav} title="Sintetizar e baixar WAV">Exportar WAV</button>
      <button class="syx-btn" onclick={downloadSyx} title="Baixar arquivo SysEx .syx para Yamaha DX7, Volca FM ou Dexed">🎹 .SYX (DX7)</button>
      <button class="sf2-btn" onclick={downloadSoundFont} title="Baixar SoundFont 2 (.sf2) multisample de 5 oitavas">🎼 .SF2</button>
      <button class="transport-btn" onclick={transportToEditor} title="Transportar áudio gerado para o WebGL Editor">Editor 2D ↗</button>
    </div>
  </header>

  <!-- Barra de Audição e Teclado Musical -->
  <div class="transport-bar">
    <button class="play-chord-btn" onclick={() => playMidiNote(60, 2.0)}>
      ▶ Tocar C4 (261 Hz)
    </button>
    <button class="play-chord-btn secondary" onclick={() => {
      // Toca tríade C4, E4, G4
      playMidiNote(60, 2.0);
      setTimeout(() => playMidiNote(64, 2.0), 30);
      setTimeout(() => playMidiNote(67, 2.0), 60);
    }}>
      ♫ Acorde Maior (C-E-G)
    </button>

    <div class="mini-keyboard">
      {#each [
        { note: 'C3', midi: 48, white: true },
        { note: 'C#3', midi: 49, white: false },
        { note: 'D3', midi: 50, white: true },
        { note: 'D#3', midi: 51, white: false },
        { note: 'E3', midi: 52, white: true },
        { note: 'F3', midi: 53, white: true },
        { note: 'F#3', midi: 54, white: false },
        { note: 'G3', midi: 55, white: true },
        { note: 'G#3', midi: 56, white: false },
        { note: 'A3', midi: 57, white: true },
        { note: 'A#3', midi: 58, white: false },
        { note: 'B3', midi: 59, white: true },
        { note: 'C4', midi: 60, white: true },
        { note: 'C#4', midi: 61, white: false },
        { note: 'D4', midi: 62, white: true },
        { note: 'D#4', midi: 63, white: false },
        { note: 'E4', midi: 64, white: true },
        { note: 'F4', midi: 65, white: true },
        { note: 'F#4', midi: 66, white: false },
        { note: 'G4', midi: 67, white: true },
        { note: 'G#4', midi: 68, white: false },
        { note: 'A4', midi: 69, white: true },
        { note: 'A#4', midi: 70, white: false },
        { note: 'B4', midi: 71, white: true },
        { note: 'C5', midi: 72, white: true }
      ] as k}
        <button 
          class="key {k.white ? 'white' : 'black'}" 
          class:pressed={activeMidiNotes.includes(k.midi)}
          onmousedown={() => playMidiNote(k.midi)}
          title="{k.note} ({Math.round(midiToFrequency(k.midi))} Hz)"
        >
          {k.note}
        </button>
      {/each}
    </div>
  </div>

  <!-- Layout Split IDE (Editor de Código à Esquerda / Widgets à Direita) -->
  <div class="workspace-split" style="grid-template-columns: {leftPaneWidth}px 6px 1fr;">
    <!-- PAINEL ESQUERDO: CODEMIRROR 6 -->
    <section class="editor-pane">
      <div class="pane-header">
        <span>📄 Código Lean 4 (Texto Puro Copiável)</span>
        <small class="doc-badge">AST Tipada</small>
      </div>
      <div class="codemirror-wrapper" bind:this={editorContainer}></div>
    </section>

    <!-- SPLITTER DRAG HANDLE -->
    <Splitter bind:size={leftPaneWidth} minFirst={280} minSecond={350} />

    <!-- PAINEL DIREITO: WIDGETS SEMÂNTICOS SVELTE 5 -->
    <section class="widgets-pane">
      <div class="pane-header">
        <span>🧩 Módulos do Sintetizador & Widgets Semânticos</span>
        <span class="active-tag">{currentPatch.name.trim()}</span>
      </div>

      <div class="widgets-scrollable">
        <!-- 1. Matriz de Algoritmo FM -->
        <OperatorMatrixWidget 
          descriptor={{
            id: 'widget-fm-matrix',
            nodeId: 'node-algorithm',
            revision: 1,
            from: 0,
            to: 0,
            kind: 'fm-algorithm',
            title: 'Topologia FM dos 6 Operadores',
            parameters: {
              algorithm: currentPatch.algorithm,
              feedback: currentPatch.feedback
            }
          }}
          onParamChange={handleParamChange}
        />

        <!-- 2. Abas de Seleção de Operador (Op 1 a Op 6) -->
        <div class="op-tabs-bar">
          {#each [1, 2, 3, 4, 5, 6] as id}
            {@const isCar = routing.carriers.includes(id)}
            {@const opObj = currentPatch.operators.find(o => o.id === id)}
            <button 
              class="op-tab-btn" 
              class:active={activeOpId === id}
              class:carrier={isCar}
              onclick={() => activeOpId = id}
            >
              <span class="op-badge">{isCar ? 'CAR' : 'MOD'}</span>
              <strong>Op {id}</strong>
              <small>{opObj ? `${opObj.freqCoarse}.${opObj.freqFine.toString().padStart(2, '0')}x` : '1.0x'}</small>
            </button>
          {/each}
        </div>

        <!-- 3. Envelope Gráfico do Operador Selecionado -->
        <EnvelopeStageWidget
          descriptor={{
            id: `widget-eg-op-${activeOpId}`,
            nodeId: `node-op-${activeOpId}-eg`,
            revision: 1,
            from: 0,
            to: 0,
            kind: 'envelope-stage',
            title: `Envelope de 4 Estágios · Operador ${activeOpId} (${isCarrier ? 'Portadora' : 'Modulador'})`,
            parameters: {
              opId: activeOpId,
              r1: activeOperator.rates[0],
              r2: activeOperator.rates[1],
              r3: activeOperator.rates[2],
              r4: activeOperator.rates[3],
              l1: activeOperator.levels[0],
              l2: activeOperator.levels[1],
              l3: activeOperator.levels[2],
              l4: activeOperator.levels[3]
            }
          }}
          onParamChange={handleParamChange}
        />

        <!-- 4. Knobs de Afinação e Nível do Operador Selecionado -->
        <ParameterKnobWidget
          descriptor={{
            id: `widget-params-op-${activeOpId}`,
            nodeId: `node-op-${activeOpId}-params`,
            revision: 1,
            from: 0,
            to: 0,
            kind: 'parameter-knob',
            title: `Afinação & Nível · Operador ${activeOpId}`,
            parameters: {
              opId: activeOpId,
              freqCoarse: activeOperator.freqCoarse,
              freqFine: activeOperator.freqFine,
              detune: activeOperator.detune,
              outputLevel: activeOperator.outputLevel,
              velSens: activeOperator.velSens
            }
          }}
          onParamChange={handleParamChange}
        />
      </div>
    </section>
  </div>
</div>

<style>
  :root {
    --bg-dark: #03080e;
    --bg-pane: #06111d;
    --border-color: #172d42;
    --accent-blue: #38bdf8;
    --accent-green: #22d69a;
    --accent-amber: #f59e0b;
    --text-primary: #e5f1fb;
    --text-muted: #8ca5ba;
  }

  .lean-ide {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: var(--bg-dark);
    color: var(--text-primary);
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, sans-serif;
    font-size: 12px;
    user-select: none;
  }

  /* Topbar Header */
  .topbar {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 52px;
    padding: 0 16px;
    background: #06121e;
    border-bottom: 1px solid var(--border-color);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo {
    width: 28px;
    height: 28px;
    border: 1px solid var(--accent-blue);
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: var(--accent-blue);
    font-size: 16px;
    font-weight: 800;
    background: rgba(56, 189, 248, 0.1);
  }

  .brand strong {
    font-size: 13px;
    color: #fff;
  }

  .brand small {
    display: block;
    color: var(--text-muted);
    font-size: 9px;
  }

  .divider {
    width: 1px;
    height: 26px;
    background: var(--border-color);
  }

  .preset-selector {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .lbl {
    color: var(--text-muted);
    font-size: 11px;
  }

  select {
    background: #0b1a29;
    border: 1px solid #1f3d59;
    color: #fff;
    padding: 5px 8px;
    border-radius: 5px;
    font-size: 11px;
    outline: none;
  }

  .upload-btn {
    background: #0f2438;
    border: 1px solid #204160;
    color: var(--text-primary);
    padding: 5px 9px;
    border-radius: 5px;
    font-size: 11px;
    cursor: pointer;
  }

  .upload-btn:hover {
    background: #173857;
    border-color: var(--accent-blue);
  }

  .grow {
    flex: 1;
  }

  .status-pill {
    font-size: 10px;
    color: var(--accent-green);
    padding: 3px 8px;
    border: 1px solid rgba(34, 214, 154, 0.3);
    border-radius: 99px;
    font-weight: 700;
  }

  .actions {
    display: flex;
    gap: 8px;
  }

  .copy-btn {
    background: #133a5b;
    border: 1px solid #285d88;
    color: #fff;
    padding: 6px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-weight: 600;
    font-size: 11px;
  }

  .copy-btn:hover {
    background: #184b77;
    border-color: var(--accent-blue);
  }

  .export-btn, .transport-btn {
    background: #0e2032;
    border: 1px solid var(--border-color);
    color: var(--text-primary);
    padding: 6px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-size: 11px;
  }

  .export-btn:hover, .transport-btn:hover {
    border-color: var(--accent-blue);
  }

  .syx-btn, .sf2-btn {
    padding: 6px 10px;
    border-radius: 5px;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s;
  }

  .syx-btn {
    background: rgba(245, 158, 11, 0.15);
    color: #fbbf24;
    border: 1px solid rgba(245, 158, 11, 0.35);
  }

  .syx-btn:hover {
    background: rgba(245, 158, 11, 0.3);
    color: #ffffff;
    border-color: #f59e0b;
  }

  .sf2-btn {
    background: rgba(168, 85, 247, 0.15);
    color: #c084fc;
    border: 1px solid rgba(168, 85, 247, 0.35);
  }

  .sf2-btn:hover {
    background: rgba(168, 85, 247, 0.3);
    color: #ffffff;
    border-color: #a855f7;
  }

  /* Transport & Keyboard Bar */
  .transport-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 14px;
    background: #040d16;
    border-bottom: 1px solid var(--border-color);
  }

  .play-chord-btn {
    background: #183e60;
    border: 1px solid #2a5a84;
    color: #fff;
    padding: 5px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-weight: 600;
    font-size: 11px;
  }

  .play-chord-btn:hover {
    background: #20537f;
    border-color: var(--accent-blue);
  }

  .play-chord-btn.secondary {
    background: #0f273d;
    border-color: #1e4566;
  }

  .mini-keyboard {
    display: flex;
    margin-left: auto;
    position: relative;
    height: 38px;
  }

  .key {
    cursor: pointer;
    font-size: 7px;
    border-radius: 0 0 3px 3px;
    outline: none;
    transition: background 0.05s;
  }

  .key.white {
    width: 20px;
    height: 38px;
    background: #dce7f2;
    color: #334155;
    border: 1px solid #94a3b8;
    z-index: 1;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 2px;
  }

  .key.white:hover {
    background: #f1f5f9;
  }

  .key.white.pressed {
    background: var(--accent-blue);
    color: #fff;
  }

  .key.black {
    width: 12px;
    height: 24px;
    background: #0f172a;
    color: #cbd5e1;
    border: 1px solid #020617;
    margin-left: -6px;
    margin-right: -6px;
    z-index: 2;
    font-size: 6px;
    display: flex;
    align-items: flex-end;
    justify-content: center;
    padding-bottom: 2px;
  }

  .key.black:hover {
    background: #1e293b;
  }

  .key.black.pressed {
    background: #0284c7;
    color: #fff;
  }

  /* Workspace Split */
  .workspace-split {
    display: grid;
    flex: 1;
    overflow: hidden;
  }

  .splitter {
    width: 6px;
    cursor: col-resize;
    background: #081624;
    border-left: 1px solid #13273c;
    border-right: 1px solid #13273c;
    transition: background 0.15s;
  }

  .splitter:hover {
    background: var(--accent-blue);
  }

  .editor-pane, .widgets-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: var(--bg-pane);
  }

  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 12px;
    background: #081726;
    border-bottom: 1px solid var(--border-color);
    font-size: 11px;
    font-weight: 700;
  }

  .doc-badge {
    background: #142a3e;
    color: var(--accent-blue);
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .active-tag {
    color: var(--accent-green);
    font-size: 10px;
    font-weight: 600;
  }

  .codemirror-wrapper {
    flex: 1;
    overflow: auto;
  }

  .widgets-scrollable {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  /* Op Tabs Bar */
  .op-tabs-bar {
    display: grid;
    grid-template-columns: repeat(6, 1fr);
    gap: 6px;
    background: #061422;
    padding: 6px;
    border-radius: 6px;
    border: 1px solid var(--border-color);
  }

  .op-tab-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    background: #0b1c2d;
    border: 1px solid #1a354c;
    padding: 6px 4px;
    border-radius: 5px;
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.1s;
  }

  .op-tab-btn:hover {
    background: #132b42;
    color: #fff;
  }

  .op-tab-btn.active {
    background: #173856;
    border-color: var(--accent-blue);
    color: #fff;
  }

  .op-tab-btn.carrier .op-badge {
    background: rgba(34, 214, 154, 0.2);
    color: var(--accent-green);
    border: 1px solid rgba(34, 214, 154, 0.4);
  }

  .op-badge {
    font-size: 8px;
    font-weight: 800;
    padding: 1px 4px;
    border-radius: 3px;
    background: rgba(56, 189, 248, 0.2);
    color: var(--accent-blue);
    border: 1px solid rgba(56, 189, 248, 0.4);
  }

  .op-tab-btn strong {
    font-size: 11px;
  }

  .op-tab-btn small {
    font-size: 9px;
  }
</style>
