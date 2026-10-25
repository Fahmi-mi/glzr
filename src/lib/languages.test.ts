import { describe, expect, it } from 'vitest';
import {
  detectLanguage,
  fuzzyFilterFiles,
  loadLanguageExtension,
  parseQuickOpenInput
} from './languages';

describe('language detection and quick open fuzzy matcher', () => {
  it('detects languages from special filenames, extensions, and shebangs', () => {
    expect(detectLanguage('src/main.rs').id).toBe('rust');
    expect(detectLanguage('cmd/server/main.go').id).toBe('go');
    expect(detectLanguage('go.mod').id).toBe('go');
    expect(detectLanguage('src/routes/+page.svelte').id).toBe('svelte');
    expect(detectLanguage('Dockerfile').id).toBe('dockerfile');
    expect(detectLanguage('Cargo.lock').id).toBe('toml');
    expect(detectLanguage('.gitignore').id).toBe('shell');
    expect(detectLanguage('.env.production').id).toBe('shell');
    expect(detectLanguage('scripts/run', '#!/usr/bin/env python3').id).toBe('python');
    expect(detectLanguage('scripts/deploy', '#!/bin/bash').id).toBe('shell');
    expect(detectLanguage('UNKNOWN').id).toBe('plaintext');
  });

  it('dynamically loads codemirror language extensions', async () => {
    const rustExt = await loadLanguageExtension('rust');
    expect(rustExt).not.toBeNull();

    const goExt = await loadLanguageExtension('go');
    expect(goExt).not.toBeNull();

    const tomlExt = await loadLanguageExtension('toml');
    expect(tomlExt).not.toBeNull();

    const plainExt = await loadLanguageExtension('plaintext');
    expect(plainExt).toBeNull();
  });

  it('parses quick open query with optional line number and ranks fuzzy matches', () => {
    expect(parseQuickOpenInput('main.rs:42')).toEqual({ query: 'main.rs', line: 42 });
    expect(parseQuickOpenInput('src/app.css')).toEqual({ query: 'src/app.css', line: null });

    const files = [
      'README.md',
      'src-tauri/src/main.rs',
      'src/lib/ipc.ts',
      'src/routes/+page.svelte'
    ];
    const matches = fuzzyFilterFiles(files, 'mainrs');
    expect(matches[0]).toBe('src-tauri/src/main.rs');
  });
});
