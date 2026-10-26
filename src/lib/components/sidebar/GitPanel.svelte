<script lang="ts">
  import { onMount } from 'svelte';
  import ArrowDown from '@lucide/svelte/icons/arrow-down';
  import ArrowUp from '@lucide/svelte/icons/arrow-up';
  import GitBranch from '@lucide/svelte/icons/git-branch';
  import Minus from '@lucide/svelte/icons/minus';
  import Plus from '@lucide/svelte/icons/plus';
  import RefreshCw from '@lucide/svelte/icons/refresh-cw';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import {
    gitBranches,
    gitCheckout,
    gitCommit,
    gitDiscard,
    gitPull,
    gitPush,
    gitStage,
    gitStatus,
    gitUnstage,
    type AppErrorPayload,
    type GitBranchesDto,
    type GitFileEntry,
    type GitStatusDto
  } from '$lib/ipc';
  import { badgeColorVar } from '$lib/git';

  interface Props {
    onOpenDiff?: (relPath: string, staged: boolean) => void;
    onStatusUpdated?: (status: GitStatusDto) => void;
  }

  let { onOpenDiff, onStatusUpdated }: Props = $props();

  let statusData = $state<GitStatusDto | null>(null);
  let branchesData = $state<GitBranchesDto | null>(null);
  let commitMessage = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let errorMsg = $state<string | null>(null);
  let confirmDiscardPath = $state<string | null>(null);
  let commitInputEl = $state<HTMLInputElement | null>(null);

  const unstagedAll = $derived<GitFileEntry[]>(
    statusData ? [...statusData.unstaged, ...statusData.untracked] : []
  );

  const totalChanges = $derived(
    statusData
      ? statusData.staged.length +
          statusData.unstaged.length +
          statusData.untracked.length +
          statusData.conflicted.length
      : 0
  );

  export function focusInput() {
    commitInputEl?.focus();
  }

  export async function refresh() {
    errorMsg = null;
    try {
      const st = await gitStatus();
      statusData = st;
      onStatusUpdated?.(st);
      if (st.isRepo) {
        branchesData = await gitBranches().catch(() => null);
      } else {
        branchesData = null;
      }
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Gagal memuat status Git';
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
  });

  async function runAction(action: () => Promise<void>) {
    busy = true;
    errorMsg = null;
    try {
      await action();
      await refresh();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Operasi Git gagal';
    } finally {
      busy = false;
    }
  }

  async function handleCommit() {
    const trimmed = commitMessage.trim();
    if (!trimmed || !statusData || statusData.staged.length === 0) return;
    await runAction(async () => {
      await gitCommit(trimmed);
      commitMessage = '';
    });
  }

  async function handleBranchChange(event: Event) {
    const select = event.currentTarget as HTMLSelectElement;
    const targetBranch = select.value;
    if (!targetBranch || targetBranch === statusData?.branch) return;
    await runAction(async () => {
      await gitCheckout(targetBranch);
    });
  }

  async function handleConfirmDiscard() {
    const target = confirmDiscardPath;
    confirmDiscardPath = null;
    if (!target) return;
    await runAction(async () => {
      await gitDiscard(target);
    });
  }
</script>

<aside class="git-panel">
  <div class="panel-header">
    <span class="panel-title">Perubahan ({totalChanges})</span>
    <div class="header-actions">
      {#if statusData?.isRepo}
        <button
          class="icon-btn"
          title="Git Pull"
          aria-label="Git Pull"
          disabled={busy}
          onclick={() => runAction(() => gitPull())}
        >
          <ArrowDown size={14} strokeWidth={2} />
        </button>
        <button
          class="icon-btn"
          title="Git Push"
          aria-label="Git Push"
          disabled={busy}
          onclick={() => runAction(() => gitPush())}
        >
          <ArrowUp size={14} strokeWidth={2} />
        </button>
      {/if}
      <button
        class="icon-btn"
        title="Segarkan status Git"
        aria-label="Segarkan status Git"
        disabled={busy}
        onclick={() => refresh()}
      >
        <RefreshCw size={14} strokeWidth={2} />
      </button>
    </div>
  </div>

  {#if loading}
    <div class="empty-state">
      <p class="muted">Memuat status Git...</p>
    </div>
  {:else if !statusData?.gitAvailable}
    <div class="empty-state">
      <p class="muted">Git tidak terpasang di sistem.</p>
    </div>
  {:else if !statusData.isRepo}
    <div class="empty-state">
      <p class="muted">Folder proyek ini bukan repositori Git.</p>
    </div>
  {:else}
    <div class="branch-bar">
      <div class="branch-selector">
        <GitBranch size={14} strokeWidth={2} />
        {#if branchesData && branchesData.branches.length > 0 && !statusData.detached}
          <select
            class="branch-select"
            value={statusData.branch ?? ''}
            disabled={busy}
            onchange={handleBranchChange}
          >
            {#each branchesData.branches as br (br)}
              <option value={br}>{br}</option>
            {/each}
          </select>
        {:else}
          <span class="branch-label">
            {statusData.detached ? 'HEAD terlepas' : (statusData.branch ?? 'main')}
          </span>
        {/if}
      </div>

      {#if statusData.ahead > 0 || statusData.behind > 0}
        <span class="ab-badge mono">+{statusData.ahead} -{statusData.behind}</span>
      {/if}
    </div>

    {#if statusData.merging || statusData.rebasing || statusData.detached || statusData.hasConflicts}
      <div class="state-banner">
        {#if statusData.rebasing}
          <span>Rebase sedang berjalan. Selesaikan lewat terminal.</span>
        {:else if statusData.merging || statusData.hasConflicts}
          <span>Merge conflict terdeteksi. Selesaikan konflik lewat terminal.</span>
        {:else if statusData.detached}
          <span>Detached HEAD aktif.</span>
        {/if}
      </div>
    {/if}

    {#if errorMsg}
      <div class="error-banner">
        <span>{errorMsg}</span>
      </div>
    {/if}

    <div class="changes-scroll">
      {#if statusData.conflicted.length > 0}
        <div class="group-section">
          <div class="group-title">Konflik ({statusData.conflicted.length})</div>
          {#each statusData.conflicted as item (item.path)}
            <div class="file-row">
              <button class="file-main" onclick={() => onOpenDiff?.(item.path, false)}>
                <span class="badge mono" style:color={badgeColorVar(item.badge)}>{item.badge}</span>
                <span class="file-path">{item.path}</span>
              </button>
            </div>
          {/each}
        </div>
      {/if}

      <div class="group-section">
        <div class="group-title">Staged ({statusData.staged.length})</div>
        {#if statusData.staged.length === 0}
          <div class="group-empty">Belum ada file di-stage</div>
        {:else}
          {#each statusData.staged as item (item.path)}
            <div class="file-row">
              <button class="file-main" onclick={() => onOpenDiff?.(item.path, true)}>
                <span class="badge mono" style:color={badgeColorVar(item.badge)}>{item.badge}</span>
                <span class="file-path">{item.path}</span>
              </button>
              <div class="row-actions">
                <button
                  class="row-btn"
                  title="Unstage file"
                  aria-label="Unstage file"
                  disabled={busy}
                  onclick={() => runAction(() => gitUnstage(item.path))}
                >
                  <Minus size={12} strokeWidth={2} />
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>

      <div class="group-section">
        <div class="group-title">Perubahan ({unstagedAll.length})</div>
        {#if unstagedAll.length === 0}
          <div class="group-empty">Tidak ada perubahan kerja</div>
        {:else}
          {#each unstagedAll as item (item.path)}
            <div class="file-row">
              <button class="file-main" onclick={() => onOpenDiff?.(item.path, false)}>
                <span class="badge mono" style:color={badgeColorVar(item.badge)}>{item.badge}</span>
                <span class="file-path">{item.path}</span>
              </button>
              <div class="row-actions">
                <button
                  class="row-btn"
                  title="Stage file"
                  aria-label="Stage file"
                  disabled={busy}
                  onclick={() => runAction(() => gitStage(item.path))}
                >
                  <Plus size={12} strokeWidth={2} />
                </button>
                <button
                  class="row-btn"
                  title="Buang perubahan file"
                  aria-label="Buang perubahan file"
                  disabled={busy}
                  onclick={() => (confirmDiscardPath = item.path)}
                >
                  <RotateCcw size={12} strokeWidth={2} />
                </button>
              </div>
            </div>
          {/each}
        {/if}
      </div>
    </div>

    {#if confirmDiscardPath}
      <div class="discard-confirm-box">
        <p class="discard-text">
          Buang perubahan pada <span class="mono">{confirmDiscardPath}</span>?
        </p>
        <div class="discard-actions">
          <button class="danger small-btn" onclick={handleConfirmDiscard}>Buang</button>
          <button class="small-btn" onclick={() => (confirmDiscardPath = null)}>Batal</button>
        </div>
      </div>
    {/if}

    <div class="commit-box">
      <input
        bind:this={commitInputEl}
        bind:value={commitMessage}
        type="text"
        class="commit-input"
        placeholder="Pesan commit..."
        onkeydown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
            e.preventDefault();
            void handleCommit();
          }
        }}
      />
      <button
        class="primary commit-btn"
        disabled={busy || statusData.staged.length === 0 || commitMessage.trim().length === 0}
        onclick={handleCommit}
      >
        Commit
      </button>
    </div>
  {/if}
</aside>

<style>
  .git-panel {
    width: 100%;
    height: 100%;
    background: var(--pn);
    display: flex;
    flex-direction: column;
    min-height: 0;
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

  .branch-bar {
    height: 34px;
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .branch-selector {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--tx);
    min-width: 0;
    flex: 1;
  }

  .branch-select {
    flex: 1;
    min-width: 0;
    height: 24px;
    background: var(--el);
    color: var(--tx);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    font-size: 12px;
    padding: 0 6px;
  }

  .branch-label {
    font-size: 12px;
    color: var(--tx);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .ab-badge {
    font-size: 11px;
    color: var(--dm);
    white-space: nowrap;
  }

  .state-banner {
    padding: 6px 12px;
    background: var(--el);
    border-bottom: 1px solid var(--bd);
    color: var(--mo);
    font-size: 12px;
  }

  .error-banner {
    padding: 6px 12px;
    background: var(--des);
    border-bottom: 1px solid var(--bd);
    color: var(--de);
    font-size: 12px;
  }

  .changes-scroll {
    flex: 1;
    overflow-y: auto;
    padding: 8px 6px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-section {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .group-title {
    padding: 2px 6px;
    font-size: 11px;
    font-weight: 500;
    color: var(--dm);
  }

  .group-empty {
    padding: 4px 6px;
    font-size: 12px;
    color: var(--ft);
  }

  .file-row {
    height: 28px;
    padding: 0 4px;
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
  }

  .file-row:hover {
    background: var(--acs);
  }

  .file-main {
    flex: 1;
    min-width: 0;
    height: 26px;
    padding: 0 4px;
    border: none;
    background: transparent;
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 8px;
    text-align: left;
  }

  .badge {
    width: 14px;
    font-size: 12px;
    font-weight: 600;
    text-align: center;
    flex-shrink: 0;
  }

  .file-path {
    font-size: 12px;
    color: var(--tx);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .row-actions {
    display: flex;
    align-items: center;
    gap: 2px;
    opacity: 0;
  }

  .file-row:hover .row-actions {
    opacity: 1;
  }

  .row-btn {
    width: 22px;
    height: 22px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .row-btn:hover {
    color: var(--tx);
  }

  .discard-confirm-box {
    padding: 8px 12px;
    background: var(--el);
    border-top: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .discard-text {
    font-size: 12px;
    color: var(--tx);
    word-break: break-all;
  }

  .discard-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .small-btn {
    height: 22px;
    padding: 0 8px;
    font-size: 12px;
  }

  .commit-box {
    padding: 12px;
    border-top: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .commit-input {
    width: 100%;
    height: 32px;
    padding: 0 10px;
    background: var(--bg);
    color: var(--tx);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    font-size: 12px;
  }

  .commit-btn {
    width: 100%;
    height: 28px;
  }

  .empty-state {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 24px;
    text-align: center;
  }

  .muted {
    color: var(--dm);
    font-size: 12px;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
