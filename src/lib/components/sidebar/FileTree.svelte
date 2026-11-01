<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Copy from '@lucide/svelte/icons/copy';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import File from '@lucide/svelte/icons/file';
  import FilePlus from '@lucide/svelte/icons/file-plus';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import Image from '@lucide/svelte/icons/image';
  import Pencil from '@lucide/svelte/icons/pencil';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import {
    fsCreateDir,
    fsCreateFile,
    fsListDir,
    fsRename,
    fsTrash,
    revealInFileManager,
    type AppErrorPayload,
    type DirEntryDto,
    type GitBadge
  } from '$lib/ipc';
  import { badgeColorVar } from '$lib/git';
  import { clampMenuCoordinates } from '$lib/menu';

  interface Props {
    workspaceRoot: string;
    gitBadges?: Record<string, GitBadge>;
    onOpenFile: (fullPath: string) => void;
    onEntryRenamed?: (oldPath: string, newPath: string) => void;
    onEntryDeleted?: (deletedPath: string) => void;
  }

  let {
    workspaceRoot,
    gitBadges = {},
    onOpenFile,
    onEntryRenamed,
    onEntryDeleted
  }: Props = $props();

  interface TreeRow {
    entry: DirEntryDto;
    depth: number;
  }

  let dirCache = $state<Record<string, DirEntryDto[]>>({});
  let expandedDirs = $state<Record<string, boolean>>({});
  let selectedPath = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let creatingMode = $state<{ parentDir: string; isDir: boolean } | null>(null);
  let newNameInput = $state('');
  let renamingEntry = $state<DirEntryDto | null>(null);
  let renameInput = $state('');
  let confirmDeleteEntry = $state<DirEntryDto | null>(null);
  let treeContainerEl = $state<HTMLElement | null>(null);

  interface ContextMenuState {
    x: number;
    y: number;
    entry: DirEntryDto;
  }

  let contextMenu = $state<ContextMenuState | null>(null);
  let contextMenuEl = $state<HTMLElement | null>(null);

  const flatRows = $derived.by<TreeRow[]>(() => {
    const out: TreeRow[] = [];
    function walk(dirKey: string, depth: number) {
      const items = dirCache[dirKey] ?? [];
      for (const item of items) {
        out.push({ entry: item, depth });
        if (item.isDir && expandedDirs[item.path]) {
          walk(item.path, depth + 1);
        }
      }
    }
    walk('', 0);
    return out;
  });

  function isImageFilename(name: string): boolean {
    const dotIdx = name.lastIndexOf('.');
    if (dotIdx <= 0) return false;
    const ext = name.slice(dotIdx + 1).toLowerCase();
    return ['png', 'jpg', 'jpeg', 'gif', 'webp', 'ico', 'svg'].includes(ext);
  }

  async function loadDirEntries(dirKey: string) {
    try {
      const entries = await fsListDir(dirKey === '' ? undefined : dirKey);
      dirCache[dirKey] = entries;
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to load directory';
    }
  }

  export async function refresh() {
    errorMsg = null;
    await loadDirEntries('');
    const expandedKeys = Object.keys(expandedDirs).filter((k) => expandedDirs[k]);
    for (const key of expandedKeys) {
      await loadDirEntries(key);
    }
  }

  export function focusTree() {
    treeContainerEl?.focus();
  }

  onMount(() => {
    void refresh();
  });

  async function toggleEntry(entry: DirEntryDto) {
    selectedPath = entry.path;
    if (entry.isDir) {
      const next = !expandedDirs[entry.path];
      expandedDirs[entry.path] = next;
      if (next && !dirCache[entry.path]) {
        await loadDirEntries(entry.path);
      }
    } else {
      onOpenFile(entry.path);
    }
  }

  function resolveActiveParentDir(): string {
    if (!selectedPath) return workspaceRoot;
    for (const row of flatRows) {
      if (row.entry.path === selectedPath) {
        if (row.entry.isDir) return row.entry.path;
        const slashIdx = row.entry.path.lastIndexOf('/');
        return slashIdx > 0 ? row.entry.path.slice(0, slashIdx) : workspaceRoot;
      }
    }
    return workspaceRoot;
  }

  function startCreate(isDir: boolean) {
    const parentDir = resolveActiveParentDir();
    if (parentDir !== workspaceRoot) {
      expandedDirs[parentDir] = true;
      if (!dirCache[parentDir]) {
        void loadDirEntries(parentDir);
      }
    }
    creatingMode = { parentDir, isDir };
    newNameInput = '';
    renamingEntry = null;
  }

  async function submitCreate() {
    if (!creatingMode) return;
    const trimmed = newNameInput.trim();
    const { parentDir, isDir } = creatingMode;
    creatingMode = null;
    if (!trimmed) return;

    const targetPath = `${parentDir.replace(/\/+$/, '')}/${trimmed}`;
    errorMsg = null;
    try {
      if (isDir) {
        await fsCreateDir(targetPath);
      } else {
        const created = await fsCreateFile(targetPath);
        onOpenFile(created);
      }
      await refresh();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to create new item';
    }
  }

  function startRename(entry: DirEntryDto) {
    renamingEntry = entry;
    renameInput = entry.name;
    creatingMode = null;
  }

  async function submitRename() {
    const entry = renamingEntry;
    renamingEntry = null;
    if (!entry) return;
    const trimmed = renameInput.trim();
    if (!trimmed || trimmed === entry.name) return;

    const slashIdx = entry.path.lastIndexOf('/');
    const parentDir = slashIdx > 0 ? entry.path.slice(0, slashIdx) : workspaceRoot;
    const toPath = `${parentDir}/${trimmed}`;

    errorMsg = null;
    try {
      const canonicalNew = await fsRename(entry.path, toPath);
      onEntryRenamed?.(entry.path, canonicalNew);
      await refresh();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to rename item';
    }
  }

  async function handleConfirmDelete() {
    const entry = confirmDeleteEntry;
    confirmDeleteEntry = null;
    if (!entry) return;
    errorMsg = null;
    try {
      await fsTrash(entry.path);
      onEntryDeleted?.(entry.path);
      await refresh();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to move to trash';
    }
  }

  function copyPathToClipboard(path: string) {
    void navigator.clipboard.writeText(path).catch(() => {});
  }

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function closeContextMenu() {
    contextMenu = null;
  }

  function handleRowContextMenu(e: MouseEvent, entry: DirEntryDto) {
    if (renamingEntry?.path === entry.path) return;
    e.preventDefault();
    e.stopPropagation();
    selectedPath = entry.path;
    creatingMode = null;
    confirmDeleteEntry = null;

    const winW = typeof window !== 'undefined' ? window.innerWidth : 1024;
    const winH = typeof window !== 'undefined' ? window.innerHeight : 768;
    const initialPos = clampMenuCoordinates(e.clientX, e.clientY, 180, 130, winW, winH);

    contextMenu = {
      x: initialPos.x,
      y: initialPos.y,
      entry
    };

    void tick().then(() => {
      if (contextMenuEl && contextMenu) {
        const rect = contextMenuEl.getBoundingClientRect();
        if (rect.width > 0 && rect.height > 0) {
          const currentWinW = typeof window !== 'undefined' ? window.innerWidth : winW;
          const currentWinH = typeof window !== 'undefined' ? window.innerHeight : winH;
          const refined = clampMenuCoordinates(
            e.clientX,
            e.clientY,
            rect.width,
            rect.height,
            currentWinW,
            currentWinH
          );
          if (refined.x !== contextMenu.x || refined.y !== contextMenu.y) {
            contextMenu = { ...contextMenu, x: refined.x, y: refined.y };
          }
        }
      }
    });
  }

  function handleMenuAction(action: 'copy-path' | 'reveal' | 'rename' | 'trash') {
    if (!contextMenu) return;
    const entry = contextMenu.entry;
    closeContextMenu();

    switch (action) {
      case 'copy-path':
        copyPathToClipboard(entry.path);
        break;
      case 'reveal':
        void revealInFileManager(entry.path).catch(() => {});
        break;
      case 'rename':
        startRename(entry);
        break;
      case 'trash':
        confirmDeleteEntry = entry;
        break;
    }
  }

  $effect(() => {
    if (!contextMenu) return;

    function handlePointerDown(e: MouseEvent | PointerEvent) {
      if (contextMenuEl && !contextMenuEl.contains(e.target as Node)) {
        closeContextMenu();
      }
    }

    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') {
        e.stopPropagation();
        closeContextMenu();
      }
    }

    function handleBlurOrResize() {
      closeContextMenu();
    }

    window.addEventListener('pointerdown', handlePointerDown, { capture: true });
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('blur', handleBlurOrResize);
    window.addEventListener('resize', handleBlurOrResize);

    return () => {
      window.removeEventListener('pointerdown', handlePointerDown, { capture: true });
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('blur', handleBlurOrResize);
      window.removeEventListener('resize', handleBlurOrResize);
    };
  });
</script>

<aside class="file-tree-panel" bind:this={treeContainerEl} tabindex="-1">
  <div class="panel-header">
    <span class="panel-title">Files</span>
    <div class="header-actions">
      <button
        class="icon-btn"
        title="New file"
        aria-label="New file"
        onclick={() => startCreate(false)}
      >
        <FilePlus size={14} strokeWidth={2} />
      </button>
      <button
        class="icon-btn"
        title="New folder"
        aria-label="New folder"
        onclick={() => startCreate(true)}
      >
        <FolderPlus size={14} strokeWidth={2} />
      </button>
      <button
        class="icon-btn"
        title="Refresh file tree"
        aria-label="Refresh file tree"
        onclick={() => refresh()}
      >
        <RefreshCw size={14} strokeWidth={2} />
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="error-banner">
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if creatingMode}
    <div class="inline-form">
      <input
        bind:value={newNameInput}
        type="text"
        class="inline-input"
        placeholder={creatingMode.isDir ? 'New folder name...' : 'New file name...'}
        onkeydown={(e) => {
          if (e.key === 'Enter') void submitCreate();
          if (e.key === 'Escape') creatingMode = null;
        }}
      />
      <button class="primary small-btn" onclick={submitCreate}>Create</button>
    </div>
  {/if}

  <div class="tree-scroll" onscroll={closeContextMenu}>
    {#if flatRows.length === 0}
      <div class="empty-box">Folder is empty</div>
    {:else}
      {#each flatRows as row (row.entry.path)}
        {@const badge = gitBadges[row.entry.relPath]}
        <div
          class="tree-row"
          class:selected={selectedPath === row.entry.path}
          style:padding-left="{8 + row.depth * 14}px"
          role="treeitem"
          aria-selected={selectedPath === row.entry.path}
          tabindex="-1"
          oncontextmenu={(e) => handleRowContextMenu(e, row.entry)}
        >
          {#if renamingEntry?.path === row.entry.path}
            <input
              bind:value={renameInput}
              type="text"
              class="inline-input"
              use:focusAndSelect
              onblur={() => void submitRename()}
              onkeydown={(e) => {
                if (e.key === 'Enter') void submitRename();
                if (e.key === 'Escape') renamingEntry = null;
              }}
            />
          {:else}
            <button class="row-main" onclick={() => void toggleEntry(row.entry)}>
              {#if row.entry.isDir}
                {#if expandedDirs[row.entry.path]}
                  <FolderOpen size={14} strokeWidth={2} />
                {:else}
                  <Folder size={14} strokeWidth={2} />
                {/if}
              {:else if isImageFilename(row.entry.name)}
                <Image size={14} strokeWidth={2} />
              {:else}
                <File size={14} strokeWidth={2} />
              {/if}
              <span class="entry-name">{row.entry.name}</span>
              {#if badge}
                <span class="git-badge mono" style:color={badgeColorVar(badge)}>{badge}</span>
              {/if}
            </button>
          {/if}
        </div>
      {/each}
    {/if}
  </div>

  {#if confirmDeleteEntry}
    <div class="confirm-bar">
      <span class="confirm-text">
        Move <span class="mono">{confirmDeleteEntry.name}</span> to trash?
      </span>
      <div class="confirm-actions">
        <button class="danger small-btn" onclick={handleConfirmDelete}>Delete</button>
        <button class="small-btn" onclick={() => (confirmDeleteEntry = null)}>Cancel</button>
      </div>
    </div>
  {/if}

  {#if contextMenu}
    <div
      bind:this={contextMenuEl}
      class="context-menu"
      style:left="{contextMenu.x}px"
      style:top="{contextMenu.y}px"
      role="menu"
      tabindex="-1"
      oncontextmenu={(e) => {
        e.preventDefault();
        e.stopPropagation();
      }}
    >
      <button
        class="context-menu-item"
        role="menuitem"
        onclick={() => handleMenuAction('copy-path')}
      >
        <Copy size={13} strokeWidth={2} />
        <span>Copy Path</span>
      </button>
      <button class="context-menu-item" role="menuitem" onclick={() => handleMenuAction('reveal')}>
        <ExternalLink size={13} strokeWidth={2} />
        <span>Reveal in File Manager</span>
      </button>
      <button class="context-menu-item" role="menuitem" onclick={() => handleMenuAction('rename')}>
        <Pencil size={13} strokeWidth={2} />
        <span>Rename</span>
      </button>
      <button
        class="context-menu-item danger"
        role="menuitem"
        onclick={() => handleMenuAction('trash')}
      >
        <Trash2 size={13} strokeWidth={2} />
        <span>Move to Trash</span>
      </button>
    </div>
  {/if}
</aside>

<style>
  .file-tree-panel {
    width: 100%;
    height: 100%;
    background: var(--pn);
    display: flex;
    flex-direction: column;
    min-height: 0;
    outline: none;
  }

  .panel-header {
    height: 36px;
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--dm);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .error-banner {
    padding: 6px 12px;
    background: var(--des);
    border-bottom: 1px solid var(--bd);
    color: var(--de);
    font-size: 12px;
  }

  .inline-form {
    padding: 6px 8px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .inline-input {
    flex: 1;
    min-width: 0;
    height: 24px;
    padding: 0 6px;
    font-size: 12px;
  }

  .small-btn {
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
  }

  .tree-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .tree-row {
    height: 28px;
    min-height: 28px;
    line-height: 28px;
    flex-shrink: 0;
    box-sizing: border-box;
    padding-right: 6px;
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .tree-row:hover,
  .tree-row.selected {
    background: var(--acs);
  }

  .row-main {
    flex: 1;
    min-width: 0;
    width: 100%;
    height: 26px;
    line-height: 26px;
    box-sizing: border-box;
    padding: 0 4px;
    border: none;
    background: transparent;
    color: var(--tx);
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 6px;
    text-align: left;
  }

  .entry-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }

  .git-badge {
    margin-left: auto;
    flex-shrink: 0;
    font-size: 11px;
    font-weight: 600;
    padding: 0 4px;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    min-width: 180px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-panel);
    box-shadow: var(--shadow-float);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    outline: none;
    user-select: none;
  }

  .context-menu-item {
    width: 100%;
    height: 28px;
    min-height: 28px;
    padding: 0 8px;
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--tx);
    font-size: 12px;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
  }

  .context-menu-item:hover {
    background: var(--acs);
    color: var(--tx);
  }

  .context-menu-item.danger {
    background: transparent;
    color: var(--de);
  }

  .context-menu-item.danger:hover {
    background: var(--des);
    color: var(--de);
  }

  .confirm-actions button.danger {
    background: var(--de);
    border-color: var(--de);
    color: #ffffff;
  }

  .confirm-actions button.danger:hover {
    opacity: 0.92;
  }

  .confirm-bar {
    padding: 8px 12px;
    background: var(--el);
    border-top: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .confirm-text {
    font-size: 12px;
    color: var(--tx);
    word-break: break-all;
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .empty-box {
    flex-shrink: 0;
    box-sizing: border-box;
    padding: 16px 12px;
    font-size: 12px;
    color: var(--dm);
    text-align: center;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
