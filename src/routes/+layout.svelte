<script lang="ts">
  import '../app.css';
  import { onMount, type Snippet } from 'svelte';

  let { children }: { children: Snippet } = $props();

  onMount(() => {
    function handleContextMenu(event: MouseEvent) {
      event.preventDefault();
    }

    function handleKeyDown(event: KeyboardEvent) {
      const isCmdOrCtrl = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();

      if (
        event.key === 'F5' ||
        event.key === 'F12' ||
        (isCmdOrCtrl && key === 'r') ||
        (isCmdOrCtrl && event.shiftKey && (key === 'i' || key === 'c' || key === 'j')) ||
        (isCmdOrCtrl && key === 'u')
      ) {
        event.preventDefault();
      }
    }

    window.addEventListener('contextmenu', handleContextMenu, { capture: true });
    window.addEventListener('keydown', handleKeyDown, { capture: true });
    document.addEventListener('contextmenu', handleContextMenu, { capture: true });

    return () => {
      window.removeEventListener('contextmenu', handleContextMenu, { capture: true });
      window.removeEventListener('keydown', handleKeyDown, { capture: true });
      document.removeEventListener('contextmenu', handleContextMenu, { capture: true });
    };
  });
</script>

{@render children()}
