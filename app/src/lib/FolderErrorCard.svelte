<script lang="ts">
  import type { FolderError } from "./types";
  import { baseName, errText } from "./util";

  export let folder: FolderError;
  export let onRemove: (path: string) => Promise<void>;

  let busy = false;
  let error: string | null = null;

  async function remove() {
    busy = true;
    error = null;
    try {
      await onRemove(folder.path);
    } catch (e) {
      error = errText(e);
    } finally {
      busy = false;
    }
  }
</script>

<article class="folder-card" data-folder-path={folder.path}>
  <div class="head">
    <h2 class="title">{baseName(folder.path)}</h2>
    <span class="badge" title="RepoBell couldn't check this folder">Can't check</span>
  </div>
  <p class="msg">{folder.message}</p>
  <p class="path" title={folder.path}>{folder.path}</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="btn-secondary" disabled={busy} title="Stop watching this folder. Your files aren't touched." onclick={remove}>
      {busy ? "Removing…" : "Remove"}
    </button>
  </div>
</article>

<style>
  .folder-card {
    background: #ffffff;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .title {
    margin: 0;
    flex-grow: 1;
    font-size: 15px;
    font-weight: 700;
  }
  .badge {
    font-size: 12px;
    font-weight: 500;
    padding: 3px 9px;
    border-radius: 6px;
    background: #fbe7e5;
    color: #a12a20;
  }
  .msg {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .path {
    margin: 0;
    font-size: 12px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
    color: #5b5b66;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .err {
    margin: 0;
    font-size: 13px;
    color: #a12a20;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
  }
  .btn-secondary {
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
  .btn-secondary:disabled {
    opacity: 0.6;
    cursor: default;
  }
</style>
