<script lang="ts">
  import { onDestroy } from 'svelte';
  import { convertFileSrc } from '@tauri-apps/api/core';

  interface Props {
    path: string;
    sizeBytes?: number;
  }

  let { path, sizeBytes = 0 }: Props = $props();

  let imageSrc = $state<string | null>(null);
  let errorMsg = $state<string | null>(null);
  let loading = $state(true);
  let naturalWidth = $state(0);
  let naturalHeight = $state(0);

  function formatBytes(bytes: number): string {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }

  function resolveSrc(targetPath: string) {
    loading = true;
    errorMsg = null;
    naturalWidth = 0;
    naturalHeight = 0;
    if (!targetPath || !targetPath.trim()) {
      loading = false;
      errorMsg = 'Failed to load image';
      imageSrc = null;
      return;
    }
    try {
      if (typeof window !== 'undefined' && (window as any).__TAURI_INTERNALS__?.convertFileSrc) {
        imageSrc = convertFileSrc(targetPath);
      } else {
        imageSrc = targetPath;
      }
    } catch {
      errorMsg = 'Failed to load image';
      imageSrc = null;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    resolveSrc(path);
  });

  function handleImageLoad(event: Event) {
    const img = event.currentTarget as HTMLImageElement;
    naturalWidth = img.naturalWidth;
    naturalHeight = img.naturalHeight;
  }

  function handleImageError() {
    errorMsg = 'Failed to load image';
  }

  onDestroy(() => {
    imageSrc = null;
  });
</script>

<div class="image-viewer">
  {#if loading}
    <p class="muted">Loading image...</p>
  {:else if errorMsg}
    <p class="error-text">{errorMsg}</p>
  {:else if imageSrc}
    <div class="image-stage">
      <img src={imageSrc} alt={path} onload={handleImageLoad} onerror={handleImageError} class="preview-image" />
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
    border-radius: var(--radius-menu);
    background: var(--el);
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
