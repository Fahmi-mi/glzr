<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Files from '@lucide/svelte/icons/files';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Moon from '@lucide/svelte/icons/moon';
  import Plus from '@lucide/svelte/icons/plus';
  import Search from '@lucide/svelte/icons/search';
  import Sun from '@lucide/svelte/icons/sun';
  import X from '@lucide/svelte/icons/x';
  import {
    createDefaultSettings,
    fsClearDraft,
    fsUnwatch,
    fsWatch,
    gitPull,
    gitPush,
    gitStatus,
    normalizeTabPath,
    onBackendEvent,
    pickProjectFolderDialog,
    projectGetState,
    projectOpenFolder,
    sessionLoad,
    sessionSave,
    settingsLoad,
    settingsSave,
    type AppErrorPayload,
    type AppSettingsDto,
    type GitBadge,
    type GitStatusDto,
    type ProjectStateDto
  } from '$lib/ipc';
  import { resolveGlobalShortcut, type PaletteEntry } from '$lib/keymap';
  import {
    closePane,
    findDirectionalNeighborPane,
    findPaneById,
    getFirstPaneId,
    listAllPanes,
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
  import { formatPreviewTabTitle, normalizePreviewUrl } from '$lib/preview';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import PaneNode from '$lib/components/PaneNode.svelte';
  import QuickOpen from '$lib/components/QuickOpen.svelte';
  import FileTree from '$lib/components/sidebar/FileTree.svelte';
  import GitPanel from '$lib/components/sidebar/GitPanel.svelte';
  import SearchPanel from '$lib/components/sidebar/SearchPanel.svelte';
  import type { FileTabMeta } from '$lib/components/tabs/FileTab.svelte';
  import type FileTab from '$lib/components/tabs/FileTab.svelte';

  let projectState = $state<ProjectStateDto>({
    activeFolder: null,
    folderName: null,
    recentFolders: []
  });
  let session = $state<SessionFile>(createInitialSession(null));
  let settings = $state<AppSettingsDto>(createDefaultSettings());
  let errorMessage = $state<string | null>(null);
  let sessionNotice = $state<string | null>(null);
  let loading = $state(true);
  let quickOpenVisible = $state(false);
  let commandPaletteVisible = $state(false);
  let draggingSidebar = $state(false);

  let focusedPaneByWs = $state<Record<string, string>>({});
  let maximizedPaneByWs = $state<Record<string, string | null>>({});
  let renamingWsId = $state<string | null>(null);
  let renameInputText = $state('');

  let dirtyTabs = $state<Record<string, boolean>>({});
  let tabMetaMap = $state<Record<string, FileTabMeta>>({});
  let fileTabRefs = $state<Record<string, ReturnType<typeof FileTab> | null>>({});
  let pendingClose = $state<{ paneId: string; tabId: string } | null>(null);
  let fileTreeRef = $state<ReturnType<typeof FileTree> | null>(null);
  let gitPanelRef = $state<ReturnType<typeof GitPanel> | null>(null);
  let searchPanelRef = $state<ReturnType<typeof SearchPanel> | null>(null);
  let repoStatus = $state<GitStatusDto | null>(null);

  let idSeq = $state(100);
  let saveSessionTimer: ReturnType<typeof setTimeout> | null = null;
  let sessionReady = $state(false);
  let unlistenFsChanged: (() => void) | null = null;

  const gitBadges = $derived.by<Record<string, GitBadge>>(() => {
    const map: Record<string, GitBadge> = {};
    if (!repoStatus?.isRepo) return map;
    for (const item of repoStatus.untracked) {
      map[normalizeTabPath(item.path)] = item.badge;
    }
    for (const item of repoStatus.unstaged) {
      map[normalizeTabPath(item.path)] = item.badge;
    }
    for (const item of repoStatus.staged) {
      map[normalizeTabPath(item.path)] = item.badge;
    }
    for (const item of repoStatus.conflicted) {
      map[normalizeTabPath(item.path)] = item.badge;
    }
    return map;
  });

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

  $effect(() => {
    if (typeof document !== 'undefined') {
      document.documentElement.dataset.theme = settings.appearance.theme;
      document.documentElement.dataset.font = settings.appearance.fontFamily;
      document.documentElement.style.setProperty(
        '--font-size-code',
        `${settings.appearance.fontSize}px`
      );
    }
  });

  async function persistSettings() {
    await settingsSave(settings).catch(() => {});
  }

  function toggleTheme() {
    settings.appearance.theme = settings.appearance.theme === 'dark' ? 'light' : 'dark';
    void persistSettings();
  }

  function toggleFontFamily() {
    settings.appearance.fontFamily =
      settings.appearance.fontFamily === 'system-mono' ? 'jetbrains-mono-nerd' : 'system-mono';
    void persistSettings();
  }

  function adjustCodeFontSize(delta: number | 'reset') {
    if (delta === 'reset') {
      settings.appearance.fontSize = 14;
    } else {
      const next = Math.min(28, Math.max(10, settings.appearance.fontSize + delta));
      settings.appearance.fontSize = next;
    }
    void persistSettings();
  }

  function startSidebarResize(event: MouseEvent) {
    event.preventDefault();
    draggingSidebar = true;
    const startX = event.clientX;
    const startWidth = session.sidebar.width;

    function handleMove(moveEvent: MouseEvent) {
      const next = Math.min(
        480,
        Math.max(160, Math.round(startWidth + moveEvent.clientX - startX))
      );
      session.sidebar.width = next;
    }

    function handleUp() {
      draggingSidebar = false;
      window.removeEventListener('mousemove', handleMove);
      window.removeEventListener('mouseup', handleUp);
    }

    window.addEventListener('mousemove', handleMove);
    window.addEventListener('mouseup', handleUp);
  }

  const paletteActions = $derived.by<PaletteEntry[]>(() => {
    const kb = settings.keybindings;
    const entries: PaletteEntry[] = [
      {
        id: 'act:split-right',
        label: 'Split kanan',
        category: 'Aksi',
        shortcut: kb.splitRight,
        keywords: 'pane belah kanan horizontal',
        run: () => handleSplitPane(focusedPaneId, 'row')
      },
      {
        id: 'act:split-down',
        label: 'Split bawah',
        category: 'Aksi',
        shortcut: kb.splitDown,
        keywords: 'pane belah bawah vertikal',
        run: () => handleSplitPane(focusedPaneId, 'column')
      },
      {
        id: 'act:new-terminal',
        label: 'Terminal baru di pane aktif',
        category: 'Aksi',
        shortcut: kb.newTerminal,
        keywords: 'shell pty bash zsh',
        run: () => addTerminalTab(focusedPaneId)
      },
      {
        id: 'act:new-preview',
        label: 'Preview localhost baru',
        category: 'Aksi',
        keywords: 'browser iframe server port 5173',
        run: () => openPreviewInPane('http://localhost:5173/', focusedPaneId)
      },
      {
        id: 'act:quick-open',
        label: 'Buka cepat file...',
        category: 'Aksi',
        shortcut: kb.quickOpen,
        keywords: 'quick open cari file',
        run: () => {
          quickOpenVisible = true;
        }
      },
      {
        id: 'act:open-folder',
        label: 'Buka folder',
        category: 'Aksi',
        keywords: 'proyek direktori pilih',
        run: () => {
          void handleChooseFolder();
        }
      },
      {
        id: 'act:sidebar-files',
        label: 'Toggle Sidebar Files',
        category: 'Aksi',
        shortcut: kb.sidebarFiles,
        keywords: 'file tree pohon berkas',
        run: () => toggleSidebarPanel('files')
      },
      {
        id: 'act:sidebar-git',
        label: 'Toggle Sidebar Git',
        category: 'Aksi',
        shortcut: kb.sidebarGit,
        keywords: 'version control diff commit',
        run: () => toggleSidebarPanel('git')
      },
      {
        id: 'act:sidebar-search',
        label: 'Toggle Sidebar Search (Cari di Proyek)',
        category: 'Aksi',
        shortcut: kb.sidebarSearch,
        keywords: 'ripgrep rg grep cari teks',
        run: () => toggleSidebarPanel('search')
      },
      {
        id: 'act:maximize-pane',
        label: 'Maximize / Kembalikan ukuran pane',
        category: 'Aksi',
        shortcut: kb.maximizePane,
        keywords: 'penuh fokus perbesar',
        run: () => handleToggleMaximizePane(focusedPaneId)
      },
      {
        id: 'act:close-tab',
        label: 'Tutup tab aktif',
        category: 'Aksi',
        shortcut: kb.closeTab,
        run: () => {
          if (focusedPane && activeTab) {
            requestCloseTab(focusedPane.id, activeTab.id);
          }
        }
      },
      {
        id: 'act:close-pane',
        label: 'Tutup pane aktif',
        category: 'Aksi',
        shortcut: kb.closePane,
        run: () => handleClosePane(focusedPaneId)
      },
      {
        id: 'act:new-workspace',
        label: 'Workspace baru',
        category: 'Aksi',
        keywords: 'ruang kerja tambah',
        run: () => handleAddWorkspace()
      },
      {
        id: 'act:close-workspace',
        label: 'Tutup workspace aktif',
        category: 'Aksi',
        keywords: 'ruang kerja hapus',
        run: () => handleCloseWorkspace(session.activeWorkspaceId)
      },
      {
        id: 'act:toggle-theme',
        label: `Ganti tema ke ${settings.appearance.theme === 'dark' ? 'Light' : 'Dark'}`,
        category: 'Aksi',
        keywords: 'theme dark light terang gelap warna',
        run: () => toggleTheme()
      },
      {
        id: 'act:toggle-font',
        label: `Ganti font ke ${
          settings.appearance.fontFamily === 'system-mono'
            ? 'JetBrains Mono Nerd'
            : 'Monospace Sistem'
        }`,
        category: 'Aksi',
        keywords: 'font huruf tipografi jetbrains',
        run: () => toggleFontFamily()
      },
      {
        id: 'act:zoom-in',
        label: 'Perbesar ukuran font',
        category: 'Aksi',
        shortcut: 'Alt+=',
        run: () => adjustCodeFontSize(1)
      },
      {
        id: 'act:zoom-out',
        label: 'Perkecil ukuran font',
        category: 'Aksi',
        shortcut: 'Alt+-',
        run: () => adjustCodeFontSize(-1)
      },
      {
        id: 'act:zoom-reset',
        label: 'Reset ukuran font (14px)',
        category: 'Aksi',
        shortcut: 'Alt+0',
        run: () => adjustCodeFontSize('reset')
      },
      {
        id: 'act:git-pull',
        label: 'Git: Pull dari remote',
        category: 'Aksi',
        run: () => {
          void gitPull()
            .then(() => refreshRepoStatus())
            .catch((err: AppErrorPayload) => {
              sessionNotice = err.message ?? 'Git pull gagal';
            });
        }
      },
      {
        id: 'act:git-push',
        label: 'Git: Push ke remote',
        category: 'Aksi',
        run: () => {
          void gitPush()
            .then(() => refreshRepoStatus())
            .catch((err: AppErrorPayload) => {
              sessionNotice = err.message ?? 'Git push gagal';
            });
        }
      }
    ];

    for (const ws of session.workspaces) {
      entries.push({
        id: `ws:${ws.id}`,
        label: `Pindah ke workspace ${ws.name}`,
        category: 'Workspace',
        run: () => {
          session.activeWorkspaceId = ws.id;
        }
      });
    }

    for (const recent of projectState.recentFolders) {
      if (recent !== projectState.activeFolder) {
        entries.push({
          id: `recent:${recent}`,
          label: `Buka folder terakhir: ${recent}`,
          category: 'Aksi',
          run: () => {
            void openFolderByPath(recent);
          }
        });
      }
    }

    return entries;
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
      if (typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window) {
        unlistenFsChanged = await onBackendEvent('fs:changed', () => {
          void fileTreeRef?.refresh();
          void refreshRepoStatus();
        });
      }

      const [state, loadedSession, loadedSettings] = await Promise.all([
        projectGetState(),
        sessionLoad().catch(() => ({ rawJson: null, backedUpCorrupt: false })),
        settingsLoad().catch(() => ({ settings: createDefaultSettings(), warnings: [] }))
      ]);
      projectState = state;
      settings = loadedSettings.settings;

      if (loadedSession.backedUpCorrupt) {
        sessionNotice =
          'File sesi sebelumnya tidak kompatibel atau rusak dan telah dicadangkan ke session.json.bak.';
      } else if (loadedSettings.warnings.length > 0) {
        sessionNotice = loadedSettings.warnings[0];
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
        await Promise.all([fsWatch().catch(() => {}), refreshRepoStatus()]);
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
    unlistenFsChanged?.();
    void fsUnwatch().catch(() => {});
    if (saveSessionTimer !== null) {
      clearTimeout(saveSessionTimer);
      saveSessionTimer = null;
    }
  });

  function setFocusedPane(paneId: string) {
    if (!activeWorkspace) return;
    focusedPaneByWs[activeWorkspace.id] = paneId;
  }

  function toggleSidebarPanel(panel: 'files' | 'git' | 'search') {
    if (session.sidebar.visible && session.sidebar.panel === panel) {
      session.sidebar.visible = false;
    } else {
      session.sidebar.visible = true;
      session.sidebar.panel = panel;
      queueMicrotask(() => {
        if (panel === 'files') {
          fileTreeRef?.focusTree();
        } else if (panel === 'git') {
          gitPanelRef?.focusInput();
        } else if (panel === 'search') {
          searchPanelRef?.focusInput();
        }
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
    const action = resolveGlobalShortcut(
      {
        key: event.key,
        altKey: event.altKey,
        ctrlKey: event.ctrlKey,
        shiftKey: event.shiftKey,
        metaKey: event.metaKey
      },
      terminalFocused,
      settings.keybindings
    );

    if (!action) return;

    event.preventDefault();

    if (typeof action === 'object' && action.type === 'switch_workspace') {
      if (action.index >= 0 && action.index < session.workspaces.length) {
        session.activeWorkspaceId = session.workspaces[action.index].id;
      }
      return;
    }

    switch (action) {
      case 'command_palette':
        commandPaletteVisible = !commandPaletteVisible;
        break;
      case 'quick_open':
        quickOpenVisible = true;
        break;
      case 'sidebar_files':
        toggleSidebarPanel('files');
        break;
      case 'sidebar_git':
        toggleSidebarPanel('git');
        break;
      case 'sidebar_search':
        toggleSidebarPanel('search');
        break;
      case 'new_terminal':
        addTerminalTab(focusedPaneId);
        break;
      case 'close_tab':
        if (focusedPane && activeTab) {
          requestCloseTab(focusedPane.id, activeTab.id);
        }
        break;
      case 'split_right':
        handleSplitPane(focusedPaneId, 'row');
        break;
      case 'split_down':
        handleSplitPane(focusedPaneId, 'column');
        break;
      case 'close_pane':
        handleClosePane(focusedPaneId);
        break;
      case 'maximize_pane':
        handleToggleMaximizePane(focusedPaneId);
        break;
      case 'pane_left':
      case 'pane_right':
      case 'pane_up':
      case 'pane_down': {
        const dirMap = {
          pane_left: 'left',
          pane_right: 'right',
          pane_up: 'up',
          pane_down: 'down'
        } as const;
        const neighbor = findDirectionalNeighborPane(
          activeWorkspace.root,
          focusedPaneId,
          dirMap[action]
        );
        if (neighbor) {
          setFocusedPane(neighbor);
        }
        break;
      }
      case 'next_tab':
      case 'prev_tab': {
        if (focusedPane && focusedPane.tabs.length > 1) {
          const curIdx = focusedPane.tabs.findIndex((t) => t.id === focusedPane.activeTabId);
          const delta = action === 'next_tab' ? 1 : -1;
          const nextIdx = (curIdx + delta + focusedPane.tabs.length) % focusedPane.tabs.length;
          focusedPane.activeTabId = focusedPane.tabs[nextIdx].id;
        }
        break;
      }
      case 'zoom_in':
        adjustCodeFontSize(1);
        break;
      case 'zoom_out':
        adjustCodeFontSize(-1);
        break;
      case 'zoom_reset':
        adjustCodeFontSize('reset');
        break;
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
      await Promise.all([fsWatch().catch(() => {}), refreshRepoStatus()]);
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMessage = normalized.message ?? 'Gagal membuka folder proyek';
    }
  }

  function handleEntryRenamed(oldPath: string, newPath: string) {
    if (!projectState.activeFolder) return;
    const oldAbs = normalizeTabPath(`${projectState.activeFolder}/${oldPath}`);
    const newAbs = normalizeTabPath(`${projectState.activeFolder}/${newPath}`);

    for (const ws of session.workspaces) {
      const panes = listAllPanes(ws.root);
      for (const pane of panes) {
        for (const tab of pane.tabs) {
          if (tab.kind !== 'file' || typeof tab.data.path !== 'string') continue;
          const currentPath = normalizeTabPath(tab.data.path);
          if (currentPath === oldAbs) {
            tab.data = { ...tab.data, path: newAbs };
            tab.title = extractBaseName(newAbs);
          } else if (currentPath.startsWith(`${oldAbs}/`)) {
            const updatedPath = `${newAbs}${currentPath.slice(oldAbs.length)}`;
            tab.data = { ...tab.data, path: updatedPath };
            tab.title = extractBaseName(updatedPath);
          }
        }
      }
    }
    void refreshRepoStatus();
  }

  function handleEntryDeleted(deletedPath: string) {
    if (!projectState.activeFolder) return;
    const deletedAbs = normalizeTabPath(`${projectState.activeFolder}/${deletedPath}`);

    for (const ws of session.workspaces) {
      const panes = listAllPanes(ws.root);
      for (const pane of panes) {
        const tabsToClose = pane.tabs.filter((tab) => {
          if (tab.kind !== 'file' || typeof tab.data.path !== 'string') return false;
          if (dirtyTabs[tab.id]) return false;
          const currentPath = normalizeTabPath(tab.data.path);
          return currentPath === deletedAbs || currentPath.startsWith(`${deletedAbs}/`);
        });
        for (const tab of tabsToClose) {
          if (ws.id === session.activeWorkspaceId) {
            forceCloseTab(pane.id, tab.id);
          } else {
            const idx = pane.tabs.findIndex((t) => t.id === tab.id);
            if (idx !== -1 && pane.tabs.length > 1) {
              pane.tabs.splice(idx, 1);
              if (pane.activeTabId === tab.id) {
                pane.activeTabId = pane.tabs[Math.min(idx, pane.tabs.length - 1)].id;
              }
            }
          }
        }
      }
    }
    void refreshRepoStatus();
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

  export function openPreviewInPane(
    rawUrl: string = 'http://localhost:5173/',
    targetPaneId?: string
  ) {
    if (!activeWorkspace) return;
    const pane = findPaneById(activeWorkspace.root, targetPaneId ?? focusedPaneId);
    if (!pane) return;

    const normalizedUrl = normalizePreviewUrl(rawUrl) ?? 'http://localhost:5173/';
    const existing = pane.tabs.find(
      (t) =>
        t.kind === 'preview' &&
        (normalizePreviewUrl(String(t.data.url ?? '')) ?? '') === normalizedUrl
    );

    if (existing) {
      existing.data = { ...existing.data, url: normalizedUrl };
      existing.title = formatPreviewTabTitle(normalizedUrl);
      pane.activeTabId = existing.id;
      setFocusedPane(pane.id);
      return;
    }

    const newId = nextId('tab');
    pane.tabs.push({
      id: newId,
      kind: 'preview',
      title: formatPreviewTabTitle(normalizedUrl),
      data: { url: normalizedUrl }
    });
    pane.activeTabId = newId;
    setFocusedPane(pane.id);
  }

  function updatePreviewTab(
    wsRoot: SessionFile['workspaces'][number]['root'],
    paneId: string,
    tabId: string,
    nextUrl: string,
    nextTitle: string
  ) {
    const pane = findPaneById(wsRoot, paneId);
    if (!pane) return;
    const tab = pane.tabs.find((t) => t.id === tabId);
    if (!tab || tab.kind !== 'preview') return;
    tab.data = { ...tab.data, url: nextUrl };
    tab.title = nextTitle;
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
          class:active-toggle={session.sidebar.visible && session.sidebar.panel === 'files'}
          title="File Tree (Alt+E)"
          onclick={() => toggleSidebarPanel('files')}
        >
          <Files size={14} strokeWidth={2} />
          <span>File</span>
        </button>
        <button
          class="folder-switch"
          class:active-toggle={session.sidebar.visible && session.sidebar.panel === 'git'}
          title="Panel Git (Alt+V)"
          onclick={() => toggleSidebarPanel('git')}
        >
          <GitBranch size={14} strokeWidth={2} />
          <span>Git</span>
        </button>
        <button
          class="folder-switch"
          class:active-toggle={session.sidebar.visible && session.sidebar.panel === 'search'}
          title="Cari di Proyek (Alt+G)"
          onclick={() => toggleSidebarPanel('search')}
        >
          <Search size={14} strokeWidth={2} />
          <span>Cari</span>
        </button>
        <button
          class="folder-switch"
          title="Command Palette (Alt+P)"
          onclick={() => (commandPaletteVisible = true)}
        >
          <span>Perintah</span>
        </button>
        <button
          class="icon-btn"
          title="Ganti tema ({settings.appearance.theme === 'dark' ? 'Light' : 'Dark'})"
          aria-label="Ganti tema"
          onclick={toggleTheme}
        >
          {#if settings.appearance.theme === 'dark'}
            <Sun size={14} strokeWidth={2} />
          {:else}
            <Moon size={14} strokeWidth={2} />
          {/if}
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
      {#if session.sidebar.visible}
        <div class="sidebar-shell" style:width="{session.sidebar.width}px">
          <div class="sidebar-tabs-bar">
            <button
              class="sidebar-tab-btn"
              class:active={session.sidebar.panel === 'files'}
              onclick={() => (session.sidebar.panel = 'files')}
            >
              Files
            </button>
            <button
              class="sidebar-tab-btn"
              class:active={session.sidebar.panel === 'git'}
              onclick={() => (session.sidebar.panel = 'git')}
            >
              Git
            </button>
            <button
              class="sidebar-tab-btn"
              class:active={session.sidebar.panel === 'search'}
              onclick={() => (session.sidebar.panel = 'search')}
            >
              Search
            </button>
          </div>

          <div class="sidebar-body">
            {#if session.sidebar.panel === 'files'}
              <FileTree
                bind:this={fileTreeRef}
                workspaceRoot={projectState.activeFolder}
                {gitBadges}
                onOpenFile={(fullPath) => openFileInPane(fullPath)}
                onEntryRenamed={(oldPath, newPath) => handleEntryRenamed(oldPath, newPath)}
                onEntryDeleted={(deletedPath) => handleEntryDeleted(deletedPath)}
              />
            {:else if session.sidebar.panel === 'git'}
              <GitPanel
                bind:this={gitPanelRef}
                onOpenDiff={(relPath, staged) => openDiffInPane(relPath, staged)}
                onStatusUpdated={(st) => (repoStatus = st)}
              />
            {:else if session.sidebar.panel === 'search'}
              <SearchPanel
                bind:this={searchPanelRef}
                onSelectMatch={(relPath, line) => {
                  if (projectState.activeFolder) {
                    openFileInPane(`${projectState.activeFolder}/${relPath}`, line);
                  }
                }}
              />
            {/if}
          </div>

          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div
            class="sidebar-resizer"
            class:active={draggingSidebar}
            role="separator"
            aria-orientation="vertical"
            onmousedown={startSidebarResize}
          ></div>
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
              onAddPreviewTab={(paneId, url) => openPreviewInPane(url, paneId)}
              onUpdatePreviewTab={(paneId, tabId, nextUrl, nextTitle) => {
                updatePreviewTab(ws.root, paneId, tabId, nextUrl, nextTitle);
              }}
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

    <CommandPalette
      open={commandPaletteVisible}
      workspaceRoot={projectState.activeFolder}
      actions={paletteActions}
      onSelectFile={(fullPath, line) => openFileInPane(fullPath, line)}
      onClose={() => (commandPaletteVisible = false)}
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
    position: relative;
    height: 100%;
    border-right: 1px solid var(--bd);
    flex-shrink: 0;
    min-height: 0;
    background: var(--pn);
    display: flex;
    flex-direction: column;
  }

  .sidebar-tabs-bar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 10px 6px;
    border-bottom: 1px solid var(--bd);
  }

  .sidebar-tab-btn {
    height: 26px;
    padding: 4px 10px;
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--dm);
    font-size: 12px;
  }

  .sidebar-tab-btn.active {
    background: var(--el);
    color: var(--tx);
  }

  .sidebar-body {
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-resizer {
    position: absolute;
    top: 0;
    right: -3px;
    bottom: 0;
    width: 6px;
    cursor: col-resize;
    z-index: 25;
  }

  .sidebar-resizer:hover,
  .sidebar-resizer.active {
    background: var(--ac);
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
