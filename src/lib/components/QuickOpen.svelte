<script lang="ts">
  import FileText from '@lucide/svelte/icons/file-text';
  import Search from '@lucide/svelte/icons/search';
  import { fsListWorkspaceFiles } from '$lib/ipc';
  import { fuzzyFilterFiles, parseQuickOpenInput } from '$lib/languages';

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

  const parsed = $derived(parseQuickOpenInput(queryInput));
  const filteredFiles = $derived(fuzzyFilterFiles(allFiles, parsed.query, 50));

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
      void refreshFileList();
      queueMicrotask(() => {
        inputEl?.focus();
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
      }
      return;
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      if (filteredFiles.length > 0) {
        selectedIndex = (selectedIndex - 1 + filteredFiles.length) % filteredFiles.length;
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
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="modal-backdrop"
    onmousedown={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}
  >
    <div class="quick-open-dialog" role="dialog" aria-modal="true" aria-label="Buka cepat file">
      <div class="search-header">
        <Search size={14} strokeWidth={2} />
        <input
          bind:this={inputEl}
          bind:value={queryInput}
          onkeydown={handleKeyDown}
          type="text"
          class="search-input"
          placeholder="Cari nama file (gunakan :baris untuk lompat ke baris)..."
        />
        {#if parsed.line !== null}
          <span class="line-badge">Baris {parsed.line}</span>
        {/if}
      </div>

      <div class="results-list">
        {#if loading}
          <div class="empty-item">Memuat daftar file...</div>
        {:else if filteredFiles.length === 0}
          <div class="empty-item">Tidak ada file yang cocok</div>
        {:else}
          {#each filteredFiles as relPath, idx (relPath)}
            {@const parts = splitFileParts(relPath)}
            <button
              class="result-row"
              class:selected={idx === selectedIndex}
              onmouseenter={() => (selectedIndex = idx)}
              onclick={() => chooseItem(relPath)}
            >
              <FileText size={14} strokeWidth={2} />
              <span class="file-name">{parts.name}</span>
              {#if parts.dir}
                <span class="file-dir">{parts.dir}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: flex-start;
    justify-content: center;
    padding-top: 72px;
    z-index: 100;
  }

  .quick-open-dialog {
    width: 100%;
    max-width: 520px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-dialog);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .search-header {
    height: 40px;
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--dm);
  }

  .search-input {
    flex: 1;
    height: 28px;
    border: none;
    background: transparent;
    color: var(--tx);
    font-size: 13px;
    padding: 0;
  }

  .search-input:focus {
    outline: none;
  }

  .line-badge {
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 2px 6px;
    border-radius: var(--radius-ctrl);
    background: var(--acs);
    color: var(--ac);
  }

  .results-list {
    max-height: 320px;
    overflow-y: auto;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .result-row {
    width: 100%;
    height: 30px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--dm);
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    text-align: left;
  }

  .result-row.selected {
    background: var(--acs);
    color: var(--tx);
  }

  .file-name {
    color: var(--tx);
    font-size: 13px;
    white-space: nowrap;
  }

  .file-dir {
    color: var(--dm);
    font-family: var(--font-mono);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty-item {
    padding: 16px 12px;
    color: var(--dm);
    font-size: 12px;
    text-align: center;
  }
</style>
