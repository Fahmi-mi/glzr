<script lang="ts">
  import { onDestroy } from 'svelte';
  import ArrowLeft from '@lucide/svelte/icons/arrow-left';
  import ArrowRight from '@lucide/svelte/icons/arrow-right';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import RotateCw from '@lucide/svelte/icons/rotate-cw';
  import { openExternal, previewCheckUrl } from '$lib/ipc';
  import {
    createPreviewHistory,
    formatPreviewDisplayUrl,
    formatPreviewHostLabel,
    formatPreviewTabTitle,
    normalizePreviewUrl,
    pushPreviewHistory,
    resolveIframeSrc,
    stepPreviewHistory
  } from '$lib/preview';

  interface Props {
    url: string;
    visible?: boolean;
    onUrlChange?: (nextUrl: string, nextTitle: string) => void;
  }

  let { url, visible = true, onUrlChange }: Props = $props();

  let history = $state(createPreviewHistory('http://localhost:5173/'));
  let urlInput = $state('');
  let validationError = $state<string | null>(null);
  let serverOnline = $state<boolean | null>(null);
  let checkingServer = $state(false);
  let reloadCounter = $state(0);
  let urlInputEl = $state<HTMLInputElement | null>(null);
  let lastSyncedPropUrl = $state('');

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  const currentUrl = $derived(history.entries[history.index] ?? 'http://localhost:5173/');
  const canGoBack = $derived(history.index > 0);
  const canGoForward = $derived(history.index < history.entries.length - 1);
  const hostLabel = $derived(formatPreviewHostLabel(currentUrl));

  const iframeSrc = $derived.by(() => {
    const baseSrc = resolveIframeSrc(currentUrl, visible && serverOnline === true);
    if (baseSrc === 'about:blank' || reloadCounter === 0) {
      return baseSrc;
    }
    try {
      const parsed = new URL(baseSrc);
      parsed.searchParams.set('__glzr_r', String(reloadCounter));
      return parsed.toString();
    } catch {
      return baseSrc;
    }
  });

  async function verifyReachability(targetUrl: string) {
    checkingServer = true;
    try {
      const ok = await previewCheckUrl(targetUrl);
      serverOnline = ok;
    } catch {
      serverOnline = false;
    } finally {
      checkingServer = false;
    }
  }

  $effect(() => {
    const normalized = normalizePreviewUrl(url) ?? 'http://localhost:5173/';
    if (normalized !== lastSyncedPropUrl) {
      lastSyncedPropUrl = normalized;
      if (history.entries[history.index] !== normalized) {
        history = pushPreviewHistory(history, normalized);
      }
      urlInput = formatPreviewDisplayUrl(normalized);
      validationError = null;
      void verifyReachability(normalized);
    }
  });

  $effect(() => {
    if (pollTimer !== null) {
      clearInterval(pollTimer);
      pollTimer = null;
    }

    if (visible) {
      const activeTarget = currentUrl;
      void verifyReachability(activeTarget);
      pollTimer = setInterval(() => {
        if (!checkingServer) {
          void previewCheckUrl(activeTarget)
            .then((ok) => {
              serverOnline = ok;
            })
            .catch(() => {
              serverOnline = false;
            });
        }
      }, 3000);
    }

    return () => {
      if (pollTimer !== null) {
        clearInterval(pollTimer);
        pollTimer = null;
      }
    };
  });

  onDestroy(() => {
    if (pollTimer !== null) {
      clearInterval(pollTimer);
      pollTimer = null;
    }
  });

  function applyNavigation(targetUrl: string) {
    urlInput = formatPreviewDisplayUrl(targetUrl);
    validationError = null;
    lastSyncedPropUrl = targetUrl;
    onUrlChange?.(targetUrl, formatPreviewTabTitle(targetUrl));
    void verifyReachability(targetUrl);
  }

  function handleUrlSubmit() {
    const normalized = normalizePreviewUrl(urlInput);
    if (!normalized) {
      validationError = 'Hanya http(s)://localhost atau 127.0.0.1 yang diizinkan.';
      return;
    }
    history = pushPreviewHistory(history, normalized);
    applyNavigation(normalized);
  }

  function handleStepHistory(delta: -1 | 1) {
    const next = stepPreviewHistory(history, delta);
    if (next.index === history.index) return;
    history = next;
    const targetUrl = history.entries[history.index];
    applyNavigation(targetUrl);
  }

  export function reloadPreview() {
    reloadCounter += 1;
    void verifyReachability(currentUrl);
  }

  export function focusUrlBar() {
    urlInputEl?.focus();
    urlInputEl?.select();
  }

  function handleOpenInBrowser() {
    void openExternal(currentUrl).catch(() => {});
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!visible) return;
    const ctrlOrMeta = event.ctrlKey || event.metaKey;
    const keyLower = event.key.toLowerCase();

    if (ctrlOrMeta && !event.altKey && !event.shiftKey && keyLower === 'l') {
      event.preventDefault();
      focusUrlBar();
      return;
    }

    if (
      (ctrlOrMeta && !event.altKey && !event.shiftKey && keyLower === 'r') ||
      event.key === 'F5'
    ) {
      event.preventDefault();
      reloadPreview();
    }
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="preview-tab" class:hidden-preview={!visible}>
  <div class="url-bar-shell">
    <div class="url-bar">
      <div class="nav-controls">
        <button
          class="icon-btn"
          title="Kembali"
          aria-label="Kembali"
          disabled={!canGoBack}
          onclick={() => handleStepHistory(-1)}
        >
          <ArrowLeft size={14} strokeWidth={2} />
        </button>
        <button
          class="icon-btn"
          title="Maju"
          aria-label="Maju"
          disabled={!canGoForward}
          onclick={() => handleStepHistory(1)}
        >
          <ArrowRight size={14} strokeWidth={2} />
        </button>
        <button
          class="icon-btn"
          title="Muat ulang (Ctrl+R)"
          aria-label="Muat ulang"
          onclick={reloadPreview}
        >
          <RotateCw size={13} strokeWidth={2} />
        </button>
      </div>

      <input
        bind:this={urlInputEl}
        bind:value={urlInput}
        type="text"
        class="url-input"
        placeholder="localhost:5173"
        aria-label="URL preview localhost"
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            handleUrlSubmit();
          } else if (e.key === 'Escape') {
            validationError = null;
            urlInput = formatPreviewDisplayUrl(currentUrl);
            urlInputEl?.blur();
          }
        }}
      />

      <button
        class="icon-btn"
        title="Buka di browser"
        aria-label="Buka di browser"
        onclick={handleOpenInBrowser}
      >
        <ExternalLink size={14} strokeWidth={2} />
      </button>
    </div>

    {#if validationError}
      <p class="url-error">{validationError}</p>
    {/if}
  </div>

  <div class="preview-stage">
    {#if serverOnline === false}
      <div class="offline-card">
        <p class="offline-title">Server belum berjalan di {hostLabel}</p>
        <div class="offline-actions">
          <button class="primary" onclick={reloadPreview}>Coba lagi</button>
          <button onclick={handleOpenInBrowser}>Buka di browser</button>
        </div>
      </div>
    {:else}
      <iframe
        title="Preview {hostLabel}"
        src={iframeSrc}
        class="preview-frame"
        sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
      ></iframe>
    {/if}
  </div>
</div>

<style>
  .preview-tab {
    flex: 1;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }

  .preview-tab.hidden-preview {
    display: none;
  }

  .url-bar-shell {
    padding: 16px 16px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .url-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    border: 1px solid var(--bd);
    border-radius: var(--radius-panel);
    background: var(--sf);
    color: var(--dm);
  }

  .nav-controls {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .icon-btn:hover:not(:disabled) {
    color: var(--tx);
    background: var(--acs);
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .url-input {
    flex: 1;
    height: 24px;
    padding: 0 6px;
    border: none;
    background: transparent;
    color: var(--tx);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .url-input:focus {
    outline: none;
  }

  .url-error {
    padding: 0 4px;
    color: var(--de);
    font-size: 12px;
  }

  .preview-stage {
    flex: 1;
    display: flex;
    min-width: 0;
    min-height: 0;
    padding: 0 16px 16px;
  }

  .preview-frame {
    flex: 1;
    width: 100%;
    height: 100%;
    border: 1px solid var(--bd);
    border-radius: var(--radius-panel);
    background: var(--sf);
  }

  .offline-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    border: 1px solid var(--bd);
    border-radius: var(--radius-panel);
    background: var(--sf);
    padding: 24px;
  }

  .offline-title {
    color: var(--tx);
    font-size: 13px;
  }

  .offline-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }
</style>
