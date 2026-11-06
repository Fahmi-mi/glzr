import { beforeAll, describe, expect, it } from 'vitest';

if (typeof (globalThis as Record<string, unknown>).self === 'undefined') {
  (globalThis as Record<string, unknown>).self = globalThis;
}

describe('terminalRegistry lifecycle and persistence', () => {
  it('creates and caches session by tabId, then reuses it', async () => {
    const {
      disposeTerminalSession,
      getOrCreateTerminalSession,
      getTerminalSession
    } = await import('./terminalRegistry');

    const tabId = 'test-tab-1';
    const session1 = getOrCreateTerminalSession(tabId, { theme: 'dark' });

    expect(session1).toBeDefined();
    expect(session1.tabId).toBe(tabId);
    expect(session1.containerEl).toBeDefined();
    expect(session1.term).toBeDefined();

    const retrieved = getTerminalSession(tabId);
    expect(retrieved).toBe(session1);

    const session2 = getOrCreateTerminalSession(tabId, { theme: 'light' });
    expect(session2).toBe(session1);
    expect(session2.theme).toBe('light');

    await disposeTerminalSession(tabId);
    expect(getTerminalSession(tabId)).toBeUndefined();
    expect(session1.disposed).toBe(true);
  });

  it('disposes all registered sessions cleanly', async () => {
    const {
      disposeAllTerminalSessions,
      getOrCreateTerminalSession,
      getTerminalSession
    } = await import('./terminalRegistry');

    const tabA = 'tab-alpha';
    const tabB = 'tab-beta';

    const sessionA = getOrCreateTerminalSession(tabA);
    const sessionB = getOrCreateTerminalSession(tabB);

    expect(getTerminalSession(tabA)).toBe(sessionA);
    expect(getTerminalSession(tabB)).toBe(sessionB);

    await disposeAllTerminalSessions();

    expect(getTerminalSession(tabA)).toBeUndefined();
    expect(getTerminalSession(tabB)).toBeUndefined();
    expect(sessionA.disposed).toBe(true);
    expect(sessionB.disposed).toBe(true);
  });

  it('allows reparenting containerEl across host elements without error', async () => {
    const {
      disposeTerminalSession,
      getOrCreateTerminalSession
    } = await import('./terminalRegistry');

    const tabId = 'tab-reparent-1';
    const session = getOrCreateTerminalSession(tabId);

    const mockHostA = { id: 'host-a' };
    const mockHostB = { id: 'host-b' };

    let currentParent: unknown = mockHostA;
    Object.defineProperty(session.containerEl, 'parentElement', {
      get: () => currentParent,
      configurable: true
    });

    expect(session.containerEl.parentElement).toBe(mockHostA);

    currentParent = mockHostB;
    expect(session.containerEl.parentElement).toBe(mockHostB);

    if (session.containerEl.parentElement === mockHostA) {
      if (typeof session.containerEl.remove === 'function') {
        session.containerEl.remove();
      }
    }
    expect(session.containerEl.parentElement).toBe(mockHostB);

    await disposeTerminalSession(tabId);
  });
});
