import { describe, expect, it } from 'vitest';
import { clampMenuCoordinates } from './menu';

describe('clampMenuCoordinates for floating context menu', () => {
  it('returns click position when menu is well within viewport bounds', () => {
    const pos = clampMenuCoordinates(100, 150, 180, 120, 1024, 768);
    expect(pos).toEqual({ x: 100, y: 150 });
  });

  it('clamps x coordinate when menu would overflow past right window boundary', () => {
    // 950 + 180 = 1130 > 1024 - 8 (1016)
    // expected x = 1024 - 180 - 8 = 836
    const pos = clampMenuCoordinates(950, 100, 180, 120, 1024, 768);
    expect(pos.x).toBe(836);
    expect(pos.y).toBe(100);
  });

  it('clamps y coordinate when menu would overflow past bottom window boundary', () => {
    // 700 + 120 = 820 > 768 - 8 (760)
    // expected y = 768 - 120 - 8 = 640
    const pos = clampMenuCoordinates(100, 700, 180, 120, 1024, 768);
    expect(pos.x).toBe(100);
    expect(pos.y).toBe(640);
  });

  it('clamps both x and y when right-clicking at the bottom-right corner', () => {
    const pos = clampMenuCoordinates(1020, 760, 180, 120, 1024, 768);
    expect(pos.x).toBe(836);
    expect(pos.y).toBe(640);
  });

  it('clamps to minimum padding when click is negative or smaller than padding', () => {
    const pos = clampMenuCoordinates(2, -10, 180, 120, 1024, 768);
    expect(pos.x).toBe(8);
    expect(pos.y).toBe(8);
  });

  it('respects custom padding parameter', () => {
    const pos = clampMenuCoordinates(990, 750, 180, 120, 1000, 800, 16);
    // x = 1000 - 180 - 16 = 804
    // y = 800 - 120 - 16 = 664
    expect(pos).toEqual({ x: 804, y: 664 });
  });

  it('gracefully handles viewport smaller than menu dimensions without negative values', () => {
    const pos = clampMenuCoordinates(50, 50, 200, 200, 150, 150, 8);
    expect(pos.x).toBe(8);
    expect(pos.y).toBe(8);
  });

  it('keeps exact boundary position when click matches available viewport width exactly', () => {
    // windowWidth (1000) - padding (8) - menuWidth (180) = 812
    // 812 + 180 = 992 === 1000 - 8
    const pos = clampMenuCoordinates(812, 662, 180, 130, 1000, 800, 8);
    expect(pos).toEqual({ x: 812, y: 662 });
  });

  it('handles fractional subpixel coordinates cleanly', () => {
    const pos = clampMenuCoordinates(120.5, 230.25, 180, 120, 1024, 768);
    expect(pos.x).toBe(120.5);
    expect(pos.y).toBe(230.25);
  });

  it('clamps coordinates exceeding window dimensions', () => {
    const pos = clampMenuCoordinates(1500, 1200, 180, 120, 1024, 768);
    expect(pos.x).toBe(836);
    expect(pos.y).toBe(640);
  });
});
