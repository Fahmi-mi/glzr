<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import ExternalLink from '@lucide/svelte/icons/external-link';
  import { EditorSelection, EditorState, Compartment } from '@codemirror/state';
  import { EditorView, drawSelection, keymap, lineNumbers } from '@codemirror/view';
  import {
    defaultKeymap,
    history,
    historyKeymap,
    indentWithTab,
    toggleComment
  } from '@codemirror/commands';
  import { HighlightStyle, bracketMatching, syntaxHighlighting } from '@codemirror/language';
  import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
  import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search';
  import { tags } from '@lezer/highlight';
  import {
    fsClearDraft,
    fsLoadDraft,
    fsReadFile,
    fsSaveDraft,
    fsWriteFile,
    normalizeTabPath,
    onBackendEvent,
    openExternal,
    type AppErrorPayload,
    type DraftDto,
    type FileReadDto
  } from '$lib/ipc';
  import { detectLanguage, loadLanguageExtension } from '$lib/languages';
  import ImageTab from './ImageTab.svelte';

  export interface FileTabMeta {
    language: string;
    encoding: string;
    lineEnding: 'LF' | 'CRLF';
    line: number;
    col: number;
    kind: string;
  }

  interface Props {
    path: string;
    targetLine?: number | null;
    onDirtyChange?: (dirty: boolean) => void;
    onMetaChange?: (meta: FileTabMeta) => void;
  }

  let { path, targetLine = null, onDirtyChange, onMetaChange }: Props = $props();

  let editorHost = $state<HTMLDivElement | null>(null);
  let fileData = $state<FileReadDto | null>(null);
  let pendingDraft = $state<DraftDto | null>(null);
  let externalConflict = $state<FileReadDto | null>(null);
  let loading = $state(true);
  let errorMsg = $state<string | null>(null);
  let saveErrorMsg = $state<string | null>(null);
  let svgMode = $state<'preview' | 'code'>('preview');
  let isDirty = $state(false);
  let savedContent = $state('');
  let lastKnownMtimeMs = $state(0);
  let cursorLine = $state(1);
  let cursorCol = $state(1);

  let editorView: EditorView | null = null;
  const languageCompartment = new Compartment();
  const readOnlyCompartment = new Compartment();
  let draftTimer: ReturnType<typeof setTimeout> | null = null;
  let activeLoadToken = 0;
  let unlistenFsChanged: (() => void) | null = null;

  const glzrHighlightStyle = HighlightStyle.define([
    {
      tag: [tags.keyword, tags.controlKeyword, tags.operatorKeyword, tags.modifier],
      color: 'var(--k)'
    },
    {
      tag: [tags.function(tags.variableName), tags.function(tags.propertyName), tags.labelName],
      color: 'var(--f)'
    },
    { tag: [tags.string, tags.special(tags.string), tags.regexp], color: 'var(--s)' },
    {
      tag: [tags.number, tags.bool, tags.null, tags.atom, tags.constant(tags.name)],
      color: 'var(--mo)'
    },
    {
      tag: [tags.typeName, tags.className, tags.namespace, tags.tagName, tags.attributeName],
      color: 'var(--cy)'
    },
    {
      tag: [tags.comment, tags.lineComment, tags.blockComment, tags.docComment],
      color: 'var(--ft)'
    },
    { tag: [tags.invalid], color: 'var(--de)' },
    {
      tag: [tags.variableName, tags.propertyName, tags.punctuation, tags.operator],
      color: 'var(--tx)'
    }
  ]);

  const glzrEditorTheme = EditorView.theme({
    '&': {
      height: '100%',
      backgroundColor: 'var(--bg)',
      color: 'var(--tx)',
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--font-size-code)',
      lineHeight: '24px'
    },
    '.cm-scroller': {
      fontFamily: 'var(--font-mono)',
      lineHeight: '24px',
      overflow: 'auto',
      overscrollBehavior: 'contain',
      WebkitOverflowScrolling: 'touch',
      WebkitFontSmoothing: 'antialiased',
      scrollBehavior: 'auto'
    },
    '.cm-line': {
      lineHeight: '24px',
      height: '24px',
      minHeight: '24px',
      boxSizing: 'border-box'
    },
    '.cm-lineNumbers': {
      contain: 'layout style'
    },
    '.cm-lineNumbers .cm-gutterElement': {
      lineHeight: '24px',
      height: '24px',
      minHeight: '24px',
      boxSizing: 'border-box',
      minWidth: '32px',
      textAlign: 'right'
    },
    '.cm-content': {
      caretColor: 'var(--ac)',
      padding: '16px 24px 16px 0'
    },
    '.cm-cursor, .cm-dropCursor': {
      borderLeftColor: 'var(--ac)',
      borderLeftWidth: '2px'
    },
    '&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection': {
      backgroundColor: 'var(--acs)'
    },
    '.cm-gutters': {
      backgroundColor: 'var(--bg)',
      color: 'var(--ft)',
      border: 'none',
      paddingLeft: '16px',
      paddingRight: '12px',
      contain: 'layout style'
    },
    '.cm-activeLineGutter': {
      backgroundColor: 'transparent',
      color: 'var(--tx)'
    },
    '.cm-activeLine': {
      backgroundColor: 'transparent'
    },
    '.cm-selectionMatch': {
      backgroundColor: 'var(--acs)'
    },
    '.cm-matchingBracket': {
      backgroundColor: 'var(--acs)',
      color: 'var(--ac)',
      outline: '1px solid var(--ac)'
    },
    '.cm-panels': {
      backgroundColor: 'var(--pn)',
      color: 'var(--tx)',
      borderBottom: '1px solid var(--bd)'
    },
    '.cm-search': {
      fontFamily: 'var(--font-ui)',
      fontSize: '12px',
      padding: '6px 12px',
      display: 'flex',
      alignItems: 'center',
      gap: '6px',
      flexWrap: 'wrap'
    },
    '.cm-textfield': {
      backgroundColor: 'var(--el)',
      color: 'var(--tx)',
      border: '1px solid var(--bd)',
      borderRadius: '5px',
      padding: '2px 8px',
      fontSize: '12px'
    },
    '.cm-button': {
      backgroundImage: 'none',
      backgroundColor: 'var(--el)',
      color: 'var(--tx)',
      border: '1px solid var(--bd)',
      borderRadius: '5px',
      padding: '2px 8px',
      fontSize: '12px',
      cursor: 'pointer'
    }
  });

  function clearDraftTimer() {
    if (draftTimer !== null) {
      clearTimeout(draftTimer);
      draftTimer = null;
    }
  }

  function scheduleDraftSave(targetPath: string, content: string) {
    clearDraftTimer();
    draftTimer = setTimeout(() => {
      void fsSaveDraft(targetPath, content).catch(() => {});
    }, 5000);
  }

  function formatEncodingLabel(enc: FileReadDto['encoding']): string {
    if (enc === 'utf8-bom') return 'UTF-8 BOM';
    if (enc === 'non-utf8') return 'Non-UTF-8';
    return 'UTF-8';
  }

  function emitMeta(data: FileReadDto | null, line: number, col: number) {
    if (!data || !onMetaChange) return;
    const langInfo = detectLanguage(path);
    onMetaChange({
      language: data.kind === 'image' ? 'Image' : langInfo.label,
      encoding: formatEncodingLabel(data.encoding),
      lineEnding: data.lineEnding === 'crlf' ? 'CRLF' : 'LF',
      line,
      col,
      kind: data.kind
    });
  }

  export async function save(): Promise<boolean> {
    if (!fileData || !editorView) return true;
    if (fileData.encoding === 'non-utf8') return false;

    saveErrorMsg = null;
    const content = editorView.state.doc.toString();
    try {
      const writeRes = await fsWriteFile(path, content, fileData.encoding, fileData.lineEnding);
      lastKnownMtimeMs = writeRes.mtimeMs;
      savedContent = content;
      externalConflict = null;
      isDirty = false;
      onDirtyChange?.(false);
      clearDraftTimer();
      await fsClearDraft(path).catch(() => {});
      return true;
    } catch (err) {
      const normalized = err as AppErrorPayload;
      saveErrorMsg = normalized.message ?? 'Failed to save file';
      return false;
    }
  }

  function jumpToLine(view: EditorView, lineNum: number) {
    const totalLines = view.state.doc.lines;
    if (totalLines === 0) return;
    const clamped = Math.max(1, Math.min(totalLines, Math.floor(lineNum)));
    const lineInfo = view.state.doc.line(clamped);
    view.dispatch({
      selection: EditorSelection.cursor(lineInfo.from),
      effects: EditorView.scrollIntoView(lineInfo.from, { y: 'center' })
    });
    view.focus();
  }

  function destroyEditor() {
    clearDraftTimer();
    if (editorView) {
      editorView.destroy();
      editorView = null;
    }
  }

  async function mountEditor(host: HTMLDivElement, data: FileReadDto, loadToken: number) {
    destroyEditor();
    const isReadOnly = data.encoding === 'non-utf8';
    const initialDoc = data.content ?? '';

    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        const current = update.state.doc.toString();
        const dirtyNow = current !== savedContent;
        if (dirtyNow !== isDirty) {
          isDirty = dirtyNow;
          onDirtyChange?.(dirtyNow);
        }
        if (dirtyNow) {
          scheduleDraftSave(path, current);
        } else {
          clearDraftTimer();
        }
      }
      if (update.selectionSet || update.docChanged) {
        const head = update.state.selection.main.head;
        const lineObj = update.state.doc.lineAt(head);
        cursorLine = lineObj.number;
        cursorCol = head - lineObj.from + 1;
        emitMeta(fileData, cursorLine, cursorCol);
      }
    });

    const state = EditorState.create({
      doc: initialDoc,
      extensions: [
        lineNumbers(),
        history(),
        drawSelection(),
        bracketMatching(),
        closeBrackets(),
        search({ top: true }),
        highlightSelectionMatches(),
        syntaxHighlighting(glzrHighlightStyle),
        glzrEditorTheme,
        readOnlyCompartment.of(EditorState.readOnly.of(isReadOnly)),
        languageCompartment.of([]),
        keymap.of([
          {
            key: 'Mod-s',
            run: () => {
              void save();
              return true;
            }
          },
          {
            key: 'Mod-/',
            run: toggleComment
          },
          ...closeBracketsKeymap,
          ...defaultKeymap,
          ...searchKeymap,
          ...historyKeymap,
          indentWithTab
        ]),
        updateListener
      ]
    });

    editorView = new EditorView({
      state,
      parent: host
    });

    if (targetLine && targetLine > 0) {
      jumpToLine(editorView, targetLine);
    }

    if (!data.disableHighlight) {
      const langInfo = detectLanguage(path);
      const ext = await loadLanguageExtension(langInfo.id);
      if (loadToken === activeLoadToken && editorView && ext) {
        editorView.dispatch({
          effects: languageCompartment.reconfigure(ext)
        });
      }
    }
  }

  async function loadFile(targetPath: string) {
    const token = ++activeLoadToken;
    loading = true;
    errorMsg = null;
    saveErrorMsg = null;
    pendingDraft = null;
    isDirty = false;
    onDirtyChange?.(false);
    destroyEditor();

    try {
      const data = await fsReadFile(targetPath);
      if (token !== activeLoadToken) return;
      fileData = data;
      savedContent = data.content ?? '';
      lastKnownMtimeMs = data.mtimeMs;
      externalConflict = null;
      svgMode = 'preview';
      cursorLine = 1;
      cursorCol = 1;
      emitMeta(data, 1, 1);

      if (data.kind === 'text' || data.kind === 'svg') {
        const draft = await fsLoadDraft(targetPath).catch(() => null);
        if (token !== activeLoadToken) return;
        if (draft && draft.content !== savedContent) {
          pendingDraft = draft;
        }
      }
    } catch (err) {
      if (token !== activeLoadToken) return;
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Failed to read file';
      fileData = null;
    } finally {
      if (token === activeLoadToken) {
        loading = false;
      }
    }
  }

  function applyDraft() {
    if (!pendingDraft || !editorView) return;
    const draftText = pendingDraft.content;
    editorView.dispatch({
      changes: {
        from: 0,
        to: editorView.state.doc.length,
        insert: draftText
      }
    });
    pendingDraft = null;
  }

  async function discardDraft() {
    pendingDraft = null;
    await fsClearDraft(path).catch(() => {});
  }

  async function handleOpenExternal() {
    try {
      await openExternal(path);
    } catch (err) {
      const normalized = err as AppErrorPayload;
      saveErrorMsg = normalized.message ?? 'Failed to open external application';
    }
  }

  function applyExternalReload(latest: FileReadDto) {
    fileData = latest;
    savedContent = latest.content ?? '';
    lastKnownMtimeMs = latest.mtimeMs;
    externalConflict = null;
    isDirty = false;
    onDirtyChange?.(false);
    clearDraftTimer();
    void fsClearDraft(path).catch(() => {});

    if (editorView && latest.content !== null) {
      const prevHead = editorView.state.selection.main.head;
      const clampedHead = Math.min(prevHead, latest.content.length);
      editorView.dispatch({
        changes: {
          from: 0,
          to: editorView.state.doc.length,
          insert: latest.content
        },
        selection: EditorSelection.cursor(clampedHead)
      });
    }
  }

  onMount(async () => {
    unlistenFsChanged = await onBackendEvent('fs:changed', async (payload) => {
      const myNorm = normalizeTabPath(path);
      const matched = payload.paths.some((p) => normalizeTabPath(p) === myNorm);
      if (!matched || loading) return;

      try {
        const latest = await fsReadFile(path);
        if (latest.mtimeMs <= lastKnownMtimeMs || latest.content === savedContent) {
          return;
        }
        if (!isDirty) {
          applyExternalReload(latest);
        } else {
          externalConflict = latest;
        }
      } catch {
        // File may have been deleted or moved
      }
    });
  });

  $effect(() => {
    void loadFile(path);
  });

  $effect(() => {
    const shouldMountEditor =
      !loading &&
      fileData !== null &&
      editorHost !== null &&
      (fileData.kind === 'text' || (fileData.kind === 'svg' && svgMode === 'code'));

    if (shouldMountEditor && editorHost && fileData) {
      void mountEditor(editorHost, fileData, activeLoadToken);
    } else {
      destroyEditor();
    }
  });

  $effect(() => {
    if (editorView && targetLine && targetLine > 0) {
      jumpToLine(editorView, targetLine);
    }
  });

  onDestroy(() => {
    activeLoadToken++;
    unlistenFsChanged?.();
    destroyEditor();
  });
</script>

<div class="file-tab-container">
  {#if loading}
    <div class="state-box">
      <p class="muted">Loading file...</p>
    </div>
  {:else if errorMsg}
    <div class="state-box">
      <p class="error-text">{errorMsg}</p>
    </div>
  {:else if fileData}
    {#if fileData.kind === 'svg'}
      <div class="sub-bar">
        <span class="sub-bar-label">SVG Mode</span>
        <div class="segmented-switch">
          <button
            class="seg-btn"
            class:active={svgMode === 'preview'}
            onclick={() => (svgMode = 'preview')}
          >
            Preview
          </button>
          <button
            class="seg-btn"
            class:active={svgMode === 'code'}
            onclick={() => (svgMode = 'code')}
          >
            Code
          </button>
        </div>
      </div>
    {/if}

    {#if fileData.encoding === 'non-utf8' && (fileData.kind === 'text' || fileData.kind === 'svg')}
      <div class="notice-bar warning">
        <span>File has non-UTF-8 encoding and is opened in read-only mode.</span>
      </div>
    {/if}

    {#if fileData.disableHighlight && (fileData.kind === 'text' || fileData.kind === 'svg')}
      <div class="notice-bar">
        <span>Syntax highlighting disabled for large file performance.</span>
      </div>
    {/if}

    {#if externalConflict}
      <div class="notice-bar warning">
        <span>This file was modified on disk outside the editor.</span>
        <div class="notice-actions">
          <button class="primary small-btn" onclick={() => void save()}>Keep Mine</button>
          <button
            class="small-btn"
            onclick={() => {
              if (externalConflict) applyExternalReload(externalConflict);
            }}
          >
            Reload
          </button>
        </div>
      </div>
    {/if}

    {#if pendingDraft && (fileData.kind === 'text' || (fileData.kind === 'svg' && svgMode === 'code'))}
      <div class="notice-bar draft">
        <span>An unsaved draft was found for this file.</span>
        <div class="notice-actions">
          <button class="primary small-btn" onclick={applyDraft}>Restore Draft</button>
          <button class="small-btn" onclick={discardDraft}>Discard</button>
        </div>
      </div>
    {/if}

    {#if saveErrorMsg}
      <div class="notice-bar error">
        <span>{saveErrorMsg}</span>
      </div>
    {/if}

    {#if fileData.kind === 'binary'}
      <div class="state-box">
        <p class="muted">Binary file cannot be displayed in the editor.</p>
        <button onclick={handleOpenExternal}>
          <ExternalLink size={14} strokeWidth={2} />
          <span>Open in default application</span>
        </button>
      </div>
    {:else if fileData.kind === 'image' || (fileData.kind === 'svg' && svgMode === 'preview')}
      <div class="viewer-body">
        <ImageTab {path} sizeBytes={fileData.sizeBytes} />
      </div>
    {:else}
      <div class="editor-host" bind:this={editorHost}></div>
    {/if}
  {/if}
</div>

<style>
  .file-tab-container {
    display: flex;
    flex-direction: column;
    width: 100%;
    height: 100%;
    min-height: 0;
    background: var(--bg);
  }

  .state-box {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    padding: 24px;
  }

  .sub-bar {
    height: 32px;
    padding: 0 16px;
    background: var(--pn);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    font-size: 12px;
  }

  .sub-bar-label {
    color: var(--dm);
  }

  .segmented-switch {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .seg-btn {
    height: 22px;
    padding: 0 10px;
    font-size: 12px;
    border: none;
    color: var(--dm);
    background: transparent;
  }

  .seg-btn.active {
    background: var(--acs);
    color: var(--ac);
  }

  .notice-bar {
    min-height: 30px;
    padding: 4px 16px;
    background: var(--pn);
    border-bottom: 1px solid var(--bd);
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    font-size: 12px;
    color: var(--dm);
  }

  .notice-bar.warning {
    color: var(--mo);
  }

  .notice-bar.draft {
    color: var(--ac);
  }

  .notice-bar.error {
    color: var(--de);
  }

  .notice-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .small-btn {
    height: 22px;
    padding: 0 8px;
    font-size: 12px;
  }

  .viewer-body {
    flex: 1;
    min-height: 0;
    width: 100%;
    overflow: hidden;
  }

  .editor-host {
    flex: 1;
    min-height: 0;
    width: 100%;
    overflow: hidden;
    contain: strict;
  }

  .muted {
    color: var(--dm);
  }

  .error-text {
    color: var(--de);
    font-size: 12px;
  }
</style>
