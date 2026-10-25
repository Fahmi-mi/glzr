import { describe, expect, it } from 'vitest';
import {
  closePane,
  findDirectionalNeighborPane,
  listAllPanes,
  migrateAndSanitizeSession,
  moveTabBetweenPanes,
  splitPane,
  updateSplitRatioByPath
} from './tree';
import { createInitialWorkspace, type Tab } from './types';

describe('layout tree pure operations and session migration', () => {
  it('splits a pane, updates split ratio, and collapses split when a pane is closed', () => {
    const ws = createInitialWorkspace('ws-1', '1 Utama', '/tmp/proj');
    const newTab: Tab = {
      id: 'tab-2',
      kind: 'terminal',
      title: 'shell',
      data: { cwd: '/tmp/proj' }
    };

    const splitRoot = splitPane(ws.root, 'ws-1-pane-1', 'row', 'ws-1-pane-2', newTab);
    expect(splitRoot.type).toBe('split');
    expect(listAllPanes(splitRoot)).toHaveLength(2);

    const resized = updateSplitRatioByPath(splitRoot, [], 0.65);
    expect(resized.type === 'split' && resized.ratio).toBe(0.65);

    const clamped = updateSplitRatioByPath(splitRoot, [], 0.99);
    expect(clamped.type === 'split' && clamped.ratio).toBe(0.85);

    const closed = closePane(resized, 'ws-1-pane-2');
    expect(closed).not.toBeNull();
    expect(closed?.root.type).toBe('pane');
    expect(closed?.nextFocusPaneId).toBe('ws-1-pane-1');
  });

  it('moves tabs between panes and resolves directional neighbor panes', () => {
    const ws = createInitialWorkspace('ws-1', '1 Utama', '/tmp/proj');
    const tab2: Tab = {
      id: 'tab-2',
      kind: 'file',
      title: 'main.rs',
      data: { path: '/tmp/proj/src/main.rs' }
    };
    const tab3: Tab = {
      id: 'tab-3',
      kind: 'terminal',
      title: 'shell',
      data: { cwd: '/tmp/proj' }
    };

    let root = splitPane(ws.root, 'ws-1-pane-1', 'row', 'ws-1-pane-2', tab2);
    root = splitPane(root, 'ws-1-pane-2', 'column', 'ws-1-pane-3', tab3);

    expect(findDirectionalNeighborPane(root, 'ws-1-pane-1', 'right')).toBe('ws-1-pane-2');
    expect(findDirectionalNeighborPane(root, 'ws-1-pane-2', 'down')).toBe('ws-1-pane-3');
    expect(findDirectionalNeighborPane(root, 'ws-1-pane-3', 'up')).toBe('ws-1-pane-2');
    expect(findDirectionalNeighborPane(root, 'ws-1-pane-2', 'left')).toBe('ws-1-pane-1');

    // Move tab-3 from pane-3 into pane-1; pane-3 becomes empty and collapses
    const afterMove = moveTabBetweenPanes(root, 'ws-1-pane-3', 'tab-3', 'ws-1-pane-1');
    const panes = listAllPanes(afterMove);
    expect(panes.map((p) => p.id)).toEqual(['ws-1-pane-1', 'ws-1-pane-2']);
    expect(panes[0].tabs.map((t) => t.id)).toEqual(['ws-1-tab-1', 'tab-3']);
  });

  it('sanitizes and migrates v1 session payloads and repairs broken nodes', () => {
    const fixtureV1 = {
      version: 1,
      folder: '/tmp/proj',
      sidebar: { visible: false, panel: 'git', width: 300 },
      activeWorkspaceId: 'ws-custom',
      workspaces: [
        {
          id: 'ws-custom',
          name: 'Backend',
          root: {
            type: 'split',
            direction: 'row',
            ratio: 0.4,
            first: {
              type: 'pane',
              id: 'p-left',
              activeTabId: 't-file',
              tabs: [
                { id: 't-file', kind: 'file', title: 'lib.rs', data: { path: '/tmp/proj/lib.rs' } },
                { id: 't-bad', kind: 'file', title: 'broken', data: {} }
              ]
            },
            second: {
              type: 'pane',
              id: 'p-right',
              activeTabId: 'missing',
              tabs: []
            }
          }
        }
      ]
    };

    const sanitized = migrateAndSanitizeSession(fixtureV1, '/tmp/fallback');
    expect(sanitized.version).toBe(1);
    expect(sanitized.folder).toBe('/tmp/proj');
    expect(sanitized.sidebar.visible).toBe(false);
    expect(sanitized.sidebar.panel).toBe('git');
    const panes = listAllPanes(sanitized.workspaces[0].root);
    expect(panes).toHaveLength(2);
    expect(panes[0].tabs).toHaveLength(1);
    expect(panes[0].tabs[0].id).toBe('t-file');
    expect(panes[1].tabs).toHaveLength(1);
    expect(panes[1].tabs[0].kind).toBe('terminal');
  });
});
