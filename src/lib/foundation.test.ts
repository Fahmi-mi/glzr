import { describe, expect, it } from 'vitest';
import { normalizeIpcError, projectGetState } from './ipc';
import { createInitialSession, createInitialWorkspace } from './layout/types';

describe('core foundation types and ipc error normalization', () => {
  it('creates initial workspace with one pane and one terminal tab', () => {
    const ws = createInitialWorkspace('ws-1', '1 Main', '/home/user/project');
    expect(ws.id).toBe('ws-1');
    expect(ws.root.type).toBe('pane');
    if (ws.root.type === 'pane') {
      expect(ws.root.tabs).toHaveLength(1);
      expect(ws.root.tabs[0].kind).toBe('terminal');
      expect(ws.root.tabs[0].data.cwd).toBe('/home/user/project');
      expect(ws.root.activeTabId).toBe(ws.root.tabs[0].id);
    }
  });

  it('creates initial session with version 1 and visible files sidebar', () => {
    const session = createInitialSession('/home/user/project');
    expect(session.version).toBe(1);
    expect(session.folder).toBe('/home/user/project');
    expect(session.sidebar).toEqual({
      visible: true,
      panel: 'files',
      width: 264
    });
    expect(session.workspaces).toHaveLength(1);
    expect(session.workspaces[0].name).toBe('1');
  });

  it('creates default workspace with name 1 when name argument is omitted', () => {
    const ws = createInitialWorkspace();
    expect(ws.name).toBe('1');
  });

  it('normalizes structured and standard ipc errors', () => {
    expect(normalizeIpcError({ kind: 'not_found', message: 'missing dir' })).toEqual({
      kind: 'not_found',
      message: 'missing dir'
    });
    expect(normalizeIpcError(new Error('network failure'))).toEqual({
      kind: 'error',
      message: 'network failure'
    });
  });

  it('handles invocation outside tauri environment gracefully with normalized error', async () => {
    await expect(projectGetState()).rejects.toMatchObject({
      kind: expect.any(String),
      message: expect.any(String)
    });
  });
});
