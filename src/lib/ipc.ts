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

export interface BackendEventMap {
  'pty:output': { sessionId: string; data: string };
  'pty:exit': { sessionId: string; code: number | null };
  'fs:changed': { paths: string[] };
  'fs:watch-error': { message: string };
  'search:results': { queryId: string; items: unknown[] };
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
    title: 'Buka folder proyek'
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

export async function revealInFileManager(path: string): Promise<void> {
  return callCommand<void>('reveal_in_file_manager', { path });
}

export async function openExternal(target: string): Promise<void> {
  return callCommand<void>('open_external', { target });
}

export async function onBackendEvent<K extends keyof BackendEventMap>(
  event: K,
  handler: (payload: BackendEventMap[K]) => void
): Promise<UnlistenFn> {
  return listen<BackendEventMap[K]>(event, (evt) => {
    handler(evt.payload);
  });
}
