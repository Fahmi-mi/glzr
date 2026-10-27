import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { open } from '@tauri-apps/plugin-dialog';

export interface AppErrorPayload {
  kind: string;
  message: string;
}

export interface ProjectStateDto {
  activeFolder: string | null;
  folderName: string | null;
  recentFolders: string[];
}

export type FileKind = 'text' | 'image' | 'svg' | 'binary';
export type EncodingKind = 'utf8' | 'utf8-bom' | 'non-utf8';
export type LineEnding = 'lf' | 'crlf';

export interface DirEntryDto {
  name: string;
  path: string;
  relPath: string;
  isDir: boolean;
  isSymlink: boolean;
}

export interface FileReadDto {
  path: string;
  relPath: string;
  name: string;
  kind: FileKind;
  content: string | null;
  sizeBytes: number;
  mtimeMs: number;
  encoding: EncodingKind;
  lineEnding: LineEnding;
  hasMixedLineEndings: boolean;
  isLargeFile: boolean;
  hasLongLines: boolean;
  disableHighlight: boolean;
  readOnly: boolean;
}

export interface WriteResultDto {
  path: string;
  mtimeMs: number;
  changed: boolean;
}

export interface DraftDto {
  path: string;
  content: string;
  savedAtMs: number;
}

export type GitBadge = 'M' | 'A' | 'D' | 'R' | '?' | '!';

export interface GitFileEntry {
  path: string;
  origPath: string | null;
  badge: GitBadge;
  staged: boolean;
}

export interface GitStatusDto {
  gitAvailable: boolean;
  isRepo: boolean;
  branch: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  detached: boolean;
  merging: boolean;
  rebasing: boolean;
  hasConflicts: boolean;
  staged: GitFileEntry[];
  unstaged: GitFileEntry[];
  untracked: GitFileEntry[];
  conflicted: GitFileEntry[];
}

export interface DiffHunkDto {
  index: number;
  header: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  content: string;
}

export interface FileDiffDto {
  fileHeader: string;
  hunks: DiffHunkDto[];
}

export interface GitDiffDto {
  path: string;
  staged: boolean;
  isBinary: boolean;
  tooLarge: boolean;
  rawDiff: string;
  fileDiff: FileDiffDto | null;
}

export interface GitBranchesDto {
  current: string | null;
  branches: string[];
}

export interface PtySpawnDto {
  sessionId: string;
  program: string;
  cwd: string;
  cols: number;
  rows: number;
}

export interface SessionLoadDto {
  rawJson: string | null;
  backedUpCorrupt: boolean;
}

export interface AppearanceSettingsDto {
  theme: 'dark' | 'light';
  fontFamily: 'system-mono' | 'jetbrains-mono-nerd';
  fontSize: number;
}

export interface EditorSettingsDto {
  tabSize: number;
  insertSpaces: boolean;
  wordWrap: boolean;
  lineNumbers: boolean;
  largeFileLimitMb: number;
  longLineLimit: number;
  draftAutosaveSeconds: number;
}

export interface TerminalSettingsDto {
  shell: string;
  shellArgs: string[];
  extraEnv: Record<string, string>;
  scrollback: number;
  cursorStyle: string;
  confirmPasteOverKb: number;
}

export interface KeybindingsSettingsDto {
  commandPalette: string;
  quickOpen: string;
  sidebarFiles: string;
  sidebarGit: string;
  sidebarSearch: string;
  newTerminal: string;
  closeTab: string;
  splitRight: string;
  splitDown: string;
  closePane: string;
  maximizePane: string;
  terminalFind: string;
}

export interface AppSettingsDto {
  appearance: AppearanceSettingsDto;
  editor: EditorSettingsDto;
  terminal: TerminalSettingsDto;
  files: { ignore: string[] };
  git: { commandTimeoutSeconds: number; diffSizeLimitMb: number };
  search: { maxResults: number; snippetMaxChars: number; debounceMs: number };
  preview: { allowedHosts: string[]; autoDetectPort: boolean };
  session: { restore: boolean; recentFoldersLimit: number };
  logging: { level: string };
  keybindings: KeybindingsSettingsDto;
}

export interface SettingsLoadDto {
  settings: AppSettingsDto;
  warnings: string[];
}

export function createDefaultSettings(): AppSettingsDto {
  return {
    appearance: {
      theme: 'dark',
      fontFamily: 'system-mono',
      fontSize: 14
    },
    editor: {
      tabSize: 4,
      insertSpaces: true,
      wordWrap: false,
      lineNumbers: true,
      largeFileLimitMb: 5,
      longLineLimit: 10000,
      draftAutosaveSeconds: 5
    },
    terminal: {
      shell: '',
      shellArgs: ['-l', '-i'],
      extraEnv: {},
      scrollback: 5000,
      cursorStyle: 'block',
      confirmPasteOverKb: 64
    },
    files: {
      ignore: ['.git', 'node_modules', 'target']
    },
    git: {
      commandTimeoutSeconds: 30,
      diffSizeLimitMb: 2
    },
    search: {
      maxResults: 2000,
      snippetMaxChars: 300,
      debounceMs: 200
    },
    preview: {
      allowedHosts: ['localhost', '127.0.0.1'],
      autoDetectPort: false
    },
    session: {
      restore: true,
      recentFoldersLimit: 5
    },
    logging: {
      level: 'info'
    },
    keybindings: {
      commandPalette: 'Alt+P',
      quickOpen: 'Alt+O',
      sidebarFiles: 'Alt+E',
      sidebarGit: 'Alt+V',
      sidebarSearch: 'Alt+G',
      newTerminal: 'Alt+T',
      closeTab: 'Alt+W',
      splitRight: 'Alt+R',
      splitDown: 'Alt+J',
      closePane: 'Alt+Q',
      maximizePane: 'Alt+M',
      terminalFind: 'Alt+S'
    }
  };
}

export interface SearchMatchDto {
  relPath: string;
  line: number;
  col: number;
  snippet: string;
  matchStart: number;
  matchEnd: number;
}

export interface BackendEventMap {
  'pty:output': { sessionId: string; data: string };
  'pty:exit': { sessionId: string; code: number | null };
  'fs:changed': { paths: string[] };
  'fs:watch-error': { message: string };
  'search:results': { queryId: string; items: SearchMatchDto[] };
  'search:done': { queryId: string; truncated: boolean };
  'search:error': { queryId: string; message: string };
}

export function normalizeIpcError(err: unknown): AppErrorPayload {
  if (typeof err === 'object' && err !== null && 'kind' in err && 'message' in err) {
    const typed = err as { kind: unknown; message: unknown };
    return {
      kind: String(typed.kind),
      message: String(typed.message)
    };
  }
  if (err instanceof Error) {
    return { kind: 'error', message: err.message };
  }
  return { kind: 'unknown', message: String(err) };
}

export function normalizeTabPath(rawPath: string): string {
  const unified = rawPath.replace(/\\/g, '/').trim();
  const segments: string[] = [];
  const isAbs = unified.startsWith('/');

  for (const part of unified.split('/')) {
    if (!part || part === '.') {
      continue;
    }
    if (part === '..') {
      segments.pop();
    } else {
      segments.push(part);
    }
  }

  return (isAbs ? '/' : '') + segments.join('/');
}

async function callCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (err) {
    throw normalizeIpcError(err);
  }
}

export async function projectGetState(): Promise<ProjectStateDto> {
  return callCommand<ProjectStateDto>('project_get_state');
}

export async function projectOpenFolder(path: string): Promise<ProjectStateDto> {
  return callCommand<ProjectStateDto>('project_open_folder', { path });
}

export async function projectRemoveRecent(path: string): Promise<ProjectStateDto> {
  return callCommand<ProjectStateDto>('project_remove_recent', { path });
}

export async function pickProjectFolderDialog(): Promise<string | null> {
  const selected = await open({
    directory: true,
    multiple: false,
    title: 'Open Project Folder'
  });
  return typeof selected === 'string' ? selected : null;
}

export async function fsListDir(dirPath?: string): Promise<DirEntryDto[]> {
  return callCommand<DirEntryDto[]>('fs_list_dir', { dirPath: dirPath ?? null });
}

export async function fsReadFile(path: string): Promise<FileReadDto> {
  return callCommand<FileReadDto>('fs_read_file', { path });
}

export async function fsWriteFile(
  path: string,
  content: string,
  encoding: EncodingKind,
  lineEnding: LineEnding
): Promise<WriteResultDto> {
  return callCommand<WriteResultDto>('fs_write_file', {
    path,
    content,
    encoding,
    lineEnding
  });
}

export async function fsCreateFile(path: string): Promise<string> {
  return callCommand<string>('fs_create_file', { path });
}

export async function fsCreateDir(path: string): Promise<string> {
  return callCommand<string>('fs_create_dir', { path });
}

export async function fsRename(fromPath: string, toPath: string): Promise<string> {
  return callCommand<string>('fs_rename', { fromPath, toPath });
}

export async function fsTrash(path: string): Promise<void> {
  return callCommand<void>('fs_trash', { path });
}

export async function fsListWorkspaceFiles(): Promise<string[]> {
  return callCommand<string[]>('fs_list_workspace_files');
}

export async function fsReadImageDataUrl(path: string): Promise<string> {
  return callCommand<string>('fs_read_image_data_url', { path });
}

export async function fsSaveDraft(path: string, content: string): Promise<DraftDto> {
  return callCommand<DraftDto>('fs_save_draft', { path, content });
}

export async function fsLoadDraft(path: string): Promise<DraftDto | null> {
  return callCommand<DraftDto | null>('fs_load_draft', { path });
}

export async function fsClearDraft(path: string): Promise<void> {
  return callCommand<void>('fs_clear_draft', { path });
}

export async function fsWatch(): Promise<void> {
  return callCommand<void>('fs_watch');
}

export async function fsUnwatch(): Promise<void> {
  return callCommand<void>('fs_unwatch');
}

export async function searchStart(opts: {
  queryId: string;
  pattern: string;
  caseSensitive: boolean;
  regex: boolean;
  glob?: string | null;
}): Promise<void> {
  return callCommand<void>('search_start', {
    queryId: opts.queryId,
    pattern: opts.pattern,
    caseSensitive: opts.caseSensitive,
    regex: opts.regex,
    glob: opts.glob ?? null
  });
}

export async function searchCancel(): Promise<void> {
  return callCommand<void>('search_cancel');
}

export async function gitStatus(): Promise<GitStatusDto> {
  return callCommand<GitStatusDto>('git_status');
}

export async function gitDiff(path: string, staged: boolean): Promise<GitDiffDto> {
  return callCommand<GitDiffDto>('git_diff', { path, staged });
}

export async function gitHunks(path: string, staged: boolean): Promise<DiffHunkDto[]> {
  return callCommand<DiffHunkDto[]>('git_hunks', { path, staged });
}

export async function gitStage(path: string): Promise<void> {
  return callCommand<void>('git_stage', { path });
}

export async function gitUnstage(path: string): Promise<void> {
  return callCommand<void>('git_unstage', { path });
}

export async function gitDiscard(path: string): Promise<void> {
  return callCommand<void>('git_discard', { path });
}

export async function gitStageHunk(path: string, hunkIndex: number): Promise<void> {
  return callCommand<void>('git_stage_hunk', { path, hunkIndex });
}

export async function gitUnstageHunk(path: string, hunkIndex: number): Promise<void> {
  return callCommand<void>('git_unstage_hunk', { path, hunkIndex });
}

export async function gitDiscardHunk(path: string, hunkIndex: number): Promise<void> {
  return callCommand<void>('git_discard_hunk', { path, hunkIndex });
}

export async function gitCommit(message: string): Promise<void> {
  return callCommand<void>('git_commit', { message });
}

export async function gitBranches(): Promise<GitBranchesDto> {
  return callCommand<GitBranchesDto>('git_branches');
}

export async function gitCheckout(branch: string): Promise<void> {
  return callCommand<void>('git_checkout', { branch });
}

export async function gitPull(): Promise<void> {
  return callCommand<void>('git_pull');
}

export async function gitPush(): Promise<void> {
  return callCommand<void>('git_push');
}

export async function ptySpawn(
  sessionId?: string,
  cols?: number,
  rows?: number
): Promise<PtySpawnDto> {
  return callCommand<PtySpawnDto>('pty_spawn', {
    sessionId: sessionId ?? null,
    cols: cols ?? null,
    rows: rows ?? null
  });
}

export async function ptyWrite(sessionId: string, data: string): Promise<void> {
  return callCommand<void>('pty_write', { sessionId, data });
}

export async function ptyResize(sessionId: string, cols: number, rows: number): Promise<void> {
  return callCommand<void>('pty_resize', { sessionId, cols, rows });
}

export async function ptyKill(sessionId: string): Promise<void> {
  return callCommand<void>('pty_kill', { sessionId });
}

export async function sessionLoad(): Promise<SessionLoadDto> {
  return callCommand<SessionLoadDto>('session_load');
}

export async function sessionSave(rawJson: string): Promise<void> {
  return callCommand<void>('session_save', { rawJson });
}

export async function settingsLoad(): Promise<SettingsLoadDto> {
  return callCommand<SettingsLoadDto>('settings_load');
}

export async function settingsSave(settings: AppSettingsDto): Promise<void> {
  return callCommand<void>('settings_save', { settings });
}

export async function revealInFileManager(path: string): Promise<void> {
  return callCommand<void>('reveal_in_file_manager', { path });
}

export async function openExternal(target: string): Promise<void> {
  return callCommand<void>('open_external', { target });
}

export async function previewCheckUrl(url: string): Promise<boolean> {
  return callCommand<boolean>('preview_check_url', { url });
}

export async function onBackendEvent<K extends keyof BackendEventMap>(
  event: K,
  handler: (payload: BackendEventMap[K]) => void
): Promise<UnlistenFn> {
  return listen<BackendEventMap[K]>(event, (evt) => {
    handler(evt.payload);
  });
}
