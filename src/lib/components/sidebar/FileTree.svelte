<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Copy from '@lucide/svelte/icons/copy';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import FilePlus from '@lucide/svelte/icons/file-plus';
  import Folder from '@lucide/svelte/icons/folder';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import FolderPlus from '@lucide/svelte/icons/folder-plus';
  import Pencil from '@lucide/svelte/icons/pencil';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import Trash2 from '@lucide/svelte/icons/trash-2';
  import FileIcon from '$lib/components/icons/FileIcon.svelte';
  import {
    clipboardWriteText,
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
  import { smoothScroll } from '$lib/scroll';

  interface Props {
    workspaceRoot: string;
    gitBadges?: Record<string, GitBadge>;
    gitDirtyFolders?: Set<string>;
    onOpenFile: (fullPath: string) => void;
    onEntryRenamed?: (oldPath: string, newPath: string) => void;
    onEntryDeleted?: (deletedPath: string) => void;
  }

  let {
    workspaceRoot,
    gitBadges = {},
    gitDirtyFolders = new Set<string>(),
    onOpenFile,
    onEntryRenamed,
    onEntryDeleted
  }: Props = $props();

  interface TreeRow {
    entry: DirEntryDto;
    depth: number;
    cleanRel: string;
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

  function portal(node: HTMLElement) {
    if (typeof document !== 'undefined' && document.body) {
      document.body.appendChild(node);
    }
    return {
      destroy() {
        node.remove();
      }
    };
  }

  const cleanRelCache = new Map<string, string>();

  function getCleanRel(relPath: string): string {
    const cached = cleanRelCache.get(relPath);
    if (cached !== undefined) return cached;
    const cleaned = relPath.replace(/\\/g, '/').replace(/^\/+/, '').replace(/\/+$/, '');
    cleanRelCache.set(relPath, cleaned);
    return cleaned;
  }

  const flatRows = $derived.by<TreeRow[]>(() => {
    const out: TreeRow[] = [];
    function walk(dirKey: string, depth: number) {
      const items = dirCache[dirKey] ?? [];
      for (const item of items) {
        out.push({ entry: item, depth, cleanRel: getCleanRel(item.relPath) });
        if (item.isDir && expandedDirs[item.path]) {
          walk(item.path, depth + 1);
        }
      }
    }
    walk('', 0);
    return out;
  });

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
    cleanRelCache.clear();
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
    void clipboardWriteText(path).catch(() => {});
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

  <div class="tree-scroll" use:smoothScroll onscroll={closeContextMenu}>
    {#if flatRows.length === 0}
      <div class="empty-box">Folder is empty</div>
    {:else}
      {#each flatRows as row (row.entry.path)}
        {@const badge = gitBadges[row.entry.relPath]}
        {@const isDirtyFolder = row.entry.isDir && gitDirtyFolders.has(row.cleanRel)}
        <div
          class="tree-row"
          class:selected={selectedPath === row.entry.path}
          class:has-unstaged-changes={isDirtyFolder}
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
              {:else}
                <FileIcon filename={row.entry.name} size={14} />
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
      use:portal
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
    background: var(--pbg);
    display: flex;
    flex-direction: column;
    min-height: 0;
    outline: none;
  }

  .panel-header {
    height: auto;
    min-height: calc(32px * var(--ui-scale, 1));
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    font-size: var(--font-size-ui, 13px);
    font-weight: 500;
    color: var(--dm);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .icon-btn {
    width: auto;
    min-width: calc(24px * var(--ui-scale, 1));
    height: auto;
    min-height: calc(24px * var(--ui-scale, 1));
    padding: 2px;
    border: none;
    background: transparent;
    color: var(--dm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .icon-btn :global(svg) {
    width: 1.1em;
    height: 1.1em;
    flex-shrink: 0;
  }

  .error-banner {
    padding: 6px 12px;
    background: var(--des);
    border-bottom: 1px solid var(--bd);
    color: var(--de);
    font-size: var(--font-size-ui-sm, 12px);
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
    height: auto;
    min-height: calc(26px * var(--ui-scale, 1));
    padding: 0 8px;
    font-size: var(--font-size-ui, 13px);
  }

  .small-btn {
    height: auto;
    min-height: calc(24px * var(--ui-scale, 1));
    padding: 0 8px;
    font-size: var(--font-size-ui-sm, 12px);
  }

  .tree-scroll {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    overscroll-behavior: contain;
    background: var(--pbg);
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .tree-row {
    content-visibility: auto;
    contain-intrinsic-size: auto 28px;
    height: auto;
    min-height: calc(28px * var(--ui-scale, 1));
    flex-shrink: 0;
    box-sizing: border-box;
    padding-right: 6px;
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    justify-content: flex-start;
  }

  .tree-row:hover {
    background: var(--el);
  }

  .tree-row.selected {
    background: var(--acs);
  }

  .tree-row.has-unstaged-changes .row-main {
    color: var(--mo);
  }

  .tree-row.has-unstaged-changes .row-main :global(svg),
  .tree-row.has-unstaged-changes .entry-name {
    color: var(--mo);
  }

  .row-main {
    flex: 1;
    min-width: 0;
    width: 100%;
    height: auto;
    min-height: calc(26px * var(--ui-scale, 1));
    line-height: normal;
    box-sizing: border-box;
    padding: 2px 4px;
    border: none;
    background: transparent;
    color: var(--tx);
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: calc(6px * var(--ui-scale, 1));
    text-align: left;
  }

  .row-main :global(svg) {
    width: 1.15em;
    height: 1.15em;
    flex-shrink: 0;
  }

  .row-main:hover {
    background: transparent;
  }

  .entry-name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-ui, 13px);
  }

  .git-badge {
    margin-left: auto;
    flex-shrink: 0;
    font-size: var(--font-size-ui-sm, 12px);
    font-weight: 600;
    padding: 0 4px;
  }

  .context-menu {
    position: fixed;
    z-index: 1000;
    min-width: 185px;
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-menu);
    box-shadow: var(--shadow-float);
    padding: 5px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    outline: none;
    user-select: none;
  }

  .context-menu-item {
    width: 100%;
    height: auto;
    min-height: calc(28px * var(--ui-scale, 1));
    padding: 2px calc(8px * var(--ui-scale, 1));
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--tx);
    font-size: var(--font-size-ui, 13px);
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: calc(8px * var(--ui-scale, 1));
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
  }

  .context-menu-item :global(svg) {
    width: 1.1em;
    height: 1.1em;
    flex-shrink: 0;
  }

  .context-menu-item:hover {
    background: var(--acs);
    color: var(--tx);
  }

  .context-menu-item.danger,
  button.context-menu-item.danger {
    background: transparent !important;
    color: var(--de) !important;
  }

  .context-menu-item.danger:hover,
  button.context-menu-item.danger:hover {
    background: var(--des) !important;
    color: var(--de) !important;
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
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border-top: 1px solid var(--pbd);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .confirm-text {
    font-size: var(--font-size-ui, 13px);
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
    font-size: var(--font-size-ui, 13px);
    color: var(--dm);
    text-align: center;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
