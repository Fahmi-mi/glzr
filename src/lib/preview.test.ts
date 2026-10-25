import { describe, expect, it } from 'vitest';
import { normalizePreviewUrl, resolveIframeSrc } from './preview';

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
});
