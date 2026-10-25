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
