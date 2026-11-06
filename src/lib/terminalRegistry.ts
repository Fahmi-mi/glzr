if (typeof globalThis !== 'undefined' && typeof (globalThis as Record<string, unknown>).self === 'undefined') {
  (globalThis as Record<string, unknown>).self = globalThis;
}

import '@xterm/xterm/css/xterm.css';
import { Terminal, type ITheme } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { SearchAddon } from '@xterm/addon-search';
import { Unicode11Addon } from '@xterm/addon-unicode11';
import { WebLinksAddon } from '@xterm/addon-web-links';
import { CanvasAddon } from '@xterm/addon-canvas';
import {
  clipboardReadText,
  clipboardWriteText,
  onBackendEvent,
  openExternal,
  ptyKill,
  ptyResize,
  ptySpawn,
  ptyWrite,
  type AppErrorPayload
} from '$lib/ipc';
import {
  clampTerminalFontSize,
  extractLocalhostUrl,
  getTerminalTheme,
  isLargePaste,
  resolveTerminalKey
} from '$lib/terminal';

export interface TerminalSessionDelegate {
  onToggleSearch?: () => void;
  onLargePaste?: (text: string) => void;
  onZoom?: (delta: number) => void;
}

export interface RegisteredTerminalSession {
  tabId: string;
  activeSessionId: string;
  term: Terminal;
  fitAddon: FitAddon;
  searchAddon: SearchAddon;
  canvasAddon: CanvasAddon | null;
  containerEl: HTMLElement;
  exited: boolean;
  exitCode: number | null;
  errorMsg: string | null;
  detectedLocalUrl: string | null;
  fontSize: number;
  theme: 'dark' | 'light';
  restarting: boolean;
  delegate?: TerminalSessionDelegate;
  onOpenLocalhost?: (url: string) => void;
  unlistenOutput?: () => void;
  unlistenExit?: () => void;
  disposed: boolean;
  subscribe: (fn: () => void) => () => void;
  notify: () => void;
  setTheme: (theme: 'dark' | 'light') => void;
  applyZoom: (delta: number) => void;
  restart: () => Promise<void>;
  handlePaste: (selection?: 'clipboard' | 'primary') => Promise<void>;
}

function generateSessionToken(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
}

function buildXtermTheme(mode: 'dark' | 'light'): ITheme {
  const base = getTerminalTheme(mode);
  return {
    ...base,
    background: 'rgba(0, 0, 0, 0)'
  };
}

function createTerminalSessionInstance(
  tabId: string,
  options?: { theme?: 'dark' | 'light'; onOpenLocalhost?: (url: string) => void }
): RegisteredTerminalSession {
  const initialTheme = options?.theme ?? 'dark';
  const activeSessionId = `${tabId}:${generateSessionToken()}`;
  const fontSize = 14;

  const containerEl =
    typeof document !== 'undefined'
      ? document.createElement('div')
      : ({} as HTMLElement);

  if (typeof document !== 'undefined' && containerEl.style) {
    containerEl.className = 'xterm-viewport-container';
    containerEl.style.width = '100%';
    containerEl.style.height = '100%';
    containerEl.style.minHeight = '0';
    containerEl.style.minWidth = '0';
    containerEl.style.overflow = 'hidden';
  }

  const instance = new Terminal({
    cursorBlink: true,
    cursorStyle: 'block',
    scrollback: 5000,
    allowProposedApi: true,
    allowTransparency: true,
    fontFamily: "'JetBrains Mono Nerd', ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
    fontSize,
    lineHeight: 1.35,
    smoothScrollDuration: 0,
    theme: buildXtermTheme(initialTheme)
  });

  const fit = new FitAddon();
  const search = new SearchAddon();
  const unicode11 = new Unicode11Addon();
  const webLinks = new WebLinksAddon((_event, uri) => {
    const localUrl = extractLocalhostUrl(uri);
    if (localUrl && session.onOpenLocalhost) {
      session.onOpenLocalhost(localUrl);
    } else {
      void openExternal(uri).catch(() => {});
    }
  });

  instance.loadAddon(fit);
  instance.loadAddon(search);
  instance.loadAddon(unicode11);
  instance.loadAddon(webLinks);
  instance.unicode.activeVersion = '11';

  if (typeof document !== 'undefined') {
    instance.open(containerEl);
  }

  let canvasAddon: CanvasAddon | null = null;
  try {
    canvasAddon = new CanvasAddon();
    instance.loadAddon(canvasAddon);
  } catch {
    canvasAddon = null;
  }

  const subscribers = new Set<() => void>();

  const session: RegisteredTerminalSession = {
    tabId,
    activeSessionId,
    term: instance,
    fitAddon: fit,
    searchAddon: search,
    canvasAddon,
    containerEl,
    exited: false,
    exitCode: null,
    errorMsg: null,
    detectedLocalUrl: null,
    fontSize,
    theme: initialTheme,
    restarting: false,
    delegate: undefined,
    onOpenLocalhost: options?.onOpenLocalhost,
    disposed: false,
    subscribe(fn: () => void) {
      subscribers.add(fn);
      return () => {
        subscribers.delete(fn);
      };
    },
    notify() {
      for (const fn of subscribers) {
        fn();
      }
    },
    setTheme(themeMode: 'dark' | 'light') {
      session.theme = themeMode;
      session.term.options.theme = buildXtermTheme(themeMode);
      session.canvasAddon?.clearTextureAtlas();
      try {
        session.term.refresh(0, Math.max(0, session.term.rows - 1));
      } catch {}
    },
    applyZoom(delta: number) {
      session.fontSize = clampTerminalFontSize(session.fontSize + delta);
      session.term.options.fontSize = session.fontSize;
      session.canvasAddon?.clearTextureAtlas();
      try {
        session.fitAddon.fit();
        if (typeof window !== 'undefined') {
          void ptyResize(session.activeSessionId, session.term.cols, session.term.rows).catch(() => {});
        }
      } catch {}
      session.delegate?.onZoom?.(delta);
      session.notify();
    },
    async restart() {
      if (session.restarting || session.disposed) return;
      session.restarting = true;
      session.notify();
      try {
        const oldSessionId = session.activeSessionId;
        session.activeSessionId = `${session.tabId}:${generateSessionToken()}`;
        if (typeof window !== 'undefined') {
          void ptyKill(oldSessionId).catch(() => {});
        }
        session.exited = false;
        session.exitCode = null;
        session.errorMsg = null;
        session.term.reset();
        try {
          session.fitAddon.fit();
        } catch {}
        await spawnShell();
      } finally {
        session.restarting = false;
        session.notify();
      }
    },
    async handlePaste(selection?: 'clipboard' | 'primary') {
      if (session.disposed) return;
      try {
        let text = '';
        try {
          text = await clipboardReadText(selection);
        } catch {
          text = '';
        }
        if (!text) {
          try {
            text = await navigator.clipboard.readText();
          } catch {
            text = '';
          }
        }
        if (!text) return;
        if (isLargePaste(text, 64)) {
          if (session.delegate?.onLargePaste) {
            session.delegate.onLargePaste(text);
          }
          return;
        }
        session.term.paste(text);
      } catch {
        return;
      }
    }
  };

  async function spawnShell() {
    if (typeof window === 'undefined') return;
    session.exited = false;
    session.exitCode = null;
    session.errorMsg = null;
    session.notify();
    try {
      await ptySpawn(session.activeSessionId, session.term.cols, session.term.rows);
      session.term.focus();
    } catch (err) {
      const normalized = err as AppErrorPayload;
      session.errorMsg = normalized.message ?? 'Failed to start terminal session';
      session.notify();
    }
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
        void clipboardWriteText(selected).catch(() => {});
        void navigator.clipboard.writeText(selected).catch(() => {});
        instance.clearSelection();
      }
      return false;
    }

    if (disposition === 'paste') {
      ev.preventDefault();
      void session.handlePaste('clipboard');
      return false;
    }

    if (disposition === 'app-shortcut') {
      const lower = ev.key.toLowerCase();
      if (lower === 's') {
        ev.preventDefault();
        session.delegate?.onToggleSearch?.();
        return false;
      }
      if (ev.key === '=' || ev.key === '+') {
        ev.preventDefault();
        session.applyZoom(1);
        return false;
      }
      if (ev.key === '-') {
        ev.preventDefault();
        session.applyZoom(-1);
        return false;
      }
      return false;
    }

    return true;
  });

  instance.onData((data) => {
    if (!session.exited && !session.disposed && typeof window !== 'undefined') {
      void ptyWrite(session.activeSessionId, data).catch(() => {});
    }
  });

  if (typeof document !== 'undefined' && 'fonts' in document) {
    document.fonts.ready.then(() => {
      if (session.disposed) return;
      session.canvasAddon?.clearTextureAtlas();
      try {
        session.term.refresh(0, Math.max(0, session.term.rows - 1));
        session.fitAddon.fit();
      } catch {}
    }).catch(() => {});
  }

  if (typeof window !== 'undefined') {
    void onBackendEvent('pty:output', (payload) => {
      if (session.disposed) return;
      if (payload.sessionId === session.activeSessionId) {
        session.term.write(payload.data);
        if (
          payload.data.length <= 2048 &&
          (payload.data.includes('localhost') || payload.data.includes('127.0.0.1'))
        ) {
          const found = extractLocalhostUrl(payload.data);
          if (found) {
            session.detectedLocalUrl = found;
            session.notify();
          }
        }
      }
    }).then((unlisten) => {
      if (session.disposed) {
        unlisten();
      } else {
        session.unlistenOutput = unlisten;
      }
    });

    void onBackendEvent('pty:exit', (payload) => {
      if (session.disposed) return;
      if (payload.sessionId === session.activeSessionId) {
        session.exited = true;
        session.exitCode = payload.code;
        session.notify();
      }
    }).then((unlisten) => {
      if (session.disposed) {
        unlisten();
      } else {
        session.unlistenExit = unlisten;
      }
    });

    void spawnShell();
  }

  return session;
}

const sessionRegistry = new Map<string, RegisteredTerminalSession>();

export function getTerminalSession(tabId: string): RegisteredTerminalSession | undefined {
  return sessionRegistry.get(tabId);
}

export function getOrCreateTerminalSession(
  tabId: string,
  options?: { theme?: 'dark' | 'light'; onOpenLocalhost?: (url: string) => void }
): RegisteredTerminalSession {
  const existing = sessionRegistry.get(tabId);
  if (existing && !existing.disposed) {
    if (options?.theme) {
      existing.setTheme(options.theme);
    }
    if (options?.onOpenLocalhost) {
      existing.onOpenLocalhost = options.onOpenLocalhost;
    }
    return existing;
  }
  const created = createTerminalSessionInstance(tabId, options);
  sessionRegistry.set(tabId, created);
  return created;
}

export async function disposeTerminalSession(tabId: string): Promise<void> {
  const session = sessionRegistry.get(tabId);
  if (!session) return;
  sessionRegistry.delete(tabId);
  session.disposed = true;
  session.unlistenOutput?.();
  session.unlistenExit?.();
  if (typeof window !== 'undefined') {
    void ptyKill(session.activeSessionId).catch(() => {});
  }
  if (typeof session.containerEl?.remove === 'function') {
    session.containerEl.remove();
  }
  session.canvasAddon?.dispose();
  session.term.dispose();
}

export async function disposeAllTerminalSessions(): Promise<void> {
  const ids = Array.from(sessionRegistry.keys());
  for (const id of ids) {
    await disposeTerminalSession(id);
  }
}
