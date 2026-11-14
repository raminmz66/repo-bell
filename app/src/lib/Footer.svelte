<script lang="ts">
  export let onAddFolder: () => void;
</script>

<footer class="app-footer">
  <button class="btn-add-folder" onclick={onAddFolder}>
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M12 5v14M5 12h14"></path>
    </svg>
    Add folder
  </button>
</footer>

<style>
  .app-footer {
    height: 52px;
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
    height: 36px;
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
</style>
