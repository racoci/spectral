<script lang="ts">
  import type { Snippet } from 'svelte';
  import { computeSmartTooltipPosition } from './ui_helpers';

  interface Props {
    visible: boolean;
    x: number;
    y: number;
    offset?: number;
    children?: Snippet;
  }

  let { visible = false, x = 0, y = 0, offset = 12, children }: Props = $props();

  let tooltipEl = $state<HTMLDivElement | null>(null);
  let pos = $derived.by(() => {
    if (typeof window === 'undefined') return { x: x + offset, y: y + offset };
    const w = tooltipEl?.offsetWidth || 160;
    const h = tooltipEl?.offsetHeight || 50;
    return computeSmartTooltipPosition(x, y, w, h, window.innerWidth, window.innerHeight, offset);
  });
</script>

{#if visible}
  <div
    bind:this={tooltipEl}
    class="smart-tooltip"
    style="left: {pos.x}px; top: {pos.y}px;"
    role="tooltip"
  >
    {#if children}
      {@render children()}
    {/if}
  </div>
{/if}

<style>
  .smart-tooltip {
    position: fixed;
    background: rgba(4, 13, 22, 0.95);
    border: 1px solid var(--accent-cyan);
    border-radius: 5px;
    padding: 6px 10px;
    font-size: 11px;
    color: var(--text-primary);
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.6);
    pointer-events: none;
    z-index: 1000;
    backdrop-filter: blur(8px);
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
</style>
