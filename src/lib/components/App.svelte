<script lang="ts">
  import { onMount } from 'svelte';
  import FileText from '@lucide/svelte/icons/file-text';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import Terminal from '@lucide/svelte/icons/terminal';
  import X from '@lucide/svelte/icons/x';
  import {
    fsClearDraft,
    normalizeTabPath,
    pickProjectFolderDialog,
    projectGetState,
    projectOpenFolder,
    type AppErrorPayload,
    type ProjectStateDto
  } from '$lib/ipc';
  import { createInitialSession, type Pane, type SessionFile, type Tab } from '$lib/layout/types';
  import QuickOpen from '$lib/components/QuickOpen.svelte';
  import FileTab, { type FileTabMeta } from '$lib/components/tabs/FileTab.svelte';

  let projectState = $state<ProjectStateDto>({
    activeFolder: null,
    folderName: null,
    recentFolders: []
  });
  let session = $state<SessionFile>(createInitialSession(null));
  let errorMessage = $state<string | null>(null);
  let loading = $state(true);
  let quickOpenVisible = $state(false);
  let newTabMenuOpen = $state(false);

  let dirtyTabs = $state<Record<string, boolean>>({});
  let tabMetaMap = $state<Record<string, FileTabMeta>>({});
  let pendingCloseTabId = $state<string | null>(null);
  let activeFileTabRef = $state<ReturnType<typeof FileTab> | null>(null);
  let tabCounter = $state(2);

  const activeWorkspace = $derived(
    session.workspaces.find((w) => w.id === session.activeWorkspaceId) ?? session.workspaces[0]
  );

  const rootPane = $derived<Pane | null>(
    activeWorkspace && activeWorkspace.root.type === 'pane' ? activeWorkspace.root : null
  );

  const activeTab = $derived<Tab | null>(
    rootPane?.tabs.find((t) => t.id === rootPane.activeTabId) ?? rootPane?.tabs[0] ?? null
  );

  const activeFileMeta = $derived<FileTabMeta | null>(
    activeTab && activeTab.kind === 'file' ? (tabMetaMap[activeTab.id] ?? null) : null
  );

  onMount(async () => {
    try {
      const state = await projectGetState();
      projectState = state;
      if (state.activeFolder) {
        session = createInitialSession(state.activeFolder);
      }
    } catch {
      projectState = {
        activeFolder: null,
        folderName: null,
        recentFolders: []
      };
    } finally {
      loading = false;
    }
  });

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (!projectState.activeFolder) return;

    const isAltO =
      event.altKey && !event.ctrlKey && !event.metaKey && event.key.toLowerCase() === 'o';
    const isCtrlP =
      (event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 'p';

    if (isAltO || isCtrlP) {
      event.preventDefault();
      quickOpenVisible = true;
      newTabMenuOpen = false;
      return;
    }

    const isCtrlW =
      (event.ctrlKey || event.metaKey) && !event.altKey && event.key.toLowerCase() === 'w';
    if (isCtrlW && activeTab) {
      event.preventDefault();
      requestCloseTab(activeTab.id);
    }
  }

  async function handleChooseFolder() {
    errorMessage = null;
    try {
      const chosen = await pickProjectFolderDialog();
      if (chosen) {
        await openFolderByPath(chosen);
      }
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMessage = normalized.message ?? 'Gagal membuka dialog folder';
    }
  }

  async function openFolderByPath(path: string) {
    errorMessage = null;
    try {
      const updated = await projectOpenFolder(path);
      projectState = updated;
      session = createInitialSession(updated.activeFolder);
      dirtyTabs = {};
      tabMetaMap = {};
      pendingCloseTabId = null;
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMessage = normalized.message ?? 'Gagal membuka folder proyek';
    }
  }

  function extractBaseName(filePath: string): string {
    const normalized = normalizeTabPath(filePath);
    const idx = normalized.lastIndexOf('/');
    return idx === -1 ? normalized : normalized.slice(idx + 1);
  }

  export function openFileInPane(fullPath: string, line: number | null = null) {
    if (!rootPane) return;
    const normalizedTarget = normalizeTabPath(fullPath);

    const existing = rootPane.tabs.find(
      (t) => t.kind === 'file' && normalizeTabPath(String(t.data.path ?? '')) === normalizedTarget
    );

    if (existing) {
      existing.data = {
        ...existing.data,
        path: normalizedTarget,
        line
      };
      rootPane.activeTabId = existing.id;
      return;
    }

    tabCounter += 1;
    const newId = `tab-${tabCounter}`;
    const newTab: Tab = {
      id: newId,
      kind: 'file',
      title: extractBaseName(normalizedTarget),
      data: {
        path: normalizedTarget,
        line
      }
    };

    rootPane.tabs.push(newTab);
    rootPane.activeTabId = newId;
  }

  function addTerminalTab() {
    if (!rootPane) return;
    tabCounter += 1;
    const newId = `tab-${tabCounter}`;
    rootPane.tabs.push({
      id: newId,
      kind: 'terminal',
      title: 'shell',
      data: { cwd: projectState.activeFolder ?? '' }
    });
    rootPane.activeTabId = newId;
    newTabMenuOpen = false;
  }

  function requestCloseTab(tabId: string) {
    if (dirtyTabs[tabId]) {
      pendingCloseTabId = tabId;
      return;
    }
    forceCloseTab(tabId);
  }

  function forceCloseTab(tabId: string) {
    if (!rootPane) return;
    const index = rootPane.tabs.findIndex((t) => t.id === tabId);
    if (index === -1) return;

    const targetTab = rootPane.tabs[index];
    if (targetTab.kind === 'file' && typeof targetTab.data.path === 'string') {
      void fsClearDraft(targetTab.data.path).catch(() => {});
    }

    rootPane.tabs.splice(index, 1);
    delete dirtyTabs[tabId];
    delete tabMetaMap[tabId];

    if (rootPane.tabs.length === 0) {
      addTerminalTab();
    } else if (rootPane.activeTabId === tabId) {
      const nextIdx = Math.min(index, rootPane.tabs.length - 1);
      rootPane.activeTabId = rootPane.tabs[nextIdx].id;
    }
  }

  async function handleConfirmSaveAndClose() {
    const targetId = pendingCloseTabId;
    if (!targetId || !rootPane) return;

    if (rootPane.activeTabId !== targetId) {
      rootPane.activeTabId = targetId;
    }

    if (activeFileTabRef) {
      const ok = await activeFileTabRef.save();
      if (!ok) {
        pendingCloseTabId = null;
        return;
      }
    }
    pendingCloseTabId = null;
    forceCloseTab(targetId);
  }

  function handleDiscardAndClose() {
    const targetId = pendingCloseTabId;
    pendingCloseTabId = null;
    if (targetId) {
      forceCloseTab(targetId);
    }
  }
</script>

<svelte:window onkeydown={handleGlobalKeydown} />

<div class="app-shell">
  {#if loading}
    <main class="empty-screen">
      <p class="muted">Memuat ruang kerja...</p>
    </main>
  {:else if !projectState.activeFolder}
    <main class="empty-screen">
      <div class="welcome-card">
        <h1 class="welcome-title">glzr</h1>
        <p class="muted">Pilih folder proyek untuk memulai ruang kerja.</p>

        {#if errorMessage}
          <p class="error-banner">{errorMessage}</p>
        {/if}

        <div class="welcome-actions">
          <button class="primary" onclick={handleChooseFolder}>
            <FolderOpen size={14} strokeWidth={2} />
            <span>Buka folder</span>
          </button>
        </div>

        {#if projectState.recentFolders.length > 0}
          <div class="recent-section">
            <span class="section-label">Folder terakhir</span>
            <ul class="recent-list">
              {#each projectState.recentFolders as folderPath (folderPath)}
                <li>
                  <button class="recent-item" onclick={() => openFolderByPath(folderPath)}>
                    <span class="recent-path">{folderPath}</span>
                  </button>
                </li>
              {/each}
            </ul>
          </div>
        {/if}
      </div>
    </main>
  {:else}
    <header class="workspace-bar">
      <div class="workspace-list">
        {#each session.workspaces as ws (ws.id)}
          <button
            class="workspace-item"
            class:active={ws.id === session.activeWorkspaceId}
            onclick={() => (session.activeWorkspaceId = ws.id)}
          >
            {ws.name}
          </button>
        {/each}
        <button class="icon-btn" title="Workspace baru" aria-label="Workspace baru">
          <Plus size={14} strokeWidth={2} />
        </button>
      </div>
      <div class="workspace-actions">
        <button
          class="folder-switch"
          title="Buka Cepat File (Alt+O)"
          onclick={() => (quickOpenVisible = true)}
        >
          <Search size={14} strokeWidth={2} />
          <span>Buka file</span>
        </button>
        <button class="folder-switch" onclick={handleChooseFolder}>
          <FolderOpen size={14} strokeWidth={2} />
          <span>{projectState.folderName ?? projectState.activeFolder}</span>
        </button>
      </div>
    </header>

    <div class="workbench">
      <section class="pane focused">
        {#if rootPane}
          <div class="tab-strip">
            {#each rootPane.tabs as tab (tab.id)}
              <div class="tab-wrapper" class:active={tab.id === rootPane.activeTabId}>
                <button
                  class="tab-item"
                  class:active={tab.id === rootPane.activeTabId}
                  onclick={() => (rootPane.activeTabId = tab.id)}
                >
                  {#if tab.kind === 'file'}
                    <FileText size={14} strokeWidth={2} />
                  {:else}
                    <Terminal size={14} strokeWidth={2} />
                  {/if}
                  <span>{tab.title}</span>
                  {#if dirtyTabs[tab.id]}
                    <span class="dirty-dot" aria-label="Belum disimpan"></span>
                  {/if}
                </button>
                <button
                  class="tab-close-btn"
                  title="Tutup tab (Ctrl+W)"
                  aria-label="Tutup tab"
                  onclick={(e) => {
                    e.stopPropagation();
                    requestCloseTab(tab.id);
                  }}
                >
                  <X size={12} strokeWidth={2} />
                </button>
              </div>
            {/each}

            <div class="new-tab-dropdown">
              <button
                class="icon-btn"
                title="Tab baru"
                aria-label="Tab baru"
                onclick={() => (newTabMenuOpen = !newTabMenuOpen)}
              >
                <Plus size={14} strokeWidth={2} />
              </button>
              {#if newTabMenuOpen}
                <div class="dropdown-menu">
                  <button
                    class="dropdown-item"
                    onclick={() => {
                      newTabMenuOpen = false;
                      quickOpenVisible = true;
                    }}
                  >
                    <FileText size={14} strokeWidth={2} />
                    <span>Buka file...</span>
                  </button>
                  <button class="dropdown-item" onclick={addTerminalTab}>
                    <Terminal size={14} strokeWidth={2} />
                    <span>Terminal baru</span>
                  </button>
                </div>
              {/if}
            </div>
          </div>

          <div class="pane-body" class:no-pad={activeTab?.kind === 'file'}>
            {#if activeTab}
              {#if activeTab.kind === 'file' && typeof activeTab.data.path === 'string'}
                {#key activeTab.id}
                  <FileTab
                    bind:this={activeFileTabRef}
                    path={activeTab.data.path}
                    targetLine={typeof activeTab.data.line === 'number'
                      ? activeTab.data.line
                      : null}
                    onDirtyChange={(dirty) => {
                      dirtyTabs[activeTab.id] = dirty;
                    }}
                    onMetaChange={(meta) => {
                      tabMetaMap[activeTab.id] = meta;
                    }}
                  />
                {/key}
              {:else}
                <div class="terminal-placeholder">
                  <span class="mono">{projectState.activeFolder} $</span>
                </div>
              {/if}
            {/if}
          </div>
        {/if}
      </section>
    </div>

    <footer class="status-bar">
      <div class="status-left">
        <span>{projectState.folderName ?? 'proyek'}</span>
      </div>
      <div class="status-right">
        {#if activeFileMeta}
          {#if activeFileMeta.kind === 'text' || activeFileMeta.kind === 'svg'}
            <span class="mono">Ln {activeFileMeta.line}, Col {activeFileMeta.col}</span>
          {/if}
          <span>{activeFileMeta.language}</span>
          <span>{activeFileMeta.encoding}</span>
          <span>{activeFileMeta.lineEnding}</span>
        {:else}
          <span>UTF-8</span>
          <span>LF</span>
        {/if}
      </div>
    </footer>

    <QuickOpen
      open={quickOpenVisible}
      workspaceRoot={projectState.activeFolder}
      onClose={() => (quickOpenVisible = false)}
      onSelect={(fullPath, line) => openFileInPane(fullPath, line)}
    />

    {#if pendingCloseTabId}
      <div class="modal-backdrop">
        <div class="confirm-dialog" role="dialog" aria-modal="true" aria-label="Konfirmasi simpan">
          <h2 class="confirm-title">Simpan perubahan sebelum menutup?</h2>
          <p class="muted">Perubahan yang belum disimpan akan hilang jika tab ditutup.</p>
          <div class="confirm-actions">
            <button class="primary" onclick={handleConfirmSaveAndClose}>Simpan</button>
            <button class="danger" onclick={handleDiscardAndClose}>Buang</button>
            <button onclick={() => (pendingCloseTabId = null)}>Batal</button>
          </div>
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    background: var(--bg);
    color: var(--tx);
  }

  .empty-screen {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
  }

  .welcome-card {
    width: 100%;
    max-width: 420px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-dialog);
    padding: 24px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .welcome-title {
    font-size: 15px;
    font-weight: 600;
    color: var(--tx);
  }

  .muted {
    color: var(--dm);
  }

  .error-banner {
    color: var(--de);
    font-size: 12px;
  }

  .welcome-actions {
    display: flex;
    gap: 8px;
  }

  .recent-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-top: 8px;
    border-top: 1px solid var(--bd);
  }

  .section-label {
    font-size: 12px;
    color: var(--dm);
  }

  .recent-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .recent-item {
    width: 100%;
    height: 28px;
    padding: 0 8px;
    justify-content: flex-start;
    border: none;
    color: var(--dm);
  }

  .recent-item:hover {
    background: var(--acs);
    color: var(--tx);
  }

  .recent-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .workspace-bar {
    height: 40px;
    padding: 0 16px;
    background: var(--pn);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .workspace-list,
  .workspace-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .workspace-item {
    height: 26px;
    padding: 4px 12px;
    border: none;
    border-radius: var(--radius-ctrl);
    color: var(--dm);
  }

  .workspace-item.active {
    background: var(--acs);
    color: var(--ac);
  }

  .folder-switch {
    height: 26px;
    padding: 4px 10px;
    border: none;
    color: var(--dm);
  }

  .icon-btn {
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    color: var(--dm);
  }

  .workbench {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    min-height: 200px;
    background: var(--bg);
  }

  .pane.focused {
    border-top: 2px solid var(--ac);
  }

  .tab-strip {
    height: 36px;
    padding: 0 12px;
    background: var(--pn);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .tab-wrapper {
    display: flex;
    align-items: center;
    border-radius: var(--radius-ctrl);
    padding-right: 4px;
  }

  .tab-wrapper.active {
    background: var(--el);
  }

  .tab-item {
    height: 26px;
    padding: 0 8px 0 10px;
    border: none;
    background: transparent;
    color: var(--dm);
    gap: 8px;
  }

  .tab-item.active {
    color: var(--tx);
  }

  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ac);
  }

  .tab-close-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
    opacity: 0;
  }

  .tab-wrapper:hover .tab-close-btn,
  .tab-wrapper.active .tab-close-btn {
    opacity: 1;
  }

  .new-tab-dropdown {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 30px;
    left: 0;
    min-width: 160px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-dialog);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 30;
  }

  .dropdown-item {
    width: 100%;
    height: 28px;
    padding: 0 10px;
    border: none;
    background: transparent;
    color: var(--tx);
    justify-content: flex-start;
    gap: 8px;
  }

  .dropdown-item:hover {
    background: var(--acs);
  }

  .pane-body {
    flex: 1;
    padding: 16px 24px;
    overflow: auto;
    min-height: 0;
  }

  .pane-body.no-pad {
    padding: 0;
    overflow: hidden;
    display: flex;
  }

  .terminal-placeholder {
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    line-height: 1.8;
    color: var(--dm);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .status-bar {
    height: 28px;
    padding: 0 16px;
    background: var(--pn);
    border-top: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--dm);
  }

  .status-left,
  .status-right {
    display: flex;
    align-items: center;
    gap: 20px;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    z-index: 120;
  }

  .confirm-dialog {
    width: 100%;
    max-width: 400px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-dialog);
    padding: 20px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .confirm-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--tx);
  }

  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding-top: 4px;
  }
</style>
