<script lang="ts">
  import Columns2 from '@lucide/svelte/icons/columns-2';
  import FileText from '@lucide/svelte/icons/file-text';
  import GitCompare from '@lucide/svelte/icons/git-compare';
  import Globe from '@lucide/svelte/icons/globe';
  import Maximize2 from '@lucide/svelte/icons/maximize-2';
  import Minimize2 from '@lucide/svelte/icons/minimize-2';
  import Plus from '@lucide/svelte/icons/plus';
  import Rows2 from '@lucide/svelte/icons/rows-2';
  import Terminal from '@lucide/svelte/icons/terminal';
  import X from '@lucide/svelte/icons/x';
  import FileIcon from '$lib/components/icons/FileIcon.svelte';
  import type { LayoutNode, Pane, Tab } from '$lib/layout/types';
  import DiffTab from '$lib/components/tabs/DiffTab.svelte';
  import FileTab, { type FileTabMeta } from '$lib/components/tabs/FileTab.svelte';
  import PreviewTab from '$lib/components/tabs/PreviewTab.svelte';
  import TerminalTab from '$lib/components/tabs/TerminalTab.svelte';
  import { getTerminalSession } from '$lib/terminalRegistry';
  import Self from './PaneNode.svelte';

  interface Props {
    node: LayoutNode;
    path?: ('first' | 'second')[];
    focusedPaneId: string;
    maximizedPaneId: string | null;
    workspaceVisible: boolean;
    dirtyTabs: Record<string, boolean>;
    theme?: 'dark' | 'light';
    onFocusPane: (paneId: string) => void;
    onSelectTab: (paneId: string, tabId: string) => void;
    onCloseTab: (paneId: string, tabId: string) => void;
    onAddTerminalTab: (paneId: string) => void;
    onAddPreviewTab: (paneId: string, url?: string) => void;
    onUpdatePreviewTab: (paneId: string, tabId: string, url: string, title: string) => void;
    onRequestQuickOpen: (paneId: string) => void;
    onSplitPane: (paneId: string, direction: 'row' | 'column') => void;
    onToggleMaximizePane: (paneId: string) => void;
    onUpdateRatio: (path: ('first' | 'second')[], ratio: number) => void;
    onMoveTab: (fromPaneId: string, tabId: string, toPaneId: string, targetIndex?: number) => void;
    onTabDirtyChange: (tabId: string, dirty: boolean) => void;
    onTabMetaChange: (tabId: string, meta: FileTabMeta) => void;
    onOpenFileFromDiff: (relPath: string, line: number | null) => void;
    onRepoChanged: () => void;
    onRegisterFileTabRef?: (tabId: string, ref: ReturnType<typeof FileTab> | null) => void;
  }

  let {
    node,
    path = [],
    focusedPaneId,
    maximizedPaneId,
    workspaceVisible,
    dirtyTabs,
    theme = 'dark',
    onFocusPane,
    onSelectTab,
    onCloseTab,
    onAddTerminalTab,
    onAddPreviewTab,
    onUpdatePreviewTab,
    onRequestQuickOpen,
    onSplitPane,
    onToggleMaximizePane,
    onUpdateRatio,
    onMoveTab,
    onTabDirtyChange,
    onTabMetaChange,
    onOpenFileFromDiff,
    onRepoChanged,
    onRegisterFileTabRef
  }: Props = $props();

  let splitContainerEl = $state<HTMLDivElement | null>(null);
  let draggingSplitter = $state(false);
  let menuOpen = $state(false);
  let dragOverPane = $state(false);
  let activeFileRef = $state<ReturnType<typeof FileTab> | null>(null);

  const activePaneTab = $derived<Tab | null>(
    node.type === 'pane'
      ? (node.tabs.find((t) => t.id === node.activeTabId) ?? node.tabs[0] ?? null)
      : null
  );

  $effect(() => {
    if (node.type === 'pane' && activePaneTab?.kind === 'file' && onRegisterFileTabRef) {
      onRegisterFileTabRef(activePaneTab.id, activeFileRef);
      return () => {
        onRegisterFileTabRef(activePaneTab.id, null);
      };
    }
  });

  function focusActiveContent(explicitTabId?: string, explicitKind?: string) {
    if (node.type !== 'pane') return;
    const targetTab = explicitTabId
      ? node.tabs.find((t) => t.id === explicitTabId)
      : activePaneTab;
    const kind = explicitKind ?? targetTab?.kind;
    const tabId = explicitTabId ?? targetTab?.id;
    if (!tabId || !kind) return;

    if (kind === 'terminal') {
      const session = getTerminalSession(tabId);
      if (session) {
        session.focus();
        session.term.scrollToBottom();
      }
    } else if (kind === 'file') {
      activeFileRef?.focus();
    }
  }

  $effect(() => {
    if (node.type === 'pane' && node.id === focusedPaneId && workspaceVisible && activePaneTab) {
      const tabId = activePaneTab.id;
      const tabKind = activePaneTab.kind;
      queueMicrotask(() => {
        if (node.type !== 'pane' || node.id !== focusedPaneId || !workspaceVisible) return;
        focusActiveContent(tabId, tabKind);
      });
    }
  });

  function handlePaneMouseDown(event: MouseEvent) {
    if (node.type !== 'pane') return;
    onFocusPane(node.id);
    const target = event.target as HTMLElement | null;
    if (!target) return;
    if (target.closest('button, input, select, textarea, a, .dropdown-menu')) {
      return;
    }
    queueMicrotask(() => {
      focusActiveContent();
    });
  }

  function startSplitterDrag(event: MouseEvent) {
    if (node.type !== 'split' || !splitContainerEl) return;
    event.preventDefault();
    draggingSplitter = true;
    window.dispatchEvent(new CustomEvent('glzr:drag-start'));

    const direction = node.direction;
    const container = splitContainerEl;
    const rect = container.getBoundingClientRect();

    function handleMouseMove(moveEvent: MouseEvent) {
      if (direction === 'row') {
        if (rect.width <= 0) return;
        const nextRatio = (moveEvent.clientX - rect.left) / rect.width;
        onUpdateRatio(path, nextRatio);
      } else {
        if (rect.height <= 0) return;
        const nextRatio = (moveEvent.clientY - rect.top) / rect.height;
        onUpdateRatio(path, nextRatio);
      }
    }

    function handleMouseUp() {
      draggingSplitter = false;
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);
      window.dispatchEvent(new CustomEvent('glzr:commit-session'));
    }

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);
  }

  function handleTabDragStart(event: DragEvent, paneId: string, tabId: string) {
    if (!event.dataTransfer) return;
    event.dataTransfer.effectAllowed = 'move';
    event.dataTransfer.setData('application/x-glzr-tab', JSON.stringify({ paneId, tabId }));
  }

  function handleDropOnPane(event: DragEvent, targetPane: Pane, targetIndex?: number) {
    event.preventDefault();
    dragOverPane = false;
    const raw = event.dataTransfer?.getData('application/x-glzr-tab');
    if (!raw) return;
    try {
      const parsed = JSON.parse(raw) as { paneId?: string; tabId?: string };
      if (typeof parsed.paneId === 'string' && typeof parsed.tabId === 'string') {
        onMoveTab(parsed.paneId, parsed.tabId, targetPane.id, targetIndex);
      }
    } catch {
      return;
    }
  }
</script>

{#if node.type === 'split'}
  <div
    bind:this={splitContainerEl}
    class="split-container {node.direction}"
    class:dragging={draggingSplitter}
  >
    <div class="split-child" style:flex="{node.ratio} 1 0%">
      <Self
        node={node.first}
        path={[...path, 'first']}
        {focusedPaneId}
        {maximizedPaneId}
        {workspaceVisible}
        {dirtyTabs}
        {theme}
        {onFocusPane}
        {onSelectTab}
        {onCloseTab}
        {onAddTerminalTab}
        {onAddPreviewTab}
        {onUpdatePreviewTab}
        {onRequestQuickOpen}
        {onSplitPane}
        {onToggleMaximizePane}
        {onUpdateRatio}
        {onMoveTab}
        {onTabDirtyChange}
        {onTabMetaChange}
        {onOpenFileFromDiff}
        {onRepoChanged}
        {onRegisterFileTabRef}
      />
    </div>

    <div
      class="split-divider {node.direction}"
      class:active={draggingSplitter}
      role="separator"
      aria-orientation={node.direction === 'row' ? 'vertical' : 'horizontal'}
      onmousedown={startSplitterDrag}
    ></div>

    <div class="split-child" style:flex="{1 - node.ratio} 1 0%">
      <Self
        node={node.second}
        path={[...path, 'second']}
        {focusedPaneId}
        {maximizedPaneId}
        {workspaceVisible}
        {dirtyTabs}
        {theme}
        {onFocusPane}
        {onSelectTab}
        {onCloseTab}
        {onAddTerminalTab}
        {onAddPreviewTab}
        {onUpdatePreviewTab}
        {onRequestQuickOpen}
        {onSplitPane}
        {onToggleMaximizePane}
        {onUpdateRatio}
        {onMoveTab}
        {onTabDirtyChange}
        {onTabMetaChange}
        {onOpenFileFromDiff}
        {onRepoChanged}
        {onRegisterFileTabRef}
      />
    </div>
  </div>
{:else}
  <section
    class="pane"
    class:focused={node.id === focusedPaneId}
    class:drag-over={dragOverPane}
    onmousedown={handlePaneMouseDown}
    onfocusin={() => onFocusPane(node.id)}
    ondragover={(e) => {
      e.preventDefault();
      dragOverPane = true;
    }}
    ondragleave={() => (dragOverPane = false)}
    ondrop={(e) => handleDropOnPane(e, node)}
  >
    <div class="tab-strip">
      <div class="tabs-scroll">
        {#each node.tabs as tab, idx (tab.id)}
          <div
            class="tab-wrapper"
            class:active={tab.id === activePaneTab?.id}
            draggable="true"
            ondragstart={(e) => handleTabDragStart(e, node.id, tab.id)}
            ondragover={(e) => e.preventDefault()}
            ondrop={(e) => {
              e.stopPropagation();
              handleDropOnPane(e, node, idx);
            }}
          >
            <button
              class="tab-item"
              class:active={tab.id === activePaneTab?.id}
              onclick={() => {
                onFocusPane(node.id);
                onSelectTab(node.id, tab.id);
                queueMicrotask(() => {
                  focusActiveContent(tab.id, tab.kind);
                });
              }}
            >
              {#if tab.kind === 'file'}
                <FileIcon
                  filename={typeof tab.data?.path === 'string' ? tab.data.path : tab.title}
                  size={14}
                />
              {:else if tab.kind === 'diff'}
                <GitCompare size={14} strokeWidth={2} />
              {:else if tab.kind === 'preview'}
                <Globe size={14} strokeWidth={2} />
              {:else}
                <Terminal size={14} strokeWidth={2} />
              {/if}
              <span>{tab.title}</span>
              {#if dirtyTabs[tab.id]}
                <span class="dirty-dot" aria-label="Unsaved"></span>
              {/if}
            </button>
            <button
              class="tab-close-btn"
              title="Close tab (Alt+W)"
              aria-label="Close tab"
              onclick={(e) => {
                e.stopPropagation();
                onCloseTab(node.id, tab.id);
              }}
            >
              <X size={12} strokeWidth={2} />
            </button>
          </div>
        {/each}

        <div class="new-tab-dropdown">
          <button
            class="icon-btn"
            title="Tab and split options"
            aria-label="Tab and split options"
            onclick={(e) => {
              e.stopPropagation();
              onFocusPane(node.id);
              menuOpen = !menuOpen;
            }}
          >
            <Plus size={14} strokeWidth={2} />
          </button>
          {#if menuOpen}
            <div class="dropdown-menu">
              <button
                class="dropdown-item"
                onclick={() => {
                  menuOpen = false;
                  onRequestQuickOpen(node.id);
                }}
              >
                <FileText size={14} strokeWidth={2} />
                <span>Open file...</span>
              </button>
              <button
                class="dropdown-item"
                onclick={() => {
                  menuOpen = false;
                  onAddTerminalTab(node.id);
                }}
              >
                <Terminal size={14} strokeWidth={2} />
                <span>New terminal</span>
              </button>
              <button
                class="dropdown-item"
                onclick={() => {
                  menuOpen = false;
                  onAddPreviewTab(node.id);
                }}
              >
                <Globe size={14} strokeWidth={2} />
                <span>New preview</span>
              </button>
              <button
                class="dropdown-item"
                onclick={() => {
                  menuOpen = false;
                  onSplitPane(node.id, 'row');
                }}
              >
                <Columns2 size={14} strokeWidth={2} />
                <span>Split right</span>
              </button>
              <button
                class="dropdown-item"
                onclick={() => {
                  menuOpen = false;
                  onSplitPane(node.id, 'column');
                }}
              >
                <Rows2 size={14} strokeWidth={2} />
                <span>Split down</span>
              </button>
            </div>
          {/if}
        </div>
      </div>

      <div class="pane-tools">
        <button
          class="icon-btn"
          title="Split right (Alt+R)"
          aria-label="Split right"
          onclick={() => onSplitPane(node.id, 'row')}
        >
          <Columns2 size={13} strokeWidth={2} />
        </button>
        <button
          class="icon-btn"
          title="Split down (Alt+J)"
          aria-label="Split down"
          onclick={() => onSplitPane(node.id, 'column')}
        >
          <Rows2 size={13} strokeWidth={2} />
        </button>
        <button
          class="icon-btn"
          title="Maximize pane (Alt+M)"
          aria-label="Maximize pane"
          onclick={() => onToggleMaximizePane(node.id)}
        >
          {#if maximizedPaneId === node.id}
            <Minimize2 size={13} strokeWidth={2} />
          {:else}
            <Maximize2 size={13} strokeWidth={2} />
          {/if}
        </button>
      </div>
    </div>

    <div class="pane-body no-pad">
      {#each node.tabs.filter((t) => t.kind === 'terminal') as termTab (termTab.id)}
        <TerminalTab
          sessionId={termTab.id}
          visible={workspaceVisible && activePaneTab?.id === termTab.id}
          focused={workspaceVisible && node.id === focusedPaneId && activePaneTab?.id === termTab.id}
          {theme}
          onOpenLocalhost={(url) => onAddPreviewTab(node.id, url)}
        />
      {/each}

      {#each node.tabs.filter((t) => t.kind === 'preview') as previewTab (previewTab.id)}
        <PreviewTab
          url={typeof previewTab.data.url === 'string'
            ? previewTab.data.url
            : 'http://localhost:5173/'}
          visible={workspaceVisible && activePaneTab?.id === previewTab.id}
          onUrlChange={(nextUrl, nextTitle) => {
            onUpdatePreviewTab(node.id, previewTab.id, nextUrl, nextTitle);
          }}
        />
      {/each}

      {#if activePaneTab}
        {#if activePaneTab.kind === 'file' && typeof activePaneTab.data.path === 'string'}
          {#key activePaneTab.id}
            <FileTab
              bind:this={activeFileRef}
              path={activePaneTab.data.path}
              targetLine={typeof activePaneTab.data.line === 'number'
                ? activePaneTab.data.line
                : null}
              onDirtyChange={(dirty) => onTabDirtyChange(activePaneTab.id, dirty)}
              onMetaChange={(meta) => onTabMetaChange(activePaneTab.id, meta)}
            />
          {/key}
        {:else if activePaneTab.kind === 'diff' && typeof activePaneTab.data.relPath === 'string'}
          {#key activePaneTab.id}
            <DiffTab
              relPath={activePaneTab.data.relPath}
              initialStaged={Boolean(activePaneTab.data.staged)}
              onOpenFile={onOpenFileFromDiff}
              onChanged={onRepoChanged}
            />
          {/key}
        {/if}
      {:else}
        <div class="empty-pane-card">
          <p class="muted">Empty pane</p>
          <div class="empty-pane-actions">
            <button class="primary" onclick={() => onAddTerminalTab(node.id)}>
              <Terminal size={14} strokeWidth={2} />
              <span>New terminal</span>
            </button>
            <button onclick={() => onRequestQuickOpen(node.id)}>
              <FileText size={14} strokeWidth={2} />
              <span>Open file</span>
            </button>
          </div>
        </div>
      {/if}
    </div>
  </section>
{/if}

<style>
  .split-container {
    display: flex;
    width: 100%;
    height: 100%;
    min-width: 0;
    min-height: 0;
  }

  .split-container.row {
    flex-direction: row;
  }

  .split-container.column {
    flex-direction: column;
  }

  .split-container.dragging {
    user-select: none;
  }

  .split-child {
    display: flex;
    min-width: 200px;
    min-height: 200px;
    overflow: hidden;
  }

  .split-divider {
    flex-shrink: 0;
    background: transparent;
    z-index: 10;
  }

  .split-divider.row {
    width: var(--gap);
    cursor: col-resize;
  }

  .split-divider.column {
    height: var(--gap);
    cursor: row-resize;
  }

  .pane {
    position: relative;
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    min-height: 200px;
    background: var(--pbg);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-panel);
    box-shadow: var(--psh);
    contain: layout style;
  }

  .pane.focused {
    box-shadow: var(--fcsh);
    border-color: var(--pbd);
  }

  .pane.focused::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: calc(var(--radius-panel) - 1px);
    box-shadow: var(--fc-ring);
    pointer-events: none;
    z-index: 20;
  }

  .pane.drag-over {
    outline: 1px dashed var(--ac);
    outline-offset: -2px;
  }

  .tab-strip {
    height: 42px;
    padding: 0 10px;
    background: transparent;
    border-bottom: 1px solid var(--bd);
    border-top-left-radius: calc(var(--radius-panel) - 1px);
    border-top-right-radius: calc(var(--radius-panel) - 1px);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    flex-shrink: 0;
  }

  .tabs-scroll {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    flex: 1;
  }

  .pane-tools {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .tab-wrapper {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: var(--radius-ctrl);
    padding-right: 4px;
  }

  .tab-wrapper:hover {
    background: var(--el);
  }

  .tab-wrapper.active {
    background: var(--el);
  }

  .tab-item {
    height: 26px;
    padding: 0 8px 0 10px;
    border: none;
    background: transparent;
    color: var(--dm);
    gap: 8px;
    line-height: normal;
    text-align: center;
  }

  .tab-item :global(.jb-file-icon) {
    width: 1.15em;
    height: 1.15em;
    flex-shrink: 0;
  }

  .tab-item:hover {
    background: transparent;
  }

  .tab-item.active {
    color: var(--tx);
  }

  .dirty-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ac);
  }

  .tab-close-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
    opacity: 0;
  }

  .tab-wrapper:hover .tab-close-btn,
  .tab-wrapper.active .tab-close-btn {
    opacity: 1;
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .icon-btn:hover {
    color: var(--tx);
  }

  .new-tab-dropdown {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 30px;
    left: 0;
    min-width: 165px;
    background: var(--pop-bg);
    -webkit-backdrop-filter: var(--pop-blur);
    backdrop-filter: var(--pop-blur);
    border: 1px solid var(--pbd);
    border-radius: var(--radius-menu);
    box-shadow: var(--shadow-float);
    padding: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    z-index: 40;
  }

  .dropdown-item {
    width: 100%;
    height: 28px;
    padding: 0 10px;
    border: none;
    border-radius: var(--radius-ctrl);
    background: transparent;
    color: var(--tx);
    justify-content: flex-start;
    gap: 8px;
  }

  .dropdown-item:hover {
    background: var(--acs);
  }

  .pane-body {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    display: flex;
    border-bottom-left-radius: calc(var(--radius-panel) - 1px);
    border-bottom-right-radius: calc(var(--radius-panel) - 1px);
  }

  .empty-pane-card {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
  }

  .empty-pane-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .muted {
    color: var(--dm);
    font-size: 12px;
  }
</style>
