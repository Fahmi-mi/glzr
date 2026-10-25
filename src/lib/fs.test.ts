import { describe, expect, it } from 'vitest';
import { normalizeTabPath } from './ipc';

describe('fs path normalization for single-path-per-tab rule', () => {
  it('normalizes redundant slashes and dot segments', () => {
    expect(normalizeTabPath('/home/fahmi//glzr/./src/../src/main.rs')).toBe(
      '/home/fahmi/glzr/src/main.rs'
    );
    expect(normalizeTabPath('src//lib/./ipc.ts')).toBe('src/lib/ipc.ts');
  });
});
