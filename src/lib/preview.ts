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
