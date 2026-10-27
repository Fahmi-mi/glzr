<script lang="ts">
  import { fsListWorkspaceFiles } from '$lib/ipc';
  import { filterPaletteEntries, type PaletteEntry } from '$lib/keymap';

  interface Props {
    open: boolean;
    workspaceRoot: string | null;
    actions: PaletteEntry[];
    onSelectFile: (fullPath: string, line: number | null) => void;
    onClose: () => void;
  }

  let { open, workspaceRoot, actions, onSelectFile, onClose }: Props = $props();

  let query = $state('');
  let filePaths = $state<string[]>([]);
  let selectedIndex = $state(0);
  let inputEl = $state<HTMLInputElement | null>(null);

  const combinedEntries = $derived.by<PaletteEntry[]>(() => {
    const parsedQuery = parseFileLineQuery(query);
    const trimmed = parsedQuery.filePart.trim();

    if (!workspaceRoot || filePaths.length === 0) {
      return filterPaletteEntries(actions, trimmed);
    }

    const fileEntries: PaletteEntry[] = filePaths.map((relPath) => ({
      id: `file:${relPath}`,
      label: relPath,
      category: 'File',
      run: () => {
        onSelectFile(`${workspaceRoot}/${relPath}`, parsedQuery.line);
      }
    }));

    if (!trimmed) {
      return [...actions, ...fileEntries.slice(0, 25)].slice(0, 60);
    }

    return filterPaletteEntries([...actions, ...fileEntries], trimmed);
  });

  function parseFileLineQuery(raw: string): { filePart: string; line: number | null } {
    const trimmed = raw.trim();
    const colonIdx = trimmed.lastIndexOf(':');
    if (colonIdx <= 0) {
      return { filePart: trimmed, line: null };
    }
    const after = trimmed.slice(colonIdx + 1);
    if (/^\d+$/.test(after)) {
      const parsed = Number.parseInt(after, 10);
      return {
        filePart: trimmed.slice(0, colonIdx),
        line: parsed > 0 ? parsed : null
      };
    }
    return { filePart: trimmed, line: null };
  }

  $effect(() => {
    if (open) {
      query = '';
      selectedIndex = 0;
      if (workspaceRoot) {
        void fsListWorkspaceFiles()
          .then((list) => {
            filePaths = list;
          })
          .catch(() => {
            filePaths = [];
          });
      } else {
        filePaths = [];
      }
      queueMicrotask(() => {
        inputEl?.focus();
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
      }
      return;
    }

    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (combinedEntries.length > 0) {
        selectedIndex = (selectedIndex - 1 + combinedEntries.length) % combinedEntries.length;
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
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
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
          placeholder="Type a command or file name..."
          oninput={() => {
            selectedIndex = 0;
          }}
        />
      </div>

      <ul class="palette-list">
        {#if combinedEntries.length === 0}
          <li class="empty-row">No matching commands or files</li>
        {:else}
          {#each combinedEntries as item, i (item.id)}
            <li>
              <button
                class="palette-row"
                class:selected={i === selectedIndex}
                onmouseenter={() => (selectedIndex = i)}
                onclick={() => executeIndex(i)}
              >
                <span class="row-label" class:mono={item.category === 'File'}>{item.label}</span>
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
    background: rgba(0, 0, 0, 0.25);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 84px;
    z-index: 110;
  }

  .palette-card {
    width: 100%;
    max-width: 560px;
    background: var(--el);
    border: 1px solid var(--bd);
    border-radius: var(--radius-dialog);
    box-shadow: var(--shadow-float);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .palette-header {
    padding: 16px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
  }

  .palette-input {
    width: 100%;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--tx);
    padding: 0;
    font-size: 13px;
  }

  .palette-input:focus {
    outline: none;
  }

  .palette-list {
    list-style: none;
    max-height: 360px;
    overflow-y: auto;
    padding: 6px 0;
  }

  .palette-row {
    width: 100%;
    height: 36px;
    padding: 0 16px;
    border: none;
    border-radius: 0;
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

  .row-label.mono {
    font-family: var(--font-mono);
    font-size: 12px;
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
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: 4px;
    padding: 1px 6px;
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
