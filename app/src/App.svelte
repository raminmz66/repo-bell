<script lang="ts">
  import { onMount, tick as nextTick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Header from "./lib/Header.svelte";
  import Footer from "./lib/Footer.svelte";
  import RepoCard from "./lib/RepoCard.svelte";
  import FolderErrorCard from "./lib/FolderErrorCard.svelte";
  import SettingsView from "./lib/SettingsView.svelte";
  import WelcomeView from "./lib/WelcomeView.svelte";
  import type { AppStateView, RepoCardView } from "./lib/types";
  import { errText, plural, timeAgo } from "./lib/util";

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
  }

  async function removeFolder(path: string) {
    state = await invoke<AppStateView>("remove_folder", { path });
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

    const unlistenFocus = listen<string>("focus-repo", (event) => {
      const path = event.payload;
      currentView = "main";
      if (upToDate.some((r) => r.path === path)) showUpToDate = true;
      highlightPath = path;
      setTimeout(() => {
        if (highlightPath === path) highlightPath = null;
      }, 2200);
    });

    return () => {
      clearInterval(tick);
      unlistenState.then((f) => f());
      unlistenFocus.then((f) => f());
    };
  });

  // Repos that are up to date and need nothing from the user stay tucked away.
  const isPlain = (r: RepoCardView) => r.on_main && r.behind === 0 && !r.can_undo;

  $: if (state?.first_run) wizardActive = true;
  $: watched = state?.repos.filter((r) => !r.ignored) ?? [];
  $: errorRepos = watched.filter((r) => r.error);
  $: newsRepos = watched.filter((r) => !r.error && r.new_count > 0);
  $: quietRepos = watched.filter((r) => !r.error && r.new_count === 0);
  $: quietVisible = quietRepos.filter((r) => !isPlain(r));
  $: upToDate = quietRepos.filter(isPlain);
  $: folderErrors = state?.folder_errors ?? [];
  $: errorTotal = errorRepos.length + folderErrors.length;
  $: firstCheck = !!state && state.checking && state.last_checked === null;
  $: isChecking = localChecking || !!state?.checking;
  $: lastChecked = !state
    ? ""
    : state.checking
      ? "Checking…"
      : state.last_checked
        ? `Last checked ${timeAgo(state.last_checked, now)}`
        : "";
  $: subline = [newsRepos.length > 0 ? "Since you last looked" : "", lastChecked].filter(Boolean).join(" · ");
  $: errorTitle = [
    errorRepos.length > 0 ? `${errorRepos.length} ${plural(errorRepos.length, "repo")} can't be checked` : "",
    folderErrors.length > 0 ? `${folderErrors.length} ${plural(folderErrors.length, "folder")} not found` : "",
  ]
    .filter(Boolean)
    .join(", ");
  $: headline =
    newsRepos.length > 0 || errorRepos.length > 0 ? (state?.headline ?? "") : errorTitle;
  $: allPlain = quietVisible.length === 0 && errorTotal === 0;
</script>

{#snippet quietList()}
  {#if quietVisible.length > 0}
    <div class="cards-list tight">
      {#each quietVisible as r (r.path)}
        <RepoCard repo={r} highlight={highlightPath === r.path} onStateRefresh={loadState} />
      {/each}
    </div>
  {/if}
  {#if upToDate.length > 0}
    <button class="accordion-btn" onclick={() => (showUpToDate = !showUpToDate)} aria-expanded={showUpToDate}>
      <svg class="chevron" class:open={showUpToDate} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M9 6l6 6-6 6"></path>
      </svg>
      <span class="accordion-label">Up to date</span>
      <span class="accordion-count">{upToDate.length} {plural(upToDate.length, "repo")}</span>
    </button>
    {#if showUpToDate}
      <div class="cards-list tight">
        {#each upToDate as r (r.path)}
          <RepoCard repo={r} highlight={highlightPath === r.path} onStateRefresh={loadState} />
        {/each}
      </div>
    {/if}
  {/if}
{/snippet}

{#snippet errorList()}
  <div class="cards-list">
    {#each errorRepos as r (r.path)}
      <RepoCard repo={r} checking={isChecking} highlight={highlightPath === r.path} onTryAgain={handleCheckNow} onStopWatching={stopWatching} onStateRefresh={loadState} />
    {/each}
    {#each folderErrors as f (f.path)}
      <FolderErrorCard folder={f} onRemove={removeFolder} />
    {/each}
  </div>
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
      {:else if newsRepos.length > 0 || errorTotal > 0}
        <div class="headline-row">
          <!-- Same icons as the tray: a bell with news, a crossed bell when there is only trouble. -->
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#1B1B1F" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true" data-icon={newsRepos.length > 0 ? "bell" : "bell-off"}>
            {#if newsRepos.length > 0}
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
            <h1 class="headline-title">{headline}</h1>
            <p class="headline-sub">
              {subline}
            </p>
          </div>
        </div>

        {#if newsRepos.length > 0}
          <div class="cards-list">
            {#each newsRepos as r (r.path)}
              <RepoCard repo={r} highlight={highlightPath === r.path} onStateRefresh={loadState} />
            {/each}
          </div>
        {/if}

        {#if errorTotal > 0}
          {#if newsRepos.length > 0}
            <h2 class="section-title">{errorTitle}</h2>
          {/if}
          {@render errorList()}
        {/if}

        {@render quietList()}
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
            {allPlain ? `Your ${watched.length} ${plural(watched.length, "repo")} ${watched.length === 1 ? "is" : "are"} up to date.` : `Nothing new in your ${watched.length} ${plural(watched.length, "repo")}.`}
            {#if lastChecked}<br />{lastChecked}.{/if}
          </p>
        </div>
        {@render quietList()}
      {/if}
    </div>

    <Footer hasNews={newsRepos.length > 0} onAddFolder={handleAddFolder} onMarkAllSeen={handleMarkAllSeen} />
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
  .section-title {
    margin: 6px 0 0;
    font-size: 15px;
    font-weight: 700;
    flex-shrink: 0;
  }
  .cards-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
    flex-shrink: 0;
  }
  .cards-list.tight {
    gap: 8px;
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
