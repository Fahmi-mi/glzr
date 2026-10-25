# glzr

Terminal-first workspace yang ringan untuk Linux: terminal, editor file, git panel, dan preview localhost dalam satu aplikasi. Tanpa AI, tanpa plugin, tanpa LSP.

## Fitur Ringkas

1. **Terminal:** Multi-tab dan split, login/interactive shell, xterm.js + WebGL.
2. **Editor File:** CodeMirror 6 dengan syntax highlighting dinamis, atomic write, auto-reload.
3. **Git Panel:** Status, diff, stage/unstage/discard per file dan per hunk, commit, branch.
4. **Pencarian Proyek:** Pencarian cepat seluruh proyek dengan sidecar `ripgrep` (`rg`).
5. **Preview Localhost:** Preview server dev lokal di dalam tab terisolasi (auto-suspend saat tidak aktif).
6. **Layout Fleksibel:** Hierarki Workspace -> Pane (Split) -> Tab yang dipulihkan otomatis.

## Ukuran dan Penggunaan RAM

_(Akan diisi setelah pengukuran build release menggunakan `smem -P glzr`)_

| Metrik                    | Target     | Hasil Pengukuran |
| ------------------------- | ---------- | ---------------- |
| Ukuran Installer          | 12-25 MB   | TBD              |
| RAM Baseline (1 terminal) | 80-150 MB  | TBD              |
| Sesi Harian Wajar         | 150-250 MB | TBD              |

## Keybind Ringkas

| Aksi                         | Shortcut                    |
| ---------------------------- | --------------------------- |
| Command Palette              | `Alt+P`                     |
| Quick Open File              | `Alt+O`                     |
| Sidebar Files / Git / Search | `Alt+E` / `Alt+V` / `Alt+G` |
| Terminal Baru                | `Alt+T`                     |
| Tutup Tab Aktif              | `Alt+W`                     |
| Split Kanan / Bawah          | `Alt+R` / `Alt+J`           |
| Tutup / Maximize Pane        | `Alt+Q` / `Alt+M`           |
| Pindah Workspace 1-9         | `Alt+1` .. `Alt+9`          |

## Build dari Sumber

Prasyarat sistem (Ubuntu 24.04 / Zorin OS 18):

```bash
sudo apt install build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev git
```

Menjalankan dalam mode pengembangan:

```bash
npm install
npm run tauri dev
```

## Batasan (Yang Tidak Dibuat)

- Tidak ada LSP, debugger, plugin/extension, atau fitur AI bawaan.
- Hanya mendukung Linux (target utama Ubuntu 24.04 / WebKitGTK 4.1).

## Lisensi

[MIT](./LICENSE)
