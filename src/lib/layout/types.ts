export type TabKind = 'file' | 'terminal' | 'preview' | 'diff';

export interface Tab {
  id: string;
  kind: TabKind;
  title: string;
  data: Record<string, unknown>;
}

export interface Pane {
  type: 'pane';
  id: string;
  tabs: Tab[];
  activeTabId: string | null;
}

export interface Split {
  type: 'split';
  direction: 'row' | 'column';
  ratio: number;
  first: LayoutNode;
  second: LayoutNode;
}

export type LayoutNode = Pane | Split;

export interface Workspace {
  id: string;
  name: string;
  root: LayoutNode;
}

export interface SidebarState {
  visible: boolean;
  panel: 'files' | 'git' | 'search';
  width: number;
}

export interface SessionFile {
  version: number;
  folder: string | null;
  sidebar: SidebarState;
  activeWorkspaceId: string;
  workspaces: Workspace[];
}

export function createInitialWorkspace(
  workspaceId = 'ws-1',
  name = '1',
  cwd = ''
): Workspace {
  const tabId = `${workspaceId}-tab-1`;
  const paneId = `${workspaceId}-pane-1`;

  return {
    id: workspaceId,
    name,
    root: {
      type: 'pane',
      id: paneId,
      activeTabId: tabId,
      tabs: [
        {
          id: tabId,
          kind: 'terminal',
          title: 'shell',
          data: { cwd }
        }
      ]
    }
  };
}

export function createInitialSession(folder: string | null = null): SessionFile {
  const initialWorkspace = createInitialWorkspace('ws-1', '1', folder ?? '');
  return {
    version: 1,
    folder,
    sidebar: {
      visible: true,
      panel: 'files',
      width: 264
    },
    activeWorkspaceId: initialWorkspace.id,
    workspaces: [initialWorkspace]
  };
}
