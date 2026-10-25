import type { Extension } from '@codemirror/state';

export type LanguageId =
  | 'javascript'
  | 'typescript'
  | 'jsx'
  | 'tsx'
  | 'json'
  | 'html'
  | 'css'
  | 'markdown'
  | 'rust'
  | 'python'
  | 'yaml'
  | 'toml'
  | 'shell'
  | 'svelte'
  | 'dockerfile'
  | 'plaintext';

export interface LanguageEntry {
  id: LanguageId;
  label: string;
  extensions: string[];
  filenames: string[];
  shebangs: string[];
}

export const LANGUAGE_TABLE: readonly LanguageEntry[] = [
  {
    id: 'rust',
    label: 'Rust',
    extensions: ['rs'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'typescript',
    label: 'TypeScript',
    extensions: ['ts', 'mts', 'cts'],
    filenames: [],
    shebangs: ['ts-node', 'deno', 'bun']
  },
  {
    id: 'tsx',
    label: 'TSX',
    extensions: ['tsx'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'javascript',
    label: 'JavaScript',
    extensions: ['js', 'mjs', 'cjs'],
    filenames: [],
    shebangs: ['node', 'nodejs']
  },
  {
    id: 'jsx',
    label: 'JSX',
    extensions: ['jsx'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'svelte',
    label: 'Svelte',
    extensions: ['svelte'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'json',
    label: 'JSON',
    extensions: ['json', 'jsonc', 'webmanifest'],
    filenames: ['.prettierrc', '.eslintrc'],
    shebangs: []
  },
  {
    id: 'html',
    label: 'HTML',
    extensions: ['html', 'htm', 'svg'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'css',
    label: 'CSS',
    extensions: ['css'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'markdown',
    label: 'Markdown',
    extensions: ['md', 'markdown'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'python',
    label: 'Python',
    extensions: ['py', 'pyw'],
    filenames: [],
    shebangs: ['python', 'python3']
  },
  {
    id: 'yaml',
    label: 'YAML',
    extensions: ['yaml', 'yml'],
    filenames: [],
    shebangs: []
  },
  {
    id: 'toml',
    label: 'TOML',
    extensions: ['toml'],
    filenames: ['cargo.lock'],
    shebangs: []
  },
  {
    id: 'shell',
    label: 'Shell',
    extensions: ['sh', 'bash', 'zsh'],
    filenames: [
      'makefile',
      '.gitignore',
      '.dockerignore',
      '.prettierignore',
      '.env',
      '.env.local',
      '.env.example',
      '.bashrc',
      '.zshrc',
      '.profile'
    ],
    shebangs: ['sh', 'bash', 'zsh', 'dash']
  },
  {
    id: 'dockerfile',
    label: 'Dockerfile',
    extensions: ['dockerfile'],
    filenames: ['dockerfile', 'containerfile'],
    shebangs: []
  }
];

const PLAIN_TEXT_ENTRY: LanguageEntry = {
  id: 'plaintext',
  label: 'Plain Text',
  extensions: [],
  filenames: [],
  shebangs: []
};

export function detectLanguage(filePath: string, firstLine?: string): LanguageEntry {
  const normalized = filePath.replace(/\\/g, '/');
  const baseName = (normalized.split('/').pop() ?? normalized).toLowerCase();

  for (const entry of LANGUAGE_TABLE) {
    if (entry.filenames.includes(baseName)) {
      return entry;
    }
  }

  if (baseName.startsWith('dockerfile.') || baseName.startsWith('.env.')) {
    const matched = LANGUAGE_TABLE.find(
      (e) => e.id === (baseName.startsWith('dockerfile.') ? 'dockerfile' : 'shell')
    );
    if (matched) {
      return matched;
    }
  }

  const dotIdx = baseName.lastIndexOf('.');
  if (dotIdx > 0 && dotIdx < baseName.length - 1) {
    const ext = baseName.slice(dotIdx + 1);
    for (const entry of LANGUAGE_TABLE) {
      if (entry.extensions.includes(ext)) {
        return entry;
      }
    }
  }

  if (firstLine && firstLine.startsWith('#!')) {
    const shebangLower = firstLine.toLowerCase();
    for (const entry of LANGUAGE_TABLE) {
      for (const token of entry.shebangs) {
        if (
          shebangLower.endsWith(`/${token}`) ||
          shebangLower.includes(` ${token}`) ||
          shebangLower.includes(`/${token} `)
        ) {
          return entry;
        }
      }
    }
  }

  return PLAIN_TEXT_ENTRY;
}

export async function loadLanguageExtension(langId: LanguageId): Promise<Extension | null> {
  switch (langId) {
    case 'javascript': {
      const mod = await import('@codemirror/lang-javascript');
      return mod.javascript();
    }
    case 'jsx': {
      const mod = await import('@codemirror/lang-javascript');
      return mod.javascript({ jsx: true });
    }
    case 'typescript': {
      const mod = await import('@codemirror/lang-javascript');
      return mod.javascript({ typescript: true });
    }
    case 'tsx': {
      const mod = await import('@codemirror/lang-javascript');
      return mod.javascript({ jsx: true, typescript: true });
    }
    case 'json': {
      const mod = await import('@codemirror/lang-json');
      return mod.json();
    }
    case 'html': {
      const mod = await import('@codemirror/lang-html');
      return mod.html();
    }
    case 'css': {
      const mod = await import('@codemirror/lang-css');
      return mod.css();
    }
    case 'markdown': {
      const mod = await import('@codemirror/lang-markdown');
      return mod.markdown();
    }
    case 'rust': {
      const mod = await import('@codemirror/lang-rust');
      return mod.rust();
    }
    case 'python': {
      const mod = await import('@codemirror/lang-python');
      return mod.python();
    }
    case 'yaml': {
      const mod = await import('@codemirror/lang-yaml');
      return mod.yaml();
    }
    case 'svelte': {
      const mod = await import('@replit/codemirror-lang-svelte');
      return mod.svelte();
    }
    case 'toml': {
      const [{ StreamLanguage }, { toml }] = await Promise.all([
        import('@codemirror/language'),
        import('@codemirror/legacy-modes/mode/toml')
      ]);
      return StreamLanguage.define(toml);
    }
    case 'shell': {
      const [{ StreamLanguage }, { shell }] = await Promise.all([
        import('@codemirror/language'),
        import('@codemirror/legacy-modes/mode/shell')
      ]);
      return StreamLanguage.define(shell);
    }
    case 'dockerfile': {
      const [{ StreamLanguage }, { dockerFile }] = await Promise.all([
        import('@codemirror/language'),
        import('@codemirror/legacy-modes/mode/dockerfile')
      ]);
      return StreamLanguage.define(dockerFile);
    }
    case 'plaintext':
    default:
      return null;
  }
}

export interface QuickOpenQuery {
  query: string;
  line: number | null;
}

export function parseQuickOpenInput(raw: string): QuickOpenQuery {
  const trimmed = raw.trim();
  const colonIdx = trimmed.lastIndexOf(':');
  if (colonIdx > 0) {
    const maybeLine = trimmed.slice(colonIdx + 1);
    if (/^\d+$/.test(maybeLine)) {
      const parsedLine = Number.parseInt(maybeLine, 10);
      return {
        query: trimmed.slice(0, colonIdx).trim(),
        line: parsedLine > 0 ? parsedLine : 1
      };
    }
  }
  return { query: trimmed, line: null };
}

export function fuzzyScore(candidate: string, query: string): number | null {
  if (!query) {
    return 0;
  }

  const lowerCandidate = candidate.toLowerCase();
  const lowerQuery = query.toLowerCase();

  let score = 0;
  let qIdx = 0;
  let prevMatchIdx = -2;
  const fileNameStart = lowerCandidate.lastIndexOf('/') + 1;

  for (let cIdx = 0; cIdx < lowerCandidate.length && qIdx < lowerQuery.length; cIdx++) {
    if (lowerCandidate[cIdx] === lowerQuery[qIdx]) {
      score += 10;
      if (cIdx === prevMatchIdx + 1) {
        score += 15;
      }
      if (cIdx >= fileNameStart) {
        score += 8;
      }
      if (cIdx === 0 || cIdx === fileNameStart || '/._-'.includes(lowerCandidate[cIdx - 1])) {
        score += 12;
      }
      prevMatchIdx = cIdx;
      qIdx++;
    }
  }

  if (qIdx < lowerQuery.length) {
    return null;
  }

  return score - candidate.length * 0.1;
}

export function fuzzyFilterFiles(files: readonly string[], query: string, limit = 50): string[] {
  const trimmed = query.trim();
  if (!trimmed) {
    return files.slice(0, limit);
  }

  const scored: Array<{ path: string; score: number }> = [];
  for (const path of files) {
    const score = fuzzyScore(path, trimmed);
    if (score !== null) {
      scored.push({ path, score });
    }
  }

  scored.sort((a, b) => b.score - a.score || a.path.localeCompare(b.path));
  return scored.slice(0, limit).map((item) => item.path);
}
