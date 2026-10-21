<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { EditorState } from '@codemirror/state';
  import { EditorView, lineNumbers, keymap, highlightActiveLineGutter, highlightActiveLine } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';

  import { 
    computeTaylorJetAt, 
    evaluateBaseFunction, 
    computeHessianEigensystem,
    type ScalarFieldModel, 
    type TaylorJet, 
    type BaseFunctionType,
    type Point2D,
    type BrushStroke
  } from './scalar_field/taylor_jet_math';

  import { 
    emitLeanScalarField, 
    parseLeanScalarField, 
    locateScalarFieldParamRange 
  } from './scalar_field/scalar_field_codec';

  import { 
    ScalarFieldGlRenderer, 
    type ColorPaletteType 
  } from './scalar_field/ScalarFieldGlRenderer';

  import { 
    PolarHandlesManager, 
    type PolarHandle, 
    screenToWorld 
  } from './scalar_field/PolarHandlesOverlay';

  import { 
    InertialPointerFilter,
    computeEquipotentialSnap
  } from './scalar_field/BrushEngine';

  import { 
    computeKinematics, 
    verifyKinematicChainRule, 
    type KinematicPoint, 
    type ChainRuleVerificationResult 
  } from './scalar_field/kinematic_chain_analyzer';

  import { 
    synthesizeOrbitWavetable, 
    synthesizeSpectralSurface, 
    encodeScalarFieldWav 
  } from './scalar_field/ScalarFieldAudioEngine';

  import KinematicProfileViewer from './scalar_field/KinematicProfileViewer.svelte';

  interface Props {
    onNavigate?: (view: 'converter' | 'editor' | 'synth' | 'stn' | 'lean' | 'field') => void;
    onTransportToEditor?: (wavBytes: Uint8Array) => void;
  }

  let { onNavigate, onTransportToEditor }: Props = $props();

  // --------------------------------------------------------------------------
  // ESTADO CENTRAL DO CAMPO ESCALAR (SVELTE 5 RUNES)
  // --------------------------------------------------------------------------
  let fieldModel = $state<ScalarFieldModel>({
    domain: { xMin: -2.0, xMax: 2.0, yMin: -2.0, yMax: 2.0 },
    baseFunction: { type: 'saddle', gain: 1.0, bias: 0.0, sigma: 1.0 },
    anchor: { x: 0.0, y: 0.0 },
    maxOrder: 8,
    brushLayers: []
  });

  let leanSource = $state<string>(emitLeanScalarField(fieldModel));
  let selectedOrder = $state<number>(2);
  let activeHandleId = $state<string | null>(null);
  let currentPalette = $state<ColorPaletteType>('inferno');
  let isolinesEnabled = $state<boolean>(true);
  let isolineSpacing = $state<number>(0.5);

  // Ferramentas de Interação: 'select_jet' ou 'brush'
  let currentTool = $state<'select_jet' | 'brush'>('select_jet');
  let brushOp = $state<'add' | 'sub' | 'smooth' | 'snap'>('add');
  let brushRadius = $state<number>(0.3);
  let brushIntensity = $state<number>(0.8);
  let inertialAlpha = $state<number>(0.5);

  // HUD no Cursor
  let cursorHud = $state({
    visible: false,
    screenX: 0,
    screenY: 0,
    worldX: 0,
    worldY: 0,
    val: 0,
    gradMag: 0,
    typeStr: 'Ponto de Sela'
  });

  // Cinemática e Perfil 1D
  let activeKinematics = $state<KinematicPoint[]>([]);
  let activeFieldValues = $state<number[]>([]);
  let activeVerification = $state<ChainRuleVerificationResult | null>(null);

  // Síntese Sonora Dupla
  let audioMode = $state<'orbit' | 'spectral'>('orbit');
  let isPlayingAudio = $state<boolean>(false);
  let fundamentalHz = $state<number>(220.0);
  let copyFeedback = $state<string>('📋 Copiar Texto Puro');
  let splitWidth = $state<number>(500);

  // Elementos do DOM e Instâncias
  let glCanvas = $state<HTMLCanvasElement | null>(null);
  let overlayCanvas = $state<HTMLCanvasElement | null>(null);
  let editorContainer = $state<HTMLDivElement | null>(null);
  let editorView: EditorView | null = null;
  let glRenderer: ScalarFieldGlRenderer | null = null;
  let handlesManager: PolarHandlesManager | null = null;
  let pointerFilter = new InertialPointerFilter();

  let audioCtx: AudioContext | null = null;
  let audioSource: AudioBufferSourceNode | null = null;

  // Derivados Matemáticos
  let currentJet = $derived(
    computeTaylorJetAt(fieldModel, fieldModel.anchor.x, fieldModel.anchor.y, fieldModel.maxOrder)
  );

  // --------------------------------------------------------------------------
  // SINCRONIZAÇÃO CIRÚRGICA BIDIRECIONAL
  // --------------------------------------------------------------------------
  function handleParamChange(paramPath: string, newValue: number | string) {
    if (!editorView) return;

    const currentDoc = editorView.state.doc.toString();
    const range = locateScalarFieldParamRange(currentDoc, paramPath);

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

    if (paramPath === 'base.gain') fieldModel.baseFunction.gain = Number(newValue);
    else if (paramPath === 'base.bias') fieldModel.baseFunction.bias = Number(newValue);
    else if (paramPath === 'anchor.x') fieldModel.anchor.x = Number(newValue);
    else if (paramPath === 'anchor.y') fieldModel.anchor.y = Number(newValue);

    renderAllViews();
  }

  function handleEditorDocumentChange(newCode: string) {
    leanSource = newCode;
    try {
      const parsed = parseLeanScalarField(newCode);
      fieldModel = parsed;
      renderAllViews();
    } catch {}
  }

  // --------------------------------------------------------------------------
  // RENDERIZAÇÃO E ATUALIZAÇÃO DOS CANVASES
  // --------------------------------------------------------------------------
  function renderAllViews() {
    if (glRenderer) {
      glRenderer.setDomain(fieldModel.domain);
      glRenderer.setBaseFunction(fieldModel.baseFunction);
      glRenderer.setPalette(currentPalette);
      glRenderer.setIsolines(isolinesEnabled, isolineSpacing);
      glRenderer.setBrushes(fieldModel.brushLayers);
      glRenderer.render();
    }

    if (overlayCanvas && handlesManager) {
      const ctx = overlayCanvas.getContext('2d');
      if (ctx) {
        ctx.clearRect(0, 0, overlayCanvas.width, overlayCanvas.height);
        const handles = handlesManager.computeHandles(currentJet);
        handlesManager.draw(ctx, handles, currentJet, activeHandleId, selectedOrder);
      }
    }
  }

  // --------------------------------------------------------------------------
  // INTERAÇÃO COM O MOUSE E CANETA NO CANVAS
  // --------------------------------------------------------------------------
  let isDraggingHandle = false;
  let isDrawingBrush = false;
  let currentStrokePoints: Point2D[] = [];
  let currentStrokeTimes: number[] = [];

  function handlePointerDown(e: PointerEvent) {
    if (!overlayCanvas || !handlesManager) return;
    const rect = overlayCanvas.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;

    overlayCanvas.setPointerCapture(e.pointerId);

    if (currentTool === 'select_jet') {
      const handles = handlesManager.computeHandles(currentJet);
      const hit = handlesManager.hitTest(px, py, handles, 14, activeHandleId || undefined);

      if (hit) {
        activeHandleId = hit.id;
        selectedOrder = hit.order;
        isDraggingHandle = true;
        renderAllViews();
      } else {
        // Clicar fora reposiciona o ponto de ancoragem do jato
        const wld = screenToWorld(px, py, overlayCanvas.width, overlayCanvas.height, fieldModel.domain);
        fieldModel.anchor.x = parseFloat(wld.x.toFixed(2));
        fieldModel.anchor.y = parseFloat(wld.y.toFixed(2));
        handleParamChange('anchor.x', fieldModel.anchor.x);
        handleParamChange('anchor.y', fieldModel.anchor.y);
      }
    } else if (currentTool === 'brush') {
      isDrawingBrush = true;
      const wld = screenToWorld(px, py, overlayCanvas.width, overlayCanvas.height, fieldModel.domain);
      pointerFilter.reset(wld.x, wld.y);
      currentStrokePoints = [{ x: wld.x, y: wld.y }];
      currentStrokeTimes = [performance.now()];
    }
  }

  function handlePointerMove(e: PointerEvent) {
    if (!overlayCanvas || !handlesManager) return;
    const rect = overlayCanvas.getBoundingClientRect();
    const px = e.clientX - rect.left;
    const py = e.clientY - rect.top;
    const wld = screenToWorld(px, py, overlayCanvas.width, overlayCanvas.height, fieldModel.domain);

    // Atualiza HUD no cursor
    const curVal = evaluateBaseFunction(fieldModel.baseFunction.type, wld.x, wld.y, fieldModel.baseFunction.gain, fieldModel.baseFunction.bias);
    cursorHud.visible = true;
    cursorHud.screenX = px;
    cursorHud.screenY = py;
    cursorHud.worldX = wld.x;
    cursorHud.worldY = wld.y;
    cursorHud.val = curVal;

    if (isDraggingHandle && activeHandleId) {
      // Arraste radial da alça selecionada
      const handles = handlesManager.computeHandles(currentJet);
      const h = handles.find((x) => x.id === activeHandleId);
      if (h) {
        if (h.order === 0) {
          fieldModel.baseFunction.bias = parseFloat(curVal.toFixed(2));
          handleParamChange('base.bias', fieldModel.baseFunction.bias);
        } else if (h.order === 1) {
          fieldModel.baseFunction.gain = parseFloat(Math.hypot(wld.x, wld.y).toFixed(2));
          handleParamChange('base.gain', fieldModel.baseFunction.gain);
        }
      }
    } else if (isDrawingBrush) {
      // Aplica filtro inercial para suavização
      const smoothed = pointerFilter.update(wld.x, wld.y, inertialAlpha);
      currentStrokePoints.push({ x: smoothed.x, y: smoothed.y });
      currentStrokeTimes.push(performance.now());

      // Desenha prévia temporária no overlay
      const ctx = overlayCanvas.getContext('2d');
      if (ctx) {
        ctx.strokeStyle = '#38bdf8';
        ctx.lineWidth = 3.0;
        ctx.beginPath();
        const start = currentStrokePoints[0];
        ctx.moveTo(px, py);
        ctx.arc(px, py, brushRadius * 40, 0, Math.PI * 2);
        ctx.stroke();
      }
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (!overlayCanvas) return;
    overlayCanvas.releasePointerCapture(e.pointerId);

    if (isDraggingHandle) {
      isDraggingHandle = false;
      renderAllViews();
    } else if (isDrawingBrush) {
      isDrawingBrush = false;

      if (currentStrokePoints.length > 2) {
        const newStroke: BrushStroke = {
          id: `stroke_${fieldModel.brushLayers.length + 1}`,
          operator: brushOp,
          radius: brushRadius,
          intensity: brushIntensity,
          points: [...currentStrokePoints],
          profile1D: currentStrokePoints.map(() => 1.0)
        };
        fieldModel.brushLayers = [...fieldModel.brushLayers, newStroke];

        // Avalia cinemática da trajetória desenhada
        const startTime = currentStrokeTimes[0];
        const timedTraj = currentStrokePoints.map((pt, idx) => ({
          t: (currentStrokeTimes[idx] - startTime) * 0.001,
          x: pt.x,
          y: pt.y
        }));

        activeKinematics = computeKinematics(timedTraj);
        activeFieldValues = currentStrokePoints.map((p) =>
          evaluateBaseFunction(fieldModel.baseFunction.type, p.x, p.y, fieldModel.baseFunction.gain, fieldModel.baseFunction.bias)
        );
        activeVerification = verifyKinematicChainRule(fieldModel, activeKinematics);

        // Atualiza código Lean 4
        leanSource = emitLeanScalarField(fieldModel);
        if (editorView) {
          editorView.dispatch({
            changes: { from: 0, to: editorView.state.doc.length, insert: leanSource }
          });
        }
      }

      currentStrokePoints = [];
      currentStrokeTimes = [];
      renderAllViews();
    }
  }

  // --------------------------------------------------------------------------
  // SÍNTESE SONORA EM TEMPO REAL
  // --------------------------------------------------------------------------
  async function togglePlayAudio() {
    if (isPlayingAudio) {
      stopAudio();
      return;
    }

    try {
      const AC = window.AudioContext || (window as any).webkitAudioContext;
      if (!audioCtx) audioCtx = new AC({ sampleRate: 48000 });
      if (audioCtx.state === 'suspended') await audioCtx.resume();

      const samples =
        audioMode === 'orbit'
          ? synthesizeOrbitWavetable(fieldModel, { durationSeconds: 2.5, fundamentalHz })
          : synthesizeSpectralSurface(fieldModel, { durationSeconds: 2.5, fundamentalHz });

      const buffer = audioCtx.createBuffer(1, samples.length, 48000);
      buffer.getChannelData(0).set(samples);

      audioSource = audioCtx.createBufferSource();
      audioSource.buffer = buffer;
      audioSource.connect(audioCtx.destination);
      audioSource.start();

      isPlayingAudio = true;
      audioSource.onended = () => {
        isPlayingAudio = false;
      };
    } catch (e) {
      console.warn('Erro ao reproduzir áudio do campo:', e);
      isPlayingAudio = false;
    }
  }

  function stopAudio() {
    if (audioSource) {
      try { audioSource.stop(); } catch {}
      audioSource.disconnect();
      audioSource = null;
    }
    isPlayingAudio = false;
  }

  function exportWav() {
    try {
      const samples =
        audioMode === 'orbit'
          ? synthesizeOrbitWavetable(fieldModel, { durationSeconds: 3.0, fundamentalHz })
          : synthesizeSpectralSurface(fieldModel, { durationSeconds: 3.0, fundamentalHz });

      const wavBytes = encodeScalarFieldWav(samples, 48000);
      const blob = new Blob([wavBytes as any], { type: 'audio/wav' });
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = `scalar_field_${fieldModel.baseFunction.type}_${audioMode}.wav`;
      a.click();
      setTimeout(() => URL.revokeObjectURL(url), 1000);
    } catch (e) {
      console.error('Erro ao exportar WAV:', e);
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
            backgroundColor: '#040d16',
            color: '#d4e4f7',
            fontFamily: 'JetBrains Mono, Menlo, Monaco, monospace'
          },
          '.cm-content': { padding: '12px 6px' },
          '.cm-gutters': { backgroundColor: '#02070d', color: '#415a77', borderRight: '1px solid #13273c' }
        })
      ]
    });

    editorView = new EditorView({
      state: startState,
      parent: editorContainer
    });
  }

  onMount(() => {
    initCodeMirror();
    if (glCanvas) {
      glRenderer = new ScalarFieldGlRenderer(glCanvas);
    }
    if (overlayCanvas) {
      handlesManager = new PolarHandlesManager(overlayCanvas.width, overlayCanvas.height, fieldModel.domain);
    }
    renderAllViews();
  });

  onDestroy(() => {
    if (editorView) editorView.destroy();
    if (glRenderer) glRenderer.destroy();
    if (audioCtx) audioCtx.close();
  });
</script>

<div class="field-ide-app">
  <!-- Topbar Principal -->
  <header class="topbar">
    <div class="brand">
      <div class="logo">∇²</div>
      <div>
        <strong>Scalar Field & Taylor Jets IDE</strong>
        <small>Campos 2D, Jatos (Ordens 0 a 8) e Brushes</small>
      </div>
    </div>

    <div class="divider"></div>

    <div class="tool-controls">
      <span class="lbl">Função Base:</span>
      <select bind:value={fieldModel.baseFunction.type} onchange={renderAllViews}>
        <option value="saddle">Ponto de Sela (x² - y²)</option>
        <option value="basin">Bacia Parabólica (x² + y²)</option>
        <option value="waves">Ondas Mistas (sin 3x · cos 3y)</option>
        <option value="gaussian">Gaussiana Localizada</option>
        <option value="procedural">Morfologia Não-Separável</option>
        <option value="zero">Plano Nulo (Zero)</option>
      </select>

      <span class="lbl">Paleta:</span>
      <select bind:value={currentPalette} onchange={renderAllViews}>
        <option value="inferno">Inferno</option>
        <option value="magma">Magma</option>
        <option value="viridis">Viridis</option>
        <option value="mono">Monocromático</option>
      </select>

      <label class="check-opt">
        <input type="checkbox" bind:checked={isolinesEnabled} onchange={renderAllViews} />
        Isolinhas
      </label>
    </div>

    <div class="grow"></div>

    <div class="actions">
      <button class="copy-btn" onclick={copyPlainText}>{copyFeedback}</button>
      <button class="export-btn" onclick={exportWav}>Exportar WAV</button>
    </div>
  </header>

  <!-- Toolbar Secundária: Modos de Edição e Áudio -->
  <div class="sub-toolbar">
    <div class="pill-group">
      <button class:active={currentTool === 'select_jet'} onclick={() => currentTool = 'select_jet'}>
        🎯 Jatos de Taylor (Ordens 0..8)
      </button>
      <button class:active={currentTool === 'brush'} onclick={() => currentTool = 'brush'}>
        🖌️ Brush Espacial
      </button>
    </div>

    {#if currentTool === 'brush'}
      <div class="brush-settings">
        <select bind:value={brushOp}>
          <option value="add">➕ Elevar (Add)</option>
          <option value="sub">➖ Rebaixar (Sub)</option>
          <option value="smooth">🌊 Suavizar (Smooth)</option>
          <option value="snap">🧲 Atração Equipotencial</option>
        </select>

        <span class="lbl">Raio:</span>
        <input type="range" min="0.1" max="0.8" step="0.05" bind:value={brushRadius} />

        <span class="lbl">Inércia:</span>
        <input type="range" min="0.1" max="1.0" step="0.05" bind:value={inertialAlpha} />
      </div>
    {/if}

    <div class="grow"></div>

    <div class="audio-panel">
      <span class="lbl">Modo de Síntese:</span>
      <select bind:value={audioMode}>
        <option value="orbit">Modo A: Órbita / Wavetable</option>
        <option value="spectral">Modo B: Superfície DDSP</option>
      </select>

      <span class="lbl">f₀:</span>
      <input type="number" min="40" max="1000" bind:value={fundamentalHz} class="freq-input" /> Hz

      <button class="play-btn" class:active={isPlayingAudio} onclick={togglePlayAudio}>
        {isPlayingAudio ? '⏹ Parar' : '▶ Ouvir Campo'}
      </button>
    </div>
  </div>

  <!-- Workspace Split: Editor CodeMirror à esquerda, Canvas Central à direita -->
  <div class="workspace-split" style="grid-template-columns: {splitWidth}px 6px 1fr;">
    <!-- PAINEL ESQUERDO: CODEMIRROR 6 -->
    <section class="editor-pane">
      <div class="pane-header">
        <span>📄 Código Lean 4 (Texto Puro Copiável)</span>
        <span class="badge">Definição Analítica</span>
      </div>
      <div class="editor-wrapper" bind:this={editorContainer}></div>
    </section>

    <!-- SPLITTER -->
    <div class="splitter" title="Arrastar para ajustar largura"></div>

    <!-- PAINEL DIREITO: CANVAS HÍBRIDO (WEBGL 2 + OVERLAY 2D) -->
    <section class="canvas-pane">
      <div class="pane-header">
        <span>🎨 Campo Escalar f(x, y) & Alças Polares</span>
        <span class="badge">Ordem Ativa: {selectedOrder}</span>
      </div>

      <div class="canvas-stack-container">
        <!-- Canvas 1: Fundo WebGL 2 (Campo + Isolinhas a 60 FPS) -->
        <canvas bind:this={glCanvas} width={800} height={460} class="gl-canvas"></canvas>

        <!-- Canvas 2: Overlay 2D Vetorial (Alças Polares e Trajetórias) -->
        <canvas 
          bind:this={overlayCanvas} 
          width={800} 
          height={460} 
          class="overlay-canvas"
          onpointerdown={handlePointerDown}
          onpointermove={handlePointerMove}
          onpointerup={handlePointerUp}
        ></canvas>

        <!-- HUD Dinâmico no Cursor -->
        {#if cursorHud.visible}
          <div class="cursor-hud" style="left: {cursorHud.screenX + 15}px; top: {cursorHud.screenY + 15}px;">
            <div>Pos: ({cursorHud.worldX.toFixed(2)}, {cursorHud.worldY.toFixed(2)})</div>
            <div>f(x,y): <strong>{cursorHud.val.toFixed(3)}</strong></div>
            <div>Gradiente: ||∇f|| = {cursorHud.gradMag.toFixed(2)}</div>
          </div>
        {/if}
      </div>

      <!-- Painel Inferior: Gráfico 1D q(t) e Verificação Cinemática -->
      <div class="profile-container">
        <KinematicProfileViewer 
          kinematics={activeKinematics}
          fieldValues={activeFieldValues}
          verification={activeVerification}
        />
      </div>
    </section>
  </div>
</div>

<style>
  .field-ide-app {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    background: #03080e;
    color: #e5f1fb;
    font-family: Inter, system-ui, sans-serif;
    font-size: 12px;
    user-select: none;
  }

  .topbar {
    display: flex;
    align-items: center;
    gap: 14px;
    height: 50px;
    padding: 0 16px;
    background: #06111d;
    border-bottom: 1px solid #162c40;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .logo {
    width: 28px;
    height: 28px;
    border: 1px solid #38bdf8;
    border-radius: 6px;
    display: grid;
    place-items: center;
    color: #38bdf8;
    font-size: 15px;
    font-weight: 800;
    background: rgba(56, 189, 248, 0.1);
  }

  .divider {
    width: 1px;
    height: 24px;
    background: #162c40;
  }

  .tool-controls {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .lbl {
    color: #8ca5ba;
    font-size: 11px;
  }

  select, input[type="number"] {
    background: #091726;
    border: 1px solid #1c3650;
    color: #fff;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 11px;
  }

  .check-opt {
    display: flex;
    align-items: center;
    gap: 5px;
    color: #d1e2f1;
    font-size: 11px;
    cursor: pointer;
  }

  .grow { flex: 1; }

  .actions {
    display: flex;
    gap: 8px;
  }

  .copy-btn {
    background: #183e60;
    border: 1px solid #2a5a84;
    color: #fff;
    padding: 5px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-weight: 600;
  }

  .export-btn {
    background: #0b1a29;
    border: 1px solid #1f3d59;
    color: #e5f1fb;
    padding: 5px 12px;
    border-radius: 5px;
    cursor: pointer;
  }

  .sub-toolbar {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 16px;
    background: #050d18;
    border-bottom: 1px solid #162c40;
  }

  .pill-group {
    display: flex;
    background: #0b1c2d;
    border-radius: 5px;
    border: 1px solid #17334d;
    overflow: hidden;
  }

  .pill-group button {
    background: transparent;
    border: none;
    color: #8ca5ba;
    padding: 5px 10px;
    font-size: 11px;
    cursor: pointer;
  }

  .pill-group button.active {
    background: #1d4b73;
    color: #fff;
    font-weight: 600;
  }

  .brush-settings {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .audio-panel {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .freq-input {
    width: 60px;
  }

  .play-btn {
    background: #1e5a43;
    border: 1px solid #22d69a;
    color: #fff;
    padding: 4px 12px;
    border-radius: 5px;
    cursor: pointer;
    font-weight: 600;
  }

  .play-btn.active {
    background: #842029;
    border-color: #f87171;
  }

  .workspace-split {
    display: grid;
    flex: 1;
    overflow: hidden;
  }

  .editor-pane, .canvas-pane {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
    background: #050e18;
  }

  .pane-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 12px;
    background: #071524;
    border-bottom: 1px solid #162c40;
    font-size: 11px;
    font-weight: 700;
  }

  .badge {
    background: #132a3e;
    color: #38bdf8;
    font-size: 9px;
    padding: 2px 6px;
    border-radius: 4px;
  }

  .splitter {
    width: 6px;
    background: #081624;
    border-left: 1px solid #13273c;
    border-right: 1px solid #13273c;
    cursor: col-resize;
  }

  .editor-wrapper {
    flex: 1;
    overflow: auto;
  }

  .canvas-stack-container {
    position: relative;
    width: 100%;
    height: 460px;
    background: #000;
  }

  .gl-canvas, .overlay-canvas {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
  }

  .overlay-canvas {
    cursor: crosshair;
    z-index: 2;
  }

  .cursor-hud {
    position: absolute;
    background: rgba(4, 13, 22, 0.9);
    border: 1px solid #38bdf8;
    border-radius: 4px;
    padding: 4px 8px;
    font-size: 10px;
    color: #e5f1fb;
    pointer-events: none;
    z-index: 10;
  }

  .profile-container {
    padding: 8px;
    background: #03080e;
    border-top: 1px solid #162c40;
  }
</style>
