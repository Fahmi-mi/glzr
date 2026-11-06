export interface SmoothScrollOptions {
  onScrollActivity?: () => void;
  tauMs?: number;
}

export function isTrackpadEvent(
  event: { deltaMode: number; deltaY: number; deltaX: number },
  lastTrackpadTime = 0,
  now = performance.now()
): boolean {
  if (event.deltaMode !== 0) {
    return false;
  }
  const hasFractional = !Number.isInteger(event.deltaY) || !Number.isInteger(event.deltaX);
  const isSmallDelta =
    (Math.abs(event.deltaY) > 0 && Math.abs(event.deltaY) < 16) ||
    (Math.abs(event.deltaX) > 0 && Math.abs(event.deltaX) < 16);
  if (hasFractional || isSmallDelta) {
    return true;
  }
  if (lastTrackpadTime > 0 && now - lastTrackpadTime < 160) {
    return true;
  }
  return false;
}

export function resolveDeltaPixels(
  event: { deltaMode: number; deltaY: number; deltaX: number },
  lineHeight = 24,
  pageHeight = 400,
  pageWidth = 400
): { dy: number; dx: number } {
  let dy = event.deltaY;
  let dx = event.deltaX;
  if (event.deltaMode === 1) {
    dy *= lineHeight;
    dx *= lineHeight;
  } else if (event.deltaMode === 2) {
    dy *= pageHeight;
    dx *= pageWidth;
  }
  return { dy, dx };
}

export function clampScroll(pos: number, maxScroll: number): number {
  return Math.max(0, Math.min(Math.max(0, maxScroll), pos));
}

export function computeScrollStep(
  current: number,
  target: number,
  dt: number,
  tau = 20
): { next: number; done: boolean } {
  const dist = target - current;
  if (Math.abs(dist) < 0.5) {
    return { next: target, done: true };
  }
  const factor = 1 - Math.exp(-dt / tau);
  const next = current + dist * factor;
  if (Math.abs(target - next) < 0.5) {
    return { next: target, done: true };
  }
  return { next, done: false };
}

export function attachSmoothScroll(
  node: HTMLElement,
  options?: SmoothScrollOptions
): { destroy: () => void } {
  let targetY = node.scrollTop;
  let targetX = node.scrollLeft;
  let expectedScrollTop = node.scrollTop;
  let expectedScrollLeft = node.scrollLeft;
  let rafId: number | null = null;
  let lastTime = 0;
  let lastTrackpadTime = 0;
  const tau = options?.tauMs ?? 20;

  let maxScrollTop = 0;
  let maxScrollLeft = 0;
  let cachedClientHeight = 0;
  let cachedClientWidth = 0;

  function updateDimensions() {
    cachedClientHeight = node.clientHeight;
    cachedClientWidth = node.clientWidth;
    maxScrollTop = Math.max(0, node.scrollHeight - cachedClientHeight);
    maxScrollLeft = Math.max(0, node.scrollWidth - cachedClientWidth);
  }

  updateDimensions();

  let resizeObserver: ResizeObserver | null = null;
  if (typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(() => {
      updateDimensions();
    });
    try {
      resizeObserver.observe(node);
    } catch {
      resizeObserver = null;
    }
  }

  function stopKineticScroll() {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
    targetY = node.scrollTop;
    targetX = node.scrollLeft;
    expectedScrollTop = node.scrollTop;
    expectedScrollLeft = node.scrollLeft;
  }

  function tick(now: number) {
    const dt = Math.max(1, Math.min(32, now - lastTime));
    lastTime = now;

    targetY = clampScroll(targetY, maxScrollTop);
    targetX = clampScroll(targetX, maxScrollLeft);

    const currentY = expectedScrollTop;
    const currentX = expectedScrollLeft;

    const resY = computeScrollStep(currentY, targetY, dt, tau);
    const resX = computeScrollStep(currentX, targetX, dt, tau);

    node.scrollTop = resY.next;
    node.scrollLeft = resX.next;
    expectedScrollTop = resY.next;
    expectedScrollLeft = resX.next;

    options?.onScrollActivity?.();

    if (resY.done && resX.done) {
      rafId = null;
      return;
    }

    rafId = requestAnimationFrame(tick);
  }

  function handleWheel(event: WheelEvent) {
    const now = performance.now();
    const isTp = isTrackpadEvent(event, lastTrackpadTime, now);
    if (isTp) {
      lastTrackpadTime = now;
      stopKineticScroll();
      options?.onScrollActivity?.();
      return;
    }

    if (event.deltaY === 0 && event.deltaX === 0) {
      return;
    }

    if (rafId === null) {
      updateDimensions();
      expectedScrollTop = node.scrollTop;
      expectedScrollLeft = node.scrollLeft;
    }

    if (maxScrollTop === 0 && maxScrollLeft === 0) {
      return;
    }

    event.preventDefault();
    options?.onScrollActivity?.();

    const { dy, dx } = resolveDeltaPixels(
      event,
      24,
      cachedClientHeight || node.clientHeight,
      cachedClientWidth || node.clientWidth
    );

    if (rafId === null) {
      targetY = node.scrollTop + dy;
      targetX = node.scrollLeft + dx;
    } else {
      targetY += dy;
      targetX += dx;
    }

    targetY = clampScroll(targetY, maxScrollTop);
    targetX = clampScroll(targetX, maxScrollLeft);

    if (targetY === node.scrollTop && targetX === node.scrollLeft) {
      stopKineticScroll();
      return;
    }

    if (rafId === null) {
      lastTime = performance.now();
      rafId = requestAnimationFrame(tick);
    }
  }

  function handleNativeScroll() {
    if (rafId !== null) {
      const diffY = Math.abs(node.scrollTop - expectedScrollTop);
      const diffX = Math.abs(node.scrollLeft - expectedScrollLeft);
      if (diffY > 1 || diffX > 1) {
        stopKineticScroll();
        options?.onScrollActivity?.();
      }
    } else {
      targetY = node.scrollTop;
      targetX = node.scrollLeft;
      expectedScrollTop = node.scrollTop;
      expectedScrollLeft = node.scrollLeft;
      options?.onScrollActivity?.();
    }
  }

  function handlePointerDown() {
    stopKineticScroll();
  }

  node.addEventListener('wheel', handleWheel, { passive: false });
  node.addEventListener('scroll', handleNativeScroll, { passive: true });
  node.addEventListener('pointerdown', handlePointerDown, { passive: true });
  node.addEventListener('touchstart', handlePointerDown, { passive: true });

  return {
    destroy() {
      stopKineticScroll();
      if (resizeObserver) {
        resizeObserver.disconnect();
        resizeObserver = null;
      }
      node.removeEventListener('wheel', handleWheel);
      node.removeEventListener('scroll', handleNativeScroll);
      node.removeEventListener('pointerdown', handlePointerDown);
      node.removeEventListener('touchstart', handlePointerDown);
    }
  };
}

export function smoothScroll(
  node: HTMLElement,
  options?: SmoothScrollOptions
): { destroy: () => void } {
  return attachSmoothScroll(node, options);
}
