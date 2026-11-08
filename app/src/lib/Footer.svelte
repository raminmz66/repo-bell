<script lang="ts">
  export let onAddFolder: () => void;
  export let onMarkAllSeen: () => void;
  export let hasNews: boolean = false;
</script>

<footer class="app-footer">
  <button class="btn-add-folder" onclick={onAddFolder}>
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12 5v14M5 12h14"></path>
    </svg>
    Add folder
  </button>
  <button class="btn-mark-seen" disabled={!hasNews} onclick={onMarkAllSeen}>
    Mark all as seen
  </button>
</footer>

<style>
  .app-footer {
    height: 64px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 0 16px;
    border-top: 1px solid #e3e3e0;
    background: #ffffff;
    user-select: none;
  }
  .btn-add-folder {
    height: 40px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    font-size: 14px;
    cursor: pointer;
  }
  .btn-add-folder:hover {
    background: #f3f3f1;
  }
  .btn-mark-seen {
    height: 40px;
    padding: 0 16px;
    border: 1px solid #c9c9c4;
    border-radius: 8px;
    background: #ffffff;
    color: #1b1b1f;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-mark-seen:hover:not(:disabled) {
    background: #f9f9f8;
  }
  .btn-mark-seen:disabled {
    opacity: 0.5;
    cursor: default;
  }
</style>
