import type { DiffHunkDto, GitBadge, GitStatusDto } from '$lib/ipc';

export interface ParsedDiffLine {
  kind: 'add' | 'del' | 'ctx' | 'meta';
  oldLine: number | null;
  newLine: number | null;
  text: string;
}

export function parseHunkLines(hunk: DiffHunkDto): ParsedDiffLine[] {
  const rawLines = hunk.content.split('\n');
  const out: ParsedDiffLine[] = [];
  let oldNum = hunk.oldStart;
  let newNum = hunk.newStart;

  for (let i = 0; i < rawLines.length; i++) {
    const line = rawLines[i];
    if (i === rawLines.length - 1 && line === '') {
      continue;
    }
    if (line.startsWith('@@ ')) {
      continue;
    }
    if (line.startsWith('\\')) {
      out.push({
        kind: 'meta',
        oldLine: null,
        newLine: null,
        text: line
      });
      continue;
    }
    const prefix = line[0] ?? ' ';
    const body = line.slice(1);
    if (prefix === '+') {
      out.push({
        kind: 'add',
        oldLine: null,
        newLine: newNum,
        text: body
      });
      newNum += 1;
    } else if (prefix === '-') {
      out.push({
        kind: 'del',
        oldLine: oldNum,
        newLine: null,
        text: body
      });
      oldNum += 1;
    } else {
      out.push({
        kind: 'ctx',
        oldLine: oldNum,
        newLine: newNum,
        text: body
      });
      oldNum += 1;
      newNum += 1;
    }
  }

  return out;
}

export function badgeColorVar(badge: GitBadge): string {
  switch (badge) {
    case 'A':
    case '?':
      return 'var(--ad)';
    case 'D':
    case '!':
      return 'var(--de)';
    case 'M':
    case 'R':
    default:
      return 'var(--mo)';
  }
}

export function deriveUnstagedFolderPaths(status: GitStatusDto | null): Set<string> {
  const dirty = new Set<string>();
  if (!status?.isRepo) return dirty;
  const entries = [...status.unstaged, ...status.untracked, ...status.conflicted];
  for (const entry of entries) {
    const clean = entry.path.replace(/\\/g, '/').replace(/^\/+/, '');
    const parts = clean.split('/').filter(Boolean);
    parts.pop();
    let acc = '';
    for (const part of parts) {
      acc = acc ? `${acc}/${part}` : part;
      dirty.add(acc);
    }
  }
  return dirty;
}

