<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

  export let title: string = "RepoBell";
  export let onCheckNow: (() => void) | undefined = undefined;
  export let onOpenSettings: (() => void) | undefined = undefined;
  export let isChecking: boolean = false;
  export let showBack: boolean = false;
  export let onBack: (() => void) | undefined = undefined;

  export async function closeWindow() {
    try {
      await invoke("hide_window");
      return;
    } catch {
      // Fallback to WebviewWindow API
    }

    try {
      const win = getCurrentWebviewWindow();
      await win.hide();
    } catch {
      // Fallback if not inside Tauri window
    }
  }
</script>

<header class="app-header" data-tauri-drag-region>
  {#if showBack}
    <button aria-label="Back" class="icon-btn" onclick={onBack}>
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M15 6l-6 6 6 6"></path>
      </svg>
    </button>
  {:else if onCheckNow}
    <button aria-label="Check now" class="icon-btn" class:spin={isChecking} disabled={isChecking} onclick={() => { if (!isChecking && onCheckNow) onCheckNow(); }}>
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M21 12a9 9 0 1 1-2.6-6.4"></path>
        <path d="M21 4v5h-5"></path>
      </svg>
    </button>
  {:else}
    <div class="spacer"></div>
  {/if}

  <div class="header-title">{title}</div>

  {#if onOpenSettings}
    <button aria-label="Settings" class="icon-btn" onclick={onOpenSettings}>
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="3"></circle>
        <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"></path>
      </svg>
    </button>
  {/if}

  <button aria-label="Close window" class="icon-btn" onclick={closeWindow}>
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true">
      <path d="M6 6l12 12M18 6L6 18"></path>
    </svg>
  </button>
</header>

<style>
  .app-header {
    height: 52px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    background: #ebebe8;
    border-bottom: 1px solid #d9d9d5;
    user-select: none;
  }
  .icon-btn {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    cursor: pointer;
  }
  .icon-btn:hover {
    background: #e0e0dc;
  }
  .spacer {
    width: 40px;
  }
  .header-title {
    flex-grow: 1;
    text-align: center;
    font-size: 15px;
    font-weight: 700;
  }
  .spin svg {
    animation: rotate 1s linear infinite;
  }
  @keyframes rotate {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
</style>
