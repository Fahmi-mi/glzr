<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Globe from '@lucide/svelte/icons/globe';
  import RotateCcw from '@lucide/svelte/icons/rotate-ccw';
  import Search from '@lucide/svelte/icons/search';
  import X from '@lucide/svelte/icons/x';
  import { ptyResize } from '$lib/ipc';
  import { formatPreviewTabTitle } from '$lib/preview';
  import {
    getOrCreateTerminalSession,
    type RegisteredTerminalSession,
    type TerminalSessionDelegate
  } from '$lib/terminalRegistry';

  interface Props {
    sessionId: string;
    visible?: boolean;
    focused?: boolean;
    theme?: 'dark' | 'light';
    onOpenLocalhost?: (url: string) => void;
  }

  let {
    sessionId,
    visible = true,
    focused = false,
    theme = 'dark',
    onOpenLocalhost
  }: Props = $props();

  let hostEl = $state<HTMLDivElement | null>(null);
  let searchInputEl = $state<HTMLInputElement | null>(null);
  let searchOpen = $state(false);
  let searchQuery = $state('');
  let pendingLargePaste = $state<string | null>(null);

  let session = $state<RegisteredTerminalSession | null>(null);
  let exited = $state(false);
  let exitCode = $state<number | null>(null);
  let errorMsg = $state<string | null>(null);
  let detectedLocalUrl = $state<string | null>(null);
  let restarting = $state(false);

  let resizeObserver: ResizeObserver | null = null;
  let resizeTimer: ReturnType<typeof setTimeout> | null = null;
  let handleAuxClick: ((ev: MouseEvent) => void) | null = null;
  let handleDomPaste: ((ev: ClipboardEvent) => void) | null = null;
  let unsubscribeSession: (() => void) | null = null;
  let localDelegate: TerminalSessionDelegate | null = null;

  function syncState(s: RegisteredTerminalSession) {
    exited = s.exited;
    exitCode = s.exitCode;
    errorMsg = s.errorMsg;
    detectedLocalUrl = s.detectedLocalUrl;
    restarting = s.restarting;
  }

  function scheduleFitAndResize() {
    if (!visible || !session || !hostEl) return;
    if (resizeTimer !== null) {
      clearTimeout(resizeTimer);
    }
    resizeTimer = setTimeout(() => {
      if (!visible || !session || !hostEl) return;
      try {
        session.fitAddon.fit();
        session.canvasAddon?.clearTextureAtlas();
        session.term.refresh(0, Math.max(0, session.term.rows - 1));
        session.term.scrollToBottom();
        void ptyResize(session.activeSessionId, session.term.cols, session.term.rows).catch(() => {});
      } catch {
        return;
      }
    }, 40);
  }

  function confirmLargePaste() {
    if (session && pendingLargePaste) {
      session.term.paste(pendingLargePaste);
    }
    pendingLargePaste = null;
    session?.focus();
  }

  export function focus() {
    if (!session) return;
    session.focus();
  }

  export async function restartSession() {
    if (!session || restarting) return;
    await session.restart();
  }

  $effect(() => {
    if (session) {
      session.setTheme(theme);
    }
  });

  $effect(() => {
    if (session) {
      session.onOpenLocalhost = onOpenLocalhost;
    }
  });

  onMount(() => {
    if (!hostEl) return;

    const s = getOrCreateTerminalSession(sessionId, { theme, onOpenLocalhost });
    session = s;
    syncState(s);

    localDelegate = {
      onToggleSearch: () => {
        searchOpen = !searchOpen;
        if (searchOpen) {
          queueMicrotask(() => searchInputEl?.focus({ preventScroll: true }));
        } else {
          s.focus();
        }
      },
      onLargePaste: (text: string) => {
        pendingLargePaste = text;
      },
      onZoom: () => {
        scheduleFitAndResize();
      }
    };
    s.delegate = localDelegate;

    unsubscribeSession = s.subscribe(() => {
      syncState(s);
    });

    hostEl.appendChild(s.containerEl);

    try {
      s.canvasAddon?.clearTextureAtlas();
      s.fitAddon.fit();
      s.term.refresh(0, Math.max(0, s.term.rows - 1));
      s.term.scrollToBottom();
    } catch {}

    handleAuxClick = (ev: MouseEvent) => {
      if (ev.button === 1) {
        ev.preventDefault();
        void s.handlePaste('primary');
      }
    };
    handleDomPaste = (ev: ClipboardEvent) => {
      ev.preventDefault();
      void s.handlePaste('clipboard');
    };
    hostEl.addEventListener('auxclick', handleAuxClick);
    hostEl.addEventListener('paste', handleDomPaste);

    resizeObserver = new ResizeObserver(() => {
      scheduleFitAndResize();
    });
    resizeObserver.observe(hostEl);

    if (visible) {
      queueMicrotask(() => {
        scheduleFitAndResize();
        if (focused) {
          s.focus();
        }
      });
      requestAnimationFrame(() => {
        try {
          s.canvasAddon?.clearTextureAtlas();
          s.fitAddon.fit();
          s.term.refresh(0, Math.max(0, s.term.rows - 1));
          s.term.scrollToBottom();
        } catch {}
      });
    }
  });

  $effect(() => {
    if (visible && session) {
      queueMicrotask(() => {
        scheduleFitAndResize();
        if (focused) {
          session?.focus();
        }
      });
    }
  });

  onDestroy(() => {
    if (resizeTimer !== null) {
      clearTimeout(resizeTimer);
      resizeTimer = null;
    }
    if (handleAuxClick && hostEl) {
      hostEl.removeEventListener('auxclick', handleAuxClick);
      handleAuxClick = null;
    }
    if (handleDomPaste && hostEl) {
      hostEl.removeEventListener('paste', handleDomPaste);
      handleDomPaste = null;
    }
    resizeObserver?.disconnect();
    unsubscribeSession?.();
    unsubscribeSession = null;

    if (session) {
      if (session.delegate === localDelegate) {
        session.delegate = undefined;
      }
      if (hostEl && session.containerEl.parentElement === hostEl) {
        session.containerEl.remove();
      }
    }
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
              session?.searchAddon.findPrevious(searchQuery);
            } else {
              session?.searchAddon.findNext(searchQuery);
            }
          } else if (e.key === 'Escape') {
            e.preventDefault();
            searchOpen = false;
            session?.focus();
          }
        }}
      />
      <button
        class="icon-btn"
        aria-label="Close search"
        onclick={() => {
          searchOpen = false;
          session?.focus();
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
    background: transparent;
    padding: 14px 16px;
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
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-input);
    box-shadow: var(--shadow-float);
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
    box-shadow: none;
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
    background: var(--el);
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
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-input);
    box-shadow: var(--shadow-float);
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
