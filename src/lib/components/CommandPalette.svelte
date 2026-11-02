<script lang="ts">
  import { filterPaletteEntries, type PaletteEntry } from '$lib/keymap';

  interface Props {
    open: boolean;
    actions: PaletteEntry[];
    onClose: () => void;
  }

  let { open, actions, onClose }: Props = $props();

  let query = $state('');
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);

  const combinedEntries = $derived.by<PaletteEntry[]>(() => {
    return filterPaletteEntries(actions, query.trim());
  });

  function scrollSelectedIntoView(index: number) {
    if (!listEl) return;
    const rows = listEl.querySelectorAll<HTMLElement>('.palette-row');
    const target = rows[index];
    if (target) {
      target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  }

  $effect(() => {
    if (open) {
      query = '';
      selectedIndex = 0;
      if (listEl) listEl.scrollTop = 0;
      queueMicrotask(() => {
        inputEl?.focus();
        if (listEl) listEl.scrollTop = 0;
      });
    }
  });

  function executeIndex(idx: number) {
    const item = combinedEntries[idx];
    if (!item) return;
    onClose();
    item.run();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      onClose();
      return;
    }

    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (combinedEntries.length > 0) {
        selectedIndex = (selectedIndex + 1) % combinedEntries.length;
        scrollSelectedIntoView(selectedIndex);
      }
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (combinedEntries.length > 0) {
        selectedIndex = (selectedIndex - 1 + combinedEntries.length) % combinedEntries.length;
        scrollSelectedIntoView(selectedIndex);
      }
      return;
    }

    if (event.key === 'Enter') {
      event.preventDefault();
      executeIndex(selectedIndex);
    }
  }
</script>

{#if open}
  <div class="palette-backdrop" onclick={onClose}>
    <div
      class="palette-card"
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeydown}
    >
      <div class="palette-header">
        <input
          bind:this={inputEl}
          bind:value={query}
          type="text"
          class="palette-input"
          placeholder="Type a command..."
          oninput={() => {
            selectedIndex = 0;
            if (listEl) listEl.scrollTop = 0;
          }}
        />
      </div>

      <ul bind:this={listEl} class="palette-list">
        {#if combinedEntries.length === 0}
          <li class="empty-row">No matching commands</li>
        {:else}
          {#each combinedEntries as item, i (item.id)}
            <li>
              <button
                class="palette-row"
                class:selected={i === selectedIndex}
                onmousemove={(e) => {
                  if (e.movementX !== 0 || e.movementY !== 0) selectedIndex = i;
                }}
                onclick={() => executeIndex(i)}
              >
                <span class="row-label">{item.label}</span>
                <span class="row-meta">
                  {#if item.shortcut}
                    <kbd class="shortcut-badge">{item.shortcut}</kbd>
                  {/if}
                  <span class="category-tag">{item.category}</span>
                </span>
              </button>
            </li>
          {/each}
        {/if}
      </ul>
    </div>
  </div>
{/if}

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: var(--backdrop-bg);
    -webkit-backdrop-filter: var(--backdrop-blur);
    backdrop-filter: var(--backdrop-blur);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 84px;
    z-index: 110;
  }

  .palette-card {
    width: 100%;
    max-width: 520px;
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-dialog);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .palette-header {
    padding: 14px 16px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
  }

  input[type='text'].palette-input {
    width: 100%;
    height: 24px;
    border: none;
    border-radius: 0;
    background: transparent;
    box-shadow: none;
    color: var(--tx);
    padding: 0;
    font-size: 13px;
  }

  input[type='text'].palette-input:focus,
  input[type='text'].palette-input:focus-visible {
    outline: none;
    border: none;
    box-shadow: none;
  }

  .palette-list {
    list-style: none;
    max-height: 360px;
    overflow-y: auto;
    padding: 6px;
  }

  .palette-row {
    width: 100%;
    height: 34px;
    padding: 0 12px;
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--tx);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    text-align: left;
  }

  .palette-row.selected {
    background: var(--acs);
  }

  .row-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 13px;
  }

  .row-meta {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }

  .shortcut-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--dm);
    background: var(--el);
    border: 1px solid var(--bd);
    border-radius: 6px;
    padding: 2px 6px;
  }

  .category-tag {
    color: var(--dm);
    font-size: 12px;
  }

  .empty-row {
    padding: 16px;
    color: var(--dm);
    font-size: 12px;
    text-align: center;
  }
</style>
