import { createDefaultSettings, type KeybindingsSettingsDto } from '$lib/ipc';

export interface KeyEventInput {
  key: string;
  altKey: boolean;
  ctrlKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

export type GlobalShortcutAction =
  | 'command_palette'
  | 'quick_open'
  | 'sidebar_files'
  | 'sidebar_git'
  | 'sidebar_search'
  | 'new_terminal'
  | 'close_tab'
  | 'split_right'
  | 'split_down'
  | 'close_pane'
  | 'maximize_pane'
  | 'pane_left'
  | 'pane_right'
  | 'pane_up'
  | 'pane_down'
  | 'next_tab'
  | 'prev_tab'
  | 'zoom_in'
  | 'zoom_out'
  | 'zoom_reset'
  | { type: 'switch_workspace'; index: number };

export function matchesKeyCombo(event: KeyEventInput, combo: string): boolean {
  const parts = combo
    .split('+')
    .map((p) => p.trim().toLowerCase())
    .filter(Boolean);
  if (parts.length === 0) return false;

  const targetKey = parts[parts.length - 1];
  const modifiers = new Set(parts.slice(0, -1));

  const wantAlt = modifiers.has('alt');
  const wantCtrl = modifiers.has('ctrl') || modifiers.has('control') || modifiers.has('cmd');
  const wantShift = modifiers.has('shift');

  const hasCtrlOrMeta = event.ctrlKey || event.metaKey;
  if (event.altKey !== wantAlt) return false;
  if (hasCtrlOrMeta !== wantCtrl) return false;
  if (event.shiftKey !== wantShift) return false;

  return event.key.toLowerCase() === targetKey;
}

export function resolveGlobalShortcut(
  event: KeyEventInput,
  terminalFocused: boolean,
  keybindings: KeybindingsSettingsDto = createDefaultSettings().keybindings
): GlobalShortcutAction | null {
  if (terminalFocused && (event.ctrlKey || event.metaKey) && !event.altKey) {
    return null;
  }

  const actionEntries: [GlobalShortcutAction, string][] = [
    ['command_palette', keybindings.commandPalette],
    ['quick_open', keybindings.quickOpen],
    ['sidebar_files', keybindings.sidebarFiles],
    ['sidebar_git', keybindings.sidebarGit],
    ['sidebar_search', keybindings.sidebarSearch],
    ['new_terminal', keybindings.newTerminal],
    ['close_tab', keybindings.closeTab],
    ['split_right', keybindings.splitRight],
    ['split_down', keybindings.splitDown],
    ['close_pane', keybindings.closePane],
    ['maximize_pane', keybindings.maximizePane]
  ];

  for (const [action, combo] of actionEntries) {
    if (matchesKeyCombo(event, combo)) {
      return action;
    }
  }

  if (!terminalFocused) {
    if (matchesKeyCombo(event, 'Ctrl+Shift+P')) {
      return 'command_palette';
    }
    if (matchesKeyCombo(event, 'Ctrl+P')) {
      return 'quick_open';
    }
    if (matchesKeyCombo(event, 'Ctrl+W')) {
      return 'close_tab';
    }
  }

  const altOnly = event.altKey && !event.ctrlKey && !event.metaKey;
  if (!altOnly) return null;

  if (event.key === 'ArrowLeft') return 'pane_left';
  if (event.key === 'ArrowRight') return 'pane_right';
  if (event.key === 'ArrowUp') return 'pane_up';
  if (event.key === 'ArrowDown') return 'pane_down';

  if (event.key === 'PageDown') return 'next_tab';
  if (event.key === 'PageUp') return 'prev_tab';

  if (event.key === '=' || event.key === '+') return 'zoom_in';
  if (event.key === '-') return 'zoom_out';
  if (event.key === '0') return 'zoom_reset';

  if (/^[1-9]$/.test(event.key) && !event.shiftKey) {
    return {
      type: 'switch_workspace',
      index: Number.parseInt(event.key, 10) - 1
    };
  }

  return null;
}

export interface PaletteEntry {
  id: string;
  label: string;
  category: 'File' | 'Action' | 'Workspace';
  shortcut?: string;
  keywords?: string;
  run: () => void;
}

export function filterPaletteEntries(entries: PaletteEntry[], query: string): PaletteEntry[] {
  const trimmed = query.trim().toLowerCase();
  if (!trimmed) {
    return entries.slice(0, 60);
  }

  const scored: { entry: PaletteEntry; score: number }[] = [];
  for (const entry of entries) {
    const labelLower = entry.label.toLowerCase();
    const keywordsLower = (entry.keywords ?? '').toLowerCase();
    const idx = labelLower.indexOf(trimmed);
    const kwIdx = keywordsLower.indexOf(trimmed);

    if (idx !== -1) {
      let score = idx * 2 + labelLower.length * 0.1;
      if (entry.category === 'Action') score -= 4;
      scored.push({ entry, score });
    } else if (kwIdx !== -1) {
      scored.push({ entry, score: 40 + kwIdx });
    }
  }

  scored.sort((a, b) => a.score - b.score);
  return scored.slice(0, 60).map((s) => s.entry);
}
