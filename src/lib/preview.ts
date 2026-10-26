const DEFAULT_ALLOWED_HOSTS = ['localhost', '127.0.0.1'];

export function normalizePreviewUrl(
  rawInput: string,
  allowedHosts: string[] = DEFAULT_ALLOWED_HOSTS
): string | null {
  const trimmed = rawInput.trim();
  if (!trimmed) {
    return null;
  }

  let candidate = trimmed;
  if (/^:\d+(\/.*)?$/.test(candidate)) {
    candidate = `http://localhost${candidate}`;
  } else if (!/^[a-zA-Z][a-zA-Z0-9+.-]*:\/\//.test(candidate)) {
    candidate = `http://${candidate}`;
  }

  let parsed: URL;
  try {
    parsed = new URL(candidate);
  } catch {
    return null;
  }

  if (parsed.protocol !== 'http:' && parsed.protocol !== 'https:') {
    return null;
  }

  if (!allowedHosts.includes(parsed.hostname)) {
    return null;
  }

  return parsed.toString();
}

export function resolveIframeSrc(url: string | null, isActive: boolean): string {
  if (!isActive || !url) {
    return 'about:blank';
  }
  return url;
}

export function formatPreviewTabTitle(url: string): string {
  try {
    const parsed = new URL(url);
    if (parsed.port) {
      return `:${parsed.port}`;
    }
    return parsed.hostname || 'preview';
  } catch {
    return 'preview';
  }
}

export function formatPreviewHostLabel(url: string): string {
  try {
    const parsed = new URL(url);
    return parsed.host || 'localhost';
  } catch {
    return 'localhost';
  }
}

export function formatPreviewDisplayUrl(url: string): string {
  try {
    const parsed = new URL(url);
    const pathAndQuery =
      parsed.pathname === '/' && !parsed.search && !parsed.hash
        ? ''
        : `${parsed.pathname}${parsed.search}${parsed.hash}`;
    return `${parsed.host}${pathAndQuery}`;
  } catch {
    return url;
  }
}

export interface PreviewHistoryState {
  entries: string[];
  index: number;
}

export function createPreviewHistory(initialUrl: string): PreviewHistoryState {
  const normalized = normalizePreviewUrl(initialUrl) ?? 'http://localhost:5173/';
  return {
    entries: [normalized],
    index: 0
  };
}

export function pushPreviewHistory(
  state: PreviewHistoryState,
  nextUrl: string
): PreviewHistoryState {
  const normalized = normalizePreviewUrl(nextUrl);
  if (!normalized) return state;

  const current = state.entries[state.index];
  if (current === normalized) return state;

  const nextEntries = [...state.entries.slice(0, state.index + 1), normalized];
  return {
    entries: nextEntries,
    index: nextEntries.length - 1
  };
}

export function stepPreviewHistory(state: PreviewHistoryState, delta: -1 | 1): PreviewHistoryState {
  const nextIndex = state.index + delta;
  if (nextIndex < 0 || nextIndex >= state.entries.length) {
    return state;
  }
  return {
    ...state,
    index: nextIndex
  };
}
