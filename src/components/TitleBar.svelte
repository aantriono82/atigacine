<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  let isMaximized = false;
  let maximizeSyncTimer: number | undefined;
  const MAXIMIZE_SYNC_DEBOUNCE_MS = 180;
  const nativeWindow = isTauriRuntime();

  function isTauriRuntime(): boolean {
    return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
  }

  async function syncMaximizedState(): Promise<void> {
    if (!nativeWindow) return;

    try {
      isMaximized = await getCurrentWindow().isMaximized();
    } catch {
      // Keep the normal icon when the window state is unavailable.
    }
  }

  async function minimizeWindow(): Promise<void> {
    if (!nativeWindow) return;
    await getCurrentWindow().minimize().catch(() => undefined);
  }

  async function toggleMaximize(): Promise<void> {
    if (!nativeWindow) return;
    await getCurrentWindow().toggleMaximize().catch(() => undefined);
    await syncMaximizedState();
  }

  function scheduleMaximizedStateSync(): void {
    if (maximizeSyncTimer !== undefined) window.clearTimeout(maximizeSyncTimer);
    maximizeSyncTimer = window.setTimeout(() => {
      maximizeSyncTimer = undefined;
      void syncMaximizedState();
    }, MAXIMIZE_SYNC_DEBOUNCE_MS);
  }

  async function closeWindow(): Promise<void> {
    if (!nativeWindow) return;
    await getCurrentWindow().close().catch(() => undefined);
  }

  function handleDragRegionKeydown(event: KeyboardEvent): void {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    void toggleMaximize();
  }

  onMount(() => {
    if (!nativeWindow) return;

    let active = true;
    let unlistenResize: (() => void) | undefined;
    const appWindow = getCurrentWindow();

    const setup = async (): Promise<void> => {
      await syncMaximizedState();
      const unlisten = await appWindow.onResized(() => {
        scheduleMaximizedStateSync();
      });

      if (active) unlistenResize = unlisten;
      else unlisten();
    };

    void setup().catch(() => undefined);

    return () => {
      active = false;
      if (maximizeSyncTimer !== undefined) window.clearTimeout(maximizeSyncTimer);
      maximizeSyncTimer = undefined;
      unlistenResize?.();
    };
  });
</script>

<header class="title-bar" aria-label="Atiga Cine title bar">
  <div
    class="title-bar-drag-region"
    data-tauri-drag-region
    role="button"
    tabindex="0"
    aria-label="Drag titlebar or double-click to maximize"
    ondblclick={() => void toggleMaximize()}
    onkeydown={handleDragRegionKeydown}
  >
    <span class="app-mark" aria-hidden="true">AC</span>
    <span class="titlebar-title">ATIGA CINE</span>
  </div>

  <div class="window-actions" aria-label="Window controls">
    <button
      class="window-control"
      type="button"
      aria-label="Minimize window"
      title="Minimize"
      disabled={!nativeWindow}
      onclick={() => void minimizeWindow()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="M3 8h10" />
      </svg>
    </button>
    <button
      class="window-control"
      type="button"
      aria-label={isMaximized ? "Restore window" : "Maximize window"}
      title={isMaximized ? "Restore" : "Maximize"}
      disabled={!nativeWindow}
      onclick={() => void toggleMaximize()}
    >
      {#if isMaximized}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path d="M5 5h8v8H5z" />
          <path d="M3 11H2V2h9v1" />
        </svg>
      {:else}
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <rect x="3" y="3" width="10" height="10" />
        </svg>
      {/if}
    </button>
    <button
      class="window-control window-control-close"
      type="button"
      aria-label="Close window"
      title="Close"
      disabled={!nativeWindow}
      onclick={() => void closeWindow()}
    >
      <svg viewBox="0 0 16 16" aria-hidden="true">
        <path d="m4 4 8 8M12 4l-8 8" />
      </svg>
    </button>
  </div>
</header>
