<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    title: string;
    icon?: string;
    badge?: string;
    collapsible?: boolean;
    defaultExpanded?: boolean;
    headerActions?: Snippet;
    children?: Snippet;
  }

  let {
    title,
    icon = '',
    badge = '',
    collapsible = true,
    defaultExpanded = true,
    headerActions,
    children
  }: Props = $props();

  let expanded = $state(defaultExpanded);

  function toggle() {
    if (collapsible) {
      expanded = !expanded;
    }
  }
</script>

<section class="panel-card">
  <header class="panel-header" onclick={toggle}>
    <div class="title-group">
      {#if icon}
        <span class="icon">{icon}</span>
      {/if}
      <strong class="title">{title}</strong>
      {#if badge}
        <span class="badge">{badge}</span>
      {/if}
    </div>

    <div class="actions" onclick={(e) => e.stopPropagation()}>
      {#if headerActions}
        {@render headerActions()}
      {/if}

      {#if collapsible}
        <button type="button" class="collapse-btn" onclick={toggle} aria-label={expanded ? 'Recolher' : 'Expandir'}>
          {expanded ? '▼' : '▲'}
        </button>
      {/if}
    </div>
  </header>

  {#if expanded}
    <div class="panel-content">
      {#if children}
        {@render children()}
      {/if}
    </div>
  {/if}
</section>

<style>
  .panel-card {
    background: var(--bg-surface);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 10px;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border-subtle);
    cursor: pointer;
    user-select: none;
  }

  .title-group {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .icon {
    font-size: 12px;
  }

  .title {
    color: var(--text-primary);
  }

  .badge {
    background: var(--bg-input);
    color: var(--accent-cyan);
    font-size: 9px;
    font-family: var(--font-mono);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid var(--border-subtle);
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .collapse-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 9px;
    padding: 2px 4px;
    cursor: pointer;
    border-radius: 3px;
  }

  .collapse-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.05);
  }

  .panel-content {
    padding: 10px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
</style>
