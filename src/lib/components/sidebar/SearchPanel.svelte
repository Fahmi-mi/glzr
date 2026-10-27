<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import FileText from '@lucide/svelte/icons/file-text';
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
    onSelectMatch: (relPath: string, line: number) => void;
  }

  let { onSelectMatch }: Props = $props();

  type FlatSearchRow =
    | { type: 'header'; key: string; relPath: string; count: number }
    | { type: 'match'; key: string; match: SearchMatchDto };

  const ROW_HEIGHT = 26;
  const OVERSCAN = 10;

  let pattern = $state('');
  let globFilter = $state('');
  let caseSensitive = $state(false);
  let regexMode = $state(false);

  let activeQueryId = $state<string | null>(null);
  let searching = $state(false);
  let truncated = $state(false);
  let errorMsg = $state<string | null>(null);
  let results = $state<SearchMatchDto[]>([]);

  let searchInputEl = $state<HTMLInputElement | null>(null);
  let scrollTop = $state(0);
  let viewportHeight = $state(400);

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let queryCounter = 0;
  let unlistenResults: (() => void) | null = null;
  let unlistenDone: (() => void) | null = null;
  let unlistenError: (() => void) | null = null;

  const flatRows = $derived.by<FlatSearchRow[]>(() => {
    if (results.length === 0) return [];
    const grouped = new Map<string, SearchMatchDto[]>();
    for (const item of results) {
      const list = grouped.get(item.relPath);
      if (list) {
        list.push(item);
      } else {
        grouped.set(item.relPath, [item]);
      }
    }

    const out: FlatSearchRow[] = [];
    let idx = 0;
    for (const [relPath, matches] of grouped.entries()) {
      out.push({
        type: 'header',
        key: `hdr-${relPath}`,
        relPath,
        count: matches.length
      });
      for (const m of matches) {
        idx += 1;
        out.push({
          type: 'match',
          key: `m-${idx}-${m.relPath}-${m.line}-${m.col}`,
          match: m
        });
      }
    }
    return out;
  });

  const totalHeight = $derived(flatRows.length * ROW_HEIGHT);
  const startIndex = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN));
  const endIndex = $derived(
    Math.min(flatRows.length, Math.ceil((scrollTop + viewportHeight) / ROW_HEIGHT) + OVERSCAN)
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
        results.push(...payload.items);
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

  function splitSnippet(match: SearchMatchDto): { pre: string; hit: string; post: string } {
    const chars = Array.from(match.snippet);
    const start = Math.max(0, Math.min(chars.length, match.matchStart));
    const end = Math.max(start, Math.min(chars.length, match.matchEnd));
    return {
      pre: chars.slice(0, start).join(''),
      hit: chars.slice(start, end).join(''),
      post: chars.slice(end).join('')
    };
  }
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
        {#each visibleSlice as row, idx (row.key)}
          {@const topPx = (startIndex + idx) * ROW_HEIGHT}
          {#if row.type === 'header'}
            <div class="virtual-row file-header" style:top="{topPx}px">
              <FileText size={13} strokeWidth={2} />
              <span class="header-path mono">{row.relPath}</span>
              <span class="header-count mono">{row.count}</span>
            </div>
          {:else}
            {@const parts = splitSnippet(row.match)}
            <button
              class="virtual-row match-row"
              style:top="{topPx}px"
              onclick={() => onSelectMatch(row.match.relPath, row.match.line)}
            >
              <span class="line-num mono">{row.match.line}:</span>
              <span class="snippet-text mono">
                <span>{parts.pre}</span><mark class="hit-mark">{parts.hit}</mark><span
                  >{parts.post}</span
                >
              </span>
            </button>
          {/if}
        {/each}
      </div>
    {/if}
  </div>
</aside>

<style>
  .search-panel {
    width: 100%;
    height: 100%;
    background: var(--pn);
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .panel-header {
    height: 36px;
    padding: 0 12px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .panel-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--dm);
  }

  .count-label {
    font-size: 11px;
    color: var(--dm);
  }

  .search-controls {
    padding: 8px;
    border-bottom: 1px solid var(--bd);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .input-row {
    height: 30px;
    padding: 0 8px;
    background: var(--bg);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--dm);
  }

  .search-input {
    flex: 1;
    min-width: 0;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--tx);
    font-size: 12px;
    padding: 0;
  }

  .search-input:focus {
    outline: none;
  }

  .clear-btn {
    width: 18px;
    height: 18px;
    padding: 0;
    border: none;
    background: transparent;
    color: var(--dm);
  }

  .filter-row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .glob-input {
    flex: 1;
    min-width: 0;
    height: 26px;
    padding: 0 8px;
    background: var(--bg);
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    color: var(--tx);
    font-size: 11px;
  }

  .option-toggles {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .opt-btn {
    width: 26px;
    height: 26px;
    padding: 0;
    font-size: 11px;
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
    font-size: 12px;
  }

  .truncated-banner {
    padding: 4px 12px;
    background: var(--el);
    border-bottom: 1px solid var(--bd);
    color: var(--mo);
    font-size: 11px;
  }

  .results-viewport {
    flex: 1;
    overflow-y: auto;
    position: relative;
    min-height: 0;
  }

  .virtual-spacer {
    position: relative;
    width: 100%;
  }

  .virtual-row {
    position: absolute;
    left: 0;
    right: 0;
    height: 26px;
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .file-header {
    background: var(--el);
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
    font-size: 11px;
  }

  .header-count {
    font-size: 11px;
    color: var(--dm);
  }

  .match-row {
    width: 100%;
    border: none;
    border-radius: 0;
    background: transparent;
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
    font-size: 11px;
    flex-shrink: 0;
  }

  .snippet-text {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
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
    font-size: 12px;
    color: var(--dm);
    text-align: center;
  }

  .mono {
    font-family: var(--font-mono);
  }
</style>
