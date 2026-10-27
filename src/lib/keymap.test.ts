import { describe, expect, it } from 'vitest';
import { createDefaultSettings } from '$lib/ipc';
import {
  filterPaletteEntries,
  matchesKeyCombo,
  resolveGlobalShortcut,
  type PaletteEntry
} from './keymap';

describe('focus-based keymap and command palette', () => {
  it('matches key combos and blocks Ctrl shortcuts when terminal is focused', () => {
    const kb = createDefaultSettings().keybindings;

    expect(
      matchesKeyCombo(
        { key: 'p', altKey: true, ctrlKey: false, shiftKey: false, metaKey: false },
        'Alt+P'
      )
    ).toBe(true);

    expect(
      resolveGlobalShortcut(
        { key: 'p', altKey: true, ctrlKey: false, shiftKey: false, metaKey: false },
        true,
        kb
      )
    ).toBe('command_palette');

    expect(
      resolveGlobalShortcut(
        { key: 'p', altKey: false, ctrlKey: true, shiftKey: false, metaKey: false },
        true,
        kb
      )
    ).toBeNull();

    expect(
      resolveGlobalShortcut(
        { key: 'p', altKey: false, ctrlKey: true, shiftKey: false, metaKey: false },
        false,
        kb
      )
    ).toBe('quick_open');
  });

  it('resolves directional pane navigation, workspace switching, and zoom', () => {
    const kb = createDefaultSettings().keybindings;

    expect(
      resolveGlobalShortcut(
        { key: 'ArrowRight', altKey: true, ctrlKey: false, shiftKey: false, metaKey: false },
        true,
        kb
      )
    ).toBe('pane_right');

    expect(
      resolveGlobalShortcut(
        { key: '2', altKey: true, ctrlKey: false, shiftKey: false, metaKey: false },
        true,
        kb
      )
    ).toEqual({ type: 'switch_workspace', index: 1 });

    expect(
      resolveGlobalShortcut(
        { key: '=', altKey: true, ctrlKey: false, shiftKey: false, metaKey: false },
        false,
        kb
      )
    ).toBe('zoom_in');
  });

  it('filters command palette entries by label and keywords', () => {
    const noop = () => {};
    const entries: PaletteEntry[] = [
      { id: '1', label: 'Split right', category: 'Action', run: noop },
      { id: '2', label: 'src/main.rs', category: 'File', run: noop },
      { id: '3', label: 'Switch to workspace Agent', category: 'Workspace', run: noop }
    ];

    const filtered = filterPaletteEntries(entries, 'split');
    expect(filtered).toHaveLength(1);
    expect(filtered[0].id).toBe('1');
  });
});
