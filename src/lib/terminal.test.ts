import { describe, expect, it } from 'vitest';
import {
  clampTerminalFontSize,
  extractLocalhostUrl,
  getTerminalTheme,
  isLargePaste,
  resolveTerminalKey
} from './terminal';

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

  it('checks large paste threshold, font zoom bounds, and localhost links', () => {
    expect(isLargePaste('short text', 1)).toBe(false);
    expect(isLargePaste('x'.repeat(2048), 1)).toBe(true);

    expect(clampTerminalFontSize(14)).toBe(14);
    expect(clampTerminalFontSize(6)).toBe(10);
    expect(clampTerminalFontSize(40)).toBe(28);

    expect(extractLocalhostUrl('http://localhost:5173/')).toBe('http://localhost:5173/');
    expect(extractLocalhostUrl('https://example.com')).toBeNull();
  });

  it('provides distinct palettes for dark and light terminal themes', () => {
    const dark = getTerminalTheme('dark');
    const light = getTerminalTheme('light');

    expect(dark.background).toBe('#121316');
    expect(dark.foreground).toBe('#e6e7ea');

    expect(light.background).toBe('#fbfbfc');
    expect(light.foreground).toBe('#1b1d22');

    expect(dark.background).not.toBe(light.background);
    expect(dark.foreground).not.toBe(light.foreground);
  });
});
