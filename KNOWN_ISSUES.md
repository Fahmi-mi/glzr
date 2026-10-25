# Known Issues

Daftar masalah yang diketahui, hasil validasi spike teknis, beserta status dan solusinya.

## Hasil Spike Teknis

| Komponen                    | Hasil Validasi     | Catatan Teknis                                                                                                        |
| --------------------------- | ------------------ | --------------------------------------------------------------------------------------------------------------------- |
| PTY + Streaming 100 MB      | Lulus (0.23 detik) | Batching 12 ms / 64 KB dengan bounded channel backpressure (`sync_channel(32)`) mencegah banjir event ke UI           |
| UTF-8 Boundary di PTY       | Lulus              | Byte UTF-8 yang terpotong di batas chunk `read()` disimpan sementara di buffer `decode_utf8_stream`                   |
| Process Group Termination   | Lulus              | Penutupan sesi mengirim sinyal ke process group (`-pid`) agar tidak ada proses anak yatim                             |
| Git Stage/Unstage Per Hunk  | Lulus              | Satu hunk dari tiga hunk berhasil di-stage (`git apply --cached -`) dan di-unstage (`git apply --cached --reverse -`) |
| Preview Localhost & Suspend | Lulus              | Hanya host `localhost` dan `127.0.0.1` (`http`/`https`) yang diizinkan; tab tidak aktif diubah ke `about:blank`       |
| Ukuran Binary Release       | 3.4 MB (`glzr`)    | Ditambah sidecar `rg` (6.3 MB), font WOFF2 (2.0 MB), dan bundle JS/HTML (~80 KB)                                      |

## Lingkungan dan Sistem

### 1. Window blank pada WebKitGTK + GPU NVIDIA / Hybrid

- **Status:** Didokumentasikan dan ditangani di PTY
- **Gejala:** Jendela aplikasi tampak kosong atau putih saat dijalankan pada sistem Linux dengan driver NVIDIA atau laptop hybrid.
- **Solusi:** Jalankan dengan environment variable `WEBKIT_DISABLE_DMABUF_RENDERER=1` (atau `WEBKIT_DISABLE_COMPOSITING_MODE=1`). Modul `pty` otomatis menghapus variabel `WEBKIT_*` sebelum men-spawn shell agar tidak bocor ke proses anak.

### 2. Karakter UTF-8 rusak saat output terminal deras

- **Status:** Selesai (`src-tauri/src/pty/session.rs`)
- **Gejala:** Karakter multi-byte (CJK atau simbol) terpotong di batas buffer pembacaan PTY.
- **Solusi:** Fungsi `decode_utf8_stream` menahan sisa byte tidak lengkap hingga chunk pembacaan berikutnya tiba.

### 3. Proses anak tetap berjalan setelah tab terminal ditutup

- **Status:** Selesai (`src-tauri/src/pty/session.rs`)
- **Gejala:** Menghentikan shell saja tidak menghentikan subproses yang sedang berjalan di dalamnya.
- **Solusi:** Saat sesi dihentikan, sinyal terminasi dikirim ke seluruh process group (`-pid`) sebelum menutup proses shell utama.
