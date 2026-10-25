<script lang="ts">
  import { onMount } from 'svelte';
  import FolderOpen from '@lucide/svelte/icons/folder-open';
  import Plus from '@lucide/svelte/icons/plus';
  import Terminal from '@lucide/svelte/icons/terminal';
  import {
    pickProjectFolderDialog,
    projectGetState,
    projectOpenFolder,
    type AppErrorPayload,
    type ProjectStateDto
  } from '$lib/ipc';
  import { createInitialSession, type Pane, type SessionFile } from '$lib/layout/types';

  let projectState = $state<ProjectStateDto>({
    activeFolder: null,
    folderName: null,
    recentFolders: []
  });
  let session = $state<SessionFile>(createInitialSession(null));
  let errorMessage = $state<string | null>(null);
  let loading = $state(true);

  const activeWorkspace = $derived(
    session.workspaces.find((w) => w.id === session.activeWorkspaceId) ?? session.workspaces[0]
  );

  const rootPane = $derived<Pane | null>(
    activeWorkspace && activeWorkspace.root.type === 'pane' ? activeWorkspace.root : null
  );

  const activeTab = $derived(
    rootPane?.tabs.find((t) => t.id === rootPane.activeTabId) ?? rootPane?.tabs[0] ?? null
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
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMessage = normalized.message ?? 'Gagal membuka folder proyek';
    }
  }
</script>

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
      <div class="workspace-folder">
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
              <button
                class="tab-item"
                class:active={tab.id === rootPane.activeTabId}
                onclick={() => (rootPane.activeTabId = tab.id)}
              >
                <Terminal size={14} strokeWidth={2} />
                <span>{tab.title}</span>
              </button>
            {/each}
            <button class="icon-btn" title="Tab baru" aria-label="Tab baru">
              <Plus size={14} strokeWidth={2} />
            </button>
          </div>
          <div class="pane-body">
            {#if activeTab}
              <div class="terminal-placeholder">
                <span class="mono">{projectState.activeFolder} $</span>
              </div>
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
        <span>UTF-8</span>
        <span>LF</span>
      </div>
    </footer>
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

  .workspace-list {
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

  .tab-item {
    height: 26px;
    padding: 0 12px;
    border: none;
    border-radius: var(--radius-ctrl);
    color: var(--dm);
    gap: 8px;
  }

  .tab-item.active {
    background: var(--el);
    color: var(--tx);
  }

  .pane-body {
    flex: 1;
    padding: 16px 24px;
    overflow: auto;
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
</style>
