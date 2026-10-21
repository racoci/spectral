<script lang="ts">
  import { computeKnobAngle, clampValue } from './ui_helpers';

  interface Props {
    value: number;
    min: number;
    max: number;
    step?: number;
    defaultValue?: number;
    label?: string;
    unit?: string;
    precision?: number;
    size?: number;
    color?: string;
    onChange?: (val: number) => void;
  }

  let {
    value = $bindable(0),
    min = 0,
    max = 100,
    step = 1,
    defaultValue = 0,
    label = '',
    unit = '',
    precision = 1,
    size = 36,
    color = '#38bdf8',
    onChange
  }: Props = $props();

  let isDragging = $state(false);
  let startY = 0;
  let startVal = 0;

  let angleDeg = $derived(computeKnobAngle(value, min, max));
  let formattedValue = $derived(
    precision === 0 ? Math.round(value).toString() : value.toFixed(precision)
  );

  function handlePointerDown(e: PointerEvent) {
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    isDragging = true;
    startY = e.clientY;
    startVal = value;
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    const dy = startY - e.clientY;
    const range = max - min;
    const sensitivity = range / 150.0;
    let nextVal = startVal + dy * sensitivity;

    if (step > 0) {
      nextVal = Math.round(nextVal / step) * step;
    }
    nextVal = clampValue(nextVal, min, max);

    if (nextVal !== value) {
      value = nextVal;
      onChange?.(nextVal);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (isDragging) {
      isDragging = false;
      const target = e.currentTarget as HTMLElement;
      target.releasePointerCapture(e.pointerId);
    }
  }

  function handleWheel(e: WheelEvent) {
    e.preventDefault();
    const dir = e.deltaY < 0 ? 1 : -1;
    const delta = (step > 0 ? step : (max - min) / 100) * dir;
    let nextVal = clampValue(value + delta, min, max);
    if (nextVal !== value) {
      value = nextVal;
      onChange?.(nextVal);
    }
  }

  function handleDoubleClick() {
    value = defaultValue;
    onChange?.(defaultValue);
  }
</script>

<div class="knob-container">
  {#if label}
    <span class="knob-label">{label}</span>
  {/if}

  <div
    class="knob-body"
    style="width: {size}px; height: {size}px;"
    role="slider"
    tabindex="0"
    aria-valuenow={value}
    aria-valuemin={min}
    aria-valuemax={max}
    aria-label={label || 'Knob de ajuste'}
    onpointerdown={handlePointerDown}
    onpointermove={handlePointerMove}
    onpointerup={handlePointerUp}
    onwheel={handleWheel}
    ondblclick={handleDoubleClick}
  >
    <svg viewBox="0 0 40 40" class="knob-svg">
      <!-- Trilha base -->
      <circle cx="20" cy="20" r="16" class="track-bg" />
      
      <!-- Arco ativo -->
      <circle 
        cx="20" 
        cy="20" 
        r="16" 
        class="track-val"
        style="stroke: {color}; stroke-dasharray: {((angleDeg + 135) / 270) * 75} 100;"
      />

      <!-- Ponteiro indicador giratório -->
      <g transform="rotate({angleDeg} 20 20)">
        <line x1="20" y1="20" x2="20" y2="7" class="needle" style="stroke: {color};" />
        <circle cx="20" cy="20" r="3" fill="#ffffff" />
      </g>
    </svg>
  </div>

  <span class="knob-val font-mono">
    {formattedValue}{unit}
  </span>
</div>

<style>
  .knob-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 3px;
    user-select: none;
  }

  .knob-label {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-secondary);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .knob-body {
    position: relative;
    cursor: ns-resize;
    outline: none;
    touch-action: none;
  }

  .knob-body:focus-visible .track-bg {
    stroke: var(--accent-cyan);
  }

  .knob-svg {
    width: 100%;
    height: 100%;
    transform: rotate(135deg);
  }

  .track-bg {
    fill: var(--bg-surface);
    stroke: var(--border-subtle);
    stroke-width: 3.5;
    stroke-dasharray: 75 100;
    stroke-linecap: round;
  }

  .track-val {
    fill: none;
    stroke-width: 3.5;
    stroke-linecap: round;
  }

  .needle {
    stroke-width: 2.2;
    stroke-linecap: round;
  }

  .knob-val {
    font-size: 10px;
    color: var(--text-primary);
    background: var(--bg-input);
    border: 1px solid var(--border-subtle);
    padding: 1px 4px;
    border-radius: 3px;
    min-width: 32px;
    text-align: center;
  }
</style>
