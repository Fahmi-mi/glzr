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
