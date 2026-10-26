import { describe, expect, it } from 'vitest';
import {
  createPreviewHistory,
  formatPreviewDisplayUrl,
  formatPreviewHostLabel,
  formatPreviewTabTitle,
  normalizePreviewUrl,
  pushPreviewHistory,
  resolveIframeSrc,
  stepPreviewHistory
} from './preview';

describe('preview localhost spike', () => {
  it('normalizes shorthand localhost and port inputs', () => {
    expect(normalizePreviewUrl(':5173')).toBe('http://localhost:5173/');
    expect(normalizePreviewUrl('localhost:8000/test')).toBe('http://localhost:8000/test');
    expect(normalizePreviewUrl('http://127.0.0.1:3000')).toBe('http://127.0.0.1:3000/');
  });

  it('rejects non-localhost hosts and unsafe protocols', () => {
    expect(normalizePreviewUrl('file:///etc/passwd')).toBeNull();
    expect(normalizePreviewUrl('javascript:alert(1)')).toBeNull();
    expect(normalizePreviewUrl('https://example.com')).toBeNull();
  });

  it('suspends inactive preview tabs to about:blank', () => {
    const valid = normalizePreviewUrl('http://localhost:5173');
    expect(resolveIframeSrc(valid, true)).toBe('http://localhost:5173/');
    expect(resolveIframeSrc(valid, false)).toBe('about:blank');
  });

  it('formats tab titles, host labels, and display URLs cleanly', () => {
    expect(formatPreviewTabTitle('http://localhost:5173/')).toBe(':5173');
    expect(formatPreviewTabTitle('http://localhost/')).toBe('localhost');
    expect(formatPreviewHostLabel('http://localhost:5173/dashboard')).toBe('localhost:5173');
    expect(formatPreviewDisplayUrl('http://localhost:5173/')).toBe('localhost:5173');
    expect(formatPreviewDisplayUrl('http://localhost:5173/docs?tab=1')).toBe(
      'localhost:5173/docs?tab=1'
    );
  });

  it('tracks back and forward navigation history', () => {
    let history = createPreviewHistory(':5173');
    expect(history.entries[history.index]).toBe('http://localhost:5173/');

    history = pushPreviewHistory(history, 'localhost:5173/about');
    history = pushPreviewHistory(history, ':3000/api');
    expect(history.entries).toHaveLength(3);
    expect(history.index).toBe(2);

    history = stepPreviewHistory(history, -1);
    expect(history.entries[history.index]).toBe('http://localhost:5173/about');

    history = stepPreviewHistory(history, 1);
    expect(history.entries[history.index]).toBe('http://localhost:3000/api');
  });
});
