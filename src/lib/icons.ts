export type FileIconType =
  | 'rust'
  | 'typescript'
  | 'tsx'
  | 'javascript'
  | 'jsx'
  | 'svelte'
  | 'python'
  | 'go'
  | 'html'
  | 'css'
  | 'sass'
  | 'json'
  | 'toml'
  | 'yaml'
  | 'markdown'
  | 'git'
  | 'docker'
  | 'shell'
  | 'env'
  | 'lock'
  | 'image'
  | 'svg'
  | 'sql'
  | 'c'
  | 'cpp'
  | 'java'
  | 'kotlin'
  | 'php'
  | 'ruby'
  | 'archive'
  | 'text'
  | 'file';

const EXACT_NAME_MAP: Record<string, FileIconType> = {
  dockerfile: 'docker',
  containerfile: 'docker',
  'docker-compose.yml': 'docker',
  'docker-compose.yaml': 'docker',
  'compose.yml': 'docker',
  'compose.yaml': 'docker',
  '.dockerignore': 'docker',
  '.gitignore': 'git',
  '.gitattributes': 'git',
  '.gitmodules': 'git',
  '.gitconfig': 'git',
  '.gitkeep': 'git',
  '.env': 'env',
  '.env.local': 'env',
  '.env.development': 'env',
  '.env.production': 'env',
  '.env.test': 'env',
  '.env.example': 'env',
  '.env.sample': 'env',
  'cargo.lock': 'lock',
  'package-lock.json': 'lock',
  'pnpm-lock.yaml': 'lock',
  'yarn.lock': 'lock',
  'bun.lockb': 'lock',
  'bun.lock': 'lock',
  'poetry.lock': 'lock',
  'composer.lock': 'lock',
  'gemfile.lock': 'lock',
  'cargo.toml': 'toml',
  makefile: 'shell',
  gnumakefile: 'shell',
  '.bashrc': 'shell',
  '.bash_profile': 'shell',
  '.bash_aliases': 'shell',
  '.bash_logout': 'shell',
  '.zshrc': 'shell',
  '.zprofile': 'shell',
  '.zshenv': 'shell',
  '.zlogin': 'shell',
  '.profile': 'shell',
  'cmakelists.txt': 'c',
  license: 'text',
  licence: 'text',
  readme: 'markdown'
};

const EXTENSION_MAP: Record<string, FileIconType> = {
  rs: 'rust',
  ts: 'typescript',
  mts: 'typescript',
  cts: 'typescript',
  tsx: 'tsx',
  js: 'javascript',
  mjs: 'javascript',
  cjs: 'javascript',
  jsx: 'jsx',
  svelte: 'svelte',
  py: 'python',
  pyw: 'python',
  ipynb: 'python',
  go: 'go',
  html: 'html',
  htm: 'html',
  xhtml: 'html',
  xml: 'html',
  css: 'css',
  scss: 'sass',
  sass: 'sass',
  less: 'sass',
  json: 'json',
  jsonc: 'json',
  json5: 'json',
  webmanifest: 'json',
  toml: 'toml',
  yaml: 'yaml',
  yml: 'yaml',
  md: 'markdown',
  markdown: 'markdown',
  mdx: 'markdown',
  mdown: 'markdown',
  mkdn: 'markdown',
  sh: 'shell',
  bash: 'shell',
  zsh: 'shell',
  fish: 'shell',
  ksh: 'shell',
  csh: 'shell',
  tcsh: 'shell',
  c: 'c',
  h: 'c',
  cpp: 'cpp',
  hpp: 'cpp',
  cc: 'cpp',
  hh: 'cpp',
  cxx: 'cpp',
  hxx: 'cpp',
  'c++': 'cpp',
  'h++': 'cpp',
  java: 'java',
  kt: 'kotlin',
  kts: 'kotlin',
  php: 'php',
  rb: 'ruby',
  sql: 'sql',
  pgsql: 'sql',
  mysql: 'sql',
  sqlite: 'sql',
  sqlite3: 'sql',
  png: 'image',
  jpg: 'image',
  jpeg: 'image',
  gif: 'image',
  webp: 'image',
  ico: 'image',
  bmp: 'image',
  tiff: 'image',
  avif: 'image',
  svg: 'svg',
  env: 'env',
  dockerfile: 'docker',
  lock: 'lock',
  zip: 'archive',
  tar: 'archive',
  gz: 'archive',
  tgz: 'archive',
  '7z': 'archive',
  rar: 'archive',
  bz2: 'archive',
  xz: 'archive',
  zst: 'archive',
  txt: 'text',
  text: 'text',
  log: 'text',
  diff: 'text',
  patch: 'text'
};

export function getFileIconType(filename: string): FileIconType {
  if (!filename) return 'file';
  const clean = filename.replace(/\\/g, '/');
  const baseName = (clean.split('/').pop() ?? clean).trim().toLowerCase();
  if (!baseName) return 'file';

  const exact = EXACT_NAME_MAP[baseName];
  if (exact) return exact;

  if (baseName.startsWith('.env.')) return 'env';
  if (baseName.startsWith('dockerfile.')) return 'docker';
  if (baseName.startsWith('containerfile.')) return 'docker';

  const dotIdx = baseName.lastIndexOf('.');
  if (dotIdx > 0 && dotIdx < baseName.length - 1) {
    const ext = baseName.slice(dotIdx + 1);
    const matched = EXTENSION_MAP[ext];
    if (matched) return matched;
  }

  return 'file';
}
