<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Search from '@lucide/svelte/icons/search';
  import X from '@lucide/svelte/icons/x';
  import {
    onBackendEvent,
    searchCancel,
    searchStart,
    type AppErrorPayload,
    type SearchMatchDto
  } from '$lib/ipc';

  interface Props {
    fontSize?: number;
    onSelectMatch: (relPath: string, line: number) => void;
  }

  let { fontSize = 14, onSelectMatch }: Props = $props();

  interface PreparedMatch {
    relPath: string;
    line: number;
    col: number;
    pre: string;
    hit: string;
    post: string;
  }

  type FlatSearchRow =
    | { type: 'header'; key: string; relPath: string; count: number }
    | { type: 'match'; key: string; match: PreparedMatch };

  const rowHeight = $derived(Math.max(18, Math.round(28 * (fontSize / 14))));
  const OVERSCAN = 12;

  let pattern = $state('');
  let globFilter = $state('');
  let caseSensitive = $state(false);
  let regexMode = $state(false);

  let activeQueryId = $state<string | null>(null);
  let searching = $state(false);
  let truncated = $state(false);
  let errorMsg = $state<string | null>(null);
  let results = $state.raw<PreparedMatch[]>([]);

  let searchInputEl = $state<HTMLInputElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(400);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let queryCounter = 0;
  let unlistenResults: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;

  function splitSnippet(match: SearchMatchDto): PreparedMatch {
    const chars = Array.from(match.snippet);
    const start = Math.max(0, Math.min(chars.length, match.matchStart));
    const end = Math.max(start, Math.min(chars.length, match.matchEnd));
    return {
      relPath: match.relPath,
      line: match.line,
      col: match.col,
      pre: chars.slice(0, start).join(''),
      hit: chars.slice(start, end).join(''),
      post: chars.slice(end).join('')
    };
  }

  const flatRows = $derived.by<FlatSearchRow[]>(() => {
    if (results.length === 0) return [];
    const grouped = new Map<string, PreparedMatch[]>();
    for (const item of results) {
      const list = grouped.get(item.relPath);
      if (list) {
        list.push(item);
      } else {
        grouped.set(item.relPath, [item]);
      }
    }

    const out: FlatSearchRow[] = [];
    for (const [relPath, matches] of grouped.entries()) {
      out.push({
        type: 'header',
        key: `hdr-${relPath}`,
        relPath,
        count: matches.length
      });
      let fileMatchIdx = 0;
      for (const m of matches) {
        fileMatchIdx += 1;
        out.push({
          type: 'match',
          key: `m-${m.relPath}-${fileMatchIdx}-${m.line}-${m.col}`,
          match: m
        });
      }
    }
    return out;
  });

  const totalHeight = $derived(flatRows.length * rowHeight);
  const startIndex = $derived(Math.max(0, (Math.floor(scrollTop / rowHeight) & ~3) - OVERSCAN));
  const endIndex = $derived(
    Math.min(
      flatRows.length,
      ((Math.ceil((scrollTop + viewportHeight) / rowHeight) + 3) & ~3) + OVERSCAN
    )
  );
  const visibleSlice = $derived(flatRows.slice(startIndex, endIndex));

  export function focusInput() {
    searchInputEl?.focus();
    searchInputEl?.select();
  }

  function clearDebounce() {
    if (debounceTimer !== null) {
      clearTimeout(debounceTimer);
      debounceTimer = null;
    }
  }

  function scheduleSearch() {
    clearDebounce();
    const trimmed = pattern.trim();
    if (!trimmed) {
      activeQueryId = null;
      searching = false;
      truncated = false;
      errorMsg = null;
      results = [];
      void searchCancel().catch(() => {});
      return;
    }

    debounceTimer = setTimeout(() => {
      void executeSearch(trimmed);
    }, 200);
  }

  async function executeSearch(searchPattern: string) {
    queryCounter += 1;
    const qid = `sq-${Date.now()}-${queryCounter}`;
    activeQueryId = qid;
    searching = true;
    truncated = false;
    errorMsg = null;
    results = [];
    scrollTop = 0;

    try {
      await searchStart({
        queryId: qid,
        pattern: searchPattern,
        caseSensitive,
        regex: regexMode,
        glob: globFilter.trim() || null
      });
    } catch (err) {
      if (activeQueryId === qid) {
        const normalized = err as AppErrorPayload;
        errorMsg = normalized.message ?? 'Failed to start project search';
        searching = false;
      }
    }
  }

  onMount(async () => {
    unlistenResults = await onBackendEvent('search:results', (payload) => {
      if (payload.queryId === activeQueryId) {
        const prepared = payload.items.map(splitSnippet);
        results = [...results, ...prepared];
      }
    });

    unlistenDone = await onBackendEvent('search:done', (payload) => {
      if (payload.queryId === activeQueryId) {
        searching = false;
        truncated = payload.truncated;
      }
    });

    unlistenError = await onBackendEvent('search:error', (payload) => {
      if (payload.queryId === activeQueryId) {
        searching = false;
        errorMsg = payload.message;
      }
    });
  });

  onDestroy(() => {
    clearDebounce();
    unlistenResults?.();
    unlistenDone?.();
    unlistenError?.();
    void searchCancel().catch(() => {});
  });
</script>

<aside class="search-panel">
  <div class="panel-header">
    <span class="panel-title">Search</span>
    {#if results.length > 0}
      <span class="count-label mono">{results.length} results</span>
    {/if}
  </div>

  <div class="search-controls">
    <div class="input-row">
      <Search size={14} strokeWidth={2} />
      <input
        bind:this={searchInputEl}
        bind:value={pattern}
        type="text"
        class="search-input"
        placeholder="Search text in project..."
        oninput={scheduleSearch}
      />
      {#if pattern}
        <button
          class="clear-btn"
          title="Clear search"
          aria-label="Clear search"
          onclick={() => {
            pattern = '';
            scheduleSearch();
          }}
        >
          <X size={12} strokeWidth={2} />
        </button>
      {/if}
    </div>

    <div class="filter-row">
      <input
        bind:value={globFilter}
        type="text"
        class="glob-input mono"
        placeholder="Include glob (e.g. *.rs, src/**)"
        oninput={scheduleSearch}
      />
      <div class="option-toggles">
        <button
          class="opt-btn mono"
          class:active={caseSensitive}
          title="Match Case"
          onclick={() => {
            caseSensitive = !caseSensitive;
            scheduleSearch();
          }}
        >
          Aa
        </button>
        <button
          class="opt-btn mono"
          class:active={regexMode}
          title="Use Regular Expression"
          onclick={() => {
            regexMode = !regexMode;
            scheduleSearch();
          }}
        >
          .*
        </button>
      </div>
    </div>
  </div>

  {#if errorMsg}
    <div class="error-banner">
      <span>{errorMsg}</span>
    </div>
  {/if}

  {#if truncated}
    <div class="truncated-banner">
      <span>Results truncated at 2,000 matches.</span>
    </div>
  {/if}

  <div
    class="results-viewport"
    bind:clientHeight={viewportHeight}
    onscroll={(e) => {
      scrollTop = (e.currentTarget as HTMLDivElement).scrollTop;
    }}
  >
    {#if !pattern.trim()}
      <div class="empty-box">Type a keyword to search across the project</div>
    {:else if searching && results.length === 0}
      <div class="empty-box">Searching...</div>
    {:else if !searching && results.length === 0 && !errorMsg}
      <div class="empty-box">No matching results found</div>
    {:else}
      <div class="virtual-spacer" style:height="{totalHeight}px">
        <div class="virtual-window" style:transform="translate3d(0, {startIndex * rowHeight}px, 0)">
          {#each visibleSlice as row (row.key)}
            {#if row.type === 'header'}
              <div class="virtual-row file-header" style:height="{rowHeight}px">
                <svg
                  class="file-icon"
                  width="13"
                  height="13"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  aria-hidden="true"
                >
                  <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
                  <path d="M14 2v4a2 2 0 0 0 2 2h4" />
                  <path d="M10 9H8" />
                  <path d="M16 13H8" />
                  <path d="M16 17H8" />
                </svg>
                <span class="header-path mono">{row.relPath}</span>
                <span class="header-count mono">{row.count}</span>
              </div>
            {:else}
              <button
                class="virtual-row match-row"
                style:height="{rowHeight}px"
                onclick={() => onSelectMatch(row.match.relPath, row.match.line)}
              >
                <span class="line-num mono">{row.match.line}:</span>
                <span class="snippet-text mono">
                  <span>{row.match.pre}</span><mark class="hit-mark">{row.match.hit}</mark><span
                    >{row.match.post}</span
                  >
                </span>
              </button>
            {/if}
          {/each}
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .search-panel {
    width: 100%;
    height: 100%;
    background: var(--pbg);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .panel-header {
    height: auto;
    min-height: calc(32px * var(--ui-scale, 1));
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    font-size: var(--font-size-ui, 13px);
    font-weight: 500;
    color: var(--dm);
  }

  .count-label {
    font-size: var(--font-size-ui-sm, 12px);
    color: var(--dm);
  }

  .search-controls {
    padding: 8px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    gap: calc(6px * var(--ui-scale, 1));
  }

  .input-row {
    height: auto;
    min-height: calc(30px * var(--ui-scale, 1));
    padding: 2px 8px;
    background: var(--el);
    border: 1px solid var(--bd);
    border-radius: var(--radius-input);
    display: flex;
    align-items: center;
    gap: calc(6px * var(--ui-scale, 1));
    color: var(--dm);
  }

  :global(.search-icon) {
    color: var(--dm);
    flex-shrink: 0;
    width: 1.1em;
    height: 1.1em;
  }

  .search-input {
    flex: 1;
    min-width: 0;
    height: auto;
    min-height: calc(24px * var(--ui-scale, 1));
    border: none;
    background: transparent;
    color: var(--tx);
    font-size: var(--font-size-ui, 13px);
    padding: 0;
  }

  .search-input:focus {
    outline: none;
    box-shadow: none;
  }

  .clear-btn {
    width: auto;
    min-width: calc(18px * var(--ui-scale, 1));
    height: auto;
    min-height: calc(18px * var(--ui-scale, 1));
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }

  .clear-btn :global(svg) {
    width: 1em;
    height: 1em;
    flex-shrink: 0;
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: calc(6px * var(--ui-scale, 1));
  }

  .glob-input {
    flex: 1;
    min-width: 0;
    height: auto;
    min-height: calc(28px * var(--ui-scale, 1));
    padding: 2px 8px;
    background: var(--el);
    border: 1px solid var(--bd);
    border-radius: var(--radius-input);
    color: var(--tx);
    font-size: var(--font-size-ui, 13px);
  }

  .option-toggles {
    display: flex;
    align-items: center;
    gap: calc(4px * var(--ui-scale, 1));
  }

  .opt-btn {
    width: auto;
    min-width: calc(28px * var(--ui-scale, 1));
    height: auto;
    min-height: calc(28px * var(--ui-scale, 1));
    padding: 2px 6px;
    font-size: var(--font-size-ui, 13px);
    background: var(--el);
    color: var(--dm);
    border: 1px solid var(--bd);
  }

  .opt-btn.active {
    background: var(--acs);
    color: var(--ac);
    border-color: var(--ac);
  }

  .error-banner {
    padding: 6px 12px;
    background: var(--des);
    border-bottom: 1px solid var(--bd);
    color: var(--de);
    font-size: var(--font-size-ui-sm, 12px);
  }

  .truncated-banner {
    padding: 4px 12px;
    background: var(--el);
    border-bottom: 1px solid var(--bd);
    color: var(--mo);
    font-size: var(--font-size-ui-sm, 12px);
  }

  .results-viewport {
    flex: 1;
    overflow-y: auto;
    position: relative;
    min-height: 0;
    background: var(--pbg);
    overscroll-behavior: contain;
    contain: strict;
    will-change: scroll-position;
    transform: translateZ(0);
  }

  .virtual-spacer {
    position: relative;
    width: 100%;
    contain: layout style paint;
  }

  .virtual-window {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    will-change: transform;
  }

  .virtual-row {
    width: 100%;
    height: auto;
    min-height: 0;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: calc(6px * var(--ui-scale, 1));
    contain: layout style paint;
    box-sizing: border-box;
  }

  .file-icon {
    flex-shrink: 0;
    width: 1.1em;
    height: 1.1em;
  }

  .file-header {
    background: var(--pbg);
    color: var(--tx);
    font-weight: 500;
    border-top: 1px solid var(--bd);
    border-bottom: 1px solid var(--bd);
  }

  .header-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-ui, 13px);
  }

  .header-count {
    font-size: var(--font-size-ui, 13px);
    color: var(--dm);
  }

  .match-row {
    width: 100%;
    border: none;
    border-radius: 0;
    background: var(--pbg);
    color: var(--dm);
    justify-content: flex-start;
    text-align: left;
    padding-left: 20px;
  }

  .match-row:hover {
    background: var(--acs);
    color: var(--tx);
  }

  .line-num {
    color: var(--ft);
    font-size: var(--font-size-ui, 13px);
    flex-shrink: 0;
  }

  .snippet-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-size-ui, 13px);
    color: var(--tx);
  }

  .hit-mark {
    background: var(--acs);
    color: var(--ac);
    border-radius: 2px;
    padding: 0 2px;
  }

  .empty-box {
    padding: 20px 12px;
    font-size: var(--font-size-ui, 13px);
    color: var(--dm);
    text-align: center;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
