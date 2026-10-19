<script lang="ts">
  import { getWidgetComponent } from './registry';
  import type { WidgetDescriptor } from './types';

  interface Props {
    descriptor: WidgetDescriptor;
    onParamChange?: (paramPath: string, newValue: number | string | boolean) => void;
  }

  let { descriptor, onParamChange }: Props = $props();

  let Component = $derived(getWidgetComponent(descriptor.kind));
</script>

<div class="semantic-widget-host">
  {#if Component}
    <Component {descriptor} {onParamChange} />
  {:else}
    <div class="fallback-widget">
      <span>Visualizador não registrado para: {descriptor.kind}</span>
    </div>
  {/if}
</div>

<style>
  .semantic-widget-host {
    display: block;
    width: 100%;
  }

  .fallback-widget {
    background: #1e293b;
    border: 1px dashed #475569;
    padding: 6px 10px;
    border-radius: 4px;
    font-size: 10px;
    color: #94a3b8;
  }
</style>
