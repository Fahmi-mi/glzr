# Known Issues

Daftar masalah yang diketahui, temuan dari spike teknis, beserta status dan solusinya.

## Lingkungan & Sistem

### 1. Window blank pada WebKitGTK + GPU NVIDIA / Hybrid

- **Status:** Didokumentasikan (workaround tersedia)
- **Gejala:** Jendela aplikasi tampak kosong/putih saat dijalankan pada sistem Linux dengan driver NVIDIA atau laptop hybrid.
- **Solusi:** Jalankan dengan environment variable `WEBKIT_DISABLE_DMABUF_RENDERER=1` (atau `WEBKIT_DISABLE_COMPOSITING_MODE=1`). Variabel ini dibersihkan sebelum men-spawn shell di terminal agar tidak bocor ke proses anak.
