<script lang="ts">
  import FileCode from '@lucide/svelte/icons/file-code';
  import Plus from '@lucide/svelte/icons/plus';
  import Minus from '@lucide/svelte/icons/minus';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import {
    gitDiff,
    gitDiscardHunk,
    gitStageHunk,
    gitUnstageHunk,
    type AppErrorPayload,
    type GitDiffDto
  } from '$lib/ipc';
  import { parseHunkLines } from '$lib/git';
  import { smoothScroll } from '$lib/scroll';

  interface Props {
    relPath: string;
    initialStaged?: boolean;
    onOpenFile?: (relPath: string, line: number | null) => void;
    onChanged?: () => void;
  }

  let { relPath, initialStaged = false, onOpenFile, onChanged }: Props = $props();

  let stagedMode = $state(false);
  let diffData = $state<GitDiffDto | null>(null);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let confirmingDiscardHunk = $state<number | null>(null);
  let busyHunk = $state<number | null>(null);

  const memoizedHunks = $derived.by(() => {
    if (!diffData?.fileDiff?.hunks) return [];
    return diffData.fileDiff.hunks.map((hunk) => ({
      hunk,
      lines: parseHunkLines(hunk)
    }));
  });

  $effect(() => {
    stagedMode = initialStaged;
  });

  async function loadDiff(targetPath: string, staged: boolean) {
    loading = true;
    errorMsg = null;
    confirmingDiscardHunk = null;
    try {
      diffData = await gitDiff(targetPath, staged);
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to load diff';
      diffData = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadDiff(relPath, stagedMode);
  });

  async function handleStageHunk(hunkIndex: number) {
    busyHunk = hunkIndex;
    errorMsg = null;
    try {
      await gitStageHunk(relPath, hunkIndex);
      await loadDiff(relPath, stagedMode);
      onChanged?.();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to stage hunk';
    } finally {
      busyHunk = null;
    }
  }

  async function handleUnstageHunk(hunkIndex: number) {
    busyHunk = hunkIndex;
    errorMsg = null;
    try {
      await gitUnstageHunk(relPath, hunkIndex);
      await loadDiff(relPath, stagedMode);
      onChanged?.();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to unstage hunk';
    } finally {
      busyHunk = null;
    }
  }

  async function handleDiscardHunk(hunkIndex: number) {
    busyHunk = hunkIndex;
    confirmingDiscardHunk = null;
    errorMsg = null;
    try {
      await gitDiscardHunk(relPath, hunkIndex);
      await loadDiff(relPath, stagedMode);
      onChanged?.();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to discard hunk';
    } finally {
      busyHunk = null;
    }
  }
</script>

<div class="diff-tab-container">
  <div class="diff-toolbar">
    <div class="toolbar-left">
      <span class="file-path mono">{relPath}</span>
      <div class="segmented-switch">
        <button class="seg-btn" class:active={!stagedMode} onclick={() => (stagedMode = false)}>
          Unstaged
        </button>
        <button class="seg-btn" class:active={stagedMode} onclick={() => (stagedMode = true)}>
          Staged
        </button>
      </div>
    </div>

    <div class="toolbar-right">
      <button class="small-btn" onclick={() => onOpenFile?.(relPath, null)}>
        <FileCode size={14} strokeWidth={2} />
        <span>Open file</span>
      </button>
    </div>
  </div>

  {#if errorMsg}
    <div class="error-bar">
      <span>{errorMsg}</span>
    </div>
  {/if}

  <div class="diff-scroll" use:smoothScroll>
    {#if loading}
      <div class="empty-box">
        <p class="muted">Loading diff...</p>
      </div>
    {:else if !diffData}
      <div class="empty-box">
        <p class="muted">Diff unavailable.</p>
      </div>
    {:else if diffData.tooLarge}
      <div class="empty-box">
        <p class="muted">Diff is too large (> 2 MB). Use terminal to inspect changes.</p>
      </div>
    {:else if diffData.isBinary}
      <div class="empty-box">
        <p class="muted">Binary file changed. Per-hunk diff is not available.</p>
      </div>
    {:else if !diffData.fileDiff || diffData.fileDiff.hunks.length === 0}
      <div class="empty-box">
        <p class="muted">
          No {stagedMode ? 'staged' : 'unstaged'} changes in this file.
        </p>
      </div>
    {:else}
      <div class="hunk-list">
        {#each memoizedHunks as { hunk, lines } (hunk.index)}
          <section class="hunk-card">
            <header class="hunk-header">
              <span class="hunk-range mono">{hunk.header}</span>
              <div class="hunk-actions">
                {#if stagedMode}
                  <button
                    class="small-btn"
                    disabled={busyHunk === hunk.index}
                    onclick={() => handleUnstageHunk(hunk.index)}
                  >
                    <Minus size={12} strokeWidth={2} />
                    <span>Unstage</span>
                  </button>
                {:else}
                  <button
                    class="small-btn"
                    disabled={busyHunk === hunk.index}
                    onclick={() => handleStageHunk(hunk.index)}
                  >
                    <Plus size={12} strokeWidth={2} />
                    <span>Stage</span>
                  </button>
                  {#if confirmingDiscardHunk === hunk.index}
                    <button
                      class="small-btn danger"
                      disabled={busyHunk === hunk.index}
                      onclick={() => handleDiscardHunk(hunk.index)}
                    >
                      Confirm discard
                    </button>
                    <button class="small-btn" onclick={() => (confirmingDiscardHunk = null)}>
                      Cancel
                    </button>
                  {:else}
                    <button
                      class="small-btn"
                      disabled={busyHunk === hunk.index}
                      onclick={() => (confirmingDiscardHunk = hunk.index)}
                    >
                      <RotateCcw size={12} strokeWidth={2} />
                      <span>Discard</span>
                    </button>
                  {/if}
                {/if}
              </div>
            </header>

            <div class="hunk-content">
              <div class="hunk-lines">
                {#each lines as line, lineIdx (lineIdx)}
                  <div
                    class="diff-line {line.kind}"
                    role="button"
                    tabindex="0"
                    onclick={() => {
                      const sel = typeof window !== 'undefined' ? window.getSelection()?.toString() : '';
                      if (sel && sel.length > 0) return;
                      const target = line.newLine ?? line.oldLine;
                      if (target !== null) {
                        onOpenFile?.(relPath, target);
                      }
                    }}
                    onkeydown={(e) => {
                      if (e.key === 'Enter' || e.key === ' ') {
                        e.preventDefault();
                        const target = line.newLine ?? line.oldLine;
                        if (target !== null) {
                          onOpenFile?.(relPath, target);
                        }
                      }
                    }}
                  >
                    <span class="gutter-num gutter-old">{line.oldLine ?? ''}</span>
                    <span class="gutter-num gutter-new">{line.newLine ?? ''}</span>
                    <span class="line-prefix">
                      {line.kind === 'add' ? '+' : line.kind === 'del' ? '-' : ' '}
                    </span>
                    <span class="line-code">{line.text}</span>
                  </div>
                {/each}
              </div>
            </div>
          </section>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .diff-tab-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: transparent;
  }

  .diff-toolbar {
    height: 36px;
    padding: 0 16px;
    background: transparent;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .toolbar-left,
  .toolbar-right {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .file-path {
    font-size: 12px;
    color: var(--tx);
  }

  .segmented-switch {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .seg-btn {
    height: 22px;
    padding: 0 10px;
    font-size: 12px;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .seg-btn.active {
    background: var(--acs);
    color: var(--ac);
  }

  .small-btn {
    height: 24px;
    padding: 0 8px;
    font-size: 12px;
    gap: 4px;
  }

  .error-bar {
    padding: 6px 16px;
    background: var(--des);
    color: var(--de);
    border-bottom: 1px solid var(--bd);
    font-size: 12px;
  }

  .diff-scroll {
    flex: 1;
    overflow: auto;
    overscroll-behavior: contain;
    background: var(--pbg);
    padding: 16px 24px;
  }

  .empty-box {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    min-height: 160px;
  }

  .muted {
    color: var(--dm);
  }

  .hunk-list {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .hunk-card {
    border: 1px solid var(--bd);
    border-radius: var(--radius-menu);
    overflow: hidden;
    background: var(--el);
    content-visibility: auto;
    contain-intrinsic-size: auto 120px;
  }

  .hunk-header {
    height: 32px;
    padding: 0 12px;
    background: var(--el);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .hunk-range {
    font-size: 12px;
    color: var(--dm);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hunk-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .hunk-content {
    overflow-x: auto;
    width: 100%;
    overscroll-behavior-x: contain;
  }

  .hunk-lines {
    display: flex;
    flex-direction: column;
    min-width: 100%;
    width: max-content;
    background: transparent;
  }

  .diff-line {
    width: 100%;
    min-height: 24px;
    height: auto;
    padding: 0;
    border: none;
    border-radius: 0;
    background: transparent;
    color: var(--tx);
    font-family: var(--font-mono);
    font-size: var(--font-size-code);
    line-height: 1.65;
    display: grid;
    grid-template-columns: 46px 46px 20px 1fr;
    align-items: stretch;
    text-align: left;
    cursor: pointer;
    user-select: text;
  }

  .diff-line:hover {
    filter: brightness(1.08);
  }

  .diff-line:focus-visible {
    outline: 1px solid var(--ac);
    outline-offset: -1px;
  }

  .diff-line.add {
    background: var(--ads);
    color: var(--ad);
  }

  .diff-line.del {
    background: var(--des);
    color: var(--de);
  }

  .diff-line.meta {
    color: var(--dm);
    font-style: italic;
    cursor: default;
  }

  .diff-line.meta:hover {
    filter: none;
  }

  .gutter-num {
    font-size: 11px;
    text-align: right;
    padding: 2px 6px;
    user-select: none;
    font-variant-numeric: tabular-nums;
  }

  .gutter-old {
    color: var(--dm);
    border-right: 1px solid var(--bd);
  }

  .gutter-new {
    color: var(--ft);
    border-right: 1px solid var(--bd);
  }

  .line-prefix {
    user-select: none;
    color: inherit;
    text-align: center;
    padding: 2px 0;
    font-weight: 600;
  }

  .line-code {
    display: block;
    text-align: left;
    white-space: pre;
    tab-size: 2;
    min-height: 1.65em;
    padding: 2px 12px 2px 8px;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
