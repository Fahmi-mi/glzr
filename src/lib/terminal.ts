import type { ITheme } from '@xterm/xterm';
import { normalizePreviewUrl } from './preview';

export interface TerminalKeyInput {
  key: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
  hasSelection: boolean;
}

export type TerminalKeyDisposition = 'copy' | 'paste' | 'app-shortcut' | 'pass-to-pty';

export function resolveTerminalKey(input: TerminalKeyInput): TerminalKeyDisposition {
  if (input.altKey && !input.ctrlKey && !input.metaKey) {
    return 'app-shortcut';
  }

  if (input.ctrlKey && input.shiftKey && input.key.toLowerCase() === 'v') {
    return 'paste';
  }

  if (input.ctrlKey && !input.shiftKey && !input.altKey && input.key.toLowerCase() === 'c') {
    return input.hasSelection ? 'copy' : 'pass-to-pty';
  }

  return 'pass-to-pty';
}

export function isLargePaste(text: string, confirmOverKb = 64): boolean {
  const byteLength = new TextEncoder().encode(text).byteLength;
  return byteLength > confirmOverKb * 1024;
}

export function clampTerminalFontSize(size: number, min = 10, max = 28): number {
  if (!Number.isFinite(size)) return 14;
  return Math.max(min, Math.min(max, Math.round(size)));
}

export function extractLocalhostUrl(rawUrl: string): string | null {
  return normalizePreviewUrl(rawUrl);
}

export function getTerminalTheme(mode: 'dark' | 'light'): ITheme {
  if (mode === 'light') {
    return {
      background: '#fbfbfc',
      foreground: '#1b1d22',
      cursor: '#3b6ef0',
      cursorAccent: '#ffffff',
      selectionBackground: 'rgba(59, 110, 240, 0.20)',
      selectionForeground: '#1b1d22',
      black: '#1b1d22',
      red: '#cf222e',
      green: '#1a7f37',
      yellow: '#9a6700',
      blue: '#0969da',
      magenta: '#8250df',
      cyan: '#1b7c83',
      white: '#6e7781',
      brightBlack: '#57606a',
      brightRed: '#a40e26',
      brightGreen: '#116329',
      brightYellow: '#7d4e00',
      brightBlue: '#0550ae',
      brightMagenta: '#6639ba',
      brightCyan: '#116065',
      brightWhite: '#24292f'
    };
  }

  return {
    background: '#121316',
    foreground: '#e6e7ea',
    cursor: '#78a9ff',
    cursorAccent: '#121316',
    selectionBackground: 'rgba(120, 169, 255, 0.24)',
    selectionForeground: '#ffffff',
    black: '#1c1d22',
    red: '#e06c75',
    green: '#4ec994',
    yellow: '#e5a54b',
    blue: '#78a9ff',
    magenta: '#c678dd',
    cyan: '#56b6c2',
    white: '#e6e7ea',
    brightBlack: '#5c6370',
    brightRed: '#e06c75',
    brightGreen: '#4ec994',
    brightYellow: '#e5a54b',
    brightBlue: '#78a9ff',
    brightMagenta: '#c678dd',
    brightCyan: '#56b6c2',
    brightWhite: '#ffffff'
  };
}
