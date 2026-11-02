export function installIdleCallbackPolyfill(
  target: typeof globalThis & {
    requestIdleCallback?: typeof window.requestIdleCallback;
    cancelIdleCallback?: typeof window.cancelIdleCallback;
  } = globalThis
): void {
  if (typeof target.requestIdleCallback === 'undefined') {
    target.requestIdleCallback = (callback: IdleRequestCallback): number => {
      return setTimeout(() => {
        const start = performance.now();
        callback({
          didTimeout: false,
          timeRemaining: () => Math.max(0, 5 - (performance.now() - start))
        });
      }, 16) as unknown as number;
    };
    target.cancelIdleCallback = (id: number): void => {
      clearTimeout(id);
    };
  }
}

if (typeof window !== 'undefined') {
  installIdleCallbackPolyfill(window);
}
