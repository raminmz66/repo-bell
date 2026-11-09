<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { FeedSummary } from "./types";
  import { groupChangesByPersonAndDay } from "./feedGrouping";
  import { timeAgo, errText, plural } from "./util";

  export let feed: FeedSummary;
  export let repoPath: string;
  export let now: number = Date.now();

  const MAX_TITLES = 3;

  let showFiles = false;
  let fileError: string | null = null;

  $: shown = [...feed.changes].sort((a, b) => b.timestamp - a.timestamp).slice(0, MAX_TITLES);
  $: more = Math.max(0, feed.total_changes - shown.length);
  $: groups = groupChangesByPersonAndDay(shown);
  // One person on one day reads best as a plain list; the summary line already names them.
  $: grouped = groups.length > 1 || (groups[0]?.authors.length ?? 0) > 1;

  async function openFile(file: string) {
    fileError = null;
    try {
      await invoke("open_editor", { repo: repoPath, file });
    } catch (e) {
      fileError = errText(e);
    }
  }
</script>

<p class="feed-summary">
  {feed.author_summary}
  {#if feed.newest_time}
    <span class="feed-time">· {timeAgo(feed.newest_time, now)}</span>
  {/if}
</p>

{#if shown.length > 0}
  <div class="feed-groups" class:flat={!grouped}>
    {#each groups as day}
      <div class="day-group">
        {#if grouped}<span class="day-badge">{day.dayLabel}</span>{/if}
        <div class="day-content" class:day-border={grouped}>
          {#each day.authors as authorGroup}
            <div class="author-block">
              {#if grouped}
                <div class="author-line">
                  <span class="author-name" class:author-mine={authorGroup.mine}>
                    {authorGroup.mine ? "You" : authorGroup.author}
                  </span>
                  <span class="author-count">
                    {authorGroup.changes.length} {plural(authorGroup.changes.length, "change")}
                  </span>
                </div>
              {/if}
              <ul class="change-list">
                {#each authorGroup.changes as change}
                  <li class="change-item">{change.title}</li>
                {/each}
              </ul>
            </div>
          {/each}
        </div>
      </div>
    {/each}
    {#if more > 0}
      <p class="more">and {more} more</p>
    {/if}
  </div>
{/if}

{#if feed.files_changed.length > 0}
  <button class="btn-files-toggle" onclick={() => (showFiles = !showFiles)} aria-expanded={showFiles}>
    <svg class="chevron" class:open={showFiles} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
      <path d="M9 6l6 6-6 6"></path>
    </svg>
    {feed.files_changed.length} {plural(feed.files_changed.length, "file")} changed
  </button>

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
  .feed-summary {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
  }
  .feed-time {
    color: #5b5b66;
  }
  .feed-groups {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .day-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .day-badge {
    font-size: 12px;
    font-weight: 600;
    color: #494952;
    text-transform: uppercase;
    letter-spacing: 0.03em;
  }
  .day-content {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .day-border {
    padding-left: 8px;
    border-left: 2px solid #e3e3e0;
  }
  .author-block {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .author-line {
    display: flex;
    align-items: baseline;
    gap: 6px;
    font-size: 13px;
  }
  .author-name {
    font-weight: 600;
    color: #2e2e33;
  }
  .author-mine {
    color: #b3400f;
  }
  .author-count {
    font-size: 12px;
    color: #5b5b66;
  }
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
  .more {
    margin: 0;
    padding-left: 18px;
    font-size: 13px;
    line-height: 1.5;
    color: #5b5b66;
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
