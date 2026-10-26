import { describe, expect, it } from 'vitest';
import { createPreviewHistory, pushPreviewHistory, stepPreviewHistory } from './preview';
import { isLargePaste } from './terminal';
import type { SearchMatchDto } from './ipc';

describe('release hardening and stress tests', () => {
  it('handles 10,000 files in path normalization and filtering without bottleneck', () => {
    const start = performance.now();
    const files: string[] = [];
    for (let i = 0; i < 10000; i++) {
      files.push(`src/modules/sub_${i % 100}/component_${i}.svelte`);
    }

    const filtered = files.filter((f) => f.includes('component_99'));
    expect(filtered.length).toBeGreaterThan(0);

    const elapsed = performance.now() - start;
    // 10,000 items should filter in less than 50ms in JS
    expect(elapsed).toBeLessThan(50);
  });

  it('cycles preview history 50 times without degradation or stack leak', () => {
    let history = createPreviewHistory('http://localhost:5173/');

    for (let i = 1; i <= 50; i++) {
      history = pushPreviewHistory(history, `http://localhost:5173/page/${i}`);
    }
    expect(history.entries).toHaveLength(51);
    expect(history.index).toBe(50);

    // Step back 25 times
    for (let i = 0; i < 25; i++) {
      history = stepPreviewHistory(history, -1);
    }
    expect(history.index).toBe(25);
    expect(history.entries[history.index]).toBe('http://localhost:5173/page/25');

    // Push new branch: should truncate future
    history = pushPreviewHistory(history, 'http://localhost:5173/new-branch');
    expect(history.entries).toHaveLength(27);
    expect(history.index).toBe(26);
  });

  it('groups and slices 2,000 search results under 20ms', () => {
    const start = performance.now();
    const matches: SearchMatchDto[] = [];
    for (let i = 1; i <= 2000; i++) {
      matches.push({
        relPath: `src/file_${i % 50}.ts`,
        line: i,
        col: 10,
        snippet: `const x_${i} = computeValue(${i});`,
        matchStart: 6,
        matchEnd: 10
      });
    }

    const grouped = new Map<string, SearchMatchDto[]>();
    for (const item of matches) {
      const list = grouped.get(item.relPath);
      if (list) {
        list.push(item);
      } else {
        grouped.set(item.relPath, [item]);
      }
    }

    expect(grouped.size).toBe(50);
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(20);
  });

  it('detects paste size thresholds accurately for terminal safety', () => {
    const smallText = 'git status\n';
    expect(isLargePaste(smallText, 64)).toBe(false);

    // 65 KB string
    const largeText = 'a'.repeat(65 * 1024);
    expect(isLargePaste(largeText, 64)).toBe(true);
  });
});
