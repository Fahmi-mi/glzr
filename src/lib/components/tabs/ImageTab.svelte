<script lang="ts">
  import { onDestroy } from 'svelte';
  import { fsReadImageDataUrl, type AppErrorPayload } from '$lib/ipc';

  interface Props {
    path: string;
    sizeBytes?: number;
  }

  let { path, sizeBytes = 0 }: Props = $props();

  let dataUrl = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let loading = $state(true);
  let naturalWidth = $state(0);
  let naturalHeight = $state(0);

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  async function loadImage(targetPath: string) {
    loading = true;
    errorMsg = null;
    dataUrl = null;
    naturalWidth = 0;
    naturalHeight = 0;
    try {
      dataUrl = await fsReadImageDataUrl(targetPath);
    } catch (err) {
      const normalized = err as AppErrorPayload;
      errorMsg = normalized.message ?? 'Gagal memuat gambar';
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    void loadImage(path);
  });

  function handleImageLoad(event: Event) {
    const img = event.currentTarget as HTMLImageElement;
    naturalWidth = img.naturalWidth;
    naturalHeight = img.naturalHeight;
  }

  onDestroy(() => {
    dataUrl = null;
  });
</script>

<div class="image-viewer">
  {#if loading}
    <p class="muted">Memuat gambar...</p>
  {:else if errorMsg}
    <p class="error-text">{errorMsg}</p>
  {:else if dataUrl}
    <div class="image-stage">
      <img src={dataUrl} alt={path} onload={handleImageLoad} class="preview-image" />
    </div>
    <div class="image-meta">
      {#if naturalWidth > 0 && naturalHeight > 0}
        <span>{naturalWidth} x {naturalHeight} px</span>
      {/if}
      {#if sizeBytes > 0}
        <span>{formatBytes(sizeBytes)}</span>
      {/if}
    </div>
  {/if}
</div>

<style>
  .image-viewer {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    padding: 24px;
    gap: 16px;
    overflow: auto;
  }

  .image-stage {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 0;
    width: 100%;
  }

  .preview-image {
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    border: 1px solid var(--bd);
    border-radius: var(--radius-ctrl);
    background: var(--pn);
  }

  .image-meta {
    display: flex;
    align-items: center;
    gap: 16px;
    font-size: 12px;
    color: var(--dm);
    font-family: var(--font-mono);
  }

  .muted {
    color: var(--dm);
  }

  .error-text {
    color: var(--de);
    font-size: 12px;
  }
</style>
