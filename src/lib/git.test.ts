import { describe, expect, it } from 'vitest';
import { badgeColorVar, parseHunkLines } from './git';
import type { DiffHunkDto } from './ipc';

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
});
