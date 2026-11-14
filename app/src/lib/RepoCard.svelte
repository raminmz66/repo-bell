<script module lang="ts">
  import type { PlanView } from "./types";

  export type Notice = { kind: "success" | "info" | "warn" | "error"; text: string };

  /** What a row remembers while the window moves it between groups. */
  export type PanelState = {
    plan: PlanView | null;
    notice: Notice | null;
    noticeFrom: "update" | "cleanup";
    conflictFiles: string[];
    conflictMessage: string;
    stashAlert: string | null;
    stashRef: string | null;
    stashCommand: string | null;
    mainDetail: string | null;
    showDetail: boolean;
    fileError: string | null;
    actionError: string | null;
    showBranches: boolean;
    choice: Record<string, boolean>;
  };
</script>

<script lang="ts">
  import { tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import FeedList from "./FeedList.svelte";
  import type { RepoCardView, UpdateResult, SimpleResult } from "./types";
  import { errText, sentence, plural, timeAgo, shortTimeAgo } from "./util";

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
  /** The window owns open/closed, so a row keeps its state when it moves between groups. */
  export let expanded: boolean = false;
  export let onToggle: () => void = () => {};
  /** Panel memory by repo path, kept by the window. */
  export let panels: Map<string, PanelState> = new Map();
  /** Says whether an alert panel is showing, so the window can keep the row's group right. */
  export let onAlertChange: (path: string, on: boolean) => void = () => {};
  /** Says whether an action is running, so the window keeps the row in its group until it ends. */
  export let onBusyChange: (path: string, busy: boolean) => void = () => {};

  const saved = panels.get(repo.path);

  let rootEl: HTMLElement;
  let updateBtn: HTMLButtonElement | undefined;

  let plan: PlanView | null = saved?.plan ?? null;
  let busy: "" | "preview" | "update" | "main" | "undo" | "remove" | "cleanup" | "cleanup-undo" = "";
  let notice: Notice | null = saved?.notice ?? null;
  let conflictFiles: string[] = saved?.conflictFiles ?? [];
  let conflictMessage = saved?.conflictMessage ?? "";
  let stashAlert: string | null = saved?.stashAlert ?? null;
  let stashRef: string | null = saved?.stashRef ?? null;
  let stashCommand: string | null = saved?.stashCommand ?? null;
  let mainDetail: string | null = saved?.mainDetail ?? null;
  let showDetail = saved?.showDetail ?? false;
  let fileError: string | null = saved?.fileError ?? null;
  let actionError: string | null = saved?.actionError ?? null;
  let showBranches = saved?.showBranches ?? false;
  // Ticks the user changed. A row not listed here follows its default: ticked when merged.
  // Ticks are keyed by name and tip, so a branch that moved after ticking is unticked.
  let choice: Record<string, boolean> = saved?.choice ?? {};
  const tickKey = (b: { name: string; sha: string }) => `${b.name}@${b.sha}`;
  let noticeFrom: "update" | "cleanup" = saved?.noticeFrom ?? "update";

  $: listId = `old-${repo.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
  $: bodyId = `body-${repo.path.replace(/[^a-zA-Z0-9]/g, "-")}`;
  $: hasNews = !repo.error && repo.new_count > 0;
  $: expandable = !repo.error && repo.feed !== null;
  $: stopped = repo.error_kind === "folder_missing" || repo.error_kind === "not_a_repo";
  $: canUpdate = !repo.error && repo.on_main && repo.behind > 0;
  $: onSide = !repo.error && !repo.on_main;
  $: canUpdateMain = onSide && repo.main_behind > 0;
  $: branchName = repo.branch ?? "Not on a branch";
  $: showUnsaved = !repo.error && repo.unsaved;
  $: showAhead = !repo.error && repo.on_main && repo.ahead > 0;
  $: showBranchFlag = hasNews && !repo.on_main;
  // Authors of the changes, newest first: "Sara, Amine and 2 others".
  $: authors = [
    ...new Set(
      [...(repo.feed?.changes ?? [])].sort((a, b) => b.timestamp - a.timestamp).map((c) => (c.mine ? "You" : c.author)),
    ),
  ];
  $: fileCount = repo.feed?.files_changed.length ?? 0;
  $: newsText = [
    authors.length > 2
      ? `${authors.slice(0, 2).join(", ")} and ${authors.length - 2} ${plural(authors.length - 2, "other")}`
      : authors.join(", "),
    fileCount > 0 ? `${fileCount} ${plural(fileCount, "file")}` : "",
  ]
    .filter(Boolean)
    .join(" · ");
  // The one text on line 2. Flags come after it.
  $: line2 = repo.error
    ? { text: repo.error, title: undefined }
    : hasNews
      ? { text: newsText, title: undefined }
      : repo.on_main
        ? repo.behind > 0
          ? { text: `Behind by ${repo.behind}`, title: `The team made ${repo.behind} ${plural(repo.behind, "change")} you don't have yet` }
          : { text: "", title: undefined }
        : { text: `${branchName} · main ${repo.main_behind > 0 ? `${repo.main_behind} behind` : "up to date"}`, title: undefined };
  $: hasLine2 = line2.text !== "" || showBranchFlag || showUnsaved || showOld || showAhead;
  $: showOld = !repo.error && repo.old_branches.length > 0;
  $: mergedRows = repo.old_branches.filter((b) => b.merged);
  $: unmergedRows = repo.old_branches.filter((b) => !b.merged);
  $: pickedRows = repo.old_branches.filter((b) => choice[tickKey(b)] ?? b.merged);
  $: hasAlert =
    plan !== null ||
    conflictFiles.length > 0 ||
    stashAlert !== null ||
    notice?.kind === "warn" ||
    notice?.kind === "error" ||
    actionError !== null;
  $: panels.set(repo.path, { plan, notice, noticeFrom, conflictFiles, conflictMessage, stashAlert, stashRef, stashCommand, mainDetail, showDetail, fileError, actionError, showBranches, choice });
  $: onAlertChange(repo.path, hasAlert);
  $: onBusyChange(repo.path, busy !== "");
  $: if (highlight && rootEl) reveal();

  async function reveal() {
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
    noticeFrom = "update";
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

  async function toggleBranches() {
    showBranches = !showBranches;
    if (!showBranches) return;
    try {
      await invoke("refresh_old_branches", { path: repo.path });
    } catch {
      // The list stays as it was; Delete still checks every branch before it deletes.
    }
  }

  async function handleDeleteBranches() {
    const branches = pickedRows.map(({ name, sha }) => ({ name, sha }));
    if (branches.length === 0) return;
    clearMessages();
    noticeFrom = "cleanup";
    busy = "cleanup";
    try {
      const r = await invoke<SimpleResult>("delete_old_branches", { path: repo.path, branches });
      notice = { kind: r.status === "deleted" ? "success" : "warn", text: r.message };
      choice = {};
      showBranches = false;
      onStateRefresh();
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
    }
  }

  async function handleUndoCleanup() {
    noticeFrom = "cleanup";
    busy = "cleanup-undo";
    notice = null;
    try {
      const r = await invoke<SimpleResult>("undo_cleanup", { path: repo.path });
      notice = { kind: r.status === "undone" ? "success" : "warn", text: r.message };
      onStateRefresh();
    } catch (e) {
      notice = { kind: "error", text: errText(e) };
    } finally {
      busy = "";
    }
  }

  async function handleUndo() {
    noticeFrom = "update";
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
  <span class="badge" title={`Your team made ${repo.new_count} ${plural(repo.new_count, "change")} since you last looked`}>{repo.new_count} new</span>
{/snippet}

<li
  class="row"
  class:open={expandable && expanded}
  class:can-open={expandable}
  class:highlight
  class:alert-ring={conflictFiles.length > 0 || stashAlert !== null}
  data-repo-path={repo.path}
  bind:this={rootEl}
>
  <div class="row-head">
    <h3 class="title" class:indent={!expandable}>
      {#if expandable}
        <button class="toggle" aria-expanded={expanded} aria-controls={bodyId} onclick={onToggle}>
          <svg class="chevron" class:open={expanded} width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M9 18l6-6-6-6"></path>
          </svg>
          <span class="name" title={repo.name}>{repo.name}</span>
          {#if hasNews}{@render newBadge()}{/if}
        </button>
      {:else}
        <span class="name" title={repo.name}>{repo.name}</span>
        {#if hasNews}{@render newBadge()}{/if}
      {/if}
    </h3>

    <div class="side" class:two={hasLine2}>
      {#if hasNews && repo.feed?.newest_time}
        <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
        <span class="when" title={`Newest change ${timeAgo(repo.feed.newest_time, now)}`} onclick={expandable ? onToggle : undefined}>{shortTimeAgo(repo.feed.newest_time, now)}</span>
      {/if}
      {#if repo.error}
        {#if stopped}
          <button class="btn-secondary row-act" disabled={busy === "remove" || !onStopWatching} title="Stop watching this repo. Your files aren't touched." onclick={handleRemove}>
            {busy === "remove" ? "Removing…" : "Remove"}
          </button>
        {:else}
          <button class="btn-outline row-act" disabled={checking} onclick={handleTryAgain}>
            {checking ? "Checking…" : "Try again"}
          </button>
        {/if}
      {:else if canUpdate}
        {#if !plan && conflictFiles.length === 0}
          <button class="btn-update row-act" bind:this={updateBtn} disabled={busy !== ""} onclick={startUpdate}>
            {busy === "preview" ? "Checking…" : busy === "update" ? "Updating…" : "Update"}
          </button>
        {/if}
      {:else if canUpdateMain}
        <button class="btn-main row-act" title="Refreshes main in the background. Your files stay as they are." disabled={busy !== ""} onclick={handleUpdateMain}>
          {busy === "main" ? "Updating main…" : "Update main"}
        </button>
      {/if}
    </div>

    {#if hasLine2}
      <div class="l2">
        {#if line2.text}
          <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
          <span class="l2-text" class:error-text={repo.error} class:titled={line2.title} title={line2.title} onclick={line2.title && expandable ? onToggle : undefined}>{line2.text}</span>
        {/if}
        {#if showBranchFlag}<span class="flag flag-grey flag-branch">{branchName}</span>{/if}
        {#if showUnsaved}<span class="flag flag-amber" title="You edited files that aren't saved as a change yet">Unsaved</span>{/if}
        {#if showOld}
          <button
            class="flag flag-grey flag-btn"
            aria-expanded={showBranches}
            aria-controls={listId}
            title="Your team deleted these branches on the server. You can delete your copies."
            onclick={toggleBranches}
          >
            {repo.old_branches.length} old {plural(repo.old_branches.length, "branch", "branches")}
          </button>
        {/if}
        {#if showAhead}
          <span class="flag flag-grey" title={`You have ${repo.ahead} ${plural(repo.ahead, "change")} the team doesn't have yet`}>Ahead by {repo.ahead}</span>
        {/if}
      </div>
    {/if}
  </div>

  <div class="row-body">
    {#if expandable && expanded && repo.feed}
      <div class="feed" id={bodyId}>
        <FeedList feed={repo.feed} repoPath={repo.path} />
      </div>
    {/if}

    {#if showOld && showBranches}
      <div id={listId} class="old-branches">
        {#snippet group(title: string, hint: string | null, rows: typeof repo.old_branches)}
          {#if rows.length > 0}
            <div class="old-group" role="group" aria-label={title}>
              <p class="old-title">{title}</p>
              {#if hint}<p class="hint old-hint">{hint}</p>{/if}
              {#each rows as b (b.name)}
                <label class="old-row">
                  <input
                    type="checkbox"
                    aria-label={b.name}
                    checked={choice[tickKey(b)] ?? b.merged}
                    onchange={(e) => (choice = { ...choice, [tickKey(b)]: e.currentTarget.checked })}
                  />
                  <span class="old-name">{b.name}</span>
                  <span class="old-time">last change {timeAgo(b.last_commit_time)}</span>
                </label>
              {/each}
            </div>
          {/if}
        {/snippet}
        {@render group("Merged into main", null, mergedRows)}
        {@render group("May not be merged", "The server no longer has these, but Git can't find them in main. Tick only the ones you're done with.", unmergedRows)}
        <div class="old-actions">
          <button class="btn-secondary" disabled={pickedRows.length === 0 || busy !== ""} onclick={handleDeleteBranches}>
            {busy === "cleanup" ? "Deleting…" : pickedRows.length === 0 ? "Delete branches" : `Delete ${pickedRows.length} ${plural(pickedRows.length, "branch", "branches")}`}
          </button>
        </div>
      </div>
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
        <p class="undo-text">{notice?.kind === "success" && noticeFrom === "update" ? notice.text : "RepoBell updated this repo. You can undo it."}</p>
        <button class="btn-undo" disabled={busy !== ""} onclick={handleUndo}>{busy === "undo" ? "Undoing…" : "Undo"}</button>
      </div>
    {/if}

    <!-- Undo for branch cleanup -->
    {#if repo.can_undo_cleanup}
      <div class="undo-banner" role="status">
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#1E6B34" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M5 12l5 5 9-10"></path>
        </svg>
        <p class="undo-text">{notice?.kind === "success" && noticeFrom === "cleanup" ? notice.text : "RepoBell deleted branches here. You can undo it."}</p>
        <button class="btn-undo" aria-label="Undo branch delete" disabled={busy !== ""} onclick={handleUndoCleanup}>{busy === "cleanup-undo" ? "Undoing…" : "Undo"}</button>
      </div>
    {/if}

    {#if notice && !(notice.kind === "success" && ((repo.can_undo && noticeFrom === "update") || (repo.can_undo_cleanup && noticeFrom === "cleanup")))}
      <div class={`notice notice-${notice.kind}`} role={notice.kind === "error" || notice.kind === "warn" ? "alert" : "status"}>
        <p>{notice.text}</p>
        {#if mainDetail}
          <button class="link-btn" aria-expanded={showDetail} onclick={() => (showDetail = !showDetail)}>Details</button>
          {#if showDetail}<pre class="detail">{mainDetail}</pre>{/if}
        {/if}
      </div>
    {/if}

    {#if actionError}<p class="msg-error" role="alert">{actionError}</p>{/if}
  </div>
</li>

<style>
  .row {
    border-top: 1px solid #e3e3e0;
    transition: background 0.15s ease, box-shadow 0.3s ease;
  }
  .row:first-child {
    border-top: none;
  }
  .row.open {
    background: #fbfbf9;
  }
  .row.can-open:hover {
    background: #fbfbf9;
  }
  .row.highlight,
  .row.can-open.highlight:hover {
    background: #f4f8fe;
    box-shadow: inset 0 0 0 2px #1f5fd1;
  }
  .row.alert-ring {
    box-shadow: inset 0 0 0 1.5px #c4372b;
  }
  /* Line 1 and line 2. The toggle's ::after covers this whole box, so a click anywhere on it toggles the row. */
  .row-head {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    column-gap: 10px;
    row-gap: 2px;
    align-items: center;
    padding: 9px 12px;
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    margin: 0;
    font-size: 14px;
    font-weight: 700;
  }
  .title.indent {
    padding-left: 22px;
  }
  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
    max-width: 100%;
    padding: 0;
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .toggle::after {
    content: "";
    position: absolute;
    inset: 0;
  }
  .toggle:focus-visible {
    outline: none;
  }
  .toggle:focus-visible::after {
    outline: 2px solid #1f5fd1;
    outline-offset: -2px;
  }
  .name {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    position: relative;
    z-index: 1;
  }
  .chevron {
    flex-shrink: 0;
    color: #5b5b66;
    transition: transform 0.2s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .badge {
    position: relative;
    z-index: 1;
    flex-shrink: 0;
    padding: 2px 8px;
    border-radius: 6px;
    background: #1f5fd1;
    color: #ffffff;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
  }
  .side {
    grid-column: 2;
    grid-row: 1;
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .side.two {
    grid-row: 1 / span 2;
  }
  .when {
    position: relative;
    z-index: 1;
    font-size: 12px;
    color: #5b5b66;
    font-variant-numeric: tabular-nums;
  }
  .l2 {
    grid-column: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 3px 6px;
    min-width: 0;
    padding-left: 22px;
    font-size: 13px;
    color: #5b5b66;
  }
  .l2-text {
    max-width: 100%;
    overflow-wrap: anywhere;
  }
  .l2-text.titled {
    position: relative;
    z-index: 1;
    cursor: help;
  }
  .l2-text.error-text {
    color: #3d3d45;
  }
  .flag {
    position: relative;
    z-index: 1;
    padding: 1px 7px;
    border-radius: 6px;
    font-size: 12px;
    font-weight: 500;
    white-space: nowrap;
    cursor: default;
  }
  .flag-grey {
    background: #eeeeec;
    color: #3d3d45;
  }
  .flag-branch {
    max-width: 100%;
    white-space: normal;
    overflow-wrap: anywhere;
  }
  .flag-amber {
    background: #fceedd;
    color: #84470c;
  }
  .flag-btn {
    border: none;
    font-family: inherit;
    cursor: pointer;
  }
  .flag-btn:hover {
    background: #e3e3e0;
  }
  .btn-update {
    border: 1px solid #1f5fd1;
    border-radius: 8px;
    background: #ffffff;
    color: #1f5fd1;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-update:hover:not(:disabled) {
    background: #f0f5fd;
  }
  .btn-update:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn-main {
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #1f5fd1;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-main:hover:not(:disabled) {
    background: #f0f5fd;
  }
  .btn-main:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .hint {
    margin: -4px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .row-body {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 0 12px;
  }
  .row-body > :global(:last-child) {
    margin-bottom: 10px;
  }
  .feed {
    padding-left: 22px;
  }
  .old-branches {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .old-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .old-title {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #3d3d45;
  }
  .old-hint {
    margin: 0;
  }
  .old-row {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 32px;
    font-size: 14px;
  }
  .old-name {
    flex: 1;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .old-time {
    font-size: 13px;
    color: #5b5b66;
    white-space: nowrap;
  }
  .old-actions {
    display: flex;
    justify-content: flex-end;
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
  .msg-error {
    margin: 0;
    font-size: 13px;
    color: #a12a20;
  }
  /* Row buttons are 32px, below the 36px minimum: the whole-row hit area makes up for it. */
  .row-act {
    position: relative;
    z-index: 1;
    height: 32px;
    padding: 0 12px;
    font-size: 13px;
  }
  .btn-main.row-act {
    padding: 0 8px;
  }
  @media (prefers-reduced-motion: reduce) {
    .row,
    .chevron {
      transition: none;
    }
  }
</style>
