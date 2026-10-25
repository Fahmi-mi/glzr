# glzr

Terminal-first workspace yang ringan: terminal, editor file, git panel, dan preview localhost dalam satu aplikasi. Tanpa AI, tanpa plugin, tanpa LSP.

Dokumen ini adalah acuan tunggal proyek: tujuan, stack, struktur, fitur, layout, panduan kode, dan daftar masalah beserta solusinya.

---

## Daftar isi

1. [Ringkasan](#1-ringkasan)
2. [Tech stack](#2-tech-stack)
3. [Struktur proyek](#3-struktur-proyek)
4. [Detail fitur](#4-detail-fitur)
5. [Layout aplikasi](#5-layout-aplikasi)
6. [Model data dan IPC](#6-model-data-dan-ipc)
7. [Target performa](#7-target-performa)
8. [Best practice dan panduan kode](#8-best-practice-dan-panduan-kode)
9. [Kemungkinan masalah dan solusi](#9-kemungkinan-masalah-dan-solusi)
10. [Strategi pengujian](#10-strategi-pengujian)
11. [Keamanan](#11-keamanan)
12. [Lingkungan pengembangan](#12-lingkungan-pengembangan)
13. [Urutan implementasi](#13-urutan-implementasi)
14. [Kriteria selesai per fitur](#14-kriteria-selesai-per-fitur)
15. [Repo, lisensi, CI, dan README](#15-repo-lisensi-ci-dan-readme)
16. [Desain visual](#16-desain-visual)
17. [Keputusan desain](#17-keputusan-desain)

---

## 1. Ringkasan

### Tujuan

Satu aplikasi desktop kecil untuk workflow harian: buka file, jalankan terminal, lihat perubahan git, dan lihat hasil web yang sedang dikembangkan. Dibuat sebagai alternatif IDE berat dan sebagai media belajar Rust sekaligus proyek portofolio. Karena sebagian besar kode ditulis oleh agent di terminal, app juga dirancang untuk membaca dan mereview perubahan dengan nyaman: diff per hunk dan pencarian di seluruh proyek.

### Platform

Linux saja. Target utama: Zorin OS 18 (basis Ubuntu 24.04) dengan WebKitGTK 4.1. Windows dan macOS di luar cakupan.

### Prinsip

- **Ringan:** ukuran installer dan RAM jadi kriteria utama. Setiap fitur harus lolos pertanyaan "apakah sepadan dengan RAM-nya".
- **Terminal-first:** terminal adalah warga kelas satu. Agent AI atau tool CLI lain dijalankan di terminal, bukan dibangun ke dalam app.
- **Satu mekanisme:** semua isi layar adalah tab di dalam pane. Tidak ada panel khusus dengan aturan sendiri.
- **Cukup, bukan lengkap:** fokus pada enam kemampuan inti, dan tolak yang lain.
- **Dipakai sendiri:** setiap fitur diuji lewat pemakaian harian (dogfooding).

### Enam kemampuan inti

1. Terminal (banyak tab, split)
2. Buka dan edit file (syntax highlight, tanpa LSP)
3. Git panel (status, diff, stage/unstage/discard per file dan per hunk, commit, branch)
4. Pencarian di seluruh proyek
5. Preview localhost
6. Layout fleksibel: workspace, pane, tab

### Yang tidak akan dibuat

| Tidak dibuat | Alasan |
|---|---|
| LSP (autocomplete pintar, error inline, go to definition) | Berat dan memperluas scope tanpa batas |
| Plugin / extension | Butuh API stabil dan sandbox |
| Fitur AI atau agent | Dijalankan di terminal |
| Debugger | Di luar fokus |
| Stage per baris, edit patch, stash, resolusi merge conflict | Dikerjakan lewat terminal |
| Replace di seluruh proyek | Berisiko merusak banyak file sekaligus |
| Minimap, breadcrumb, activity bar, title bar custom | Tidak membantu workflow |
| Sinkronisasi cloud, kolaborasi | Di luar fokus |
| Windows dan macOS | Fokus Linux saja |

Aturan: usulan fitur baru harus menggantikan sesuatu atau lolos uji "dipakai tiap hari dalam sebulan".

---

## 2. Tech stack

| Lapisan | Pilihan | Catatan |
|---|---|---|
| Shell aplikasi | Tauri 2 | Webview sistem, binary kecil |
| Backend | Rust | Satu crate, banyak modul |
| Frontend | Svelte 5 + SvelteKit (static adapter, mode SPA) + TypeScript | Runtime kecil. Tanpa UI library besar |
| Terminal (UI) | xterm.js + addon `fit`, `search`, `web-links`, `webgl`, `unicode11` | |
| Terminal (PTY) | `portable-pty` | |
| Editor | CodeMirror 6 | Bukan Monaco |
| Diff view | `@codemirror/merge` | |
| Git | `git` CLI via `std::process::Command` | Bukan `git2`, supaya binary kecil |
| Pencarian proyek | `rg` (ripgrep) disertakan sebagai sidecar Tauri (`externalBin`) | Menambah sekitar 5 MB ke installer |
| Folder, argumen CLI, instance tunggal | `tauri-plugin-dialog`, `tauri-plugin-cli`, `tauri-plugin-single-instance` | |
| File watcher | `notify` | |
| Hapus ke trash | `trash` | |
| Error | `thiserror` | |
| Serialisasi | `serde`, `serde_json`, `toml` | |
| Async | `tokio` (secukupnya) | |
| Preview | `<iframe>` di tab preview | Fallback buka di browser |
| Ikon | Lucide, diimpor per ikon | |
| Log | `tauri-plugin-log` | Rotasi file |
| Ukuran dan posisi jendela | `tauri-plugin-window-state` | |
| Test | `cargo test`, `tempfile`, Vitest | |

### Pengaturan build Rust (`Cargo.toml`)

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### Dependency awal

`tauri`, `serde`, `serde_json`, `thiserror`, `portable-pty`, `notify`, `tokio`, `toml`, `trash`. Tambah lainnya hanya kalau benar-benar perlu.

---

## 3. Struktur proyek

```
glzr/
├── package.json
├── svelte.config.js
├── vite.config.ts
├── README.md
├── KNOWN_ISSUES.md
├── docs/
│   ├── glzr-spec.md                # dokumen ini
│   └── design/                     # ekspor tampilan UI (gambar)
├── src/                            # frontend
│   ├── app.html
│   ├── routes/
│   │   └── +page.svelte
│   └── lib/
│       ├── ipc.ts                  # wrapper invoke() dan listen()
│       ├── keymap.ts               # tabel keybind + resolusi berbasis fokus
│       ├── commands.ts             # daftar aksi untuk command palette
│       ├── settings.ts
│       ├── layout/
│       │   ├── tree.ts             # operasi pohon: split, close, move, resize
│       │   ├── types.ts            # Workspace, LayoutNode, Pane, Tab
│       │   └── session.ts          # simpan dan pulihkan sesi
│       ├── stores/                 # state: workspace aktif, tab, git
│       ├── languages.ts            # pemetaan ekstensi/nama file ke bahasa
│       └── components/
│           ├── App.svelte
│           ├── WorkspaceBar.svelte
│           ├── Sidebar.svelte
│           ├── PaneTree.svelte     # render rekursif pohon layout
│           ├── Pane.svelte
│           ├── TabStrip.svelte
│           ├── NewTabMenu.svelte   # menu tombol "+"
│           ├── StatusBar.svelte
│           ├── CommandPalette.svelte
│           ├── QuickOpen.svelte
│           ├── tabs/
│           │   ├── FileTab.svelte      # CodeMirror
│           │   ├── ImageTab.svelte
│           │   ├── TerminalTab.svelte  # xterm.js
│           │   ├── DiffTab.svelte
│           │   └── PreviewTab.svelte   # iframe
│           ├── FileTree.svelte
│           ├── GitPanel.svelte
│           └── SearchPanel.svelte
└── src-tauri/                      # backend Rust
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/               # izin Tauri 2
    └── src/
        ├── main.rs                 # entry point tipis
        ├── lib.rs                  # setup, daftar command
        ├── state.rs                # AppState (Arc<Mutex<...>>, map sesi)
        ├── error.rs                # tipe error, bisa diserialisasi
        ├── settings.rs             # baca/tulis settings TOML
        ├── session.rs              # simpan/pulihkan sesi ke disk
        ├── pty/
        │   ├── mod.rs
        │   └── session.rs          # spawn, write, resize, kill
        ├── fs/
        │   ├── mod.rs              # list, read, write (atomic), rename, trash
        │   ├── detect.rs           # biner, encoding, BOM, LF/CRLF
        │   └── watcher.rs          # notify + ignore list + debounce
        ├── git/
        │   ├── mod.rs              # status, diff, stage, commit, branch
        │   ├── parse.rs            # parsing output git CLI
        │   └── hunk.rs             # pecah diff jadi hunk, buat patch, git apply
        └── search/
            └── mod.rs              # jalankan rg, batalkan, batasi hasil
```

Catatan:
- Satu crate dengan banyak modul. Pecah jadi workspace multi-crate hanya kalau logikanya akan dipakai ulang.
- State layout (workspace, pane, tab) hidup di frontend. Rust hanya menyimpan dan memuat sesi sebagai JSON.
- Tidak ada `preview.rs`: preview memakai iframe, jadi tidak butuh kode Rust.

---

## 4. Detail fitur

### 4.1 Terminal

- Spawn shell sebagai login dan interactive (`-l -i`) supaya environment mise dan PATH manual (PHP, Composer) terbaca.
- Terminal baru (termasuk yang pertama) selalu dibuka di folder proyek.
- Environment shell: `TERM=xterm-256color`, `COLORTERM=truecolor`, `TERM_PROGRAM=glzr`. Variabel yang diatur app untuk dirinya sendiri (misalnya `WEBKIT_*`) dibersihkan supaya tidak bocor ke shell.
- Output dibatch di Rust (kirim tiap sekitar 8-16 ms) dengan backpressure.
- Scrollback dibatasi (default 5.000 baris).
- Copy/paste:
  - Ctrl+C: copy kalau ada seleksi, selain itu diteruskan sebagai SIGINT.
  - Paste: Ctrl+Shift+V (satu-satunya shortcut tiga tombol, mengikuti standar terminal Linux) atau klik tengah mouse. Bracketed paste aktif. Konfirmasi untuk paste sangat besar.
- Addon: `fit` (resize), `search` (Alt+S), `web-links` (URL klikable), `webgl`, `unicode11`.
- Link `localhost:xxxx` yang diklik membuka tab preview (atau memakai tab preview yang sudah ada).
- Zoom font (Alt+= dan Alt+-), pilihan shell di settings.
- Saat shell berakhir, tab menampilkan "proses selesai" dengan opsi restart.
- Tab terminal tidak dihancurkan saat berpindah tab atau workspace, hanya disembunyikan.

### 4.2 File dan editor

**Dasar (gratis dari CodeMirror 6):** undo/redo, copy/cut/paste, select all, find/replace, multi-cursor, indent, comment toggle, bracket matching, auto-close bracket, fold, word wrap.

**Dibuat sendiri:**
- Save (Ctrl+S) dan indikator dirty di tab. Ctrl+S tanpa perubahan adalah no-op.
- Tab dengan close dan konfirmasi kalau belum tersimpan.
- Reload otomatis saat file berubah dari luar. Kalau sedang dirty, tampilkan prompt konflik.
- Quick open (Ctrl+P) dengan fuzzy search nama file. Go to line.
- Satu path = satu tab (path dinormalisasi).
- Deteksi LF/CRLF, BOM, dan encoding. Simpan sesuai aslinya.
- Atomic write: tulis ke file sementara lalu `rename`.

**Jenis file yang dibuka:**

| Jenis | Perlakuan |
|---|---|
| Teks/kode | CodeMirror dengan highlight sesuai bahasa |
| Gambar (png, jpg, gif, webp, ico) | Preview gambar, bukan teks |
| SVG | Toggle antara gambar dan kode |
| Markdown | Teks biasa. Preview menyusul |
| Biner (zip, exe, dll.) | Tidak dibuka. Pesan "file biner" dan tombol buka di aplikasi luar |
| File > 5 MB atau baris sangat panjang | Tanpa highlight, atau read-only |
| Encoding non-UTF-8 | Peringatan dan read-only. Dukungan penuh pakai `encoding_rs` kemudian |

Deteksi biner: baca 8 KB pertama, ada byte `0x00` berarti biner.

**Bahasa:**

- Paket resmi `@codemirror/lang-*`: JavaScript/TypeScript (JSX/TSX), HTML, CSS, JSON, Markdown, Python, Rust, Go, C/C++, Java, PHP, SQL, XML, YAML, Sass/Less, Vue.
- `@codemirror/legacy-modes`: Shell/Bash, TOML, Dockerfile, Ruby, Lua, Swift, Kotlin, C#, PowerShell, Nginx, diff.
- Komunitas: Svelte lewat `@replit/codemirror-lang-svelte`.

Deteksi bahasa: ekstensi, nama file khusus (`Dockerfile`, `Makefile`, `.gitignore`, `.env`, `Cargo.lock`, `.bashrc`), lalu shebang. Semuanya satu tabel di `languages.ts`. Setiap bahasa dimuat dengan `import()` dinamis saat pertama dipakai.

Set awal: JS/TS, JSON, HTML, CSS, Markdown, Rust, Python, YAML, TOML, Shell, Svelte, Dockerfile.

### 4.3 Git panel

- Daftar file berubah: staged, unstaged, untracked, dengan badge (M, A, ?) yang juga muncul di file tree.
- Stage/unstage per file. Discard dengan konfirmasi (lewat trash/stash, bukan hapus permanen).
- Diff di tab diff (`@codemirror/merge`). Setiap hunk (blok perubahan) punya tombol **Stage**, **Unstage** (untuk hunk yang sudah staged), dan **Discard**. Hanya per hunk, tanpa stage per baris dan tanpa edit patch.
- Cara kerja per hunk: `hunk.rs` memecah output `git diff` menjadi hunk, membuat patch kecil untuk hunk yang dipilih, lalu menjalankan `git apply --cached` (stage), `git apply --cached --reverse` (unstage), atau `git apply --reverse` pada working tree (discard). Patch yang dibuang lewat discard disimpan di folder cache aplikasi sehingga bisa diterapkan kembali.
- Batas ukuran diff (misalnya 2 MB): di atas batas, tampilkan "diff terlalu besar" dan arahkan ke terminal.
- Kasus khusus yang ditangani dan diuji: file baru, file dihapus, rename, file biner (tidak ditampilkan per hunk), akhir file tanpa newline, CRLF.
- Input pesan commit, branch aktif, ganti branch, pull/push.
- Bukan repo git atau git tidak terpasang: tampilkan keadaan kosong yang jelas.
- Merge conflict, rebase berjalan, detached HEAD: hanya ditandai. Resolusi lewat terminal.
- Stash tidak didukung di panel. Gunakan terminal.

### 4.4 File tree

- Buat/rename/hapus file dan folder. Hapus lewat trash.
- Copy path, reveal di file manager.
- Filter mengabaikan `.git`, `node_modules`, `target`.
- Perubahan lewat file tree memperbarui tab yang terbuka.

### 4.5 Preview

- Tab bertipe preview berisi `<iframe>` dengan bar URL, reload, back/forward, dan tombol "buka di browser".
- Default hanya mengizinkan `http(s)://localhost` dan `127.0.0.1`.
- Tab preview yang tidak aktif di-suspend: iframe dikosongkan, URL disimpan.
- Halaman error ramah saat server belum jalan, plus tombol coba lagi.
- Opsional: deteksi port otomatis dari output terminal.

### 4.6 Pencarian proyek

- Panel **Search** di sidebar (Alt+G) dengan input teks, opsi regex, huruf besar/kecil, dan filter glob (misalnya `*.rs`).
- Rust menjalankan `rg --json` sebagai subproses dan mengirim hasil ke frontend lewat event secara bertahap. Hasil dikelompokkan per file dengan nomor baris dan potongan teks. Klik hasil membuka tab file tepat di baris itu.
- `rg` otomatis menghormati `.gitignore`, jadi `node_modules` dan `target` tidak ikut dicari.
- `rg` disertakan di installer sebagai sidecar Tauri, jadi tidak perlu dipasang terpisah. Kalau proses `rg` gagal dijalankan, tampilkan pesan error yang jelas.
- Mengetik ulang membatalkan pencarian yang masih berjalan (proses lama di-kill). Input di-debounce sekitar 200 ms.
- Hasil dibatasi (misalnya 2.000 baris) dengan penanda "hasil dipotong". Daftar dirender dengan virtual scrolling.
- Tidak ada replace di seluruh proyek.

### 4.7 Global

- Command palette (Alt+P): satu pintu untuk semua aksi, jadi keybind tidak perlu dihafal.
- Restore sesi: workspace, pane, tab, folder, dan kondisi sidebar.
- Saat pertama dibuka atau saat membuat workspace baru: satu pane berisi satu tab terminal. Tidak ada template workspace bawaan.
- Settings satu file TOML (lihat 6.6). Dua tema, **dark** (default, tidak mengikuti tema sistem) dan **light**, lewat CSS variables. Font: monospace sistem (default) atau JetBrains Mono Nerd Font sebagai opsi kedua. Font opsi kedua disertakan di installer (regular dan bold, format woff2) dan hanya dimuat saat dipilih di settings.
- Status bar: branch dan ahead/behind, bahasa, encoding, LF/CRLF, posisi kursor.
- Satu dialog untuk seluruh file belum tersimpan saat app ditutup.

### 4.8 Folder dan peluncuran

- Satu instance glzr membuka satu folder proyek. Folder itu menjadi dasar file tree, git, pencarian, dan working directory semua terminal.
- Tiga cara membuka folder: dialog (lewat command palette), argumen dari terminal (`glzr .` atau `glzr /path/proyek`), dan daftar folder terakhir (layar awal dan command palette).
- Peluncuran tanpa argumen memulihkan sesi folder terakhir. Kalau belum ada sesi atau foldernya hilang, tampilkan layar awal berisi tombol Buka folder dan daftar folder terakhir.
- Membuka folder lain dari dalam app mengganti proyek aktif, dengan konfirmasi kalau ada tab dirty atau terminal berjalan. Untuk dua proyek sekaligus, jalankan dua instance dengan folder berbeda.
- Dua instance dengan folder yang sama tidak diizinkan: instance kedua memfokuskan jendela yang sudah ada.

### 4.9 Jendela

- Satu jendela utama. Ukuran awal 1280×800, minimum 800×500. Ukuran dan posisi terakhir diingat dan dipulihkan.
- Posisi tersimpan yang berada di luar layar (misalnya monitor dicabut) dikoreksi ke monitor yang tersedia.
- Title bar bawaan sistem, tanpa title bar custom. Judul jendela: "nama-folder — glzr", atau "glzr" kalau belum ada folder.
- F11 untuk fullscreen.

---

## 5. Layout aplikasi

### 5.1 Hierarki

```
Workspace  →  Pane (hasil split)  →  Tab (file / terminal / preview / diff)
```

- **Tab** adalah isi: file, terminal, preview, atau diff. Satu tab satu jenis.
- **Pane** adalah kotak di layar yang memuat deretan tab sendiri. Hanya satu tab tampil dalam satu waktu. Satu pane boleh berisi tab dari jenis berbeda.
- **Split** membelah satu pane menjadi dua ke arah kanan atau bawah. Hasil belahan bisa dibelah lagi, jadi layout berbentuk pohon (seperti tiling window manager). Tidak ada jendela mengambang.
- **Workspace** adalah satu pohon pane lengkap. Pengguna berpindah antar workspace seperti desktop virtual (Alt+1, Alt+2, ...). Setiap workspace punya pohon sendiri, dan tab hidup di satu workspace saja.
- **Sidebar** (Files, Git, dan Search) berdiri sendiri di luar hierarki ini. Ia tampil di semua workspace dengan isi yang sama, dan tidak terikat ke workspace mana pun. Git dan Search hanya ada di sidebar, bukan sebagai tab. Sidebar bisa disembunyikan (Alt+E, Alt+V, atau Alt+G pada panel yang sedang terbuka) untuk ruang kerja yang lebih luas.

Cara pakai bebas: satu pane dengan banyak tab (seperti browser), atau beberapa pane berdampingan lewat split. Keduanya memakai mekanisme yang sama.

### 5.2 Kerangka layar

```
┌────────────────────────────────────────────────┐
│ 1 Preview │ 2 Kode │ 3 Agent │ +               │  ← bar workspace
├────────┬───────────────────────────────────────┤
│ Files  │ main.rs │ lib.rs │ +  │ zsh │ +       │
│  / Git │                      │                │
│        │   pane               │   pane         │  ← area pane
│ tree   │   (tab file)         │   (tab term)   │
│        │                      │                │
├────────┴───────────────────────────────────────┤
│ main ● 2↑ │ Rust │ UTF-8 │ LF │ Ln 12, Col 4   │  ← status bar
└────────────────────────────────────────────────┘
```

- **Bar workspace:** daftar workspace di atas. Klik atau Alt+angka untuk pindah.
- **Sidebar kiri:** Files, Git, dan Search sebagai tab kecil, satu aktif. Tampil di semua workspace dan tetap sama saat berpindah workspace. Menekan Alt+E, Alt+V, atau Alt+G pada panel yang sedang terbuka menyembunyikan sidebar, misalnya untuk tampilan penuh.
- **Area pane:** pohon pane dari workspace aktif.
- **Status bar:** satu baris tipis.

### 5.3 Contoh susunan

Nama workspace di bawah hanya contoh. Workspace baru selalu dimulai dengan satu pane berisi satu terminal, lalu disusun sendiri oleh pengguna. Sidebar tidak digambar di diagram kecil, tetapi tetap tersedia di semua susunan.

**Preview full (workspace "Preview")**
```
┌──────────────────────────────────────┐
│ :5173 │ +                            │
│                                      │
│          preview (full)              │
│                                      │
└──────────────────────────────────────┘
```

**Kode kiri-kanan (workspace "Kode")**
```
┌──────────────┬───────────────────────┐
│ main.rs │ +  │ zsh │ +               │
│   editor     │   terminal            │
└──────────────┴───────────────────────┘
```

**Kode atas-bawah**
```
┌──────────────────────────────────────┐
│ main.rs │ lib.rs │ +                 │
│              editor                  │
├──────────────────────────────────────┤
│ zsh │ build │ +                      │
│              terminal                │
└──────────────────────────────────────┘
```

**Tiga pane, preview full tinggi**
```
┌────────────────────────┬─────────────┐
│ App.svelte │ +         │ :5173 │ +   │
│        editor          │             │
├────────────────────────┤  preview    │
│ npm run dev │ +        │  (full)     │
│        terminal        │             │
└────────────────────────┴─────────────┘
```
Pohonnya:
```
Split kanan
├── Split bawah
│   ├── Pane (file)
│   └── Pane (terminal)
└── Pane (preview)
```
Urutan pembelahan menentukan bentuk: preview full tinggi karena ia anak langsung dari akar. Kalau dibelah ke bawah dulu baru ke kanan, hasilnya berbeda.

**Satu pane campur**
```
┌──────────────────────────────────────┐
│ main.rs │ zsh │ :5173 │ +            │
│        isi tab yang dipilih          │
└──────────────────────────────────────┘
```

**Agent (herdr):** satu pane penuh berisi tab terminal, dengan sidebar disembunyikan lewat shortcut sidebar.

### 5.4 Tombol "+" dan menu

Tombol "+" di tiap pane membuka menu: File…, Terminal, Preview. Tab baru dibuat di pane tersebut.

### 5.5 Interaksi penting

| Aksi | Perilaku |
|---|---|
| Split kanan / Split bawah | Membelah pane aktif. Tab aktif tetap, pane baru kosong dengan menu "+" |
| Tutup pane terakhir dalam split | Pane saudara mengisi ruangnya |
| Maximize pane (Alt+M) | Pane memenuhi area, toggle |
| Seret tab ke pane lain | Memindahkan tab |
| Klik file di git panel | Membuka tab diff |
| Klik hasil di panel Search | Membuka tab file tepat di baris hasil |
| Klik link `localhost` di terminal | Membuka atau memakai tab preview |
| Pindah workspace | Terminal dan preview di workspace lain tetap hidup. Sidebar tidak berubah |
| Alt+E / Alt+V / Alt+G | Membuka panel Files / Git / Search di sidebar dan memfokuskan inputnya. Menekannya saat panel itu sudah terbuka menyembunyikan sidebar |

### 5.6 Aturan implementasi layout

- Pohon layout adalah data murni (lihat bagian 6) dan sekaligus format sesi.
- Splitter cukup CSS flex dengan handle drag, tanpa library.
- Lebar/tinggi minimum pane sekitar 200 px.
- Fokus pane harus jelas (garis aksen tipis), karena keymap bergantung padanya.
- Terminal disembunyikan, bukan dihancurkan. Panggil `fit` lagi saat ditampilkan.
- Preview yang disembunyikan di-suspend (iframe dikosongkan).

### 5.7 Keymap berbasis fokus

- Fokus di terminal: hanya shortcut aplikasi berawalan **Alt** yang ditangkap (maksimal dua tombol). Sisanya, termasuk semua Ctrl+huruf, diteruskan ke shell.
- Fokus di editor: shortcut standar editor berlaku.
- Semua keybind didefinisikan di satu tabel `[keybindings]` di settings dan bisa diubah. Duplikat dideteksi saat settings dimuat.

**Keybind default**

Prinsip: aksi aplikasi memakai Alt + satu huruf (maksimal dua tombol). Ctrl+huruf tidak pernah ditangkap saat fokus di terminal, sehingga tool di terminal tetap normal, termasuk herdr yang memakai prefix Ctrl+B. Hanya aksi yang paling sering dipakai mendapat shortcut. Sisanya lewat command palette.

Aplikasi (aktif di mana saja, termasuk saat fokus di terminal):

| Aksi | Keybind |
|---|---|
| Command palette | Alt+P |
| Quick open file | Alt+O |
| Sidebar Files | Alt+E |
| Sidebar Git | Alt+V |
| Sidebar Search (cari di proyek) | Alt+G |
| Terminal baru di pane aktif | Alt+T |
| Tutup tab aktif | Alt+W |
| Split kanan | Alt+R |
| Split bawah | Alt+J |
| Tutup pane | Alt+Q |
| Maximize pane | Alt+M |
| Pindah fokus antar pane | Alt+←, Alt+→, Alt+↑, Alt+↓ |
| Tab berikutnya / sebelumnya | Alt+PageDown / Alt+PageUp |
| Pindah ke workspace 1-9 | Alt+1 … Alt+9 |
| Zoom font masuk / keluar / reset | Alt+= / Alt+- / Alt+0 |

Catatan:
- Alt+E, Alt+V, dan Alt+G membuka panel sidebar dan memfokuskan inputnya. Kalau panel itu sudah terbuka, tombol yang sama menyembunyikan sidebar.
- Hanya lewat command palette (tanpa shortcut): workspace baru, tutup workspace, buka folder, preview baru, serta stage, unstage, dan discard hunk.
- Beberapa huruf Alt dipakai readline di shell (misalnya Alt+T menukar kata, Alt+P mencari riwayat). Saat fokus di terminal, shortcut aplikasi didahulukan. Alt+B, Alt+F, dan Alt+D (gerak dan hapus kata) sengaja tidak dipakai. Semua keybind bisa diubah di settings.

Saat fokus di editor (standar CodeMirror, tidak ditangkap saat fokus di terminal):

| Aksi | Keybind |
|---|---|
| Simpan | Ctrl+S |
| Undo / redo | Ctrl+Z / Ctrl+Y |
| Cari dan ganti di file | Ctrl+F |
| Go to line | Ctrl+G |
| Toggle komentar | Ctrl+/ |
| Pilih kemunculan berikutnya | Ctrl+D |

Saat fokus di terminal:

| Aksi | Keybind |
|---|---|
| Copy | Ctrl+C (hanya kalau ada seleksi) |
| Paste | Ctrl+Shift+V atau klik tengah mouse |
| Cari di terminal | Alt+S |

Konteks lain:
- Preview: Ctrl+L fokus ke bar URL, Ctrl+R atau F5 reload.
- Kotak pesan commit: Ctrl+Enter melakukan commit.

---

## 6. Model data dan IPC

### 6.1 Struktur layout (frontend, TypeScript)

```ts
type TabKind = "file" | "terminal" | "preview" | "diff";

interface Tab {
  id: string;
  kind: TabKind;
  title: string;
  // file: path | terminal: ptyId, cwd | preview: url | diff: path, staged
  data: Record<string, unknown>;
}

interface Pane {
  type: "pane";
  id: string;
  tabs: Tab[];
  activeTabId: string | null;
}

interface Split {
  type: "split";
  direction: "row" | "column";
  ratio: number; // 0..1, bagian untuk anak pertama
  first: LayoutNode;
  second: LayoutNode;
}

type LayoutNode = Pane | Split;

interface Workspace {
  id: string;
  name: string;
  root: LayoutNode;
}

interface SidebarState {
  visible: boolean;
  panel: "files" | "git" | "search";
  width: number;
}

interface SessionFile {
  version: number; // versi skema, mulai dari 1
  folder: string | null;
  sidebar: SidebarState; // satu untuk seluruh aplikasi
  activeWorkspaceId: string;
  workspaces: Workspace[];
}
```

Operasi di `tree.ts` berupa fungsi murni (split, close, moveTab, resize) sehingga mudah diuji.

### 6.2 Padanan di Rust (untuk sesi)

```rust
#[derive(Serialize, Deserialize)]
struct SessionFile {
    version: u32,
    folder: Option<PathBuf>,
    sidebar: SidebarState,
    active_workspace_id: String,
    workspaces: Vec<Workspace>,
}

#[derive(Serialize, Deserialize)]
enum LayoutNode {
    Pane { id: String, tabs: Vec<TabSpec>, active: Option<String> },
    Split { direction: Direction, ratio: f32,
            first: Box<LayoutNode>, second: Box<LayoutNode> },
}
```

Rust tidak mengelola layout. Ia hanya membaca dan menulis sesi (JSON) di folder data aplikasi, dan memvalidasi isinya saat dimuat.

### 6.3 Command IPC

| Modul | Command |
|---|---|
| pty | `pty_spawn`, `pty_write`, `pty_resize`, `pty_kill` (output lewat event) |
| fs | `fs_list_dir`, `fs_read_file`, `fs_write_file`, `fs_rename`, `fs_trash`, `fs_watch`, `fs_unwatch` (perubahan lewat event) |
| git | `git_status`, `git_diff`, `git_stage`, `git_unstage`, `git_discard`, `git_hunks`, `git_stage_hunk`, `git_unstage_hunk`, `git_discard_hunk`, `git_commit`, `git_branches`, `git_checkout`, `git_pull`, `git_push` |
| search | `search_start`, `search_cancel` (hasil lewat event) |
| settings | `settings_load`, `settings_save` |
| session | `session_load`, `session_save` |
| sistem | `reveal_in_file_manager`, `open_external` |

### 6.4 Event dari Rust ke frontend

`pty:output`, `pty:exit`, `fs:changed`, `fs:watch-error`, `search:results`, `search:done`, `search:error`.

### 6.5 Format sesi dan versi skema

Sesi disimpan sebagai satu file JSON (`session.json`) di folder data aplikasi. File ini mencatat susunan layar terakhir supaya bisa dipulihkan saat app dibuka lagi.

Isi yang disimpan:
- **Umum:** folder yang dibuka, kondisi sidebar (terlihat/tersembunyi, panel aktif, lebar), workspace aktif.
- **Tiap workspace:** nama dan pohon pane beserta tab.
- **Tiap tab:** file berisi path dan posisi kursor, terminal hanya dicatat keberadaannya (proses lama tidak dihidupkan kembali, shell baru dibuka di folder proyek), preview berisi URL, diff berisi path.
- **Tidak disimpan:** isi file (draf belum tersimpan ditangani autosave terpisah), output terminal, dan isi iframe.

Field `version` adalah nomor format file. Mulai dari `1` dan hanya naik saat struktur berubah dengan cara yang membuat file lama tidak cocok lagi (misalnya field diganti nama atau dihapus). Menambah field opsional baru tidak perlu menaikkan versi.

Aturan saat memuat:
1. File tidak ada: pakai default (satu workspace, satu pane, satu terminal, sidebar terlihat).
2. `version` sama dengan versi app: validasi isinya (path ada, pohon valid), buang bagian yang rusak, pakai sisanya.
3. `version` lebih lama: jalankan fungsi migrasi berantai (`v1 → v2 → v3`) lalu muat hasilnya.
4. `version` lebih baru dari app, atau JSON rusak: salin file ke `session.json.bak`, pakai default, dan tampilkan pemberitahuan singkat. Jangan menimpa file lama tanpa cadangan.

Pengujian: sediakan file fixture untuk tiap versi lama dan test yang memastikan semuanya termigrasi ke versi terbaru. Selama masih `version: 1`, fungsi migrasi cukup berupa kerangka kosong.

Penulisan memakai atomic write seperti file biasa, dan dilakukan dengan debounce saat layout berubah serta sekali lagi saat app ditutup.

### 6.6 Skema settings

File `settings.toml` di folder konfigurasi aplikasi. Semua kunci opsional: kunci yang tidak ada memakai nilai default di bawah. Kunci tidak dikenal diabaikan dengan peringatan, dan file yang rusak tidak pernah ditimpa.

```toml
[appearance]
theme = "dark"                 # "dark" | "light". Tidak mengikuti tema sistem
font_family = "system-mono"    # "system-mono" | "jetbrains-mono-nerd"
font_size = 14

[editor]
tab_size = 4
insert_spaces = true
word_wrap = false
line_numbers = true
large_file_limit_mb = 5        # di atas ini: tanpa highlight atau read-only
long_line_limit = 10000        # karakter per baris sebelum highlight dimatikan
draft_autosave_seconds = 5

[terminal]
shell = ""                     # kosong = $SHELL
shell_args = ["-l", "-i"]
extra_env = {}
scrollback = 5000
cursor_style = "block"         # "block" | "bar" | "underline"
confirm_paste_over_kb = 64

[files]
ignore = [".git", "node_modules", "target"]

[git]
command_timeout_seconds = 30
diff_size_limit_mb = 2

[search]
max_results = 2000
snippet_max_chars = 300
debounce_ms = 200

[preview]
allowed_hosts = ["localhost", "127.0.0.1"]
auto_detect_port = false

[session]
restore = true
recent_folders_limit = 5

[logging]
level = "info"                # "error" | "warn" | "info" | "debug"

[keybindings]
# nama_aksi = "kombinasi". Default mengikuti tabel di 5.7
command_palette = "Alt+P"
quick_open = "Alt+O"
sidebar_files = "Alt+E"
sidebar_git = "Alt+V"
sidebar_search = "Alt+G"
new_terminal = "Alt+T"
close_tab = "Alt+W"
split_right = "Alt+R"
split_down = "Alt+J"
close_pane = "Alt+Q"
maximize_pane = "Alt+M"
terminal_find = "Alt+S"
```

Aturan:
- Perubahan settings berlaku langsung tanpa restart (file dipantau).
- Nilai di luar rentang atau jenis salah → pakai default untuk kunci itu saja dan tampilkan pesan.
- Keybind duplikat atau bentrok dengan aturan fokus terminal ditandai saat dimuat.

### 6.7 Lokasi file, log, dan privasi

Mengikuti standar XDG (variabel `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, dan `XDG_STATE_HOME` dihormati kalau diatur).

| Jenis | Lokasi default |
|---|---|
| Settings | `~/.config/glzr/settings.toml` |
| Sesi (dan cadangannya `.bak`) | `~/.local/share/glzr/session.json` |
| Daftar folder terakhir | `~/.local/share/glzr/recent.json` |
| Draf file belum tersimpan | `~/.cache/glzr/drafts/` |
| Log | `~/.local/state/glzr/glzr.log` |

- Log berotasi (maksimal 2 MB per file, simpan 3 file), level default `info`. Isinya error dan peristiwa penting, tidak pernah memuat isi file atau output terminal.
- Tanpa telemetri. App tidak mengirim data apa pun. Akses jaringan hanya terjadi atas tindakan pengguna: `git push/pull` lewat `git`, dan preview ke localhost.

---

## 7. Target performa

Angka berikut adalah perkiraan awal, **harus diukur ulang** setelah aplikasi berjalan.

| Item | Target / perkiraan |
|---|---|
| Ukuran installer | sekitar 12-25 MB (termasuk `rg` sekitar 5 MB dan font sekitar 1-3 MB) |
| RAM baseline (webview, UI, satu terminal) | sekitar 80-150 MB |
| Per bahasa CodeMirror yang dimuat | sekitar +1-3 MB |
| Per file di editor | sekitar 3-10x ukuran teks |
| Per terminal (scrollback penuh) | sekitar 5-12 MB |
| Gambar besar (decode) | 4000x3000 sekitar 48 MB, lepaskan saat tab ditutup |
| Diff di tab diff | dimuat hanya saat tab dibuka, batas sekitar 2 MB |
| Hasil pencarian | dibatasi sekitar 2.000 baris. Tidak ada RAM tetap karena `rg` berhenti setelah selesai |
| Sesi harian wajar | sekitar 150-250 MB |

Cara menjaga:
- Bundle frontend di luar CodeMirror di bawah 300 KB. Bahasa dimuat dinamis.
- Batasi tab editor yang hidup. Tab lama disimpan sebagai path dan posisi kursor, `EditorState` dibuat ulang saat dibuka.
- Preview di-suspend saat tidak tampil.
- Batas scrollback dan batas ukuran file tetap dipertahankan.
- Diff dan hasil pencarian dibatasi ukurannya. Daftar hasil memakai virtual scrolling.
- Watcher hanya untuk folder terbuka dan mengabaikan `node_modules`, `target`, `.git`.

**Cara mengukur di Linux:** WebKitGTK memecah proses menjadi beberapa (`WebKitWebProcess`, `WebKitNetworkProcess`). Gunakan `smem -P glzr` (kolom PSS) atau jumlahkan PSS semua prosesnya. Catat hasilnya di README.

---

## 8. Best practice dan panduan kode

### 8.1 Umum

- Satu commit satu perubahan logis. Pesan commit dengan format `tipe: ringkasan` (`feat`, `fix`, `refactor`, `test`, `docs`, `chore`).
- Cabang kecil dan berumur pendek. `main` selalu bisa di-build.
- Setiap bug nyata dicatat di `KNOWN_ISSUES.md` beserta status dan solusinya.
- Ukur sebelum mengoptimasi. Jangan menebak soal performa.

### 8.2 Rust

- Jalankan `cargo fmt` dan `cargo clippy -- -D warnings` sebelum commit.
- **Error:** satu tipe `AppError` dengan `thiserror`, diimplementasikan `Serialize` supaya bisa dikirim ke frontend. Hindari `unwrap()` dan `expect()` di jalur produksi. Pakai `?`.
- **Command Tauri tipis:** command hanya memvalidasi input dan memanggil fungsi di modul. Logika ada di modul biasa supaya mudah diuji tanpa Tauri.
- **State bersama:** `Arc<Mutex<...>>` hanya di `state.rs`. Kunci dipegang sesingkat mungkin dan tidak pernah dipegang saat I/O atau `await`.
- **Thread dan channel:** PTY reader berjalan di thread sendiri dan mengirim data lewat channel. Pastikan thread berhenti saat sesi ditutup.
- **Tipe sebagai dokumentasi:** pakai `enum` untuk status (misalnya `GitStatus`, `EncodingKind`), bukan string.
- **Path:** gunakan `PathBuf`/`Path`, bukan `String`. Normalisasi dan validasi di satu fungsi.
- **Subproses:** `std::process::Command` dengan argumen terpisah (tanpa shell). Set `LC_ALL=C`, `GIT_TERMINAL_PROMPT=0`. Beri timeout.
- **Subproses panjang** (pencarian) harus bisa dibatalkan: simpan handle proses dan kill saat pencarian baru dimulai atau tab ditutup. Pola pencarian dikirim sebagai argumen setelah `-e`, tidak pernah disambung ke string perintah.
- **Dependency:** tambah dengan hati-hati. Periksa dampak ke ukuran binary.
- **Dokumentasi:** `///` untuk fungsi publik. Tulis alasan, bukan sekadar apa.
- **Belajar:** tulis catatan singkat tiap kali menemukan konsep baru (ownership, lifetime, trait) di `docs/belajar-rust.md`.

### 8.3 Frontend (Svelte + TypeScript)

- TypeScript mode `strict`. Hindari `any`.
- Semua panggilan ke Rust lewat `ipc.ts`, dengan tipe untuk setiap command dan event. Komponen tidak memanggil `invoke()` langsung.
- Layout adalah data murni di store. Komponen hanya merender state itu.
- Operasi pohon layout berupa fungsi murni di `tree.ts` dan diuji dengan Vitest.
- Satu komponen satu tanggung jawab. Komponen tab (`FileTab`, `TerminalTab`, ...) tidak tahu posisinya di layout.
- Pembersihan wajib: lepas listener event, dispose xterm dan `EditorView`, unlisten event Tauri saat komponen dihancurkan.
- Tema dan warna lewat CSS variables, bukan nilai hardcode.
- Hindari library UI. Komponen kecil dibuat sendiri.
- Impor berat (bahasa CodeMirror, addon opsional) selalu dinamis.
- Aksesibilitas dasar: fokus terlihat, semua aksi punya jalur keyboard.

### 8.4 Desain IPC

- Command bernama `modul_aksi` (`git_status`, `fs_read_file`).
- Payload kecil dan terstruktur. Rust yang memparse (misalnya output git), frontend hanya menerima data ringkas.
- Data besar dan streaming (output terminal, perubahan file) lewat event, bukan return value.
- Setiap command mengembalikan `Result<T, AppError>`. Frontend menampilkan pesan yang bisa dipahami, bukan error mentah.
- Izinkan hanya command yang dipakai di `capabilities`.

### 8.5 Gaya kode

| Hal | Aturan |
|---|---|
| Rust | `snake_case` fungsi/variabel, `CamelCase` tipe, `SCREAMING_SNAKE_CASE` konstanta |
| TypeScript | `camelCase` fungsi/variabel, `PascalCase` tipe dan komponen |
| File komponen Svelte | `PascalCase.svelte` |
| File lain | `kebab-case` atau `snake_case` mengikuti bahasanya |
| Komentar | Jelaskan alasan, bukan apa yang sudah jelas dari kode |
| Fungsi | Pendek dan melakukan satu hal |

### 8.6 Checklist sebelum merge

- [ ] `cargo fmt`, `cargo clippy`, `cargo test` lulus
- [ ] Lint dan test frontend lulus
- [ ] Tidak ada `unwrap()` baru di jalur produksi
- [ ] Listener dan sumber daya dibersihkan
- [ ] Kasus error punya pesan jelas
- [ ] Dampak ke RAM/ukuran dipertimbangkan
- [ ] `KNOWN_ISSUES.md` diperbarui kalau ada temuan

---

## 9. Kemungkinan masalah dan solusi

Format: masalah → keputusan.

### 9.1 Terminal

- Shell tidak menemukan `node`/`python` (PATH mise) → spawn `-l -i`, sediakan setting shell dan env tambahan.
- Proses anak tetap hidup setelah tab/app ditutup → kill seluruh process group, bukan hanya shell. Saat app keluar, kill semua sesi.
- Output deras (`yes`, `cat` file besar) membuat UI macet → batching output di Rust (8-16 ms) dan backpressure.
- Ukuran terminal tidak sinkron setelah resize/split → `fit` addon plus `pty_resize` dengan debounce.
- Karakter rusak (emoji, CJK, UTF-8 terpotong di batas chunk) → buffer byte sisa sebelum decode.
- Ctrl+C bentrok dengan copy → copy hanya kalau ada seleksi.
- Shell mati sendiri → tampilkan "proses selesai" dengan opsi restart, jangan tab kosong.
- Paste multi-baris berbahaya → bracketed paste dan konfirmasi untuk paste sangat besar.

### 9.2 File dan editor

- File berubah dari luar saat dirty → prompt konflik: simpan milikku / muat ulang / bandingkan.
- File dihapus atau di-rename dari luar → tab ditandai "terhapus", tawarkan simpan ulang.
- Save gagal (permission, disk penuh, read-only) → error jelas, tab tetap dirty, tidak ada data hilang.
- Crash saat ada perubahan belum tersimpan → autosave draf ke folder cache, tawarkan pemulihan saat dibuka lagi.
- Mati listrik saat menyimpan → atomic write (file sementara lalu `rename`).
- Symlink, file tanpa izin baca, nama file dengan spasi/unicode → ditangani eksplisit di modul `fs`, masing-masing dengan test.
- File besar atau satu baris sangat panjang (minified JS) → batas ukuran dan panjang baris, otomatis tanpa highlight atau read-only.
- Biner, encoding non-UTF-8, BOM, CRLF campuran → deteksi saat baca, simpan sesuai aslinya, peringatan bila tidak bisa dijamin.
- Dua tab untuk file yang sama → satu path satu tab (normalisasi path).
- Ctrl+S tanpa perubahan → no-op, mtime tidak berubah.
- Watcher kebanjiran event (`npm install`, build) → abaikan folder berat, debounce, batasi jumlah watch.

### 9.3 Git

- Bukan repo git atau git belum terpasang → keadaan kosong yang jelas, bukan error.
- Repo besar, `status` lambat → async dengan timeout, tampilkan loading, jangan blok UI.
- Output git berubah menurut locale/versi → paksa `LC_ALL=C`, pakai `--porcelain=v2 -z`, parser diuji dengan fixture.
- Nama file aneh (spasi, unicode, newline) → pakai output `-z`.
- Merge conflict, rebase berjalan, detached HEAD → deteksi dan tampilkan status. Resolusi lewat terminal.
- Commit gagal (belum set `user.name`, hook gagal) → tampilkan stderr git apa adanya.
- Push/pull minta kredensial → jangan tangani di GUI. Set `GIT_TERMINAL_PROMPT=0` agar tidak hang, dan arahkan ke terminal.
- Discard salah klik → konfirmasi, buang lewat trash/stash.
- `index.lock` bentrok → deteksi, pesan jelas, jangan hapus lock otomatis.
- Submodule, worktree, repo di dalam repo → anggap satu repo aktif per folder dan dokumentasikan batasnya.
- Patch hunk gagal diterapkan (file berubah sejak diff dimuat) → muat ulang diff dan tampilkan pesan, jangan menebak.
- Discard hunk salah klik → konfirmasi, dan patch yang dibuang disimpan di cache sehingga bisa dipulihkan.
- Kasus tepi di parser hunk (file baru/dihapus/rename, tanpa newline di akhir file, CRLF, file biner) → fixture test untuk masing-masing.

### 9.4 Pencarian proyek

- Proses `rg` gagal dijalankan (binary rusak atau izin eksekusi hilang) → pesan jelas dengan petunjuk memasang ulang app.
- Kata terlalu umum (puluhan ribu hasil) → batas hasil, penanda "hasil dipotong", virtual scrolling.
- Mengetik cepat membuat pencarian menumpuk → debounce dan kill proses lama.
- Regex tidak valid → tampilkan pesan error dari `rg`, bukan hasil kosong.
- Baris sangat panjang (file minified) → potong potongan teks hasil (misalnya 300 karakter).
- Folder sangat besar atau lambat → timeout dan tombol batal. Folder berat diabaikan lewat `.gitignore`.
- Pola diawali `-` terbaca sebagai opsi → kirim lewat `-e`.

### 9.5 Preview

- Server belum jalan atau port salah → halaman error ramah dan tombol coba lagi.
- Situs memblokir iframe → tombol "buka di browser" sebagai cadangan.
- HMR/websocket dev server putus → tombol reload dan deteksi koneksi mati.
- Memori iframe menumpuk → suspend tab tidak aktif (iframe dikosongkan). Tes: buka-tutup 50x, RAM kembali ke baseline.
- URL bebas diketik → batasi ke `http(s)` dan `localhost`/`127.0.0.1` secara default, tolak `file://` dan sejenisnya.
- Kebutuhan devtools atau situs eksternal → pertimbangkan webview native belakangan, tanpa mengubah konsep tab.

### 9.6 Layout dan workspace

- Terlalu banyak split → ukuran minimum pane, split ditolak kalau ruang tidak cukup.
- Menutup pane yang berisi tab dirty → konfirmasi, jangan hilang diam-diam.
- Menutup pane dengan terminal aktif → konfirmasi, dan proses di-kill jelas.
- Menyeret tab antar pane → state tab (xterm, EditorView) dipindahkan, bukan dibuat ulang.
- Semua tab dalam workspace ditutup → pane tetap ada dengan menu "+", bukan layar kosong. Workspace baru dimulai dengan satu tab terminal.
- Sesi rusak atau versi tidak dikenal → cadangkan file lama, pakai layout default (satu terminal), jangan crash. Versi lama dimigrasi (lihat 6.5).

### 9.7 Lintas fitur dan sistem

- Folder yang dibuka sudah hilang saat restore sesi → lewati dengan pesan.
- Limit inotify Linux terlampaui → tangkap error, turun ke polling terbatas, tampilkan petunjuk menaikkan `fs.inotify.max_user_watches`.
- Window blank di WebKitGTK + NVIDIA → dokumentasikan `WEBKIT_DISABLE_DMABUF_RENDERER=1` (atau `WEBKIT_DISABLE_COMPOSITING_MODE=1`), sediakan wrapper launcher.
- Dua instance membuka folder yang sama → single-instance plugin atau peringatan.
- Hapus/rename file terbuka lewat file tree → tab ikut diperbarui.
- Keybind bentrok → keymap berbasis fokus dan deteksi duplikat.
- Settings TOML rusak → pakai default, tampilkan error, jangan menimpa file pengguna.
- Window kecil, HiDPI, dua monitor → tata letak minimum, uji manual di skala 100/150/200%.
- Menutup app dengan tab dirty → satu dialog untuk semua file belum tersimpan.

---

## 10. Strategi pengujian

**Unit test Rust** (deterministik):
- parser git dari fixture, deteksi biner/encoding/BOM/CRLF
- pemecahan hunk dan pembuatan patch (fixture: file baru, dihapus, rename, tanpa newline, CRLF)
- parsing keluaran `rg --json`
- atomic write, normalisasi path, filter ignore
- validasi, (de)serialisasi, dan migrasi versi sesi (dengan fixture tiap versi)

**Integration test Rust:**
- repo git sementara dengan crate `tempfile`, termasuk stage, unstage, dan discard hunk lalu cek hasil `git diff --cached`
- PTY menjalankan `echo`, `sleep`, dan proses yang di-kill

**Test frontend (Vitest):**
- operasi pohon layout: split, close, move tab, resize, serialisasi
- resolusi keymap berbasis fokus
- pemetaan bahasa dari ekstensi/nama file/shebang

**Stress test manual (daftar tetap):**
- output terminal 100 MB
- 10.000 file di tree
- file 50 MB
- buka-tutup tab preview 50x
- 10 tab terminal
- pencarian kata umum di repo besar (hasil terpotong dan UI tetap responsif)

**Checklist rilis (sulit diotomasi):**
- window blank di WebKitGTK
- HiDPI 100/150/200%
- restore sesi dan crash recovery
- pindah workspace dengan terminal berjalan
- drag tab antar pane

---

## 11. Keamanan

- **IPC:** izinkan hanya command yang dipakai lewat `capabilities`.
- **Path traversal:** normalisasi path di setiap `fs_*` dan batasi ke folder workspace, kecuali aksi yang eksplisit.
- **CSP ketat:** tidak ada HTML dari file yang di-render tanpa sanitasi.
- **Preview:** hanya `http(s)` ke `localhost`/`127.0.0.1` secara default. Atribut `sandbox` pada iframe, dilonggarkan seperlunya.
- **Subproses:** argumen terpisah, tidak lewat string shell.
- **Pencarian:** hanya di dalam folder workspace. Pola dikirim sebagai argumen terpisah (`-e`), bukan disambung ke perintah.
- **Privasi:** tanpa telemetri. Log tidak memuat isi file atau output terminal.
- **Kredensial:** app tidak menyimpan atau menangani kredensial git. Biarkan SSH agent dan credential helper bekerja lewat terminal.

---

## 12. Lingkungan pengembangan

**Perangkat:** Axioo Pongo 735 (RAM 16 GB, SSD 512 GB, RTX 3050 6 GB), Zorin OS 18 (basis Ubuntu 24.04).

**Toolchain:** Python, Node.js, Rust, dan Go lewat mise. PHP dan Composer dipasang manual.

**Dependency sistem untuk Tauri 2:**
```
sudo apt install build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev \
  libwebkit2gtk-4.1-dev
```

**Catatan khusus:**
- Tauri 2 di Linux memakai WebKitGTK 4.1, bukan Chromium. Render dan RAM berbeda dari Windows/Mac.
- Terminal harus mewarisi environment mise, jadi shell di-spawn sebagai login/interactive.
- Pada laptop hybrid, biarkan app berjalan di GPU bawaan (jangan dipaksa ke NVIDIA). Kalau window blank, pakai env var di bagian 9.7.
- Build `.deb` atau AppImage dari mesin Ubuntu 24.04 cukup untuk dipakai sendiri dan dipamerkan.

---

## 13. Urutan implementasi

Semua fitur masuk satu rilis end-to-end. Urutan di bawah hanya untuk mengatur pengerjaan dan belajar Rust, bukan pembagian rilis.

1. **Spike teknis:** prototipe kecil untuk tiga hal berisiko: xterm.js + PTY (termasuk addon WebGL) di WebKitGTK, iframe yang menampilkan server dev (Vite, `php -S`, `python -m http.server`), dan `git apply --cached` untuk satu hunk. Hasil dan masalahnya dicatat di `KNOWN_ISSUES.md`.
2. **Fondasi:** setup Tauri + SvelteKit, `error.rs`, `state.rs`, `ipc.ts`, folder proyek dan peluncuran, satu pane dengan satu tab.
3. **`fs/`:** list, read, write atomic, deteksi biner/encoding/LF-CRLF. Paling ramah untuk belajar `Result`, `?`, dan `serde`. Unit test sejak awal.
4. **Editor:** tab file dengan CodeMirror, dirty state, save, bahasa dinamis, quick open.
5. **`git/`:** `std::process::Command`, parsing `--porcelain=v2 -z`, struct dan enum. Panel, diff, stage per file dan per hunk (`hunk.rs`, `git apply --cached`), commit.
6. **`pty/`:** thread, `Arc<Mutex<...>>`, channel. Bagian tersulit, dikerjakan setelah ownership dan borrowing terasa nyaman. Tab terminal.
7. **Layout:** pohon pane, split, maximize, drag tab, workspace, sesi.
8. **Watcher, file tree, dan pencarian:** `notify`, debounce, sinkronisasi tab. Pencarian lewat `rg` dengan pembatalan dan batas hasil.
9. **Preview:** tab iframe, bar URL, suspend, integrasi link dari terminal.
10. **Global:** command palette, keymap berbasis fokus, settings, tema, status bar.
11. **Pengerasan:** stress test, ukur RAM, perbaiki `KNOWN_ISSUES.md`, checklist rilis, README dengan angka pengukuran.

Kriteria lulus spike (langkah 1):
- **xterm.js + PTY:** output 100 MB (misalnya `yes | head -c 100M`) tidak membekukan UI, Ctrl+C menghentikannya, dan herdr serta `htop` tampil benar, termasuk setelah resize. Addon WebGL berjalan, atau fallback tanpa glitch.
- **Preview:** server Vite dengan HMR dan `php -S` tampil di iframe, dan HMR memperbarui halaman.
- **Hunk:** dari satu file berisi tiga hunk di fixture, stage satu hunk menghasilkan `git diff --cached` yang hanya memuat hunk itu, dan unstage mengembalikannya.
- **Baseline:** RAM (PSS semua proses) dan ukuran build release dicatat.
- Kalau ada yang gagal, tentukan alternatifnya dan catat di `KNOWN_ISSUES.md` sebelum lanjut ke langkah berikutnya.

Begitu terminal, editor, dan git panel bisa dipakai (langkah 3 sampai 6), pengembangan fitur sisanya dikerjakan di dalam glzr sendiri (dogfooding), mengikuti dokumen ini.

---

## 14. Kriteria selesai per fitur

Satu fitur dianggap selesai kalau kalimat di kolom kanan terpenuhi dan kasus terkait di bagian 9 sudah ditangani dan diuji.

| Fitur | Selesai bila |
|---|---|
| Terminal | Beberapa tab berjalan dengan shell login yang mewarisi environment mise, agent (misalnya herdr) berjalan tanpa glitch tampilan, resize/split akurat, copy/paste benar, dan tidak ada proses yatim setelah tab atau app ditutup |
| File dan editor | File teks umum dibuka, diedit, dan disimpan secara atomic dengan highlight untuk set bahasa awal. Biner, file besar, dan encoding non-UTF-8 ditangani. Reload dari luar, konflik, dan pemulihan draf setelah crash bekerja |
| File tree | Buat, rename, dan hapus ke trash bekerja, badge git tampil, folder berat diabaikan, dan tab ikut diperbarui |
| Git | Status, diff, stage/unstage/discard per file dan per hunk, commit, ganti branch, pull/push memberi hasil identik dengan `git` CLI pada fixture, dengan pesan error yang jelas |
| Pencarian | Teks ditemukan di repo besar dengan hasil bertahap, klik hasil membuka baris yang tepat, mengetik ulang membatalkan pencarian lama, dan hasil terpotong dengan aman |
| Preview | Server dev lokal tampil dan bisa di-reload, URL dibatasi ke host yang diizinkan, tab tidak aktif di-suspend, dan ada cadangan buka di browser |
| Layout | Workspace, split, drag tab, maximize, dan restore sesi menghasilkan susunan yang sama persis setelah app dibuka ulang |
| Global | Command palette, keymap berbasis fokus, settings (termasuk file rusak), dan kedua tema bekerja |
| Kualitas | Ukuran dan RAM terukur dan dicatat, stress test lulus, CI hijau, dan penulis memakai glzr sehari-hari tanpa perlu membuka Zed untuk tujuan yang sama |

---

## 15. Repo, lisensi, CI, dan README

### 15.1 Repo dan lisensi

- Lisensi MIT (`LICENSE` di akar repo).
- File akar: `README.md`, `LICENSE`, `THIRD_PARTY_LICENSES.md`, `KNOWN_ISSUES.md`, `mise.toml` (versi Rust dan Node), `.github/` (workflow dan konfigurasi Dependabot), `docs/glzr-spec.md`, dan `docs/design/` untuk ekspor tampilan UI.
- Komponen yang disertakan di installer (ripgrep dan font JetBrains Mono Nerd Font) dicantumkan beserta lisensinya di `THIRD_PARTY_LICENSES.md`.

### 15.2 CI

GitHub Actions di `ubuntu-24.04`, dijalankan pada setiap push dan pull request. Dependency sistem Tauri dipasang lebih dulu.

- Job Rust: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.
- Job frontend: `npm ci`, `npm run check` (svelte-check), `npm run lint`, `npm test` (Vitest).
- Job keamanan: `cargo audit` dan `npm audit --omit=dev`.
- Dependabot mingguan untuk cargo (`src-tauri`), npm, dan GitHub Actions, dikonfigurasi di `.github/dependabot.yml`. Setiap pull request-nya diuji CI sebelum digabung.
- Workflow rilis terpisah, dipicu tag versi: build `.deb` dan AppImage sebagai artifact.

### 15.3 Kerangka README

1. Nama dan satu kalimat deskripsi
2. Demo GIF
3. Fitur ringkas
4. Angka ukuran installer dan RAM (tabel hasil pengukuran dan cara mengukurnya)
5. Perbandingan singkat dengan Zed
6. Instalasi (`.deb` atau AppImage) dan dependency (`git`)
7. Keybind ringkas
8. Build dari sumber
9. Batasan (daftar yang tidak dibuat)
10. Masalah yang diketahui
11. Lisensi

### 15.4 Perawatan berkala

- Lockfile (`Cargo.lock`, `package-lock.json`) disimpan di repo, dan versi Rust dan Node dikunci di `mise.toml`, supaya build selalu sama.
- Ritme: update dependency tiap 1-3 bulan, lebih cepat kalau ada peringatan keamanan.
- Tauri dan Rust: ikuti rilis minor. Pindah ke rilis mayor hanya kalau butuh fitur atau perbaikannya.
- Setelah update besar (Tauri, atau WebKitGTK dari update sistem), jalankan checklist rilis di bagian 10, terutama window blank dan HiDPI.

### 15.5 Distribusi

- Repo: `Fahmi-mi/glzr` di GitHub.
- Identifier aplikasi: `io.github.fahmi-mi.glzr`. Nama produk "glzr", nama biner `glzr`.
- Versi mengikuti SemVer, mulai dari `0.1.0`. Rilis ditandai tag `vX.Y.Z`.
- Format: paket `.deb` dan AppImage, dibangun di Ubuntu 24.04.
- `.deb` memasang perintah `glzr` ke PATH sehingga `glzr .` langsung berfungsi, dan mendeklarasikan `git` sebagai dependency. AppImage tidak mengubah PATH, jadi README menjelaskan cara membuat symlink.
- Tanpa auto-update. Pembaruan manual lewat halaman rilis GitHub.

---

## 16. Desain visual

Arah: bersih, minimalis, modern, dan longgar. Permukaan netral, satu warna aksen, jarak lapang, dan kerapatan yang lebih renggang daripada editor pada umumnya. Ekspor tampilan UI disimpan di `docs/design/`.

### 16.1 Prinsip

- Satu aksen saja. Tipe tab dibedakan lewat ikon, bukan warna.
- Area dipisahkan garis 1 px, bukan bayangan. Bayangan hanya untuk elemen mengambang (dialog dan command palette).
- Radius konsisten: 6 px untuk kontrol, 8 px untuk panel, 12 px untuk dialog dan palette.
- Animasi seminimal mungkin.
- Semua nilai warna dan ukuran lewat CSS variables. Tema default adalah dark dan tidak mengikuti tema sistem.

### 16.2 Palet warna

| Peran | Variabel | Dark (default) | Light |
|---|---|---|---|
| Latar | `--bg` | `#121316` | `#FBFBFC` |
| Panel (sidebar, bar workspace, status bar) | `--pn` | `#17181C` | `#F4F5F7` |
| Permukaan (kartu, bar URL) | `--sf` | `#1C1D22` | `#FFFFFF` |
| Mengambang dan tab aktif | `--el` | `#23252B` | `#ECEEF2` |
| Garis | `--bd` | `#26282E` | `#E6E8EC` |
| Teks | `--tx` | `#E6E7EA` | `#1B1D22` |
| Teks redup | `--dm` | `#8B8E97` | `#6B6F7A` |
| Nomor baris dan teks tersier | `--ft` | `#6F727B` | `#8A8E99` |
| Aksen | `--ac` | `#78A9FF` | `#3B6EF0` |
| Aksen lembut (latar item aktif) | `--acs` | `rgba(120,169,255,.14)` | `rgba(59,110,240,.10)` |
| Teks di atas aksen | `--onac` | `#0B1220` | `#FFFFFF` |
| Git: modified | `--mo` | `#E5C07B` | `#A9780F` |
| Git: added | `--ad` | `#8FCB8A` | `#2F8F4E` |
| Git: deleted | `--de` | `#E07B7B` | `#C94A4A` |
| Git: conflict | `--co` | `#E59A6A` | `#C26A2C` |
| Syntax: kata kunci | `--k` | `#B79CF2` | `#7B4FD0` |
| Syntax: fungsi | `--f` | `#78A9FF` | `#2F5FD6` |
| Syntax: string | `--s` | `#8FCB8A` | `#2F8F4E` |
| Cyan (tipe, ANSI cyan) | `--cy` | `#7FD1D9` | `#167C86` |

Catatan:
- Git untracked memakai warna teks redup (`--dm`). Komentar kode juga memakai `--dm`.
- Palet light diturunkan dari dark dengan peran warna yang sama, bukan dirancang terpisah.
- Teks utama dan teks redup memenuhi kontras 4.5:1 terhadap latar. Nomor baris (`--ft`) bersifat dekoratif.

**Warna terminal (ANSI)**

Latar `--bg`, teks `--tx`, kursor `--ac`, seleksi `--acs`.

| Warna | Dark normal | Dark bright | Light normal | Light bright |
|---|---|---|---|---|
| Hitam | `#23252B` | `#5C5F68` | `#1B1D22` | `#6B6F7A` |
| Merah | `#E07B7B` | `#EF9A9A` | `#C94A4A` | `#D96868` |
| Hijau | `#8FCB8A` | `#A8DBA3` | `#2F8F4E` | `#46A768` |
| Kuning | `#E5C07B` | `#EDD39B` | `#A9780F` | `#BE8E26` |
| Biru | `#78A9FF` | `#9BBFFF` | `#3B6EF0` | `#5C88F4` |
| Magenta | `#B79CF2` | `#CDB9F7` | `#7B4FD0` | `#9570DB` |
| Cyan | `#7FD1D9` | `#A0E0E6` | `#167C86` | `#2FA3AE` |
| Putih | `#C9CBD1` | `#E6E7EA` | `#8A8E99` | `#6B6F7A` |

Pada tema light, putih dan putih bright sengaja gelap supaya tetap terbaca di latar terang.

**Syntax highlight**

| Token | Variabel |
|---|---|
| Kata kunci | `--k` |
| Fungsi dan metode | `--f` |
| String | `--s` |
| Angka dan konstanta | `--mo` |
| Tipe dan kelas | `--cy` |
| Tag HTML | `--de` |
| Atribut | `--mo` |
| Komentar, operator, tanda baca | `--dm` |
| Properti dan variabel | `--tx` |

### 16.3 Tipografi

| Penggunaan | Ukuran / jarak baris | Font |
|---|---|---|
| Teks UI | 13 px / 1.6 | Font sistem |
| Judul dialog dan judul kartu | 15 px, bobot 600 | Font sistem |
| Keterangan dan status bar | 12 px | Font sistem |
| Badge git | 11 px, bobot 600 | Font sistem |
| Editor dan terminal | 14 px / 1.8 | Monospace sistem (default) atau JetBrains Mono Nerd Font |

Ukuran editor/terminal mengikuti `font_size` di settings.

### 16.4 Ukuran, spasi, dan bentuk

| Elemen | Nilai |
|---|---|
| Bar workspace | tinggi 40 px, padding horizontal 16 px |
| Tab strip | tinggi 36 px |
| Tab | tinggi 26 px, padding horizontal 12 px, jarak ikon-judul 8 px |
| Baris file tree | tinggi 28 px, padding 8 px, indentasi 20 px per level |
| Sidebar | lebar 264 px (batas 160-480 px) |
| Status bar | tinggi 28 px, jarak antar item 20 px |
| Padding konten pane | 16 px vertikal, 24 px horizontal |
| Splitter | garis 1 px, area seret 8 px |
| Skala spasi | 4, 8, 12, 16, 24, 32 px |
| Radius | 6 px kontrol, 8 px panel dan input, 12 px dialog dan palette |

### 16.5 Komponen

**Bar workspace**
- Latar `--pn`, garis bawah 1 px `--bd`.
- Item: padding 4 px × 12 px, radius 6 px, teks `--dm`. Item aktif: latar `--acs` dan teks `--ac`.
- Tombol "+" untuk workspace baru di akhir deret.

**Sidebar**
- Latar `--pn`, garis kanan 1 px, padding 16 px × 8 px.
- Tab Files, Git, dan Search di atas: padding 4 px × 10 px, radius 6 px. Tab aktif berlatar `--el` dengan teks `--tx`.
- Baris tree: teks `--dm`, baris terpilih berlatar `--acs` dengan teks `--tx`. Badge git (huruf 11 px, bobot 600) di ujung kanan memakai warna status git.

**Tab dan pane**
- Tab aktif: latar `--el`, teks `--tx`. Tab tidak aktif: teks `--dm`.
- Tab belum tersimpan menampilkan titik aksen. Tombol tutup muncul saat hover dan menggantikan titik.
- Pane yang fokus memiliki garis aksen 2 px di tepi atasnya. Pane lain tanpa garis.
- Tab menampilkan ikon sesuai jenisnya (lihat 16.7).

**Preview**
- Bar URL di atas isi: margin 16 px, padding 6 px × 12 px, border 1 px `--bd`, radius 8 px, latar `--sf`, teks `--dm`.

**Status bar**
- Latar `--pn`, garis atas 1 px, teks 12 px `--dm`. Branch dan status di kiri, bahasa, encoding, LF/CRLF, lalu posisi kursor di ujung kanan.

**Tombol**
- Tinggi 32 px, padding horizontal 14 px, radius 6 px, border 1 px `--bd`, latar transparan.
- Tombol utama: latar `--ac`, border `--ac`, teks `--onac`, bobot 500. Satu tombol utama per kelompok.

**Input**
- Tinggi 36 px, radius 8 px, border 1 px `--bd`, latar `--bg`, padding horizontal 12 px.

**Dialog**
- Lebar 420 px, latar `--el`, border 1 px `--bd`, radius 12 px, padding 24 px, bayangan `0 12px 40px rgba(0,0,0,.35)`.
- Judul 15 px bobot 600, keterangan `--dm`, jarak 24 px ke deret tombol. Tombol rata kanan dengan tombol utama paling kanan.
- Latar belakang diberi overlay `rgba(0,0,0,.25)`.

**Command palette**
- Melayang di tengah atas, lebar 560 px, radius 12 px, latar `--el`, bayangan sama dengan dialog.
- Input di atas (padding 16 px, garis bawah 1 px). Hasil berupa baris tinggi 36 px dengan padding horizontal 16 px, label jenis (File, Aksi, Workspace) rata kanan berwarna `--dm`. Baris terpilih berlatar `--acs`.

**Keadaan kosong dan error**
- Teks pendek di tengah pane, jarak 16 px, dan satu aksi utama (misalnya "Terminal baru", atau "Coba lagi" dan "Buka di browser" untuk preview gagal).

**Git panel**
- Header "Perubahan · n", lalu baris file dengan badge status. Di bawahnya input pesan commit (margin atas 16 px) dan tombol utama "Commit".

### 16.6 Tema light

Memakai peran warna dan ukuran yang sama dengan dark, hanya nilai variabelnya yang berbeda (lihat 16.2). Bayangan dialog dan palette dipertahankan.

### 16.7 Ikon

- Set ikon: Lucide, diimpor per ikon.
- Ukuran 14 px di tab dan baris, stroke 2 px, warna mengikuti teks (`currentColor`).
- Ikon tab: file, terminal, preview (globe), dan diff.
- Ikon file tree hanya `folder`, `folder-open`, `file`, dan `image` (untuk gambar). Tidak ada ikon per bahasa: jenis file dikenali dari ekstensi dan status git ditunjukkan badge huruf. Ini menjaga tampilan tenang dan bundle kecil.

### 16.8 Aksesibilitas visual

- Informasi tidak bergantung pada warna saja: badge git selalu berupa huruf (M, A, D, ?, !).
- Fokus keyboard selalu terlihat, memakai warna aksen.
- Kontras teks utama dan redup minimal 4.5:1 di kedua tema.

### 16.9 Ikon aplikasi

- Ikon aplikasi: kotak membulat berlatar `--bg` (dark) dengan glyph `>_` berwarna aksen `--ac`.
- Satu file sumber PNG 1024×1024 di `src-tauri/icons/source.png`. Set ikon Tauri dihasilkan dengan `tauri icon`.
- Wordmark untuk README: teks "glzr" huruf kecil dengan font UI.

---

## 17. Keputusan desain

Keputusan yang sudah ditetapkan:

| Topik | Keputusan |
|---|---|
| Sidebar Files/Git/Search | Tampil di semua workspace dan berdiri sendiri, tidak terikat workspace. Bisa disembunyikan (Alt+E/V/G) untuk tampilan penuh |
| Git | Bagian dari sidebar, bukan tab. Diff file yang berubah dibuka sebagai tab diff. Stage, unstage, dan discard tersedia per file dan per hunk (tanpa per baris) |
| Tab dan workspace | Satu tab hanya hidup di satu workspace |
| Template workspace | Tidak ada. Mulai kosong dengan satu pane berisi satu terminal |
| RAM di status bar | Tidak ditampilkan |
| Format sesi | Satu file JSON dengan field `version` (mulai dari 1), migrasi berantai, dan cadangan otomatis kalau file tidak cocok (lihat 6.5) |
| Pencarian proyek | Panel Search di sidebar, memakai `rg` yang disertakan di installer. Tanpa replace di seluruh proyek |
| Batas data | Diff dan hasil pencarian dibatasi ukurannya supaya RAM tetap terjaga |
| Platform | Linux saja |
| Folder proyek | Satu instance satu folder proyek |
| Cara membuka folder | Dialog, argumen `glzr .`, dan daftar folder terakhir |
| Folder terminal | Semua terminal dibuka di folder proyek |
| Tema | Dua tema: dark (default, tidak mengikuti sistem) dan light |
| Font | Monospace sistem (default) dan JetBrains Mono Nerd Font sebagai opsi kedua, disertakan di installer |
| Pencarian: binary | `rg` disertakan di installer sebagai sidecar |
| Pengembangan | Setelah terminal, editor, dan git panel berjalan, glzr dipakai untuk mengembangkan dirinya sendiri |
| Lisensi dan CI | MIT dan GitHub Actions dasar (fmt, clippy, test, lint frontend) |
| README | Mengikuti kerangka di 15.3, diisi bertahap |
| Gaya visual | Bersih, minimalis, modern, longgar. Satu aksen biru, tipe tab dibedakan lewat ikon (detail di bagian 16) |
| Ekspor desain | Gambar tampilan UI disimpan di `docs/design/` |
| Keybind | Aplikasi memakai Alt + satu huruf (maksimal dua tombol), editor memakai standar Ctrl. Ctrl+huruf diteruskan ke terminal, jadi herdr (prefix Ctrl+B) tidak bentrok |
| Lokasi data | Standar XDG: config, data, cache, dan state di bawah `~/.config`, `~/.local/share`, `~/.cache`, `~/.local/state` (lihat 6.7) |
| Telemetri | Tidak ada |
| Jendela | 1280×800 awal, minimum 800×500, title bar bawaan sistem, ukuran dan posisi diingat |
| Distribusi | `.deb` dan AppImage, identifier `io.github.fahmi-mi.glzr`, SemVer dari `0.1.0`, tanpa auto-update |
