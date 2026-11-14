<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { ChangeView, FeedSummary } from "./types";
  import { errText, plural } from "./util";

  export let feed: FeedSummary;
  export let repoPath: string;

  const MAX_TITLES = 3;

  let showFiles = false;
  let fileError: string | null = null;

  $: shown = [...feed.changes].sort((a, b) => b.timestamp - a.timestamp).slice(0, MAX_TITLES);
  $: more = Math.max(0, feed.total_changes - shown.length);
  const authorOf = (c: ChangeView) => (c.mine ? "You" : c.author);
  // One author reads best as plain titles; several need a name in front.
  $: prefixed = new Set(shown.map(authorOf)).size > 1;

  async function openFile(file: string) {
    fileError = null;
    try {
      await invoke("open_editor", { repo: repoPath, file });
    } catch (e) {
      fileError = errText(e);
    }
  }
</script>

{#if shown.length > 0}
  <ul class="change-list">
    {#each shown as change}
      <li class="change-item">{prefixed ? `${authorOf(change)}: ${change.title}` : change.title}</li>
    {/each}
  </ul>
{/if}

{#if more > 0 || feed.files_changed.length > 0}
  <div class="feed-meta">
    {#if more > 0}<span class="more">{more} more</span>{/if}
    {#if more > 0 && feed.files_changed.length > 0}{" · "}{/if}
    {#if feed.files_changed.length > 0}
      <button class="btn-files-toggle" onclick={() => (showFiles = !showFiles)} aria-expanded={showFiles}>
        <svg class="chevron" class:open={showFiles} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M9 6l6 6-6 6"></path>
        </svg>
        {feed.files_changed.length} {plural(feed.files_changed.length, "file")} changed
      </button>
    {/if}
  </div>
{/if}

{#if feed.files_changed.length > 0}
  {#if showFiles}
    <ul class="files-box">
      {#each feed.files_changed as f}
        <li>
          <button class="file-link-btn" title={`Open ${f} in your editor`} onclick={() => openFile(f)}>
            <svg class="file-icon" width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
              <path d="M14 2v6h6"></path>
            </svg>
            <span class="file-name">{f}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if fileError}
    <p class="file-error" role="alert">{fileError}</p>
  {/if}
{/if}

<style>
  .change-list {
    margin: 0;
    padding: 0 0 0 18px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 13px;
    line-height: 1.5;
    color: #3d3d45;
  }
  .change-item {
    overflow-wrap: anywhere;
  }
  .feed-meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    font-size: 13px;
    line-height: 1.5;
    color: #5b5b66;
  }
  .more {
    padding-left: 18px;
  }
  .btn-files-toggle {
    align-self: flex-start;
    min-height: 36px;
    display: flex;
    align-items: center;
    gap: 4px;
    margin: -4px 0 -6px -6px;
    padding: 0 8px 0 6px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: #5b5b66;
    font-size: 13px;
    cursor: pointer;
  }
  .btn-files-toggle:hover {
    background: #f3f3f1;
  }
  .chevron {
    transition: transform 0.15s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .files-box {
    margin: 4px 0 0;
    padding: 6px;
    list-style: none;
    background: #fafaf8;
    border: 1px solid #e3e3e0;
    border-radius: 8px;
    display: flex;
    flex-direction: column;
    gap: 2px;
    max-height: 140px;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .file-link-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border: none;
    border-radius: 5px;
    background: transparent;
    color: #1f5fd1;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
    font-size: 12px;
    text-align: left;
    cursor: pointer;
    overflow: hidden;
    width: 100%;
    box-sizing: border-box;
  }
  .file-link-btn:hover {
    background: #eef4fe;
    text-decoration: underline;
  }
  .file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
  }
  .file-icon {
    flex-shrink: 0;
    color: #55555c;
  }
  .file-error {
    margin: 0;
    font-size: 13px;
    color: #a12a20;
  }
</style>
