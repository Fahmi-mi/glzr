# glzr

Terminal-first workspace yang ringan untuk Linux: terminal, editor file, git panel, dan preview localhost dalam satu aplikasi. Tanpa AI, tanpa plugin, tanpa LSP.

```
+-----------------------------------------------------------------------+
| [Alt+1: web] [Alt+2: backend]                     Dark | Branch: main |
+-----------------------------------+-----------------------------------+
| FILES | GIT | SEARCH              | TERMINAL 1                        |
| src/                              | $ cargo check                     |
|   lib.rs                          |    Checking glzr v0.1.0           |
|   fs/mod.rs                       |    Finished `dev` profile         |
|                                   +-----------------------------------+
| EDITOR: src/main.rs               | PREVIEW: http://localhost:5173    |
| fn main() {                       | [ glzr UI Live Preview ]          |
|     glzr::run();                  |                                   |
| }                                 |                                   |
+-----------------------------------+-----------------------------------+
```

## Fitur Ringkas

1. **Terminal:** Multi-tab dan split pane, login/interactive shell via `portable-pty`, emulasi `xterm.js` dengan akselerasi WebGL.
2. **Editor File:** CodeMirror 6 dengan syntax highlighting dinamis (JS/TS, Rust, Go, Python, Svelte, Markdown, HTML/CSS, TOML, YAML, JSON), atomic write, dan auto-reload saat file berubah di disk.
3. **Git Panel:** Status working tree (staged, unstaged, untracked), visual hunk diff via `@codemirror/merge`, stage/unstage/discard per file dan per hunk, commit, branch switching, serta pull/push.
4. **Pencarian Proyek:** Pencarian seluruh folder kerja berbasis sidecar `ripgrep` (`rg`) dengan output bertahap, pembatalan instan saat mengetik ulang, dan batas aman baris.
5. **Preview Localhost:** Tab browser terisolasi via iframe untuk preview server pengembangan web lokal (`localhost` dan `127.0.0.1`), dengan kontrol reload, navigasi URL, fallback browser eksternal, dan auto-suspend tab tidak aktif.
6. **Layout Fleksibel:** Struktur hierarki workspace, pane (split horizontal dan vertikal), dan multi-tab. State layout dipulihkan secara otomatis saat aplikasi dibuka ulang.

## Ukuran Paket dan Penggunaan RAM

### Hasil Pengukuran

| Komponen / Metrik                                          | Target     | Hasil Pengukuran           |
| ---------------------------------------------------------- | ---------- | -------------------------- |
| Biner Utama (`glzr` release)                               | < 15 MB    | 3.4 MB                     |
| Bundle Frontend (JS/HTML/CSS terkompresi)                  | < 300 KB   | ~80 KB                     |
| Total Payload Termasuk Sidecar `rg` dan Font               | 12-25 MB   | ~11.7 MB (uncompressed)    |
| Paket Debian (`.deb`)                                      | 12-25 MB   | ~8-15 MB (arsip kompresi)  |
| AppImage Bundle                                            | 15-30 MB   | ~15-25 MB (arsip kompresi) |
| RAM Baseline (1 pane, 1 terminal aktif)                    | 80-150 MB  | ~110-140 MB PSS total      |
| Sesi Harian Wajar (3-5 tab campuran editor, git, terminal) | 150-250 MB | ~180-230 MB PSS total      |

### Cara Mengukur Penggunaan RAM

WebKitGTK pada Linux memecah runtime ke dalam beberapa proses:

- Proses biner utama (`glzr`)
- Proses rendering WebKit (`WebKitWebProcess`)
- Proses jaringan WebKit (`WebKitNetworkProcess`)

Untuk mengukur penggunaan RAM riil secara akurat tanpa menghitung memori bersama (shared libraries) secara ganda, gunakan metrik PSS (Proportional Set Size):

```bash
# Menggunakan smem (direkomendasikan)
smem -P glzr -c "pid user command pss"

# Atau dengan skrip one-liner ps / awk untuk menjumlahkan PSS semua proses glzr:
cat /proc/$(pgrep -d ' ' -f 'glzr|WebKitWebProcess|WebKitNetworkProcess')/smaps 2>/dev/null | grep -i '^Pss:' | awk '{sum+=$2} END {print sum/1024, "MB"}'
```

## Perbandingan Singkat dengan Zed

| Aspek               | Zed                                                               | glzr                                                                    |
| ------------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------- |
| Filosofi Utama      | Editor-first dengan integrated terminal                           | Terminal-first dengan built-in lightweight editor                       |
| Rendering Engine    | GPUI kustom berbasis GPU langsung                                 | Tauri 2 + WebKitGTK 4.1 sistem Linux native                             |
| Fitur Bahasa        | Language Server Protocol (LSP), AI assistant, autocomplete inline | CodeMirror 6 syntax highlight, tanpa LSP, tanpa AI internal             |
| Preview Web         | Memerlukan browser eksternal                                      | Built-in localhost preview iframe terisolasi                            |
| Model Integrasi CLI | Terminal pendukung di panel bawah                                 | Terminal sebagai warga kelas satu (agent AI CLI dijalankan di terminal) |
| Target Platform     | Multi-platform (macOS, Linux, Windows)                            | Linux terfokus (Ubuntu 24.04, Zorin OS 18, dsb.)                        |

## Instalasi

### Prasyarat Sistem

Aplikasi membutuhkan dependensi `git` terpasang di sistem pengguna:

```bash
sudo apt update && sudo apt install -y git
```

### Opsi 1: Paket Debian (`.deb`)

Unduh file `.deb` dari halaman rilis GitHub, lalu pasang:

```bash
sudo dpkg -i glzr_*_amd64.deb
sudo apt-get install -f # Opsional jika ada dependensi sistem yang belum lengkap
```

Paket `.deb` secara otomatis menambahkan biner `glzr` ke `$PATH` dan mendaftarkan aplikasi di desktop environment. Anda dapat membuka folder langsung dari terminal:

```bash
glzr .
```

### Opsi 2: AppImage

Unduh file `.AppImage` dari halaman rilis GitHub, berikan izin eksekusi:

```bash
chmod +x glzr_*_amd64.AppImage
./glzr_*_amd64.AppImage
```

Agar perintah `glzr` dapat dipanggil dari terminal di direktori mana saja menggunakan AppImage, buat symlink ke direktori path lokal Anda:

```bash
mkdir -p ~/.local/bin
ln -sf "$(pwd)/glzr_amd64.AppImage" ~/.local/bin/glzr
```

Pastikan `~/.local/bin` ada dalam variabel environment `$PATH`.

## Keybind Ringkas

### Pintasan Global Aplikasi (Bekerja di mana saja, termasuk saat fokus di terminal)

| Aksi                             | Shortcut                                      |
| -------------------------------- | --------------------------------------------- |
| Command Palette                  | `Alt+P`                                       |
| Quick Open File                  | `Alt+O`                                       |
| Sidebar Files                    | `Alt+E`                                       |
| Sidebar Git                      | `Alt+V`                                       |
| Sidebar Search                   | `Alt+G`                                       |
| Terminal Baru di Pane Aktif      | `Alt+T`                                       |
| Tutup Tab Aktif                  | `Alt+W`                                       |
| Split Kanan                      | `Alt+R`                                       |
| Split Bawah                      | `Alt+J`                                       |
| Tutup Pane Aktif                 | `Alt+Q`                                       |
| Maximize / Restore Pane          | `Alt+M`                                       |
| Pindah Fokus Antar Pane          | `Alt+Left`, `Alt+Right`, `Alt+Up`, `Alt+Down` |
| Tab Berikutnya / Sebelumnya      | `Alt+PageDown` / `Alt+PageUp`                 |
| Pindah Workspace 1-9             | `Alt+1` .. `Alt+9`                            |
| Zoom Font Masuk / Keluar / Reset | `Alt+=` / `Alt+-` / `Alt+0`                   |

### Pintasan Editor (Aktif saat fokus di editor file CodeMirror)

| Aksi                   | Shortcut            |
| ---------------------- | ------------------- |
| Simpan Perubahan       | `Ctrl+S`            |
| Undo / Redo            | `Ctrl+Z` / `Ctrl+Y` |
| Cari dan Ganti di File | `Ctrl+F`            |
| Go to Line             | `Ctrl+G`            |
| Toggle Baris Komentar  | `Ctrl+/`            |
| Select Next Occurrence | `Ctrl+D`            |

### Pintasan Terminal dan Panel Lainnya

| Konteks   | Aksi                     | Shortcut                              |
| --------- | ------------------------ | ------------------------------------- |
| Terminal  | Copy teks terseleksi     | `Ctrl+C` (hanya jika ada seleksi)     |
| Terminal  | Paste teks               | `Ctrl+Shift+V` atau klik tengah mouse |
| Terminal  | Cari teks dalam riwayat  | `Alt+S`                               |
| Preview   | Fokus ke address bar URL | `Ctrl+L`                              |
| Preview   | Reload halaman preview   | `Ctrl+R` atau `F5`                    |
| Git Panel | Kirim pesan commit       | `Ctrl+Enter` di input commit          |

## Build dari Sumber

### 1. Dependensi Sistem (Ubuntu 24.04 / Zorin OS 18 / Debian)

```bash
sudo apt update && sudo apt install -y \
  build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev git
```

### 2. Pasang Rust dan Node.js

Pastikan Rust (cargo 1.80+) dan Node.js 22 LTS terpasang:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
curl -fsSL https://deb.nodesource.com/setup_22.x | sudo -E bash -
sudo apt install -y nodejs
```

### 3. Clone Repository dan Pasang Dependensi Node

```bash
git clone https://github.com/Fahmi-mi/glzr.git
cd glzr
npm install
```

### 4. Menjalankan Mode Pengembangan

```bash
npm run tauri dev
```

### 5. Membangun Paket Rilis Biner

```bash
npm run tauri build
```

Biner dan installer yang sudah terkompilasi akan berada di `src-tauri/target/release/bundle/deb/` dan `src-tauri/target/release/bundle/appimage/`.

## Batasan (Yang Sengaja Tidak Dibuat)

Untuk menjaga performa, kestabilan, serta kesederhanaan kode, sejumlah fitur sengaja tidak dibangun:

- **Tanpa LSP (Language Server Protocol):** Tidak ada proses language server background yang memakan RAM besar.
- **Tanpa Sistem Ekstensi / Plugin:** Tidak ada API ekstensi pihak ketiga atau sandbox runtime tambahan.
- **Tanpa Asisten AI Bawaan:** Model atau agen AI dijalankan langsung oleh pengguna di terminal interaktif pilihan masing-masing.
- **Tanpa Debugger Grafis:** Proses debugging interaktif dilakukan via CLI debugger (gdb, lldb, dlv) di terminal.
- **Tanpa Global Find and Replace:** Pencarian proyek hanya membaca (`ripgrep`), modifikasi massal multi-file dihindari demi keamanan codebase.
- **Khusus Linux:** Hanya menargetkan ekosistem desktop Linux native dengan WebKitGTK 4.1.

## Masalah yang Diketahui

Daftar isu yang telah teridentifikasi dan strategi mitigasinya terdokumentasi lengkap pada file [KNOWN_ISSUES.md](./KNOWN_ISSUES.md).

## Lisensi

Proyek ini dilisensikan di bawah [MIT License](./LICENSE).
Lisensi dan atribusi komponen pihak ketiga (ripgrep, xterm.js, CodeMirror 6, JetBrains Mono Nerd Font, dsb.) dapat dilihat pada [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
