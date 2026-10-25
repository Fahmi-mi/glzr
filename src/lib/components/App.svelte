<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import X from '@lucide/svelte/icons/x';
  import {
    fsClearDraft,
    gitStatus,
    normalizeTabPath,
    pickProjectFolderDialog,
    projectGetState,
    projectOpenFolder,
    sessionLoad,
    sessionSave,
    type AppErrorPayload,
    type GitStatusDto,
    type ProjectStateDto
  } from '$lib/ipc';
  import {
    closePane,
    findDirectionalNeighborPane,
    findPaneById,
    getFirstPaneId,
    migrateAndSanitizeSession,
    moveTabBetweenPanes,
    splitPane,
    updateSplitRatioByPath
  } from '$lib/layout/tree';
  import {
    createInitialSession,
    createInitialWorkspace,
    type Pane,
    type SessionFile,
    type Tab
  } from '$lib/layout/types';
  import PaneNode from '$lib/components/PaneNode.svelte';
  import QuickOpen from '$lib/components/QuickOpen.svelte';
  import GitPanel from '$lib/components/sidebar/GitPanel.svelte';
  import type { FileTabMeta } from '$lib/components/tabs/FileTab.svelte';
  import type FileTab from '$lib/components/tabs/FileTab.svelte';

  let projectState = $state<ProjectStateDto>({
    activeFolder: null,
    folderName: null,
    recentFolders: []
  });
  let session = $state<SessionFile>(createInitialSession(null));
  let errorMessage = $state<string | null>(null);
  let sessionNotice = $state<string | null>(null);
  let loading = $state(true);
  let quickOpenVisible = $state(false);

  let focusedPaneByWs = $state<Record<string, string>>({});
  let maximizedPaneByWs = $state<Record<string, string | null>>({});
  let renamingWsId = $state<string | null>(null);
  let renameInputText = $state('');

  let dirtyTabs = $state<Record<string, boolean>>({});
  let tabMetaMap = $state<Record<string, FileTabMeta>>({});
  let fileTabRefs = $state<Record<string, ReturnType<typeof FileTab> | null>>({});
  let pendingClose = $state<{ paneId: string; tabId: string } | null>(null);
  let gitPanelRef = $state<ReturnType<typeof GitPanel> | null>(null);
  let repoStatus = $state<GitStatusDto | null>(null);

  let idSeq = $state(100);
  let saveSessionTimer: ReturnType<typeof setTimeout> | null = null;
  let sessionReady = $state(false);

  const activeWorkspace = $derived(
    session.workspaces.find((w) => w.id === session.activeWorkspaceId) ?? session.workspaces[0]
  );

  const focusedPaneId = $derived<string>(
    activeWorkspace
      ? (focusedPaneByWs[activeWorkspace.id] ?? getFirstPaneId(activeWorkspace.root))
      : ''
  );

  const focusedPane = $derived<Pane | null>(
    activeWorkspace ? findPaneById(activeWorkspace.root, focusedPaneId) : null
  );

  const activeTab = $derived<Tab | null>(
    focusedPane?.tabs.find((t) => t.id === focusedPane.activeTabId) ?? focusedPane?.tabs[0] ?? null
  );

  const activeFileMeta = $derived<FileTabMeta | null>(
    activeTab && activeTab.kind === 'file' ? (tabMetaMap[activeTab.id] ?? null) : null
  );

  function nextId(prefix: string): string {
    idSeq += 1;
    return `${prefix}-${Date.now().toString(36)}-${idSeq}`;
  }

  function scheduleSessionSave() {
    if (!sessionReady || !projectState.activeFolder) return;
    if (saveSessionTimer !== null) {
      clearTimeout(saveSessionTimer);
    }
    const snapshot = JSON.stringify({
      ...session,
      folder: projectState.activeFolder
    });
    saveSessionTimer = setTimeout(() => {
      void sessionSave(snapshot).catch(() => {});
    }, 400);
  }

  $effect(() => {
    if (sessionReady && projectState.activeFolder) {
      void JSON.stringify(session);
      scheduleSessionSave();
    }
  });

  async function refreshRepoStatus() {
    if (!projectState.activeFolder) {
      repoStatus = null;
      return;
    }
    if (gitPanelRef) {
      await gitPanelRef.refresh();
    } else {
      repoStatus = await gitStatus().catch(() => null);
    }
  }

  onMount(async () => {
    try {
      const [state, loadedSession] = await Promise.all([
        projectGetState(),
        sessionLoad().catch(() => ({ rawJson: null, backedUpCorrupt: false }))
      ]);
      projectState = state;
      if (loadedSession.backedUpCorrupt) {
        sessionNotice =
          'File sesi sebelumnya tidak kompatibel atau rusak dan telah dicadangkan ke session.json.bak.';
      }

      if (state.activeFolder) {
        if (loadedSession.rawJson) {
          try {
            const parsed = JSON.parse(loadedSession.rawJson);
            const restored = migrateAndSanitizeSession(parsed, state.activeFolder);
            if (!restored.folder || restored.folder === state.activeFolder) {
              session = { ...restored, folder: state.activeFolder };
            } else {
              session = createInitialSession(state.activeFolder);
            }
          } catch {
            session = createInitialSession(state.activeFolder);
          }
        } else {
          session = createInitialSession(state.activeFolder);
        }
        await refreshRepoStatus();
      }
    } catch {
      projectState = {
        activeFolder: null,
        folderName: null,
        recentFolders: []
      };
    } finally {
      loading = false;
      sessionReady = true;
    }
  });

  onDestroy(() => {
    if (saveSessionTimer !== null) {
      clearTimeout(saveSessionTimer);
      saveSessionTimer = null;
    }
  });

  function setFocusedPane(paneId: string) {
    if (!activeWorkspace) return;
    focusedPaneByWs[activeWorkspace.id] = paneId;
  }

  function toggleGitSidebar() {
    if (session.sidebar.visible && session.sidebar.panel === 'git') {
      session.sidebar.visible = false;
    } else {
      session.sidebar.visible = true;
      session.sidebar.panel = 'git';
      queueMicrotask(() => {
        gitPanelRef?.focusInput();
      });
    }
  }

  function handleSplitPane(paneId: string, direction: 'row' | 'column') {
    if (!activeWorkspace) return;
    const newPaneId = nextId('pane');
    activeWorkspace.root = splitPane(activeWorkspace.root, paneId, direction, newPaneId);
    maximizedPaneByWs[activeWorkspace.id] = null;
    focusedPaneByWs[activeWorkspace.id] = newPaneId;
  }

  function handleClosePane(paneId: string) {
    if (!activeWorkspace) return;
    const targetPane = findPaneById(activeWorkspace.root, paneId);
    if (!targetPane) return;

    const dirtyTab = targetPane.tabs.find((t) => dirtyTabs[t.id]);
    if (dirtyTab) {
      pendingClose = { paneId, tabId: dirtyTab.id };
      return;
    }

    const res = closePane(activeWorkspace.root, paneId);
    if (res) {
      activeWorkspace.root = res.root;
      maximizedPaneByWs[activeWorkspace.id] = null;
      focusedPaneByWs[activeWorkspace.id] = res.nextFocusPaneId;
    }
  }

  function handleToggleMaximizePane(paneId: string) {
    if (!activeWorkspace) return;
    const current = maximizedPaneByWs[activeWorkspace.id] ?? null;
    maximizedPaneByWs[activeWorkspace.id] = current === paneId ? null : paneId;
    focusedPaneByWs[activeWorkspace.id] = paneId;
  }

  function handleAddWorkspace() {
    const num = session.workspaces.length + 1;
    const wsId = nextId('ws');
    const newWs = createInitialWorkspace(wsId, `${num} Ruang`, projectState.activeFolder ?? '');
    session.workspaces.push(newWs);
    session.activeWorkspaceId = wsId;
  }

  function handleCloseWorkspace(wsId: string) {
    if (session.workspaces.length <= 1) return;
    const idx = session.workspaces.findIndex((w) => w.id === wsId);
    if (idx === -1) return;
    session.workspaces.splice(idx, 1);
    if (session.activeWorkspaceId === wsId) {
      const nextIdx = Math.min(idx, session.workspaces.length - 1);
      session.activeWorkspaceId = session.workspaces[nextIdx].id;
    }
  }

  function commitWorkspaceRename(wsId: string) {
    const trimmed = renameInputText.trim();
    const target = session.workspaces.find((w) => w.id === wsId);
    if (target && trimmed) {
      target.name = trimmed;
    }
    renamingWsId = null;
  }

  function handleGlobalKeydown(event: KeyboardEvent) {
    if (!projectState.activeFolder || !activeWorkspace) return;

    const terminalFocused = activeTab?.kind === 'terminal';
    const altOnly = event.altKey && !event.ctrlKey && !event.metaKey;
    const keyLower = event.key.toLowerCase();

    const isAltO = altOnly && keyLower === 'o';
    const isCtrlP =
      !terminalFocused && (event.ctrlKey || event.metaKey) && !event.altKey && keyLower === 'p';

    if (isAltO || isCtrlP) {
      event.preventDefault();
      quickOpenVisible = true;
      return;
    }

    if (altOnly && keyLower === 'v') {
      event.preventDefault();
      toggleGitSidebar();
      return;
    }

    if (altOnly && keyLower === 't') {
      event.preventDefault();
      addTerminalTab(focusedPaneId);
      return;
    }

    if (altOnly && keyLower === 'r') {
      event.preventDefault();
      handleSplitPane(focusedPaneId, 'row');
      return;
    }

    if (altOnly && keyLower === 'j') {
      event.preventDefault();
      handleSplitPane(focusedPaneId, 'column');
      return;
    }

    if (altOnly && keyLower === 'q') {
      event.preventDefault();
      handleClosePane(focusedPaneId);
      return;
    }

    if (altOnly && keyLower === 'm') {
      event.preventDefault();
      handleToggleMaximizePane(focusedPaneId);
      return;
    }

    if (
      altOnly &&
      (event.key === 'ArrowLeft' ||
        event.key === 'ArrowRight' ||
        event.key === 'ArrowUp' ||
        event.key === 'ArrowDown')
    ) {
      event.preventDefault();
      const dirMap: Record<string, 'left' | 'right' | 'up' | 'down'> = {
        ArrowLeft: 'left',
        ArrowRight: 'right',
        ArrowUp: 'up',
        ArrowDown: 'down'
      };
      const neighbor = findDirectionalNeighborPane(
        activeWorkspace.root,
        focusedPaneId,
        dirMap[event.key]
      );
      if (neighbor) {
        setFocusedPane(neighbor);
      }
      return;
    }

    if (altOnly && /^[1-9]$/.test(event.key)) {
      const idx = Number.parseInt(event.key, 10) - 1;
      if (idx >= 0 && idx < session.workspaces.length) {
        event.preventDefault();
        session.activeWorkspaceId = session.workspaces[idx].id;
      }
      return;
    }

    const isCtrlW =
      !terminalFocused && (event.ctrlKey || event.metaKey) && !event.altKey && keyLower === 'w';
    const isAltW = altOnly && keyLower === 'w';
    if ((isCtrlW || isAltW) && focusedPane && activeTab) {
      event.preventDefault();
      requestCloseTab(focusedPane.id, activeTab.id);
      return;
    }

    if (altOnly && focusedPane && focusedPane.tabs.length > 1) {
      if (event.key === 'PageDown' || event.key === 'PageUp') {
        event.preventDefault();
        const curIdx = focusedPane.tabs.findIndex((t) => t.id === focusedPane.activeTabId);
        const delta = event.key === 'PageDown' ? 1 : -1;
        const nextIdx = (curIdx + delta + focusedPane.tabs.length) % focusedPane.tabs.length;
        focusedPane.activeTabId = focusedPane.tabs[nextIdx].id;
      }
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
      focusedPaneByWs = {};
      maximizedPaneByWs = {};
      dirtyTabs = {};
      tabMetaMap = {};
      pendingClose = null;
      await refreshRepoStatus();
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

  export function openFileInPane(
    fullPath: string,
    line: number | null = null,
    targetPaneId?: string
  ) {
    if (!activeWorkspace) return;
    const pane = findPaneById(activeWorkspace.root, targetPaneId ?? focusedPaneId);
    if (!pane) return;

    const normalizedTarget = normalizeTabPath(fullPath);
    const existing = pane.tabs.find(
      (t) => t.kind === 'file' && normalizeTabPath(String(t.data.path ?? '')) === normalizedTarget
    );

    if (existing) {
      existing.data = {
        ...existing.data,
        path: normalizedTarget,
        line
      };
      pane.activeTabId = existing.id;
      setFocusedPane(pane.id);
      return;
    }

    const newId = nextId('tab');
    const newTab: Tab = {
      id: newId,
      kind: 'file',
      title: extractBaseName(normalizedTarget),
      data: {
        path: normalizedTarget,
        line
      }
    };

    pane.tabs.push(newTab);
    pane.activeTabId = newId;
    setFocusedPane(pane.id);
  }

  export function openDiffInPane(relPath: string, staged: boolean) {
    if (!activeWorkspace) return;
    const pane = findPaneById(activeWorkspace.root, focusedPaneId);
    if (!pane) return;

    const normalizedRel = normalizeTabPath(relPath);
    const existing = pane.tabs.find(
      (t) => t.kind === 'diff' && normalizeTabPath(String(t.data.relPath ?? '')) === normalizedRel
    );

    if (existing) {
      existing.data = {
        ...existing.data,
        relPath: normalizedRel,
        staged
      };
      pane.activeTabId = existing.id;
      return;
    }

    const newId = nextId('tab');
    const newTab: Tab = {
      id: newId,
      kind: 'diff',
      title: `${extractBaseName(normalizedRel)} (diff)`,
      data: {
        relPath: normalizedRel,
        staged
      }
    };

    pane.tabs.push(newTab);
    pane.activeTabId = newId;
  }

  function addTerminalTab(paneId: string) {
    if (!activeWorkspace) return;
    const pane = findPaneById(activeWorkspace.root, paneId);
    if (!pane) return;

    const newId = nextId('tab');
    pane.tabs.push({
      id: newId,
      kind: 'terminal',
      title: 'shell',
      data: { cwd: projectState.activeFolder ?? '' }
    });
    pane.activeTabId = newId;
    setFocusedPane(pane.id);
  }

  function requestCloseTab(paneId: string, tabId: string) {
    if (dirtyTabs[tabId]) {
      pendingClose = { paneId, tabId };
      return;
    }
    forceCloseTab(paneId, tabId);
  }

  function forceCloseTab(paneId: string, tabId: string) {
    if (!activeWorkspace) return;
    const pane = findPaneById(activeWorkspace.root, paneId);
    if (!pane) return;

    const index = pane.tabs.findIndex((t) => t.id === tabId);
    if (index === -1) return;

    const targetTab = pane.tabs[index];
    if (targetTab.kind === 'file' && typeof targetTab.data.path === 'string') {
      void fsClearDraft(targetTab.data.path).catch(() => {});
    }

    pane.tabs.splice(index, 1);
    delete dirtyTabs[tabId];
    delete tabMetaMap[tabId];
    delete fileTabRefs[tabId];

    if (pane.tabs.length === 0) {
      const collapsed = closePane(activeWorkspace.root, paneId);
      if (collapsed) {
        activeWorkspace.root = collapsed.root;
        maximizedPaneByWs[activeWorkspace.id] = null;
        focusedPaneByWs[activeWorkspace.id] = collapsed.nextFocusPaneId;
      } else {
        addTerminalTab(paneId);
      }
    } else if (pane.activeTabId === tabId) {
      const nextIdx = Math.min(index, pane.tabs.length - 1);
      pane.activeTabId = pane.tabs[nextIdx].id;
    }
  }

  async function handleConfirmSaveAndClose() {
    const target = pendingClose;
    if (!target) return;

    const fileRef = fileTabRefs[target.tabId];
    if (fileRef) {
      const ok = await fileRef.save();
      if (!ok) {
        pendingClose = null;
        return;
      }
    }
    pendingClose = null;
    forceCloseTab(target.paneId, target.tabId);
  }

  function handleDiscardAndClose() {
    const target = pendingClose;
    pendingClose = null;
    if (target) {
      forceCloseTab(target.paneId, target.tabId);
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
          <div class="ws-pill" class:active={ws.id === session.activeWorkspaceId}>
            {#if renamingWsId === ws.id}
              <input
                bind:value={renameInputText}
                type="text"
                class="ws-rename-input"
                onblur={() => commitWorkspaceRename(ws.id)}
                onkeydown={(e) => {
                  if (e.key === 'Enter') commitWorkspaceRename(ws.id);
                  if (e.key === 'Escape') renamingWsId = null;
                }}
              />
            {:else}
              <button
                class="workspace-item"
                class:active={ws.id === session.activeWorkspaceId}
                onclick={() => (session.activeWorkspaceId = ws.id)}
                ondblclick={() => {
                  renamingWsId = ws.id;
                  renameInputText = ws.name;
                }}
              >
                {ws.name}
              </button>
            {/if}
            {#if session.workspaces.length > 1}
              <button
                class="ws-close-btn"
                title="Hapus workspace"
                aria-label="Hapus workspace"
                onclick={(e) => {
                  e.stopPropagation();
                  handleCloseWorkspace(ws.id);
                }}
              >
                <X size={11} strokeWidth={2} />
              </button>
            {/if}
          </div>
        {/each}
        <button
          class="icon-btn"
          title="Workspace baru"
          aria-label="Workspace baru"
          onclick={handleAddWorkspace}
        >
          <Plus size={14} strokeWidth={2} />
        </button>
      </div>
      <div class="workspace-actions">
        <button
          class="folder-switch"
          class:active-toggle={session.sidebar.visible && session.sidebar.panel === 'git'}
          title="Panel Git (Alt+V)"
          onclick={toggleGitSidebar}
        >
          <GitBranch size={14} strokeWidth={2} />
          <span>Git</span>
        </button>
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

    {#if sessionNotice}
      <div class="session-notice-bar">
        <span>{sessionNotice}</span>
        <button class="small-btn" onclick={() => (sessionNotice = null)}>Tutup</button>
      </div>
    {/if}

    <div class="workbench">
      {#if session.sidebar.visible && session.sidebar.panel === 'git'}
        <div class="sidebar-shell" style:width="{session.sidebar.width}px">
          <GitPanel
            bind:this={gitPanelRef}
            onOpenDiff={(relPath, staged) => openDiffInPane(relPath, staged)}
            onStatusUpdated={(st) => (repoStatus = st)}
          />
        </div>
      {/if}

      <div class="workspaces-stage">
        {#each session.workspaces as ws (ws.id)}
          {@const isWsActive = ws.id === session.activeWorkspaceId}
          {@const wsFocusedPane = focusedPaneByWs[ws.id] ?? getFirstPaneId(ws.root)}
          {@const wsMaxPaneId = maximizedPaneByWs[ws.id] ?? null}
          {@const renderNode = wsMaxPaneId
            ? (findPaneById(ws.root, wsMaxPaneId) ?? ws.root)
            : ws.root}
          <div class="workspace-canvas" class:hidden-ws={!isWsActive}>
            <PaneNode
              node={renderNode}
              focusedPaneId={wsFocusedPane}
              maximizedPaneId={wsMaxPaneId}
              workspaceVisible={isWsActive}
              {dirtyTabs}
              onFocusPane={(paneId) => {
                focusedPaneByWs[ws.id] = paneId;
              }}
              onSelectTab={(paneId, tabId) => {
                const target = findPaneById(ws.root, paneId);
                if (target) target.activeTabId = tabId;
              }}
              onCloseTab={(paneId, tabId) => requestCloseTab(paneId, tabId)}
              onAddTerminalTab={(paneId) => addTerminalTab(paneId)}
              onRequestQuickOpen={(paneId) => {
                focusedPaneByWs[ws.id] = paneId;
                quickOpenVisible = true;
              }}
              onSplitPane={(paneId, dir) => handleSplitPane(paneId, dir)}
              onToggleMaximizePane={(paneId) => handleToggleMaximizePane(paneId)}
              onUpdateRatio={(path, nextRatio) => {
                ws.root = updateSplitRatioByPath(ws.root, path, nextRatio);
              }}
              onMoveTab={(fromPaneId, tabId, toPaneId, idx) => {
                ws.root = moveTabBetweenPanes(ws.root, fromPaneId, tabId, toPaneId, idx);
                focusedPaneByWs[ws.id] = toPaneId;
              }}
              onTabDirtyChange={(tabId, dirty) => {
                dirtyTabs[tabId] = dirty;
                if (!dirty) {
                  void refreshRepoStatus();
                }
              }}
              onTabMetaChange={(tabId, meta) => {
                tabMetaMap[tabId] = meta;
              }}
              onOpenFileFromDiff={(relPath, line) => {
                if (projectState.activeFolder) {
                  openFileInPane(`${projectState.activeFolder}/${relPath}`, line);
                }
              }}
              onRepoChanged={() => void refreshRepoStatus()}
              onRegisterFileTabRef={(tabId, ref) => {
                fileTabRefs[tabId] = ref;
              }}
            />
          </div>
        {/each}
      </div>
    </div>

    <footer class="status-bar">
      <div class="status-left">
        <span>{projectState.folderName ?? 'proyek'}</span>
        {#if repoStatus?.isRepo}
          <span class="status-branch">
            <GitBranch size={12} strokeWidth={2} />
            <span>{repoStatus.detached ? 'detached' : (repoStatus.branch ?? 'main')}</span>
            {#if repoStatus.ahead > 0 || repoStatus.behind > 0}
              <span class="mono">+{repoStatus.ahead} -{repoStatus.behind}</span>
            {/if}
          </span>
        {/if}
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

    {#if pendingClose}
      <div class="modal-backdrop">
        <div class="confirm-dialog" role="dialog" aria-modal="true" aria-label="Konfirmasi simpan">
          <h2 class="confirm-title">Simpan perubahan sebelum menutup?</h2>
          <p class="muted">Perubahan yang belum disimpan akan hilang jika tab ditutup.</p>
          <div class="confirm-actions">
            <button class="primary" onclick={handleConfirmSaveAndClose}>Simpan</button>
            <button class="danger" onclick={handleDiscardAndClose}>Buang</button>
            <button onclick={() => (pendingClose = null)}>Batal</button>
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
    gap: 6px;
  }

  .ws-pill {
    display: flex;
    align-items: center;
    border-radius: var(--radius-ctrl);
  }

  .ws-pill.active {
    background: var(--acs);
  }

  .workspace-item {
    height: 26px;
    padding: 4px 10px;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .workspace-item.active {
    color: var(--ac);
  }

  .ws-rename-input {
    width: 96px;
    height: 24px;
    padding: 0 6px;
    font-size: 12px;
  }

  .ws-close-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    margin-right: 4px;
    border: none;
    background: transparent;
    color: var(--dm);
    opacity: 0;
  }

  .ws-pill:hover .ws-close-btn {
    opacity: 1;
  }

  .folder-switch {
    height: 26px;
    padding: 4px 10px;
    border: none;
    color: var(--dm);
  }

  .folder-switch.active-toggle {
    background: var(--acs);
    color: var(--ac);
  }

  .icon-btn {
    width: 26px;
    height: 26px;
    padding: 0;
    border: none;
    color: var(--dm);
  }

  .session-notice-bar {
    height: 30px;
    padding: 0 16px;
    background: var(--el);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
    color: var(--mo);
  }

  .small-btn {
    height: 22px;
    padding: 0 8px;
    font-size: 12px;
  }

  .workbench {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .sidebar-shell {
    height: 100%;
    border-right: 1px solid var(--bd);
    flex-shrink: 0;
    min-height: 0;
  }

  .workspaces-stage {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
  }

  .workspace-canvas {
    flex: 1;
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }

  .workspace-canvas.hidden-ws {
    display: none;
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

  .status-branch {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--tx);
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
