# Known Issues

Daftar masalah yang diketahui, hasil validasi spike teknis, serta mitigasi operasional sistem Linux.

## Hasil Validasi Spike Teknis

| Komponen                    | Hasil Validasi     | Catatan Teknis                                                                                                        |
| --------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------- |
| PTY + Streaming 100 MB      | Lulus (0.23 detik) | Batching 12 ms / 64 KB dengan bounded channel backpressure (`sync_channel(32)`) mencegah banjir event ke UI           |
| UTF-8 Boundary di PTY       | Lulus              | Byte UTF-8 yang terpotong di batas chunk `read()` disimpan sementara di buffer `decode_utf8_stream`                   |
| Process Group Termination   | Lulus              | Penutupan sesi mengirim sinyal ke process group (`-pid`) agar tidak ada proses anak yatim                             |
| Git Stage/Unstage Per Hunk  | Lulus              | Satu hunk dari tiga hunk berhasil di-stage (`git apply --cached -`) dan di-unstage (`git apply --cached --reverse -`) |
| Preview Localhost & Suspend | Lulus              | Hanya host `localhost` dan `127.0.0.1` (`http`/`https`) yang diizinkan; tab tidak aktif diubah ke `about:blank`       |
| Ukuran Binary Release       | 3.4 MB (`glzr`)    | Ditambah sidecar `rg` (6.3 MB), font WOFF2 (2.0 MB), dan bundle JS/HTML (~80 KB)                                      |

## Lingkungan, Batasan Sistem, dan Mitigasi

### 1. Window blank pada WebKitGTK + GPU NVIDIA / Laptop Hybrid

- **Status:** Tersedia mitigasi lingkungan
- **Gejala:** Jendela aplikasi tampak kosong atau putih saat dijalankan pada sistem Linux dengan driver NVIDIA proprietary atau grafis hybrid.
- **Solusi:** Jalankan dengan environment variable `WEBKIT_DISABLE_DMABUF_RENDERER=1` dan `WEBKIT_DISABLE_COMPOSITING_MODE=1` (glzr otomatis menyetel kedua variabel ini jika mendeteksi driver NVIDIA). Modul `pty` otomatis membersihkan variabel `WEBKIT_*` sebelum men-spawn shell agar tidak bocor ke subproses terminal.

### 2. Karakter UTF-8 rusak saat output terminal sangat deras

- **Status:** Selesai (`src-tauri/src/pty/session.rs`)
- **Gejala:** Karakter multi-byte (CJK atau simbol) terpotong di batas buffer pembacaan PTY.
- **Solusi:** Fungsi `decode_utf8_stream` menahan sisa byte tidak lengkap hingga chunk pembacaan berikutnya tiba.

### 3. Subproses tetap berjalan setelah tab terminal ditutup

- **Status:** Selesai (`src-tauri/src/pty/session.rs`)
- **Gejala:** Menghentikan proses shell induk saja tidak menghentikan subproses anak (misalnya background server).
- **Solusi:** Saat sesi dimatikan, sinyal terminasi dikirim ke seluruh process group (`-pid`) sebelum handle shell ditutup.

### 4. Limit inotify watches sistem Linux terlampaui pada repo sangat besar

- **Status:** Penanganan error terdeteksi
- **Gejala:** Watcher file memunculkan notifikasi error saat memantau pohon direktori dengan puluhan ribu file.
- **Solusi:** Abaikan direktori berat (`node_modules`, `target`, `.git`). Jika kapasitas watch sistem habis, naikkan batas kernel Linux:
  ```bash
  sudo sysctl fs.inotify.max_user_watches=524288
  ```

### 5. File sesi rusak atau versi tidak kompatibel

- **Status:** Selesai (`src-tauri/src/state.rs`)
- **Gejala:** JSON sesi di `session.json` tidak valid atau menggunakan versi skema yang tidak didukung.
- **Solusi:** glzr mencadangkan file lama ke `session.json.bak` secara otomatis dan memulihkan aplikasi dengan layout default tanpa crash.

### 6. File biner atau teks sangat besar (> 5 MB)

- **Status:** Selesai (`src-tauri/src/fs/mod.rs` & `src/lib/components/tabs/FileTab.svelte`)
- **Gejala:** Editor memuat file log raksasa atau biner yang berpotensi membebani memori WebKitGTK.
- **Solusi:** File biner dan file teks di atas batas 5 MB otomatis dibuka dalam mode read-only dan syntax highlighting dinonaktifkan demi performa.

### 7. File diubah dari luar saat editor memiliki perubahan belum disimpan

- **Status:** Selesai (`src/lib/components/tabs/FileTab.svelte`)
- **Gejala:** File di disk dimodifikasi oleh proses eksternal (misal git checkout atau compiler) saat tab sedang dirty.
- **Solusi:** Tab menampilkan banner dialog konflik yang memberikan pilihan tegas: `Simpan milikku` atau `Muat ulang`.

### 8. Autentikasi interaktif Git Push / Pull

- **Status:** Beroperasi via terminal
- **Gejala:** Operasi push/pull melalui panel GUI meminta input username/password atau passphrase SSH dan berisiko hang jika tidak ada prompt TTY.
- **Solusi:** Backend git menyetel `GIT_TERMINAL_PROMPT=0`. Untuk repo yang membutuhkan autentikasi interaktif atau SSH passphrase baru, jalankan push/pull melalui tab terminal glzr.
