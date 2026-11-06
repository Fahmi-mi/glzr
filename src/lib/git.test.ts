import { describe, expect, it } from 'vitest';
import { badgeColorVar, deriveUnstagedFolderPaths, parseHunkLines } from './git';
import type { DiffHunkDto, GitStatusDto } from './ipc';

describe('git hunk line parser and badge helpers', () => {
  it('computes accurate old and new line numbers across context, additions, and deletions', () => {
    const hunk: DiffHunkDto = {
      index: 0,
      header: '@@ -10,4 +10,5 @@',
      oldStart: 10,
      oldLines: 4,
      newStart: 10,
      newLines: 5,
      content: [
        '@@ -10,4 +10,5 @@',
        ' unchanged 1',
        '-removed line',
        '+added line 1',
        '+added line 2',
        ' unchanged 2',
        '\\ No newline at end of file',
        ''
      ].join('\n')
    };

    const lines = parseHunkLines(hunk);
    expect(lines).toEqual([
      { kind: 'ctx', oldLine: 10, newLine: 10, text: 'unchanged 1' },
      { kind: 'del', oldLine: 11, newLine: null, text: 'removed line' },
      { kind: 'add', oldLine: null, newLine: 11, text: 'added line 1' },
      { kind: 'add', oldLine: null, newLine: 12, text: 'added line 2' },
      { kind: 'ctx', oldLine: 12, newLine: 13, text: 'unchanged 2' },
      { kind: 'meta', oldLine: null, newLine: null, text: '\\ No newline at end of file' }
    ]);
  });

  it('maps git status badges to design token CSS variables', () => {
    expect(badgeColorVar('M')).toBe('var(--mo)');
    expect(badgeColorVar('A')).toBe('var(--ad)');
    expect(badgeColorVar('?')).toBe('var(--ad)');
    expect(badgeColorVar('D')).toBe('var(--de)');
    expect(badgeColorVar('!')).toBe('var(--de)');
  });

  it('derives ancestor folders for unstaged, untracked, and conflicted files', () => {
    const mockStatus: GitStatusDto = {
      gitAvailable: true,
      isRepo: true,
      branch: 'main',
      upstream: null,
      ahead: 0,
      behind: 0,
      detached: false,
      merging: false,
      rebasing: false,
      hasConflicts: false,
      staged: [
        { path: 'src/lib/staged_only.ts', origPath: null, badge: 'A', staged: true },
        { path: 'docs/guide.md', origPath: null, badge: 'M', staged: true }
      ],
      unstaged: [
        { path: 'src/lib/components/sidebar/GitPanel.svelte', origPath: null, badge: 'M', staged: false }
      ],
      untracked: [
        { path: 'tests/unit/example.test.ts', origPath: null, badge: '?', staged: false },
        { path: 'root_file.txt', origPath: null, badge: '?', staged: false }
      ],
      conflicted: [
        { path: 'config/local/app.json', origPath: null, badge: '!', staged: false }
      ]
    };

    const dirty = deriveUnstagedFolderPaths(mockStatus);
    expect(dirty.has('src')).toBe(true);
    expect(dirty.has('src/lib')).toBe(true);
    expect(dirty.has('src/lib/components')).toBe(true);
    expect(dirty.has('src/lib/components/sidebar')).toBe(true);
    expect(dirty.has('tests')).toBe(true);
    expect(dirty.has('tests/unit')).toBe(true);
    expect(dirty.has('config')).toBe(true);
    expect(dirty.has('config/local')).toBe(true);

    expect(dirty.has('docs')).toBe(false);
  });

  it('returns empty set if all changes are fully staged', () => {
    const stagedStatus: GitStatusDto = {
      gitAvailable: true,
      isRepo: true,
      branch: 'main',
      upstream: null,
      ahead: 0,
      behind: 0,
      detached: false,
      merging: false,
      rebasing: false,
      hasConflicts: false,
      staged: [
        { path: 'src/lib/a.ts', origPath: null, badge: 'M', staged: true },
        { path: 'src/b.ts', origPath: null, badge: 'A', staged: true }
      ],
      unstaged: [],
      untracked: [],
      conflicted: []
    };

    const dirty = deriveUnstagedFolderPaths(stagedStatus);
    expect(dirty.size).toBe(0);
    expect(dirty.has('src')).toBe(false);
    expect(dirty.has('src/lib')).toBe(false);
  });

  it('handles null status and non-repo status gracefully', () => {
    expect(deriveUnstagedFolderPaths(null).size).toBe(0);

    const nonRepoStatus: GitStatusDto = {
      gitAvailable: true,
      isRepo: false,
      branch: null,
      upstream: null,
      ahead: 0,
      behind: 0,
      detached: false,
      merging: false,
      rebasing: false,
      hasConflicts: false,
      staged: [],
      unstaged: [{ path: 'foo/bar.txt', origPath: null, badge: 'M', staged: false }],
      untracked: [],
      conflicted: []
    };
    expect(deriveUnstagedFolderPaths(nonRepoStatus).size).toBe(0);
  });
});

