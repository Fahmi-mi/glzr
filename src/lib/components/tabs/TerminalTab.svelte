<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import '@xterm/xterm/css/xterm.css';
  import { Terminal } from '@xterm/xterm';
  import { FitAddon } from '@xterm/addon-fit';
  import { SearchAddon } from '@xterm/addon-search';
  import { Unicode11Addon } from '@xterm/addon-unicode11';
  import { WebLinksAddon } from '@xterm/addon-web-links';
  import { CanvasAddon } from '@xterm/addon-canvas';
  import Globe from '@lucide/svelte/icons/globe';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Search from '@lucide/svelte/icons/search';
  import X from '@lucide/svelte/icons/x';
  import {
    onBackendEvent,
    openExternal,
    ptyKill,
    ptyResize,
    ptySpawn,
    ptyWrite,
    type AppErrorPayload
  } from '$lib/ipc';
  import { formatPreviewTabTitle } from '$lib/preview';
  import {
    clampTerminalFontSize,
    extractLocalhostUrl,
    getTerminalTheme,
    isLargePaste,
    resolveTerminalKey
  } from '$lib/terminal';

  interface Props {
    sessionId: string;
    visible?: boolean;
    theme?: 'dark' | 'light';
    onOpenLocalhost?: (url: string) => void;
  }

  let { sessionId, visible = true, theme = 'dark', onOpenLocalhost }: Props = $props();

  function generateSessionToken(): string {
    return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
  }

  // svelte-ignore state_referenced_locally
  let activeSessionId = $state(`${sessionId}:${generateSessionToken()}`);
  let restarting = $state(false);
  let hostEl = $state<HTMLDivElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let searchOpen = $state(false);
  let searchQuery = $state('');
  let exited = $state(false);
  let exitCode = $state<number | null>(null);
  let errorMsg = $state<string | null>(null);
  let pendingLargePaste = $state<string | null>(null);
  let detectedLocalUrl = $state<string | null>(null);
  let fontSize = $state(14);

  let term: Terminal | null = null;
  let fitAddon: FitAddon | null = null;
  let searchAddon: SearchAddon | null = null;
  let resizeObserver: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let unlistenOutput: (() => void) | null = null;
  let unlistenExit: (() => void) | null = null;

  function scheduleFitAndResize() {
    if (!visible || !term || !fitAddon) return;
    if (resizeTimer !== null) {
      clearTimeout(resizeTimer);
    }
    resizeTimer = setTimeout(() => {
      if (!visible || !term || !fitAddon) return;
      try {
        fitAddon.fit();
        void ptyResize(activeSessionId, term.cols, term.rows).catch(() => {});
      } catch {
        // Ignore transient layout state
      }
    }, 40);
  }

  function applyFontZoom(delta: number) {
    if (!term) return;
    fontSize = clampTerminalFontSize(fontSize + delta);
    term.options.fontSize = fontSize;
    scheduleFitAndResize();
  }

  async function handleClipboardPaste() {
    if (!term) return;
    try {
      const text = await navigator.clipboard.readText();
      if (!text) return;
      if (isLargePaste(text, 64)) {
        pendingLargePaste = text;
        return;
      }
      term.paste(text);
    } catch {
      // Clipboard read denied or empty
    }
  }

  function confirmLargePaste() {
    if (term && pendingLargePaste) {
      term.paste(pendingLargePaste);
    }
    pendingLargePaste = null;
    term?.focus();
  }

  async function spawnShell() {
    if (!term) return;
    exited = false;
    exitCode = null;
    errorMsg = null;
    try {
      await ptySpawn(activeSessionId, term.cols, term.rows);
      term.focus();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to start terminal session';
    }
  }

  export async function restartSession() {
    if (!term || restarting) return;
    restarting = true;
    try {
      const oldSessionId = activeSessionId;
      activeSessionId = `${sessionId}:${generateSessionToken()}`;
      void ptyKill(oldSessionId).catch(() => {});
      exited = false;
      exitCode = null;
      errorMsg = null;
      term.reset();
      try {
        fitAddon?.fit();
      } catch {
        // Ignore transient fit error
      }
      await spawnShell();
    } finally {
      restarting = false;
    }
  }

  $effect(() => {
    if (!activeSessionId.startsWith(`${sessionId}:`)) {
      const oldSessionId = activeSessionId;
      activeSessionId = `${sessionId}:${generateSessionToken()}`;
      void ptyKill(oldSessionId).catch(() => {});
    }
  });

  $effect(() => {
    if (term) {
      term.options.theme = getTerminalTheme(theme);
      try {
        term.refresh(0, Math.max(0, term.rows - 1));
      } catch {
        // Ignore during transient state
      }
    }
  });

  onMount(async () => {
    if (!hostEl) return;

    const instance = new Terminal({
      cursorBlink: true,
      cursorStyle: 'block',
      scrollback: 5000,
      allowProposedApi: true,
      fontFamily:
        "'JetBrains Mono Nerd Font', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
      fontSize,
      lineHeight: 1.35,
      smoothScrollDuration: 0,
      theme: getTerminalTheme(theme)
    });

    const fit = new FitAddon();
    const search = new SearchAddon();
    const unicode11 = new Unicode11Addon();
    const webLinks = new WebLinksAddon((_event, uri) => {
      const localUrl = extractLocalhostUrl(uri);
      if (localUrl && onOpenLocalhost) {
        onOpenLocalhost(localUrl);
      } else {
        void openExternal(uri).catch(() => {});
      }
    });

    instance.loadAddon(fit);
    instance.loadAddon(search);
    instance.loadAddon(unicode11);
    instance.loadAddon(webLinks);
    instance.unicode.activeVersion = '11';

    instance.open(hostEl);

    try {
      instance.loadAddon(new CanvasAddon());
    } catch {
      // Fallback to default DOM renderer
    }

    instance.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
      if (ev.type !== 'keydown') return true;

      const disposition = resolveTerminalKey({
        key: ev.key,
        ctrlKey: ev.ctrlKey,
        shiftKey: ev.shiftKey,
        altKey: ev.altKey,
        metaKey: ev.metaKey,
        hasSelection: instance.hasSelection()
      });

      if (disposition === 'copy') {
        const selected = instance.getSelection();
        if (selected) {
          void navigator.clipboard.writeText(selected).catch(() => {});
          instance.clearSelection();
        }
        return false;
      }

      if (disposition === 'paste') {
        ev.preventDefault();
        void handleClipboardPaste();
        return false;
      }

      if (disposition === 'app-shortcut') {
        const lower = ev.key.toLowerCase();
        if (lower === 's') {
          ev.preventDefault();
          searchOpen = !searchOpen;
          if (searchOpen) {
            queueMicrotask(() => searchInputEl?.focus());
          } else {
            instance.focus();
          }
          return false;
        }
        if (ev.key === '=' || ev.key === '+') {
          ev.preventDefault();
          applyFontZoom(1);
          return false;
        }
        if (ev.key === '-') {
          ev.preventDefault();
          applyFontZoom(-1);
          return false;
        }
        return false;
      }

      return true;
    });

    instance.onData((data) => {
      if (!exited) {
        void ptyWrite(activeSessionId, data).catch(() => {});
      }
    });

    term = instance;
    fitAddon = fit;
    searchAddon = search;

    try {
      fit.fit();
    } catch {
      // Ignore initial fit error
    }

    unlistenOutput = await onBackendEvent('pty:output', (payload) => {
      if (payload.sessionId === activeSessionId && term) {
        term.write(payload.data);
        if (
          payload.data.length <= 2048 &&
          (payload.data.includes('localhost') || payload.data.includes('127.0.0.1'))
        ) {
          const found = extractLocalhostUrl(payload.data);
          if (found) {
            detectedLocalUrl = found;
          }
        }
      }
    });

    unlistenExit = await onBackendEvent('pty:exit', (payload) => {
      if (payload.sessionId === activeSessionId) {
        exited = true;
        exitCode = payload.code;
      }
    });

    resizeObserver = new ResizeObserver(() => {
      scheduleFitAndResize();
    });
    resizeObserver.observe(hostEl);

    await spawnShell();
  });

  $effect(() => {
    if (visible && term && fitAddon) {
      queueMicrotask(() => {
        scheduleFitAndResize();
        term?.focus();
      });
    }
  });

  onDestroy(() => {
    if (resizeTimer !== null) {
      clearTimeout(resizeTimer);
      resizeTimer = null;
    }
    resizeObserver?.disconnect();
    unlistenOutput?.();
    unlistenExit?.();
    void ptyKill(activeSessionId).catch(() => {});
    term?.dispose();
    term = null;
  });
</script>

<div class="terminal-tab-wrapper" class:hidden-tab={!visible}>
  {#if searchOpen}
    <div class="terminal-search-bar">
      <Search size={14} strokeWidth={2} />
      <input
        bind:this={searchInputEl}
        bind:value={searchQuery}
        type="text"
        class="search-input"
        placeholder="Find in terminal..."
        onkeydown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            if (e.shiftKey) {
              searchAddon?.findPrevious(searchQuery);
            } else {
              searchAddon?.findNext(searchQuery);
            }
          } else if (e.key === 'Escape') {
            e.preventDefault();
            searchOpen = false;
            term?.focus();
          }
        }}
      />
      <button
        class="icon-btn"
        aria-label="Close search"
        onclick={() => {
          searchOpen = false;
          term?.focus();
        }}
      >
        <X size={12} strokeWidth={2} />
      </button>
    </div>
  {/if}

  {#if errorMsg}
    <div class="terminal-banner error">
      <span>{errorMsg}</span>
      <button class="small-btn" onclick={restartSession}>Retry</button>
    </div>
  {/if}

  {#if pendingLargePaste}
    <div class="terminal-banner warning">
      <span>Pasted text is very large (> 64 KB). Proceed with pasting to terminal?</span>
      <div class="banner-actions">
        <button class="primary small-btn" onclick={confirmLargePaste}>Paste</button>
        <button class="small-btn" onclick={() => (pendingLargePaste = null)}>Cancel</button>
      </div>
    </div>
  {/if}

  <div class="xterm-stage" bind:this={hostEl}></div>

  {#if detectedLocalUrl && onOpenLocalhost}
    <div class="detected-url-pill">
      <button
        class="open-preview-btn"
        title="Open in preview tab"
        onclick={() => {
          if (detectedLocalUrl) {
            onOpenLocalhost(detectedLocalUrl);
            detectedLocalUrl = null;
          }
        }}
      >
        <Globe size={12} strokeWidth={2} />
        <span>Preview {formatPreviewTabTitle(detectedLocalUrl)}</span>
      </button>
      <button
        class="dismiss-pill-btn"
        aria-label="Dismiss port notification"
        onclick={() => (detectedLocalUrl = null)}
      >
        <X size={11} strokeWidth={2} />
      </button>
    </div>
  {/if}

  {#if exited}
    <div class="terminal-banner exited">
      <span>Process completed{exitCode !== null ? ` (exit code ${exitCode})` : ''}.</span>
      <button class="primary small-btn" disabled={restarting} onclick={restartSession}>
        <RotateCcw size={12} strokeWidth={2} />
        <span>Restart</span>
      </button>
    </div>
  {/if}
</div>

<style>
  .terminal-tab-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: var(--bg);
    padding: 8px 12px;
  }

  .terminal-tab-wrapper.hidden-tab {
    display: none;
  }

  .xterm-stage {
    flex: 1;
    width: 100%;
    min-height: 0;
    overflow: hidden;
  }

  .terminal-search-bar {
    position: absolute;
    top: 8px;
    right: 16px;
    z-index: 20;
    height: 30px;
    padding: 0 8px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--dm);
  }

  .search-input {
    width: 180px;
    height: 22px;
    border: none;
    background: transparent;
    color: var(--tx);
    font-size: 12px;
    padding: 0;
  }

  .search-input:focus {
    outline: none;
  }

  .icon-btn {
    width: 20px;
    height: 20px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .terminal-banner {
    min-height: 32px;
    padding: 4px 12px;
    background: var(--pn);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--dm);
    margin-bottom: 6px;
  }

  .terminal-banner.exited {
    margin-top: 6px;
    margin-bottom: 0;
  }

  .terminal-banner.warning {
    color: var(--mo);
  }

  .terminal-banner.error {
    color: var(--de);
  }

  .banner-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .small-btn {
    height: 22px;
    padding: 0 8px;
    font-size: 12px;
    gap: 4px;
  }

  .detected-url-pill {
    position: absolute;
    right: 16px;
    bottom: 14px;
    z-index: 15;
    display: flex;
    align-items: center;
    background: var(--el);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    padding: 2px;
  }

  .open-preview-btn {
    height: 24px;
    padding: 0 8px;
    border: none;
    background: transparent;
    color: var(--ac);
    font-size: 12px;
    gap: 6px;
  }

  .open-preview-btn:hover {
    background: var(--acs);
  }

  .dismiss-pill-btn {
    width: 22px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .dismiss-pill-btn:hover {
    color: var(--tx);
  }
</style>
