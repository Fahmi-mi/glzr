import { describe, expect, it } from 'vitest';
import { getFileIconType } from './icons';

describe('file icon type detection', () => {
  it('detects programming languages by extension', () => {
    expect(getFileIconType('main.rs')).toBe('rust');
    expect(getFileIconType('src/lib.rs')).toBe('rust');
    expect(getFileIconType('app.ts')).toBe('typescript');
    expect(getFileIconType('component.mts')).toBe('typescript');
    expect(getFileIconType('module.cts')).toBe('typescript');
    expect(getFileIconType('view.tsx')).toBe('tsx');
    expect(getFileIconType('index.js')).toBe('javascript');
    expect(getFileIconType('bundle.mjs')).toBe('javascript');
    expect(getFileIconType('util.cjs')).toBe('javascript');
    expect(getFileIconType('button.jsx')).toBe('jsx');
    expect(getFileIconType('App.svelte')).toBe('svelte');
    expect(getFileIconType('script.py')).toBe('python');
    expect(getFileIconType('notebook.ipynb')).toBe('python');
    expect(getFileIconType('server.go')).toBe('go');
    expect(getFileIconType('index.html')).toBe('html');
    expect(getFileIconType('pom.xml')).toBe('html');
    expect(getFileIconType('style.css')).toBe('css');
    expect(getFileIconType('theme.scss')).toBe('sass');
    expect(getFileIconType('data.json')).toBe('json');
    expect(getFileIconType('config.toml')).toBe('toml');
    expect(getFileIconType('deploy.yaml')).toBe('yaml');
    expect(getFileIconType('deploy.yml')).toBe('yaml');
    expect(getFileIconType('README.md')).toBe('markdown');
    expect(getFileIconType('docs.mdx')).toBe('markdown');
    expect(getFileIconType('run.sh')).toBe('shell');
    expect(getFileIconType('setup.bash')).toBe('shell');
    expect(getFileIconType('main.c')).toBe('c');
    expect(getFileIconType('header.h')).toBe('c');
    expect(getFileIconType('main.cpp')).toBe('cpp');
    expect(getFileIconType('header.hpp')).toBe('cpp');
    expect(getFileIconType('common.hh')).toBe('cpp');
    expect(getFileIconType('module.cxx')).toBe('cpp');
    expect(getFileIconType('Main.java')).toBe('java');
    expect(getFileIconType('Main.kt')).toBe('kotlin');
    expect(getFileIconType('index.php')).toBe('php');
    expect(getFileIconType('app.rb')).toBe('ruby');
    expect(getFileIconType('query.sql')).toBe('sql');
    expect(getFileIconType('schema.pgsql')).toBe('sql');
    expect(getFileIconType('database.sqlite3')).toBe('sql');
  });

  it('detects special exact filenames and config files', () => {
    expect(getFileIconType('Dockerfile')).toBe('docker');
    expect(getFileIconType('dockerfile.production')).toBe('docker');
    expect(getFileIconType('Containerfile')).toBe('docker');
    expect(getFileIconType('containerfile.dev')).toBe('docker');
    expect(getFileIconType('docker-compose.yml')).toBe('docker');
    expect(getFileIconType('.dockerignore')).toBe('docker');
    expect(getFileIconType('.gitignore')).toBe('git');
    expect(getFileIconType('.gitattributes')).toBe('git');
    expect(getFileIconType('.gitmodules')).toBe('git');
    expect(getFileIconType('.gitkeep')).toBe('git');
    expect(getFileIconType('.env')).toBe('env');
    expect(getFileIconType('.env.local')).toBe('env');
    expect(getFileIconType('.env.production')).toBe('env');
    expect(getFileIconType('production.env')).toBe('env');
    expect(getFileIconType('Cargo.lock')).toBe('lock');
    expect(getFileIconType('package-lock.json')).toBe('lock');
    expect(getFileIconType('pnpm-lock.yaml')).toBe('lock');
    expect(getFileIconType('yarn.lock')).toBe('lock');
    expect(getFileIconType('bun.lockb')).toBe('lock');
    expect(getFileIconType('Cargo.toml')).toBe('toml');
    expect(getFileIconType('Makefile')).toBe('shell');
    expect(getFileIconType('GNUmakefile')).toBe('shell');
    expect(getFileIconType('.bashrc')).toBe('shell');
    expect(getFileIconType('.zshrc')).toBe('shell');
    expect(getFileIconType('.profile')).toBe('shell');
    expect(getFileIconType('LICENSE')).toBe('text');
  });

  it('detects images, assets, and archives', () => {
    expect(getFileIconType('logo.png')).toBe('image');
    expect(getFileIconType('banner.jpg')).toBe('image');
    expect(getFileIconType('photo.jpeg')).toBe('image');
    expect(getFileIconType('icon.ico')).toBe('image');
    expect(getFileIconType('vector.svg')).toBe('svg');
    expect(getFileIconType('archive.zip')).toBe('archive');
    expect(getFileIconType('bundle.tar.gz')).toBe('archive');
  });

  it('handles edge cases, paths, and fallbacks', () => {
    expect(getFileIconType('')).toBe('file');
    expect(getFileIconType('unknown_file_type')).toBe('file');
    expect(getFileIconType('weird.unknownext')).toBe('file');
    expect(getFileIconType('/deeply/nested/path/to/Component.SVELTE')).toBe('svelte');
    expect(getFileIconType('C:\\windows\\path\\Cargo.lock')).toBe('lock');
    expect(getFileIconType('notes.txt')).toBe('text');
  });
});
