import { describe, expect, it } from 'vitest';
import { resolveTerminalKey } from './terminal';

describe('terminal key handling spike', () => {
  it('copies on Ctrl+C only when text is selected and passes SIGINT otherwise', () => {
    expect(
      resolveTerminalKey({
        key: 'c',
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        hasSelection: true
      })
    ).toBe('copy');

    expect(
      resolveTerminalKey({
        key: 'c',
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        hasSelection: false
      })
    ).toBe('pass-to-pty');
  });

  it('passes Ctrl+B prefix to pty for herdr and tmux while capturing Alt shortcuts', () => {
    expect(
      resolveTerminalKey({
        key: 'b',
        ctrlKey: true,
        shiftKey: false,
        altKey: false,
        metaKey: false,
        hasSelection: false
      })
    ).toBe('pass-to-pty');

    expect(
      resolveTerminalKey({
        key: 'p',
        ctrlKey: false,
        shiftKey: false,
        altKey: true,
        metaKey: false,
        hasSelection: false
      })
    ).toBe('app-shortcut');
  });

  it('triggers paste on Ctrl+Shift+V', () => {
    expect(
      resolveTerminalKey({
        key: 'V',
        ctrlKey: true,
        shiftKey: true,
        altKey: false,
        metaKey: false,
        hasSelection: false
      })
    ).toBe('paste');
  });
});
