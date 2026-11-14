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

<li class="row" data-folder-path={folder.path}>
  <h3 class="title" title={baseName(folder.path)}>{baseName(folder.path)}</h3>
  <button class="btn-secondary row-act" disabled={busy} title="Stop watching this folder. Your files aren't touched." onclick={remove}>
    {busy ? "Removing…" : "Remove"}
  </button>
  <p class="msg">{folder.message}</p>
  <p class="path" title={folder.path}>{folder.path}</p>
  {#if error}<p class="err" role="alert">{error}</p>{/if}
</li>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 10px;
    row-gap: 2px;
    align-items: center;
    padding: 9px 12px;
    border-top: 1px solid #e3e3e0;
  }
  .row:first-child {
    border-top: none;
  }
  .title {
    grid-column: 1;
    min-width: 0;
    margin: 0;
    padding-left: 22px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    font-weight: 700;
  }
  .row-act {
    grid-column: 2;
    grid-row: 1 / span 3;
    height: 32px;
    padding: 0 12px;
    border: 1px solid #c9c9c4;
    border-radius: 8px;
    background: #ffffff;
    color: #1b1b1f;
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
  }
  .row-act:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .msg,
  .path,
  .err {
    grid-column: 1;
    min-width: 0;
    margin: 0;
    padding-left: 22px;
  }
  .msg {
    font-size: 13px;
    color: #3d3d45;
    overflow-wrap: anywhere;
  }
  .path {
    font-size: 12px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
    color: #5b5b66;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .err {
    font-size: 13px;
    color: #a12a20;
  }
</style>
