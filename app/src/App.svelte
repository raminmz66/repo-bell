<script lang="ts">
  import { onMount, tick as nextTick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Header from "./lib/Header.svelte";
  import CheckStrip from "./lib/CheckStrip.svelte";
  import Footer from "./lib/Footer.svelte";
  import RepoCard from "./lib/RepoCard.svelte";
  import FolderErrorCard from "./lib/FolderErrorCard.svelte";
  import SettingsView from "./lib/SettingsView.svelte";
  import WelcomeView from "./lib/WelcomeView.svelte";
  import type { PanelState } from "./lib/RepoCard.svelte";
  import type { AppStateView, FolderError, RepoCardView } from "./lib/types";
  import { errText, plural, timeAgo } from "./lib/util";

  type Group = "error" | "news" | "onMain" | "onBranch" | "upToDate";

  let state: AppStateView | null = null;
  let loadError: string | null = null;
  let currentView: "main" | "settings" = "main";
  let localChecking = false;
  let showUpToDate = false;
  let notice: string | null = null;
  let now = Date.now();
  let highlightPath: string | null = null;
  // The first-run wizard stays up until its last step, even after settings are saved.
  let wizardActive = false;
  // What each row remembers while it moves between groups, and which rows show an alert.
  const panels = new Map<string, PanelState>();
  let alertPaths = new Set<string>();
  // A row stays in the group it was in while one of its actions runs, so its result isn't lost.
  let pinned = new Map<string, Group>();

  // Reassigns only when membership changes, otherwise the reactive call in each row loops.
  function setAlert(path: string, on: boolean) {
    if (alertPaths.has(path) === on) return;
    const next = new Set(alertPaths);
    if (on) next.add(path);
    else next.delete(path);
    alertPaths = next;
  }

  function setBusy(path: string, on: boolean) {
    if (on === pinned.has(path)) return;
    const next = new Map(pinned);
    if (on) {
      const r = watched.find((w) => w.path === path);
      if (!r) return;
      next.set(path, groupOf(r, pinned, alertPaths));
    } else next.delete(path);
    pinned = next;
  }

  // Which rows are open. A background update never closes a row; only the user, or window-shown, does.
  let openPaths = new Set<string>();
  // The News paths seen last time, so a repo that just entered News can be told apart.
  let prevNews = new Set<string>();

  function toggleRow(path: string) {
    const next = new Set(openPaths);
    if (!next.delete(path)) next.add(path);
    openPaths = next;
  }

  function openRow(path: string) {
    if (!openPaths.has(path)) openPaths = new Set(openPaths).add(path);
  }

  // Repos that enter News open while there is little news. Never closes anything.
  function syncNews(news: RepoCardView[]) {
    const entering = news.filter((r) => !prevNews.has(r.path)).map((r) => r.path);
    if (news.length <= 2 && entering.length > 0) openPaths = new Set([...openPaths, ...entering]);
    prevNews = new Set(news.map((r) => r.path));
  }

  // The window was just shown: apply the rule from scratch.
  function resetOpen() {
    openPaths = new Set(newsRepos.length <= 2 ? newsRepos.map((r) => r.path) : []);
    prevNews = new Set(newsRepos.map((r) => r.path));
  }

  function forget(path: string) {
    panels.delete(path);
    setAlert(path, false);
    setBusy(path, false);
  }

  async function loadState() {
    try {
      state = await invoke<AppStateView>("get_app_state");
      loadError = null;
    } catch (e) {
      if (!state) loadError = errText(e);
      else notice = errText(e);
    }
  }

  async function handleCheckNow() {
    localChecking = true;
    notice = null;
    try {
      // When a check is already running this returns at once with `checking: true`;
      // the result then arrives as a `state-updated` event.
      state = await invoke<AppStateView>("check_now");
    } catch (e) {
      notice = errText(e);
    } finally {
      localChecking = false;
    }
  }

  async function handleMarkAllSeen() {
    notice = null;
    try {
      state = await invoke<AppStateView>("mark_all_seen");
      // The button sat in the News label, which is gone now.
      await nextTick();
      document.querySelector<HTMLElement>('button[aria-label="Check now"]')?.focus();
    } catch (e) {
      notice = errText(e);
    }
  }

  async function handleAddFolder() {
    notice = null;
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && state && !state.settings.folders.includes(picked)) {
        state = await invoke<AppStateView>("save_settings", {
          settings: { ...state.settings, folders: [...state.settings.folders, picked] },
        });
      }
    } catch (e) {
      notice = errText(e);
    }
  }

  // Throws on failure so the card can show the reason.
  async function stopWatching(path: string) {
    if (!state) return;
    const ignored = Array.from(new Set([...state.settings.ignored, path]));
    state = await invoke<AppStateView>("save_settings", { settings: { ...state.settings, ignored } });
    forget(path);
  }

  async function removeFolder(path: string) {
    state = await invoke<AppStateView>("remove_folder", { path });
    forget(path);
  }

  // Back from Settings: put focus on the Settings button so keyboard users keep their place.
  async function closeSettings() {
    currentView = "main";
    await nextTick();
    document.querySelector<HTMLElement>('button[aria-label="Settings"]')?.focus();
  }

  function finishWizard() {
    wizardActive = false;
    loadState();
  }

  onMount(() => {
    loadState();
    const tick = setInterval(() => (now = Date.now()), 30_000);

    const unlistenState = listen<AppStateView>("state-updated", (event) => {
      state = event.payload;
      loadError = null;
    });

    const unlistenShown = listen("window-shown", resetOpen);

    const unlistenFocus = listen<string>("focus-repo", (event) => {
      const path = event.payload;
      currentView = "main";
      if (upToDate.some((r) => r.path === path)) showUpToDate = true;
      openRow(path);
      highlightPath = path;
      setTimeout(() => {
        if (highlightPath === path) highlightPath = null;
      }, 2200);
    });

    return () => {
      clearInterval(tick);
      unlistenState.then((f) => f());
      unlistenShown.then((f) => f());
      unlistenFocus.then((f) => f());
    };
  });

  // Screen readers hear only the start and the end of a check, not every step.
  let checkStatus = "";
  let wasChecking = false;
  function trackProgress(on: boolean) {
    if (on && !wasChecking) checkStatus = "Checking your repos";
    else if (!on && wasChecking) checkStatus = "Check finished";
    wasChecking = on;
  }

  // Repos that are up to date and need nothing from the user stay tucked away.
  const isPlain = (r: RepoCardView) => r.on_main && r.behind === 0 && !r.can_undo;

  // Each repo is in exactly one group. A pinned row (an action is running) keeps its group.
  function groupOf(r: RepoCardView, pins: Map<string, Group>, alerts: Set<string>): Group {
    const pin = pins.get(r.path);
    if (pin) return pin;
    if (r.error) return "error";
    if (r.new_count > 0) return "news";
    if (!r.on_main) return "onBranch";
    return isPlain(r) && !alerts.has(r.path) ? "upToDate" : "onMain";
  }

  $: if (state?.first_run) wizardActive = true;
  $: watched = state?.repos.filter((r) => !r.ignored) ?? [];
  $: errorRepos = watched.filter((r) => groupOf(r, pinned, alertPaths) === "error");
  $: newsRepos = watched.filter((r) => groupOf(r, pinned, alertPaths) === "news");
  // Rows that need Update main first, then rows with unsaved edits. The sort is stable.
  const branchTier = (r: RepoCardView) => (r.main_behind > 0 ? 0 : r.unsaved ? 1 : 2);
  $: onBranch = watched.filter((r) => groupOf(r, pinned, alertPaths) === "onBranch").sort((a, b) => branchTier(a) - branchTier(b));
  $: upToDate = watched.filter((r) => groupOf(r, pinned, alertPaths) === "upToDate");
  $: onMain = watched.filter((r) => groupOf(r, pinned, alertPaths) === "onMain");
  $: syncNews(newsRepos);
  $: folderErrors = state?.folder_errors ?? [];
  $: errorTotal = errorRepos.length + folderErrors.length;
  $: firstCheck = !!state && state.checking && state.last_checked === null;
  $: isChecking = localChecking || !!state?.checking;
  $: checkedAgo = state?.last_checked ? timeAgo(state.last_checked, now) : "";
  $: lastChecked = checkedAgo ? `Last checked ${checkedAgo}` : "";
  $: subline = checkedAgo ? `Checked ${checkedAgo}` : "";
  $: trackProgress(!!state?.progress);
  $: errorTitle = [
    errorRepos.length > 0 ? `${errorRepos.length} ${plural(errorRepos.length, "repo")} can't be checked` : "",
    folderErrors.length > 0 ? `${folderErrors.length} ${plural(folderErrors.length, "folder")} not found` : "",
  ]
    .filter(Boolean)
    .join(", ");
  $: headline =
    newsRepos.length > 0 || errorRepos.length > 0 ? (state?.headline ?? "") : errorTitle;
  $: allPlain = onMain.length === 0 && onBranch.length === 0 && errorTotal === 0;
  // No news and no trouble, but rows to act on: same compact headline, "All quiet" as its title.
  $: quiet = newsRepos.length === 0 && errorTotal === 0;
  $: headlineText = quiet ? "All quiet" : headline;
  $: headlineSub = quiet ? `Nothing new in your ${watched.length} ${plural(watched.length, "repo")}${subline ? ` · ${subline}` : ""}` : subline;
</script>

{#snippet rowList(rows: RepoCardView[], folders: FolderError[] = [])}
  <ul class="group-list">
    {#each rows as r (r.path)}
      <RepoCard
        repo={r}
        {now}
        checking={isChecking}
        highlight={highlightPath === r.path}
        {panels}
        expanded={openPaths.has(r.path)}
        onToggle={() => toggleRow(r.path)}
        onAlertChange={setAlert}
        onBusyChange={setBusy}
        onTryAgain={handleCheckNow}
        onStopWatching={stopWatching}
        onStateRefresh={loadState}
      />
    {/each}
    {#each folders as f (f.path)}
      <FolderErrorCard folder={f} onRemove={removeFolder} />
    {/each}
  </ul>
{/snippet}

{#snippet group(label: string, rows: RepoCardView[], extra: { hint?: string; folders?: FolderError[]; markSeen?: boolean } = {})}
  {@const count = rows.length + (extra.folders?.length ?? 0)}
  {#if count > 0}
    <section class="group">
      <div class="group-head">
        <h2 class="group-label">{label} <span class="count">{count}</span></h2>
        {#if extra.markSeen}
          <button class="group-act" onclick={handleMarkAllSeen}>Mark all as seen</button>
        {/if}
      </div>
      {#if extra.hint}<p class="group-hint">{extra.hint}</p>{/if}
      {@render rowList(rows, extra.folders)}
    </section>
  {/if}
{/snippet}

{#snippet quietGroups()}
  {@render group("On main", onMain)}
  {@render group("On a branch", onBranch, { hint: "Update main refreshes main. Your files stay as they are." })}
  {#if upToDate.length > 0}
    <button class="accordion-btn" onclick={() => (showUpToDate = !showUpToDate)} aria-expanded={showUpToDate}>
      <svg class="chevron" class:open={showUpToDate} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9 6l6 6-6 6"></path>
      </svg>
      <span class="accordion-label">Up to date</span>
      <span class="accordion-count">{upToDate.length} {plural(upToDate.length, "repo")}</span>
    </button>
    {#if showUpToDate}
      {@render rowList(upToDate)}
    {/if}
  {/if}
{/snippet}

<main class="app-root">
  {#if !state}
    {#if loadError}
      <div class="empty-center" role="alert">
        <h1 class="headline-title">RepoBell can't start</h1>
        <p class="headline-desc">{loadError}</p>
        <button class="btn-primary" onclick={loadState}>Try again</button>
      </div>
    {:else}
      <div class="loading-box" role="status">Loading RepoBell…</div>
    {/if}
  {:else if wizardActive}
    <WelcomeView onFinish={finishWizard} />
  {:else if currentView === "settings"}
    <SettingsView {state} onBack={closeSettings} onStateRefresh={loadState} />
  {:else}
    <Header title="RepoBell" isChecking={isChecking} onCheckNow={handleCheckNow} onOpenSettings={() => (currentView = "settings")} />
    {#if state.progress}<CheckStrip progress={state.progress} />{/if}
    <p class="check-status visually-hidden" role="status">{checkStatus}</p>

    <div class="main-body">
      {#if notice}
        <div class="banner" role="alert">
          <p>{notice}</p>
          <button class="banner-close" onclick={() => (notice = null)}>Dismiss</button>
        </div>
      {/if}

      {#if state.offline}
        <div class="empty-center" role="alert">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 8a6 6 0 0 0-9.3-5"></path>
            <path d="M6 8c0 7-3 9-3 9h14"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
            <path d="M3 3l18 18"></path>
          </svg>
          <h1 class="headline-title">Can't check your repos</h1>
          <p class="headline-desc narrow">
            RepoBell can't reach the server. Check your internet connection.
            {#if state.last_checked}Your repos were last checked {timeAgo(state.last_checked, now)}.{/if}
          </p>
          <button class="btn-outline" disabled={isChecking} onclick={handleCheckNow}>
            {isChecking ? "Checking…" : "Try again"}
          </button>
        </div>
      {:else if firstCheck}
        <div class="empty-center" role="status">
          <span class="spinner" aria-hidden="true"></span>
          <h1 class="headline-title">Checking your repos…</h1>
          <p class="headline-desc">This can take a moment the first time.</p>
        </div>
      {:else if newsRepos.length > 0 || errorTotal > 0 || !allPlain}
        <div class="headline-row">
          <!-- Same icons as the tray: a bell with news, a crossed bell when there is only trouble. -->
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#1B1B1F" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" data-icon={quiet || newsRepos.length > 0 ? "bell" : "bell-off"}>
            {#if quiet || newsRepos.length > 0}
              <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
              <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
            {:else}
              <path d="M18 8a6 6 0 0 0-9.3-5"></path>
              <path d="M6 8c0 7-3 9-3 9h14"></path>
              <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
              <path d="M3 3l18 18"></path>
            {/if}
          </svg>
          <div>
            <h1 class="headline-title">{headlineText}</h1>
            <p class="headline-sub">{headlineSub}</p>
          </div>
        </div>

        {@render group("Can't check", errorRepos, { folders: folderErrors })}
        {@render group("News", newsRepos, { markSeen: true })}
      {:else if watched.length === 0}
        <div class="empty-center">
          {#if state.repos.length > 0}
            <p class="headline-desc">You're not watching any repos. Turn one on in Settings.</p>
            <button class="btn-primary" onclick={() => (currentView = "settings")}>Open Settings</button>
          {:else}
            <p class="headline-desc">No repos found yet. Add a folder to start watching.</p>
            <button class="btn-primary" onclick={handleAddFolder}>Add folder</button>
          {/if}
        </div>
      {:else}
        <div class="empty-center">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
          </svg>
          <h1 class="headline-title">All quiet</h1>
          <p class="headline-desc">
            Your {watched.length} {plural(watched.length, "repo")} {watched.length === 1 ? "is" : "are"} up to date.
            {#if lastChecked}<br />{lastChecked}.{/if}
          </p>
        </div>
      {/if}
      {#if !state.offline && !firstCheck}{@render quietGroups()}{/if}
    </div>

    <Footer onAddFolder={handleAddFolder} />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: "Ubuntu", system-ui, -apple-system, sans-serif;
    color: #1b1b1f;
    background: #f6f6f4;
    user-select: none;
    overflow-x: hidden;
  }
  :global(:focus-visible) {
    outline: 2px solid #1f5fd1;
    outline-offset: 2px;
  }
  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;
    box-sizing: border-box;
    border: 1px solid #d5d5d0;
    border-radius: 12px;
    background: #f6f6f4;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.15);
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .loading-box {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100vh;
    color: #5b5b66;
    font-size: 14px;
  }
  .main-body {
    flex-grow: 1;
    overflow-y: auto;
    overflow-x: hidden;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border: 1px solid #e8b9b4;
    border-radius: 8px;
    background: #fbe7e5;
    color: #6f1d16;
    font-size: 14px;
    line-height: 1.4;
    flex-shrink: 0;
  }
  .banner p {
    margin: 0;
    flex: 1;
  }
  .banner-close {
    min-height: 36px;
    padding: 0 10px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: inherit;
    font: inherit;
    font-size: 13px;
    text-decoration: underline;
    cursor: pointer;
  }
  .headline-row {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-shrink: 0;
  }
  .headline-title {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }
  .headline-sub {
    margin: 2px 0 0;
    font-size: 13px;
    color: #5b5b66;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }
  .group-head {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 20px;
    margin-top: 4px;
  }
  .group-label {
    margin: 0;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #3d3d45;
  }
  .count {
    margin-left: 4px;
    font-weight: 500;
    color: #5b5b66;
  }
  .group-act {
    height: 32px;
    margin: -6px -8px -6px auto;
    padding: 0 8px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: #1f5fd1;
    font: inherit;
    font-size: 13px;
    cursor: pointer;
  }
  .group-act:hover {
    background: #f0f5fd;
  }
  .group-hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .group-list {
    margin: 0;
    padding: 0;
    list-style: none;
    display: flex;
    flex-direction: column;
    flex-shrink: 0;
    overflow: hidden;
    background: #ffffff;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
  }
  .empty-center {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 24px;
    text-align: center;
  }
  .headline-desc {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .headline-desc.narrow {
    max-width: 340px;
  }
  .accordion-btn {
    height: 44px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 14px;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    background: #ffffff;
    color: #3d3d45;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .accordion-btn:hover {
    background: #fbfbf9;
  }
  .accordion-label {
    flex-grow: 1;
  }
  .accordion-count {
    font-size: 13px;
    color: #5b5b66;
  }
  .chevron {
    transition: transform 0.15s ease;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .btn-primary,
  .btn-outline {
    height: 40px;
    padding: 0 16px;
    border-radius: 8px;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-primary {
    border: none;
    background: #1f5fd1;
    color: #ffffff;
  }
  .btn-primary:hover {
    background: #174aa6;
  }
  .btn-outline {
    margin-top: 6px;
    border: 1px solid #1f5fd1;
    background: #ffffff;
    color: #1f5fd1;
  }
  .btn-outline:hover:not(:disabled) {
    background: #f0f5fd;
  }
  .btn-outline:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .spinner {
    width: 28px;
    height: 28px;
    border: 3px solid #d9d9d5;
    border-top-color: #1f5fd1;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 3s;
    }
    .chevron {
      transition: none;
    }
  }
</style>
