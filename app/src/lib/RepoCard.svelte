<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { RepoCardView, UpdateOutcomeView } from "./types";

  export let repo: RepoCardView;
  export let onStateRefresh: () => void;
  export let onRemoveRepo: ((path: string) => void) | undefined = undefined;

  let showFiles = false;
  let confirmingUnsaved = false;
  let confirmingCombine = false;
  let conflictFiles: string[] = [];
  let updateMessage: string | null = null;
  let isUpdating = false;
  let undoPossible = true;

  function formatTime(timestamp: number | null): string {
    if (!timestamp) return "";
    const seconds = Math.floor(Date.now() / 1000 - timestamp);
    if (seconds < 60) return "just now";
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes} min ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours} h ago`;
    const days = Math.floor(hours / 24);
    return `${days} d ago`;
  }

  async function handleUpdate() {
    if (repo.behind === 0) return;

    if (repo.unsaved && repo.ahead === 0) {
      confirmingUnsaved = true;
      return;
    }
    if (repo.ahead > 0) {
      confirmingCombine = true;
      return;
    }

    await executeUpdate();
  }

  async function executeUpdate() {
    confirmingUnsaved = false;
    confirmingCombine = false;
    isUpdating = true;
    updateMessage = null;
    conflictFiles = [];

    try {
      const outcome = await invoke<UpdateOutcomeView>("update_repo", { path: repo.path });
      if (outcome.success) {
        updateMessage = outcome.message;
        undoPossible = true;
        onStateRefresh();
      } else {
        conflictFiles = outcome.conflict_files;
      }
    } catch (e) {
      alert(`Update failed: ${e}`);
    } finally {
      isUpdating = false;
    }
  }

  async function handleUndo() {
    if (!repo.undo_head) return;
    try {
      const res = await invoke<boolean>("undo_repo", {
        path: repo.path,
        undoHead: repo.undo_head,
      });
      if (res) {
        updateMessage = null;
        onStateRefresh();
      } else {
        undoPossible = false;
      }
    } catch (e) {
      alert(`Undo failed: ${e}`);
    }
  }

  async function openInEditor(filePath: string) {
    try {
      await invoke("open_editor", { filePath: `${repo.path}/${filePath}` });
    } catch (e) {
      alert(`Could not open editor: ${e}`);
    }
  }
</script>

<article class="repo-card" class:conflict-card={conflictFiles.length > 0}>
  <div class="card-header">
    <h2 class="repo-title">{repo.name}</h2>
    {#if repo.behind > 0 && !updateMessage && conflictFiles.length === 0}
      <button class="btn-update" disabled={isUpdating} onclick={handleUpdate}>
        {isUpdating ? "Updating..." : "Update"}
      </button>
    {/if}
  </div>

  <!-- Status badges -->
  <div class="badge-row">
    {#if repo.error}
      <span class="badge badge-err" title={repo.error}>Can't check</span>
    {:else}
      {#if repo.behind > 0}
        <span class="badge badge-behind" title={`The team made ${repo.behind} change${repo.behind === 1 ? "" : "s"} you don't have yet`}>
          Behind by {repo.behind}
        </span>
      {/if}
      {#if repo.ahead > 0}
        <span class="badge badge-ahead" title={`You have ${repo.ahead} change${repo.ahead === 1 ? "" : "s"} the team doesn't have yet`}>
          Ahead by {repo.ahead}
        </span>
      {/if}
      {#if repo.unsaved}
        <span class="badge badge-unsaved" title="You edited files that are not committed yet">
          Has unsaved changes
        </span>
      {/if}
      {#if repo.behind === 0 && repo.ahead === 0 && !repo.unsaved}
        <span class="badge badge-uptodate">Up to date</span>
      {/if}
    {/if}
  </div>

  <!-- Unsaved Changes Prompt -->
  {#if confirmingUnsaved}
    <div class="prompt-box prompt-amber">
      <p class="prompt-title">You have unsaved changes.</p>
      <p class="prompt-desc">Set them aside, update, then bring them back?</p>
      <p class="prompt-sub">This will download {repo.behind} change{repo.behind === 1 ? "" : "s"}.</p>
      <div class="prompt-actions">
        <button class="btn-text" onclick={() => (confirmingUnsaved = false)}>Cancel</button>
        <button class="btn-primary" onclick={executeUpdate}>Set aside and update</button>
      </div>
    </div>
  {/if}

  <!-- Both Sides Changed / Combine Prompt -->
  {#if confirmingCombine}
    <div class="prompt-box prompt-blue">
      <p class="prompt-title">Your work and the team's work both changed.</p>
      <p class="prompt-desc">RepoBell can combine them. If that fails, nothing is changed.</p>
      <p class="prompt-sub">
        This will download {repo.behind} change{repo.behind === 1 ? "" : "s"} and combine them with your {repo.ahead} change{repo.ahead === 1 ? "" : "s"}.
      </p>
      <div class="prompt-actions">
        <button class="btn-text" onclick={() => (confirmingCombine = false)}>Cancel</button>
        <button class="btn-primary" onclick={executeUpdate}>Update and combine</button>
      </div>
    </div>
  {/if}

  <!-- Conflict State -->
  {#if conflictFiles.length > 0}
    <div class="conflict-alert" role="alert">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#A12A20" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9"></circle><path d="M12 8v5M12 16.5v.5"></path>
      </svg>
      <div>
        <p class="conflict-title">Update stopped. Nothing was changed.</p>
        <p class="conflict-desc">These files were changed by you and by your team:</p>
      </div>
    </div>
    <ul class="conflict-list">
      {#each conflictFiles as file}
        <li class="conflict-item">
          <span class="file-path">{file}</span>
          <button class="btn-open-editor" onclick={() => openInEditor(file)}>Open in editor</button>
        </li>
      {/each}
    </ul>
    <div class="conflict-footer">
      <button class="btn-secondary" onclick={() => (conflictFiles = [])}>Close</button>
    </div>
  {/if}

  <!-- Post-Update Success / Undo Banner -->
  {#if updateMessage && repo.undo_head}
    <div class="undo-banner" class:undo-disabled={!undoPossible} role="status">
      {#if undoPossible}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#1E6B34" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M5 12l5 5 9-10"></path>
        </svg>
        <p class="undo-text">{updateMessage}</p>
        <button class="btn-undo" onclick={handleUndo}>Undo</button>
      {:else}
        <div>
          <p class="undo-text">Updated recently.</p>
          <p class="undo-sub">You changed files since the update.</p>
        </div>
        <button class="btn-undo" disabled>Undo</button>
      {/if}
    </div>
  {/if}

  <!-- Feed Details -->
  {#if repo.feed && !confirmingUnsaved && !confirmingCombine && conflictFiles.length === 0}
    <p class="feed-summary">
      {repo.feed.author_summary}
      {#if repo.feed.newest_time}
        <span class="feed-time">· {formatTime(repo.feed.newest_time)}</span>
      {/if}
    </p>

    <ul class="commit-list">
      {#each repo.feed.changes.slice(0, 3) as change}
        <li>{change.title}</li>
      {/each}
      {#if repo.feed.total_changes > 3}
        <li class="more-commits">and {repo.feed.total_changes - 3} more</li>
      {/if}
    </ul>

    {#if repo.feed.files_changed.length > 0}
      <button class="btn-files-toggle" onclick={() => (showFiles = !showFiles)} aria-expanded={showFiles}>
        <svg class="chevron" class:open={showFiles} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 6l6 6-6 6"></path>
        </svg>
        {repo.feed.files_changed.length} file{repo.feed.files_changed.length === 1 ? "" : "s"} changed
      </button>

      {#if showFiles}
        <ul class="files-list">
          {#each repo.feed.files_changed as f}
            <li>{f}</li>
          {/each}
        </ul>
      {/if}
    {/if}
  {/if}

  <!-- Error Action -->
  {#if repo.error}
    <p class="err-text">{repo.error}</p>
    <div class="err-actions">
      {#if repo.error.includes("not found") && onRemoveRepo}
        <button class="btn-secondary" onclick={() => onRemoveRepo?.(repo.path)}>Remove</button>
      {:else}
        <button class="btn-primary" onclick={onStateRefresh}>Try again</button>
      {/if}
    </div>
  {/if}
</article>

<style>
  .repo-card {
    background: #ffffff;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .conflict-card {
    border: 1.5px solid #c4372b;
  }
  .card-header {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .repo-title {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
    flex-grow: 1;
  }
  .btn-update {
    height: 38px;
    padding: 0 16px;
    border: 1px solid #1f5fd1;
    border-radius: 8px;
    background: #ffffff;
    color: #1f5fd1;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-update:hover {
    background: #f0f5fd;
  }
  .badge-row {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .badge {
    font-size: 12px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 6px;
  }
  .badge-behind {
    background: #e6eefc;
    color: #1a4fb0;
  }
  .badge-ahead {
    background: #eeeeec;
    color: #45454d;
  }
  .badge-unsaved {
    background: #fceedd;
    color: #84470c;
  }
  .badge-uptodate {
    background: #eeeeec;
    color: #45454d;
  }
  .badge-err {
    background: #fbe7e5;
    color: #a12a20;
  }
  .feed-summary {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
  }
  .feed-time {
    color: #5b5b66;
  }
  .commit-list {
    margin: 0;
    padding: 0 0 0 18px;
    font-size: 13px;
    line-height: 1.5;
    color: #3d3d45;
  }
  .more-commits {
    color: #5b5b66;
  }
  .btn-files-toggle {
    align-self: flex-start;
    height: 32px;
    display: flex;
    align-items: center;
    gap: 4px;
    margin: -4px 0 -4px -6px;
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
  .files-list {
    margin: 0;
    padding: 6px 12px;
    background: #fafaf8;
    border: 1px solid #efecec;
    border-radius: 6px;
    font-family: monospace;
    font-size: 12px;
    color: #3d3d45;
    list-style: none;
    display: grid;
    gap: 4px;
  }
  .prompt-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: 8px;
  }
  .prompt-amber {
    background: #faf7f2;
    border: 1px solid #ebddc8;
  }
  .prompt-blue {
    background: #f4f6fb;
    border: 1px solid #d5def0;
  }
  .prompt-title {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
  }
  .prompt-desc {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .prompt-sub {
    margin: 0;
    font-size: 13px;
    color: #5b5b66;
  }
  .prompt-actions {
    display: flex;
    gap: 8px;
    justify-content: flex-end;
    margin-top: 4px;
  }
  .btn-text {
    height: 38px;
    padding: 0 14px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    font-size: 14px;
    cursor: pointer;
  }
  .btn-primary {
    height: 38px;
    padding: 0 16px;
    border: none;
    border-radius: 8px;
    background: #1f5fd1;
    color: #ffffff;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-primary:hover {
    background: #174aa6;
  }
  .btn-secondary {
    height: 38px;
    padding: 0 16px;
    border: 1px solid #c9c9c4;
    border-radius: 8px;
    background: #ffffff;
    color: #1b1b1f;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .undo-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: #eaf4ec;
  }
  .undo-disabled {
    background: #f3f3f1;
  }
  .undo-text {
    margin: 0;
    flex-grow: 1;
    font-size: 14px;
    color: #1e4d2b;
  }
  .undo-disabled .undo-text {
    color: #3d3d45;
  }
  .undo-sub {
    margin: 2px 0 0;
    font-size: 13px;
    color: #5b5b66;
  }
  .btn-undo {
    height: 36px;
    padding: 0 14px;
    border: 1px solid #9cc4a6;
    border-radius: 8px;
    background: #ffffff;
    color: #1e4d2b;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-undo:disabled {
    border-color: #dadad6;
    background: #f3f3f1;
    color: #6e6e78;
    cursor: not-allowed;
  }
  .conflict-alert {
    display: flex;
    gap: 10px;
    align-items: flex-start;
  }
  .conflict-title {
    margin: 0;
    font-size: 14px;
    font-weight: 500;
    color: #a12a20;
  }
  .conflict-desc {
    margin: 4px 0 0;
    font-size: 14px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .conflict-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    border: 1px solid #e3e3e0;
    border-radius: 8px;
  }
  .conflict-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 14px;
    border-bottom: 1px solid #e3e3e0;
  }
  .conflict-item:last-child {
    border-bottom: none;
  }
  .file-path {
    flex-grow: 1;
    font-size: 13px;
    font-family: monospace;
  }
  .btn-open-editor {
    height: 36px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #1f5fd1;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .conflict-footer {
    display: flex;
    justify-content: flex-end;
  }
  .err-text {
    margin: 0;
    font-size: 14px;
    color: #3d3d45;
  }
  .err-actions {
    display: flex;
    justify-content: flex-end;
  }
</style>
