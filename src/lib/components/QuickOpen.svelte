<script lang="ts">
  import { fsListWorkspaceFiles } from '$lib/ipc';
  import { fuzzyFilterFiles, parseQuickOpenInput } from '$lib/languages';
  import FileIcon from '$lib/components/icons/FileIcon.svelte';

  interface Props {
    open: boolean;
    workspaceRoot: string;
    onClose: () => void;
    onSelect: (fullPath: string, line: number | null) => void;
  }

  let { open, workspaceRoot, onClose, onSelect }: Props = $props();

  let queryInput = $state('');
  let allFiles = $state<string[]>([]);
  let selectedIndex = $state(0);
  let loading = $state(false);
  let inputEl = $state<HTMLInputElement | null>(null);
  let listEl = $state<HTMLUListElement | null>(null);

  const parsed = $derived(parseQuickOpenInput(queryInput));
  const filteredFiles = $derived(fuzzyFilterFiles(allFiles, parsed.query, 50));

  function scrollSelectedIntoView(index: number) {
    if (!listEl) return;
    const rows = listEl.querySelectorAll<HTMLElement>('.palette-row');
    const target = rows[index];
    if (target) {
      target.scrollIntoView({ block: 'nearest', inline: 'nearest' });
    }
  }

  async function refreshFileList() {
    loading = true;
    try {
      allFiles = await fsListWorkspaceFiles();
    } catch {
      allFiles = [];
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (open) {
      queryInput = '';
      selectedIndex = 0;
      if (listEl) listEl.scrollTop = 0;
      void refreshFileList();
      queueMicrotask(() => {
        inputEl?.focus();
        if (listEl) listEl.scrollTop = 0;
      });
    }
  });

  $effect(() => {
    if (selectedIndex >= filteredFiles.length) {
      selectedIndex = Math.max(0, filteredFiles.length - 1);
    }
  });

  function chooseItem(relPath: string) {
    const cleanRoot = workspaceRoot.replace(/\/+$/, '');
    const fullPath = `${cleanRoot}/${relPath}`;
    onSelect(fullPath, parsed.line);
    onClose();
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
      return;
    }
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      if (filteredFiles.length > 0) {
        selectedIndex = (selectedIndex + 1) % filteredFiles.length;
        scrollSelectedIntoView(selectedIndex);
      }
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (filteredFiles.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredFiles.length) % filteredFiles.length;
        scrollSelectedIntoView(selectedIndex);
      }
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const target = filteredFiles[selectedIndex];
      if (target) {
        chooseItem(target);
      }
    }
  }

  function splitFileParts(relPath: string): { name: string; dir: string } {
    const idx = relPath.lastIndexOf('/');
    if (idx === -1) {
      return { name: relPath, dir: '' };
    }
    return {
      name: relPath.slice(idx + 1),
      dir: relPath.slice(0, idx)
    };
  }
</script>

{#if open}
  <div class="palette-backdrop" onclick={onClose}>
    <div
      class="palette-card"
      role="dialog"
      aria-modal="true"
      aria-label="Quick open file"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={handleKeyDown}
    >
      <div class="palette-header">
        <input
          bind:this={inputEl}
          bind:value={queryInput}
          type="text"
          class="palette-input"
          placeholder="Search files by name (append :line to jump to line)..."
          oninput={() => {
            selectedIndex = 0;
            if (listEl) listEl.scrollTop = 0;
          }}
        />
        {#if parsed.line !== null}
          <kbd class="shortcut-badge">Line {parsed.line}</kbd>
        {/if}
      </div>

      <ul bind:this={listEl} class="palette-list">
        {#if loading}
          <li class="empty-row">Loading file list...</li>
        {:else if filteredFiles.length === 0}
          <li class="empty-row">No matching files found</li>
        {:else}
          {#each filteredFiles as relPath, idx (relPath)}
            {@const parts = splitFileParts(relPath)}
            <li>
              <button
                class="palette-row"
                class:selected={idx === selectedIndex}
                onmousemove={(e) => {
                  if (e.movementX !== 0 || e.movementY !== 0) selectedIndex = idx;
                }}
                onclick={() => chooseItem(relPath)}
              >
                <span class="row-main">
                  <FileIcon filename={parts.name} size={15} />
                  <span class="row-label">{parts.name}</span>
                </span>
                <span class="row-meta">
                  <span class="category-tag">{parts.dir || 'File'}</span>
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
    gap: 10px;
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

  .row-main {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    overflow: hidden;
  }

  .row-main :global(.jb-file-icon) {
    width: 1.15em;
    height: 1.15em;
    flex-shrink: 0;
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
    flex-shrink: 0;
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
