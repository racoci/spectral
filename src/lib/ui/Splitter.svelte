<script lang="ts">
  import { computeSplitterBounds } from './ui_helpers';

  interface Props {
    direction?: 'horizontal' | 'vertical';
    size: number;
    minFirst?: number;
    minSecond?: number;
    defaultSize?: number;
    onResize?: (newSize: number) => void;
  }

  let {
    direction = 'vertical',
    size = $bindable(300),
    minFirst = 150,
    minSecond = 200,
    defaultSize,
    onResize
  }: Props = $props();

  let isDragging = $state(false);
  let startCoord = 0;
  let startSize = 0;

  function handlePointerDown(e: PointerEvent) {
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    isDragging = true;
    startCoord = direction === 'vertical' ? e.clientX : e.clientY;
    startSize = size;
  }

  function handlePointerMove(e: PointerEvent) {
    if (!isDragging) return;
    const currentCoord = direction === 'vertical' ? e.clientX : e.clientY;
    const delta = currentCoord - startCoord;
    const parent = (e.currentTarget as HTMLElement).parentElement;
    const total = parent ? (direction === 'vertical' ? parent.clientWidth : parent.clientHeight) : 1000;

    const nextSize = computeSplitterBounds(startSize + delta, total, minFirst, minSecond);
    if (nextSize !== size) {
      size = nextSize;
      onResize?.(nextSize);
    }
  }

  function handlePointerUp(e: PointerEvent) {
    if (isDragging) {
      isDragging = false;
      const target = e.currentTarget as HTMLElement;
      target.releasePointerCapture(e.pointerId);
    }
  }

  function handleDoubleClick() {
    if (defaultSize !== undefined) {
      size = defaultSize;
      onResize?.(defaultSize);
    }
  }
</script>

<div
  class="splitter"
  class:vertical={direction === 'vertical'}
  class:horizontal={direction === 'horizontal'}
  class:active={isDragging}
  role="separator"
  tabindex="0"
  aria-orientation={direction}
  onpointerdown={handlePointerDown}
  onpointermove={handlePointerMove}
  onpointerup={handlePointerUp}
  ondblclick={handleDoubleClick}
  title="Arrastar para ajustar divisão (Clique duplo para restaurar)"
>
  <div class="handle-grip"></div>
</div>

<style>
  .splitter {
    position: relative;
    background: var(--bg-base);
    transition: background 0.15s ease-in-out;
    user-select: none;
    z-index: 5;
    touch-action: none;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .splitter.vertical {
    width: 6px;
    height: 100%;
    cursor: col-resize;
    border-left: 1px solid var(--border-subtle);
    border-right: 1px solid var(--border-subtle);
  }

  .splitter.horizontal {
    height: 6px;
    width: 100%;
    cursor: row-resize;
    border-top: 1px solid var(--border-subtle);
    border-bottom: 1px solid var(--border-subtle);
  }

  .splitter:hover, .splitter.active {
    background: var(--border-active);
  }

  .handle-grip {
    background: var(--border-hover);
    border-radius: 2px;
    pointer-events: none;
  }

  .splitter.vertical .handle-grip {
    width: 2px;
    height: 24px;
  }

  .splitter.horizontal .handle-grip {
    width: 24px;
    height: 2px;
  }
</style>
