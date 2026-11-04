import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  attachSmoothScroll,
  clampScroll,
  computeScrollStep,
  isTrackpadEvent,
  resolveDeltaPixels,
  smoothScroll
} from './scroll';

describe('kinetic scrolling mechanics', () => {
  it('identifies discrete mouse wheel vs high resolution trackpad events', () => {
    expect(isTrackpadEvent({ deltaMode: 1, deltaY: 3, deltaX: 0 })).toBe(false);
    expect(isTrackpadEvent({ deltaMode: 2, deltaY: 1, deltaX: 0 })).toBe(false);
    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 100, deltaX: 0 })).toBe(false);
    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 53, deltaX: 0 })).toBe(false);

    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 2.75, deltaX: 0 })).toBe(true);
    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 6, deltaX: 0 })).toBe(true);
    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 0, deltaX: 1.25 })).toBe(true);
    expect(isTrackpadEvent({ deltaMode: 0, deltaY: 0, deltaX: 5 })).toBe(true);
  });

  it('preserves trackpad classification during rapid flick transitions within gesture window', () => {
    const startTime = 1000;
    const initialTp = isTrackpadEvent({ deltaMode: 0, deltaY: 4, deltaX: 0 }, 0, startTime);
    expect(initialTp).toBe(true);

    const flickWithinWindow = isTrackpadEvent(
      { deltaMode: 0, deltaY: 45, deltaX: 0 },
      startTime,
      startTime + 50
    );
    expect(flickWithinWindow).toBe(true);

    const flickAfterWindow = isTrackpadEvent(
      { deltaMode: 0, deltaY: 45, deltaX: 0 },
      startTime,
      startTime + 300
    );
    expect(flickAfterWindow).toBe(false);
  });

  it('resolves delta pixel scaling accurately for lines, pages, and pixels', () => {
    const pixelEvent = { deltaMode: 0, deltaY: 120, deltaX: 40 };
    expect(resolveDeltaPixels(pixelEvent, 24, 600, 800)).toEqual({ dy: 120, dx: 40 });

    const lineEvent = { deltaMode: 1, deltaY: 3, deltaX: 1 };
    expect(resolveDeltaPixels(lineEvent, 24, 600, 800)).toEqual({ dy: 72, dx: 24 });

    const pageEvent = { deltaMode: 2, deltaY: 1, deltaX: -1 };
    expect(resolveDeltaPixels(pageEvent, 24, 600, 800)).toEqual({ dy: 600, dx: -800 });
  });

  it('clamps scroll targets strictly within container boundaries', () => {
    expect(clampScroll(-50, 1000)).toBe(0);
    expect(clampScroll(250, 1000)).toBe(250);
    expect(clampScroll(1500, 1000)).toBe(1000);
    expect(clampScroll(50, 0)).toBe(0);
    expect(clampScroll(-10, -50)).toBe(0);
  });

  it('computes exponential easing steps and converges to target within 120ms', () => {
    let current = 0;
    const target = 100;
    const dt = 16.6;
    const tau = 20;

    const firstStep = computeScrollStep(current, target, dt, tau);
    expect(firstStep.done).toBe(false);
    expect(firstStep.next).toBeGreaterThan(50);
    expect(firstStep.next).toBeLessThan(60);
    current = firstStep.next;

    let steps = 1;
    let completed = false;
    for (let i = 0; i < 15; i++) {
      const step = computeScrollStep(current, target, dt, tau);
      current = step.next;
      steps += 1;
      if (step.done) {
        completed = true;
        break;
      }
    }

    expect(completed).toBe(true);
    expect(current).toBe(target);
    expect(steps).toBeLessThanOrEqual(9);
  });

  it('snaps to target when remaining distance is under half a pixel', () => {
    const step = computeScrollStep(99.6, 100, 16.6, 20);
    expect(step.done).toBe(true);
    expect(step.next).toBe(100);
  });
});

describe('attachSmoothScroll DOM action', () => {
  let originalRaf: typeof requestAnimationFrame;
  let originalCaf: typeof cancelAnimationFrame;
  let rafCallbacks: Map<number, FrameRequestCallback>;
  let nextRafId: number;

  beforeEach(() => {
    rafCallbacks = new Map();
    nextRafId = 1;
    originalRaf = globalThis.requestAnimationFrame;
    originalCaf = globalThis.cancelAnimationFrame;

    globalThis.requestAnimationFrame = vi.fn((cb: FrameRequestCallback) => {
      const id = nextRafId++;
      rafCallbacks.set(id, cb);
      return id;
    });

    globalThis.cancelAnimationFrame = vi.fn((id: number) => {
      rafCallbacks.delete(id);
    });
  });

  afterEach(() => {
    globalThis.requestAnimationFrame = originalRaf;
    globalThis.cancelAnimationFrame = originalCaf;
  });

  function createMockElement(overrides?: Partial<HTMLElement>) {
    const listeners: Record<string, ((e: any) => void)[]> = {};
    const el = {
      scrollTop: 0,
      scrollLeft: 0,
      scrollHeight: 1000,
      clientHeight: 400,
      scrollWidth: 500,
      clientWidth: 500,
      addEventListener: vi.fn((type: string, handler: (e: any) => void) => {
        if (!listeners[type]) listeners[type] = [];
        listeners[type].push(handler);
      }),
      removeEventListener: vi.fn((type: string, handler: (e: any) => void) => {
        if (listeners[type]) {
          listeners[type] = listeners[type].filter((h) => h !== handler);
        }
      }),
      dispatchEvent: (event: { type: string }) => {
        const handlers = listeners[event.type] || [];
        for (const h of handlers) h(event);
      },
      ...overrides
    } as unknown as HTMLElement & { dispatchEvent: (event: any) => void };
    return el;
  }

  function flushAllFrames(startTime = 100) {
    let now = startTime;
    let iterations = 0;
    while (rafCallbacks.size > 0 && iterations < 50) {
      iterations++;
      now += 16.6;
      const current = Array.from(rafCallbacks.entries());
      rafCallbacks.clear();
      for (const [, cb] of current) {
        cb(now);
      }
    }
  }

  it('intercepts discrete wheel, smoothly glides scrollTop, and invokes activity callback', () => {
    let activityCount = 0;
    const el = createMockElement();
    const handle = attachSmoothScroll(el, {
      onScrollActivity: () => {
        activityCount += 1;
      }
    });

    let prevented = false;
    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 100,
      deltaX: 0,
      preventDefault: () => {
        prevented = true;
      }
    });

    expect(prevented).toBe(true);
    expect(activityCount).toBeGreaterThanOrEqual(1);
    expect(rafCallbacks.size).toBe(1);

    flushAllFrames();

    expect(el.scrollTop).toBe(100);
    expect(rafCallbacks.size).toBe(0);
    expect(activityCount).toBeGreaterThan(1);

    handle.destroy();
  });

  it('passes through trackpad wheel events without kinetic interpolation', () => {
    let activityCalled = false;
    const el = createMockElement();
    const handle = attachSmoothScroll(el, {
      onScrollActivity: () => {
        activityCalled = true;
      }
    });

    let prevented = false;
    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 2.5,
      deltaX: 0,
      preventDefault: () => {
        prevented = true;
      }
    });

    expect(prevented).toBe(false);
    expect(activityCalled).toBe(true);
    expect(rafCallbacks.size).toBe(0);

    handle.destroy();
  });

  it('ignores wheel events with zero deltas', () => {
    const el = createMockElement();
    const handle = attachSmoothScroll(el);

    let prevented = false;
    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 0,
      deltaX: 0,
      preventDefault: () => {
        prevented = true;
      }
    });

    expect(prevented).toBe(false);
    expect(rafCallbacks.size).toBe(0);

    handle.destroy();
  });

  it('does not prevent default when element cannot scroll', () => {
    const el = createMockElement({
      scrollHeight: 400,
      clientHeight: 400,
      scrollWidth: 500,
      clientWidth: 500
    });
    const handle = attachSmoothScroll(el);

    let prevented = false;
    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 100,
      deltaX: 0,
      preventDefault: () => {
        prevented = true;
      }
    });

    expect(prevented).toBe(false);
    expect(rafCallbacks.size).toBe(0);

    handle.destroy();
  });

  it('cancels kinetic animation immediately upon pointerdown', () => {
    const el = createMockElement();
    const handle = attachSmoothScroll(el);

    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 100,
      deltaX: 0,
      preventDefault: () => {}
    });

    expect(rafCallbacks.size).toBe(1);

    el.dispatchEvent({ type: 'pointerdown' });

    expect(rafCallbacks.size).toBe(0);

    handle.destroy();
  });

  it('cancels kinetic animation when external native scroll differs from kinetic step', () => {
    const el = createMockElement();
    const handle = attachSmoothScroll(el);

    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 100,
      deltaX: 0,
      preventDefault: () => {}
    });

    expect(rafCallbacks.size).toBe(1);

    el.scrollTop = 350;
    el.dispatchEvent({ type: 'scroll' });

    expect(rafCallbacks.size).toBe(0);

    handle.destroy();
  });

  it('cleans up event listeners and stops animation on destroy', () => {
    const el = createMockElement();
    const handle = smoothScroll(el);

    el.dispatchEvent({
      type: 'wheel',
      deltaMode: 0,
      deltaY: 100,
      deltaX: 0,
      preventDefault: () => {}
    });

    expect(rafCallbacks.size).toBe(1);

    handle.destroy();

    expect(rafCallbacks.size).toBe(0);
    expect(el.removeEventListener).toHaveBeenCalledWith('wheel', expect.any(Function));
    expect(el.removeEventListener).toHaveBeenCalledWith('scroll', expect.any(Function));
    expect(el.removeEventListener).toHaveBeenCalledWith('pointerdown', expect.any(Function));
    expect(el.removeEventListener).toHaveBeenCalledWith('touchstart', expect.any(Function));
  });
});
