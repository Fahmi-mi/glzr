import {
  createInitialSession,
  createInitialWorkspace,
  type LayoutNode,
  type Pane,
  type SessionFile,
  type SidebarState,
  type Split,
  type Tab,
  type TabKind,
  type Workspace
} from './types';

export const MIN_SPLIT_RATIO = 0.15;
export const MAX_SPLIT_RATIO = 0.85;

export function clampSplitRatio(ratio: number): number {
  if (!Number.isFinite(ratio)) return 0.5;
  return Math.max(MIN_SPLIT_RATIO, Math.min(MAX_SPLIT_RATIO, ratio));
}

export function listAllPanes(node: LayoutNode): Pane[] {
  if (node.type === 'pane') {
    return [node];
  }
  return [...listAllPanes(node.first), ...listAllPanes(node.second)];
}

export function findPaneById(node: LayoutNode, paneId: string): Pane | null {
  if (node.type === 'pane') {
    return node.id === paneId ? node : null;
  }
  return findPaneById(node.first, paneId) ?? findPaneById(node.second, paneId);
}

export function getFirstPaneId(node: LayoutNode): string {
  if (node.type === 'pane') {
    return node.id;
  }
  return getFirstPaneId(node.first);
}

export function splitPane(
  node: LayoutNode,
  targetPaneId: string,
  direction: 'row' | 'column',
  newPaneId: string,
  initialTab?: Tab
): LayoutNode {
  if (node.type === 'pane') {
    if (node.id !== targetPaneId) {
      return node;
    }
    const secondPane: Pane = {
      type: 'pane',
      id: newPaneId,
      tabs: initialTab ? [initialTab] : [],
      activeTabId: initialTab ? initialTab.id : null
    };
    const splitNode: Split = {
      type: 'split',
      direction,
      ratio: 0.5,
      first: node,
      second: secondPane
    };
    return splitNode;
  }

  return {
    ...node,
    first: splitPane(node.first, targetPaneId, direction, newPaneId, initialTab),
    second: splitPane(node.second, targetPaneId, direction, newPaneId, initialTab)
  };
}

export interface ClosePaneResult {
  root: LayoutNode;
  closedTabs: Tab[];
  nextFocusPaneId: string;
}

export function closePane(node: LayoutNode, targetPaneId: string): ClosePaneResult | null {
  if (node.type === 'pane') {
    return null;
  }

  const removed = removePaneRecursive(node, targetPaneId);
  if (!removed.removedPane || !removed.nextNode) {
    return null;
  }

  return {
    root: removed.nextNode,
    closedTabs: removed.removedPane.tabs,
    nextFocusPaneId: removed.fallbackPaneId ?? getFirstPaneId(removed.nextNode)
  };
}

function removePaneRecursive(
  node: LayoutNode,
  targetPaneId: string
): {
  nextNode: LayoutNode | null;
  removedPane: Pane | null;
  fallbackPaneId: string | null;
} {
  if (node.type === 'pane') {
    if (node.id === targetPaneId) {
      return { nextNode: null, removedPane: node, fallbackPaneId: null };
    }
    return { nextNode: node, removedPane: null, fallbackPaneId: null };
  }

  const leftRes = removePaneRecursive(node.first, targetPaneId);
  if (leftRes.removedPane) {
    if (leftRes.nextNode === null) {
      return {
        nextNode: node.second,
        removedPane: leftRes.removedPane,
        fallbackPaneId: getFirstPaneId(node.second)
      };
    }
    return {
      nextNode: { ...node, first: leftRes.nextNode },
      removedPane: leftRes.removedPane,
      fallbackPaneId: leftRes.fallbackPaneId
    };
  }

  const rightRes = removePaneRecursive(node.second, targetPaneId);
  if (rightRes.removedPane) {
    if (rightRes.nextNode === null) {
      return {
        nextNode: node.first,
        removedPane: rightRes.removedPane,
        fallbackPaneId: getFirstPaneId(node.first)
      };
    }
    return {
      nextNode: { ...node, second: rightRes.nextNode },
      removedPane: rightRes.removedPane,
      fallbackPaneId: rightRes.fallbackPaneId
    };
  }

  return { nextNode: node, removedPane: null, fallbackPaneId: null };
}

export function updateSplitRatioByPath(
  node: LayoutNode,
  path: readonly ('first' | 'second')[],
  ratio: number
): LayoutNode {
  if (node.type === 'pane') {
    return node;
  }
  if (path.length === 0) {
    return {
      ...node,
      ratio: clampSplitRatio(ratio)
    };
  }
  const [head, ...rest] = path;
  if (head === 'first') {
    return {
      ...node,
      first: updateSplitRatioByPath(node.first, rest, ratio)
    };
  }
  return {
    ...node,
    second: updateSplitRatioByPath(node.second, rest, ratio)
  };
}

export function moveTabBetweenPanes(
  root: LayoutNode,
  fromPaneId: string,
  tabId: string,
  toPaneId: string,
  targetIndex?: number
): LayoutNode {
  const sourcePane = findPaneById(root, fromPaneId);
  const destPane = findPaneById(root, toPaneId);
  if (!sourcePane || !destPane) {
    return root;
  }

  const tabIdx = sourcePane.tabs.findIndex((t) => t.id === tabId);
  if (tabIdx === -1) {
    return root;
  }
  const movingTab = sourcePane.tabs[tabIdx];

  if (fromPaneId === toPaneId) {
    const clonedTabs = [...sourcePane.tabs];
    clonedTabs.splice(tabIdx, 1);
    const insertAt =
      targetIndex !== undefined
        ? Math.max(0, Math.min(clonedTabs.length, targetIndex))
        : clonedTabs.length;
    clonedTabs.splice(insertAt, 0, movingTab);
    return replacePaneTabs(root, fromPaneId, clonedTabs, movingTab.id);
  }

  const nextSourceTabs = sourcePane.tabs.filter((t) => t.id !== tabId);
  const nextSourceActive =
    sourcePane.activeTabId === tabId
      ? (nextSourceTabs[Math.min(tabIdx, nextSourceTabs.length - 1)]?.id ?? null)
      : sourcePane.activeTabId;

  const nextDestTabs = [...destPane.tabs];
  const insertAt =
    targetIndex !== undefined
      ? Math.max(0, Math.min(nextDestTabs.length, targetIndex))
      : nextDestTabs.length;
  nextDestTabs.splice(insertAt, 0, movingTab);

  let updated = replacePaneTabs(root, fromPaneId, nextSourceTabs, nextSourceActive);
  updated = replacePaneTabs(updated, toPaneId, nextDestTabs, movingTab.id);

  if (nextSourceTabs.length === 0) {
    const collapsed = closePane(updated, fromPaneId);
    if (collapsed) {
      return collapsed.root;
    }
  }

  return updated;
}

function replacePaneTabs(
  node: LayoutNode,
  paneId: string,
  tabs: Tab[],
  activeTabId: string | null
): LayoutNode {
  if (node.type === 'pane') {
    if (node.id !== paneId) return node;
    return {
      ...node,
      tabs,
      activeTabId
    };
  }
  return {
    ...node,
    first: replacePaneTabs(node.first, paneId, tabs, activeTabId),
    second: replacePaneTabs(node.second, paneId, tabs, activeTabId)
  };
}

interface PaneBox {
  id: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

function collectPaneBoxes(
  node: LayoutNode,
  x = 0,
  y = 0,
  w = 1,
  h = 1,
  out: PaneBox[] = []
): PaneBox[] {
  if (node.type === 'pane') {
    out.push({ id: node.id, x, y, w, h });
    return out;
  }

  const r = clampSplitRatio(node.ratio);
  if (node.direction === 'row') {
    const w1 = w * r;
    collectPaneBoxes(node.first, x, y, w1, h, out);
    collectPaneBoxes(node.second, x + w1, y, w - w1, h, out);
  } else {
    const h1 = h * r;
    collectPaneBoxes(node.first, x, y, w, h1, out);
    collectPaneBoxes(node.second, x, y + h1, w, h - h1, out);
  }
  return out;
}

export function findDirectionalNeighborPane(
  root: LayoutNode,
  currentPaneId: string,
  direction: 'left' | 'right' | 'up' | 'down'
): string | null {
  const boxes = collectPaneBoxes(root);
  const current = boxes.find((b) => b.id === currentPaneId);
  if (!current) return null;

  const cx = current.x + current.w / 2;
  const cy = current.y + current.h / 2;

  let bestId: string | null = null;
  let bestScore = Number.POSITIVE_INFINITY;

  for (const candidate of boxes) {
    if (candidate.id === currentPaneId) continue;
    const tx = candidate.x + candidate.w / 2;
    const ty = candidate.y + candidate.h / 2;
    const dx = tx - cx;
    const dy = ty - cy;

    if (direction === 'left' && dx >= -1e-4) continue;
    if (direction === 'right' && dx <= 1e-4) continue;
    if (direction === 'up' && dy >= -1e-4) continue;
    if (direction === 'down' && dy <= 1e-4) continue;

    const primaryDist = direction === 'left' || direction === 'right' ? Math.abs(dx) : Math.abs(dy);
    const secondaryDist =
      direction === 'left' || direction === 'right' ? Math.abs(dy) : Math.abs(dx);
    const score = primaryDist + secondaryDist * 2;

    if (score < bestScore) {
      bestScore = score;
      bestId = candidate.id;
    }
  }

  return bestId;
}

function migrateSessionV1(raw: Record<string, unknown>): Record<string, unknown> {
  return raw;
}

function isValidTabKind(kind: unknown): kind is TabKind {
  return kind === 'file' || kind === 'terminal' || kind === 'preview' || kind === 'diff';
}

function sanitizeTab(raw: unknown, fallbackCwd: string, idx: number): Tab | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const obj = raw as Record<string, unknown>;
  if (!isValidTabKind(obj.kind)) return null;

  const id = typeof obj.id === 'string' && obj.id.trim() ? obj.id : `restored-tab-${idx}`;
  const title =
    typeof obj.title === 'string' && obj.title.trim()
      ? obj.title
      : obj.kind === 'terminal'
        ? 'shell'
        : obj.kind;
  const data =
    typeof obj.data === 'object' && obj.data !== null
      ? { ...(obj.data as Record<string, unknown>) }
      : {};

  if (obj.kind === 'terminal') {
    data.cwd = fallbackCwd;
  } else if (obj.kind === 'file' && typeof data.path !== 'string') {
    return null;
  } else if (obj.kind === 'diff' && typeof data.relPath !== 'string') {
    return null;
  }

  return { id, kind: obj.kind, title, data };
}

function sanitizeLayoutNode(
  raw: unknown,
  fallbackCwd: string,
  counter: { pane: number; tab: number }
): LayoutNode | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const obj = raw as Record<string, unknown>;

  if (obj.type === 'pane') {
    counter.pane += 1;
    const id =
      typeof obj.id === 'string' && obj.id.trim() ? obj.id : `restored-pane-${counter.pane}`;
    const rawTabs = Array.isArray(obj.tabs) ? obj.tabs : [];
    const tabs: Tab[] = [];
    for (const item of rawTabs) {
      counter.tab += 1;
      const clean = sanitizeTab(item, fallbackCwd, counter.tab);
      if (clean) tabs.push(clean);
    }
    if (tabs.length === 0) {
      counter.tab += 1;
      const defaultTabId = `${id}-tab-${counter.tab}`;
      tabs.push({
        id: defaultTabId,
        kind: 'terminal',
        title: 'shell',
        data: { cwd: fallbackCwd }
      });
    }
    const requestedActive = typeof obj.activeTabId === 'string' ? obj.activeTabId : null;
    const activeTabId = tabs.some((t) => t.id === requestedActive) ? requestedActive : tabs[0].id;
    return {
      type: 'pane',
      id,
      tabs,
      activeTabId
    };
  }

  if (obj.type === 'split') {
    const direction = obj.direction === 'column' ? 'column' : 'row';
    const ratio = typeof obj.ratio === 'number' ? clampSplitRatio(obj.ratio) : 0.5;
    const first = sanitizeLayoutNode(obj.first, fallbackCwd, counter);
    const second = sanitizeLayoutNode(obj.second, fallbackCwd, counter);
    if (first && second) {
      return { type: 'split', direction, ratio, first, second };
    }
    return first ?? second;
  }

  return null;
}

export function migrateAndSanitizeSession(
  rawInput: unknown,
  fallbackFolder: string | null
): SessionFile {
  if (typeof rawInput !== 'object' || rawInput === null) {
    return createInitialSession(fallbackFolder);
  }

  let current = rawInput as Record<string, unknown>;
  const version = typeof current.version === 'number' ? current.version : 1;
  if (version === 1) {
    current = migrateSessionV1(current);
  } else {
    return createInitialSession(fallbackFolder);
  }

  const folder =
    typeof current.folder === 'string' && current.folder.trim() ? current.folder : fallbackFolder;

  const rawSidebar =
    typeof current.sidebar === 'object' && current.sidebar !== null
      ? (current.sidebar as Record<string, unknown>)
      : {};
  const sidebar: SidebarState = {
    visible: typeof rawSidebar.visible === 'boolean' ? rawSidebar.visible : true,
    panel:
      rawSidebar.panel === 'git' || rawSidebar.panel === 'search' || rawSidebar.panel === 'files'
        ? rawSidebar.panel
        : 'files',
    width:
      typeof rawSidebar.width === 'number' && rawSidebar.width >= 180 && rawSidebar.width <= 480
        ? rawSidebar.width
        : 264
  };

  const counter = { pane: 0, tab: 0 };
  const rawWorkspaces = Array.isArray(current.workspaces) ? current.workspaces : [];
  const workspaces: Workspace[] = [];

  for (let i = 0; i < rawWorkspaces.length; i++) {
    const item = rawWorkspaces[i];
    if (typeof item !== 'object' || item === null) continue;
    const wObj = item as Record<string, unknown>;
    const id = typeof wObj.id === 'string' && wObj.id.trim() ? wObj.id : `ws-${i + 1}`;
    const name = typeof wObj.name === 'string' && wObj.name.trim() ? wObj.name : `${i + 1} Ruang`;
    const root = sanitizeLayoutNode(wObj.root, folder ?? '', counter);
    if (root) {
      workspaces.push({ id, name, root });
    }
  }

  if (workspaces.length === 0) {
    workspaces.push(createInitialWorkspace('ws-1', '1 Utama', folder ?? ''));
  }

  const requestedWs =
    typeof current.activeWorkspaceId === 'string' ? current.activeWorkspaceId : '';
  const activeWorkspaceId = workspaces.some((w) => w.id === requestedWs)
    ? requestedWs
    : workspaces[0].id;

  return {
    version: 1,
    folder,
    sidebar,
    activeWorkspaceId,
    workspaces
  };
}
