<script lang="ts">
  interface TabItem {
    id: string;
    label: string;
    icon?: string;
  }

  interface Props {
    items: TabItem[];
    activeId: string;
    onChange?: (id: string) => void;
  }

  let { items = [], activeId = $bindable(''), onChange }: Props = $props();

  function selectTab(id: string) {
    activeId = id;
    onChange?.(id);
  }
</script>

<div class="pill-tabs" role="tablist">
  {#each items as item}
    {@const isActive = item.id === activeId}
    <button
      type="button"
      role="tab"
      aria-selected={isActive}
      class="pill-btn"
      class:active={isActive}
      onclick={() => selectTab(item.id)}
    >
      {#if item.icon}
        <span class="icon">{item.icon}</span>
      {/if}
      <span>{item.label}</span>
    </button>
  {/each}
</div>

<style>
  .pill-tabs {
    display: inline-flex;
    align-items: center;
    background: var(--bg-base);
    border: 1px solid var(--border-subtle);
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
    user-select: none;
  }

  .pill-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    padding: 4px 10px;
    font-size: 11px;
    font-weight: 500;
    border-radius: 4px;
    cursor: pointer;
    transition: all 0.15s ease-in-out;
  }

  .pill-btn:hover {
    color: var(--text-primary);
    background: rgba(255, 255, 255, 0.04);
  }

  .pill-btn.active {
    background: var(--border-hover);
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  }

  .icon {
    font-size: 12px;
    line-height: 1;
  }
</style>
