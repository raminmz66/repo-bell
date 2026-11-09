<script lang="ts">
  import { tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import FeedList from "./FeedList.svelte";
  import type { RepoCardView, PlanView, UpdateResult, SimpleResult } from "./types";
  import { errText, sentence, plural } from "./util";

  export let repo: RepoCardView;
  /** Current time in ms, so "5 min ago" stays fresh. */
  export let now: number = Date.now();
  /** A check is running, so "Try again" would only queue another one. */
  export let checking: boolean = false;
  /** Scroll to this card and flash it (from the tray menu). */
  export let highlight: boolean = false;
  export let onStateRefresh: () => void = () => {};
  export let onTryAgain: () => Promise<void> | void = () => {};
  export let onStopWatching: ((path: string) => Promise<void>) | undefined = undefined;

  type Notice = { kind: "success" | "info" | "warn" | "error"; text: string };

  let rootEl: HTMLElement;
  let updateBtn: HTMLButtonElement | undefined;
  let isExpanded = repo.new_count > 0 && !repo.error;
  let prevNew = repo.new_count;

  let plan: PlanView | null = null;
  let busy: "" | "preview" | "update" | "main" | "undo" | "remove" = "";
  let notice: Notice | null = null;
  let conflictFiles: string[] = [];
  let conflictMessage = "";
  let stashAlert: string | null = null;
  let stashRef: string | null = null;
  let stashCommand: string | null = null;
  let mainDetail: string | null = null;
  let showDetail = false;
  let fileError: string | null = null;
  let actionError: string | null = null;

  $: bodyId = `body-${repo.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
  $: hasNews = !repo.error && repo.new_count > 0;
  $: compact = !repo.error && repo.new_count === 0;
  $: expandable = !repo.error && repo.feed !== null;
  $: stopped = repo.error_kind === "folder_missing" || repo.error_kind === "not_a_repo";
  $: canUpdate = !repo.error && repo.on_main && repo.behind > 0;
  $: onSide = !repo.error && !repo.on_main;
  $: canUpdateMain = onSide && repo.main_behind > 0;
  $: plainUpToDate = repo.on_main && repo.behind === 0 && repo.ahead === 0;
  $: sideLabel = repo.branch ? `You're on ${repo.branch}` : "You're not on a branch";
  $: trackNews(repo.new_count);
  $: if (highlight && rootEl) reveal();

  // Fresh news on a card that was quiet opens it.
  function trackNews(n: number) {
    if (n > 0 && prevNew === 0 && !repo.error) isExpanded = true;
    prevNew = n;
  }

  async function reveal() {
    if (expandable) isExpanded = true;
    await tick();
    const reduce = window.matchMedia?.("(prefers-reduced-motion: reduce)").matches;
    rootEl?.scrollIntoView?.({ block: "center", behavior: reduce ? "auto" : "smooth" });
  }

  // Puts focus back on the Update button once the preview is gone, so keyboard users keep their place.
  async function focusUpdate() {
    await tick();
    updateBtn?.focus();
  }

  function closePlan() {
    plan = null;
    focusUpdate();
  }

  function onPromptKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    closePlan();
  }

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  function clearMessages() {
    notice = null;
    conflictFiles = [];
    stashAlert = null;
    stashRef = null;
    stashCommand = null;
    mainDetail = null;
    showDetail = false;
    actionError = null;
  }

  async function startUpdate() {
    clearMessages();
    plan = null;
    busy = "preview";
    try {
      plan = await invoke<PlanView>("preview_update", { path: repo.path });
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
    }
  }

  async function confirmUpdate() {
    if (!plan) return;
    const expected = plan;
    plan = null;
    busy = "update";
    try {
      const r = await invoke<UpdateResult>("update_repo", { path: repo.path, expected });
      switch (r.status) {
        case "updated":
          notice = { kind: "success", text: r.message };
          break;
        case "up_to_date":
          notice = { kind: "info", text: r.message };
          break;
        case "conflict":
          conflictMessage = r.message;
          conflictFiles = r.conflict_files;
          break;
        case "plan_changed":
          notice = { kind: "info", text: r.message };
          plan = r.plan;
          break;
        case "edits_in_stash":
          stashAlert = r.message;
          stashRef = r.stash;
          stashCommand = r.stash_command;
          break;
        default:
          notice = { kind: "warn", text: r.message };
      }
      onStateRefresh();
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
      if (!plan) focusUpdate();
    }
  }

  async function handleUndo() {
    busy = "undo";
    notice = null;
    try {
      const r = await invoke<SimpleResult>("undo_repo", { path: repo.path });
      notice = { kind: r.status === "undone" ? "success" : "warn", text: r.message };
      onStateRefresh();
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
    }
  }

  async function handleUpdateMain() {
    clearMessages();
    plan = null;
    busy = "main";
    try {
      const r = await invoke<SimpleResult>("update_main", { path: repo.path });
      const kind = r.status === "updated" ? "success" : r.status === "up_to_date" ? "info" : r.status === "failed" ? "error" : "warn";
      notice = { kind, text: r.message };
      mainDetail = r.detail;
      onStateRefresh();
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
    }
  }

  async function openInEditor(file: string) {
    fileError = null;
    try {
      await invoke("open_editor", { repo: repo.path, file });
    } catch (e) {
      fileError = errText(e);
    }
  }

  async function handleRemove() {
    if (!onStopWatching) return;
    actionError = null;
    busy = "remove";
    try {
      await onStopWatching(repo.path);
    } catch (e) {
      actionError = errText(e);
    } finally {
      busy = "";
    }
  }

  async function handleTryAgain() {
    actionError = null;
    try {
      await onTryAgain();
    } catch (e) {
      actionError = errText(e);
    }
  }
</script>

{#snippet newBadge()}
  <span class="badge badge-new" title={`Your team made ${repo.new_count} ${plural(repo.new_count, "change")} since you last looked`}>{repo.new_count} new</span>
{/snippet}

{#snippet behindText()}
  <span class="status-behind" title={`The team made ${repo.behind} ${plural(repo.behind, "change")} you don't have yet`}>
    Behind by {repo.behind}
  </span>
{/snippet}

<article
  class="repo-card"
  class:compact
  class:highlight
  class:conflict-card={conflictFiles.length > 0 || stashAlert !== null}
  data-repo-path={repo.path}
  bind:this={rootEl}
>
  <div class="card-header">
    <div class="title-group">
      <h2 class="repo-title">
        {#if expandable}
          <button class="disclosure" aria-expanded={isExpanded} aria-controls={bodyId} onclick={() => (isExpanded = !isExpanded)}>
            <svg class="drawer-chevron" class:open={isExpanded} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M9 18l6-6-6-6"></path>
            </svg>
            <span class="name">{repo.name}</span>
            {#if hasNews}{@render newBadge()}{/if}
          </button>
        {:else}
          <span class="name plain-name">{repo.name}</span>
        {/if}
      </h2>
      {#if repo.error}
        <span class="badge badge-err" title="RepoBell couldn't check this repo">Can't check</span>
      {:else if hasNews && !expandable}
        {@render newBadge()}
      {:else if compact && plainUpToDate}
        <span class="status-quiet">Up to date</span>
      {/if}
    </div>

    <div class="action-group">
      {#if canUpdate}
        {#if !hasNews}{@render behindText()}{/if}
        {#if !plan && conflictFiles.length === 0}
          <button class="btn-update" bind:this={updateBtn} disabled={busy !== ""} onclick={startUpdate}>
            {busy === "preview" ? "Checking…" : busy === "update" ? "Updating…" : "Update"}
          </button>
        {/if}
      {:else if canUpdateMain}
        <button class="btn-update" disabled={busy !== ""} onclick={handleUpdateMain}>
          {busy === "main" ? "Updating main…" : "Update main"}
        </button>
      {:else if onSide}
        <span class="status-quiet">Main is up to date</span>
      {/if}
    </div>
  </div>

  {#if !repo.error && ((hasNews && canUpdate) || !repo.on_main || repo.unsaved || (repo.on_main && repo.ahead > 0))}
    <div class="chips">
      {#if hasNews && canUpdate}{@render behindText()}{/if}
      {#if !repo.on_main}
        <span class="chip chip-branch">{sideLabel}</span>
      {/if}
      {#if repo.on_main && repo.ahead > 0}
        <span class="chip chip-grey" title={`You have ${repo.ahead} ${plural(repo.ahead, "change")} the team doesn't have yet`}>Ahead by {repo.ahead}</span>
      {/if}
      {#if repo.unsaved}
        <span class="chip chip-amber" title="You edited files that aren't saved as a change yet">Has unsaved changes</span>
      {/if}
    </div>
  {/if}

  {#if canUpdateMain}
    <p class="hint">Refreshes the main version in the background. Your files stay as they are.</p>
  {/if}

  <!-- Update preview and confirm -->
  {#if plan}
    {@const runnable = plan.kind === "fast_forward" || plan.kind === "combine"}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="prompt-box"
      class:prompt-amber={plan.set_aside && plan.kind !== "combine"}
      class:prompt-blue={plan.kind === "combine"}
      tabindex="-1"
      role="group"
      aria-label="Update preview"
      onkeydown={onPromptKey}
      use:focusOnMount
    >
      {#if plan.kind === "combine"}
        <p class="prompt-title">Your work and the team's work both changed.</p>
        <p class="prompt-desc">RepoBell can combine them. If that fails, nothing is changed.</p>
      {:else if plan.set_aside}
        <p class="prompt-title">You have unsaved changes.</p>
        <p class="prompt-desc">Set them aside, update, then bring them back?</p>
      {/if}
      <p class="prompt-sub" class:prompt-main={!runnable || plan.kind === "fast_forward"}>{sentence(plan.message)}</p>
      <div class="prompt-actions">
        {#if runnable}
          <button class="btn-text" onclick={closePlan}>Cancel</button>
          <button class="btn-primary" onclick={confirmUpdate}>
            {plan.kind === "combine" ? "Update and combine" : plan.set_aside ? "Set aside and update" : "Update"}
          </button>
        {:else}
          <button class="btn-secondary" onclick={closePlan}>OK</button>
        {/if}
      </div>
    </div>
  {/if}

  <!-- Edits are in Git's stash: the one case where the user's work needs a developer -->
  {#if stashAlert}
    <div class="stash-alert" role="alert">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#A12A20" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M12 3l9.5 17h-19z"></path><path d="M12 10v4M12 17v.5"></path>
      </svg>
      <div class="stash-text">
        <p>{stashAlert}</p>
        {#if stashRef}
          <details class="stash-details">
            <summary>Details</summary>
            <p>Git's name for the saved changes: <code>{stashRef}</code></p>
            {#if stashCommand}<p>To bring them back, run: <code>{stashCommand}</code></p>{/if}
          </details>
        {/if}
      </div>
      <button class="btn-secondary" onclick={() => (stashAlert = null)}>Close</button>
    </div>
  {/if}

  <!-- Conflict -->
  {#if conflictFiles.length > 0}
    <div class="conflict-alert" role="alert">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="#A12A20" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <circle cx="12" cy="12" r="9"></circle><path d="M12 8v5M12 16.5v.5"></path>
      </svg>
      <div>
        <p class="conflict-title">{conflictMessage}</p>
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
    {#if fileError}<p class="msg-error" role="alert">{fileError}</p>{/if}
    <div class="conflict-footer">
      <button class="btn-secondary" onclick={() => (conflictFiles = [])}>Close</button>
    </div>
  {/if}

  <!-- Undo (the backend says when it is still possible) -->
  {#if repo.can_undo}
    <div class="undo-banner" role="status">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#1E6B34" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M5 12l5 5 9-10"></path>
      </svg>
      <p class="undo-text">{notice?.kind === "success" ? notice.text : "RepoBell updated this repo. You can undo it."}</p>
      <button class="btn-undo" disabled={busy !== ""} onclick={handleUndo}>{busy === "undo" ? "Undoing…" : "Undo"}</button>
    </div>
  {/if}

  {#if notice && !(repo.can_undo && notice.kind === "success")}
    <div class={`notice notice-${notice.kind}`} role={notice.kind === "error" || notice.kind === "warn" ? "alert" : "status"}>
      <p>{notice.text}</p>
      {#if mainDetail}
        <button class="link-btn" aria-expanded={showDetail} onclick={() => (showDetail = !showDetail)}>Details</button>
        {#if showDetail}<pre class="detail">{mainDetail}</pre>{/if}
      {/if}
    </div>
  {/if}

  <!-- Feed -->
  {#if expandable && isExpanded && repo.feed}
    <div class="card-body" id={bodyId}>
      <FeedList feed={repo.feed} repoPath={repo.path} {now} />
    </div>
  {/if}

  <!-- Error -->
  {#if repo.error}
    <p class="err-text">{repo.error}</p>
    {#if actionError}<p class="msg-error" role="alert">{actionError}</p>{/if}
    <div class="err-actions">
      {#if stopped}
        <button class="btn-secondary" disabled={busy === "remove" || !onStopWatching} title="Stop watching this repo. Your files aren't touched." onclick={handleRemove}>
          {busy === "remove" ? "Removing…" : "Remove"}
        </button>
      {:else}
        <button class="btn-outline" disabled={checking} onclick={handleTryAgain}>
          {checking ? "Checking…" : "Try again"}
        </button>
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
    transition: background 0.15s ease, border-color 0.15s ease, box-shadow 0.3s ease;
  }
  .repo-card.compact {
    padding: 8px 14px;
    gap: 6px;
  }
  .repo-card.highlight {
    border-color: #1f5fd1;
    box-shadow: 0 0 0 3px rgba(31, 95, 209, 0.25);
    background: #f4f8fe;
  }
  .conflict-card {
    border: 1.5px solid #c4372b;
  }
  .card-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    min-height: 40px;
  }
  .title-group {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    min-width: 0;
  }
  .repo-title {
    margin: 0;
    flex: 1;
    min-width: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .disclosure {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    min-height: 36px;
    margin: 0 0 0 -6px;
    padding: 0 8px 0 6px;
    box-sizing: border-box;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .disclosure:hover {
    background: #f3f3f1;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .plain-name {
    display: block;
  }
  .drawer-chevron {
    flex-shrink: 0;
    color: #5b5b66;
    transition: transform 0.2s ease;
  }
  .drawer-chevron.open {
    transform: rotate(90deg);
  }
  .action-group {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .status-behind {
    font-size: 13px;
    white-space: nowrap;
    color: #5b5b66;
    cursor: help;
  }
  .status-quiet {
    font-size: 13px;
    color: #5b5b66;
    white-space: nowrap;
  }
  .btn-update {
    height: 40px;
    padding: 0 16px;
    border: 1px solid #1f5fd1;
    border-radius: 8px;
    background: #ffffff;
    color: #1f5fd1;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
  }
  .compact .btn-update {
    height: 36px;
  }
  .btn-update:hover:not(:disabled) {
    background: #f0f5fd;
  }
  .btn-update:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .badge {
    font-size: 12px;
    font-weight: 500;
    padding: 3px 9px;
    border-radius: 6px;
    flex-shrink: 0;
    white-space: nowrap;
  }
  .badge-new {
    background: #1f5fd1;
    color: #ffffff;
  }
  .badge-err {
    background: #fbe7e5;
    color: #a12a20;
  }
  .chips {
    display: flex;
    align-items: center;
    gap: 6px 10px;
    flex-wrap: wrap;
  }
  .chip {
    font-size: 12px;
    font-weight: 500;
    padding: 4px 10px;
    border-radius: 6px;
    cursor: default;
  }
  .chip-grey,
  .chip-branch {
    background: #eeeeec;
    color: #45454d;
  }
  .chip-amber {
    background: #fceedd;
    color: #84470c;
  }
  .hint {
    margin: -4px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .prompt-box {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-radius: 8px;
    outline: none;
    background: #f6f6f4;
    border: 1px solid #e3e3e0;
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
  .prompt-sub.prompt-main {
    font-size: 14px;
    color: #1b1b1f;
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
  .btn-text:hover {
    background: #eeeeec;
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
  .btn-secondary,
  .btn-outline {
    height: 40px;
    padding: 0 16px;
    border-radius: 8px;
    background: #ffffff;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-secondary {
    border: 1px solid #c9c9c4;
    color: #1b1b1f;
  }
  .btn-outline {
    border: 1px solid #1f5fd1;
    color: #1f5fd1;
  }
  .btn-secondary:disabled,
  .btn-outline:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn-outline:hover:not(:disabled) {
    background: #f0f5fd;
  }
  .stash-alert {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px;
    border: 1px solid #e8b9b4;
    border-radius: 8px;
    background: #fbe7e5;
  }
  .stash-alert svg {
    flex-shrink: 0;
    margin-top: 1px;
  }
  .stash-text {
    flex: 1;
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    font-weight: 500;
    color: #6f1d16;
  }
  .stash-text p {
    margin: 0;
  }
  .stash-details {
    margin-top: 6px;
    font-size: 13px;
    font-weight: 400;
  }
  .stash-details summary {
    cursor: pointer;
  }
  .stash-details p {
    margin-top: 4px;
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
    min-width: 0;
    overflow-wrap: anywhere;
    font-size: 13px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
  }
  .btn-open-editor {
    min-height: 40px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #1f5fd1;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    flex-shrink: 0;
  }
  .btn-open-editor:hover {
    background: #eef4fe;
  }
  .conflict-footer {
    display: flex;
    justify-content: flex-end;
  }
  .undo-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    background: #eaf4ec;
  }
  .undo-text {
    margin: 0;
    flex-grow: 1;
    font-size: 14px;
    color: #1e4d2b;
  }
  .btn-undo {
    height: 40px;
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
  .notice {
    padding: 10px 12px;
    border-radius: 8px;
    font-size: 14px;
    line-height: 1.4;
  }
  .notice p {
    margin: 0;
  }
  .notice-success {
    background: #eaf4ec;
    color: #1e4d2b;
  }
  .notice-info {
    background: #f3f3f1;
    color: #3d3d45;
  }
  .notice-warn {
    background: #fceedd;
    color: #6b3a08;
  }
  .notice-error {
    background: #fbe7e5;
    color: #8a2219;
  }
  .link-btn {
    margin-top: 4px;
    padding: 6px 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 13px;
    text-decoration: underline;
    cursor: pointer;
  }
  .detail {
    margin: 4px 0 0;
    padding: 8px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.7);
    font-size: 12px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .err-text {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .msg-error {
    margin: 0;
    font-size: 13px;
    color: #a12a20;
  }
  .err-actions {
    display: flex;
    justify-content: flex-end;
  }
  @media (prefers-reduced-motion: reduce) {
    .repo-card,
    .drawer-chevron {
      transition: none;
    }
  }
</style>
