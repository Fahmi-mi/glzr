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
