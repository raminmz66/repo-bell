<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Header from "./lib/Header.svelte";
  import Footer from "./lib/Footer.svelte";
  import RepoCard from "./lib/RepoCard.svelte";
  import SettingsView from "./lib/SettingsView.svelte";
  import WelcomeView from "./lib/WelcomeView.svelte";
  import type { AppStateView, RepoCardView } from "./lib/types";

  let state: AppStateView | null = null;
  let currentView: "main" | "settings" = "main";
  let isChecking = false;
  let showUpToDate = false;

  async function loadState() {
    try {
      state = await invoke<AppStateView>("get_app_state");
    } catch (e) {
      console.error("Failed to load app state", e);
    }
  }

  async function handleCheckNow() {
    isChecking = true;
    try {
      state = await invoke<AppStateView>("check_now");
    } catch (e) {
      console.error("Check now failed", e);
    } finally {
      isChecking = false;
    }
  }

  async function handleMarkAllSeen() {
    try {
      state = await invoke<AppStateView>("mark_all_seen");
    } catch (e) {
      console.error("Mark all seen failed", e);
    }
  }

  async function handleAddFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && state) {
        if (!state.settings.folders.includes(picked)) {
          state.settings.folders.push(picked);
          state = await invoke<AppStateView>("save_settings", { settings: state.settings });
        }
      }
    } catch (e) {
      alert(`Could not add folder: ${e}`);
    }
  }

  onMount(() => {
    loadState();

    const unlistenState = listen<AppStateView>("state-updated", (event) => {
      state = event.payload;
    });

    const unlistenFocus = listen<string>("focus-repo", (event) => {
      currentView = "main";
    });

    return () => {
      unlistenState.then((f) => f());
      unlistenFocus.then((f) => f());
    };
  });

  $: newsRepos = state?.repos.filter((r) => r.behind > 0 || r.unsaved || r.error) ?? [];
  $: upToDateRepos = state?.repos.filter((r) => r.behind === 0 && !r.unsaved && !r.error) ?? [];
</script>

<main class="app-root">
  {#if !state}
    <div class="loading-box">Loading RepoBell...</div>
  {:else if state.first_run}
    <WelcomeView onFinish={loadState} />
  {:else if currentView === "settings"}
    <SettingsView
      {state}
      onBack={() => (currentView = "main")}
      onStateRefresh={loadState}
    />
  {:else}
    <!-- Main Window -->
    <Header
      title="RepoBell"
      {isChecking}
      onCheckNow={handleCheckNow}
      onOpenSettings={() => (currentView = "settings")}
    />

    <div class="main-body">
      {#if state.offline}
        <!-- Offline State -->
        <div class="empty-center" role="alert">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 8a6 6 0 0 0-9.3-5"></path>
            <path d="M6 8c0 7-3 9-3 9h14"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
            <path d="M3 3l18 18"></path>
          </svg>
          <h1 class="headline-title">Can't check your repos</h1>
          <p class="headline-desc">RepoBell can't reach the server. Check your internet connection.</p>
          <button class="btn-primary" onclick={handleCheckNow}>Try again</button>
        </div>
      {:else if newsRepos.length > 0}
        <!-- News Available -->
        <div class="headline-row">
          <svg width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="#1B1B1F" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
          </svg>
          <div>
            <h1 class="headline-title">
              {newsRepos.length} repo{newsRepos.length === 1 ? "" : "s"} {newsRepos.length === 1 ? "has" : "have"} news
            </h1>
            <p class="headline-sub">Since you last looked</p>
          </div>
        </div>

        <div class="cards-list">
          {#each newsRepos as r (r.path)}
            <RepoCard repo={r} onStateRefresh={loadState} />
          {/each}
        </div>

        {#if upToDateRepos.length > 0}
          <button
            class="accordion-btn"
            onclick={() => (showUpToDate = !showUpToDate)}
            aria-expanded={showUpToDate}
          >
            <svg class="chevron" class:open={showUpToDate} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M9 6l6 6-6 6"></path>
            </svg>
            <span class="accordion-label">Up to date</span>
            <span class="accordion-count">{upToDateRepos.length} repo{upToDateRepos.length === 1 ? "" : "s"}</span>
          </button>

          {#if showUpToDate}
            <div class="cards-list">
              {#each upToDateRepos as r (r.path)}
                <RepoCard repo={r} onStateRefresh={loadState} />
              {/each}
            </div>
          {/if}
        {/if}
      {:else if state.repos.length === 0}
        <!-- No repos configured -->
        <div class="empty-center">
          <p class="headline-desc">No repos yet. Add a folder to start watching.</p>
          <button class="btn-primary" onclick={handleAddFolder}>Add folder</button>
        </div>
      {:else}
        <!-- All Quiet State -->
        <div class="empty-center">
          <svg width="40" height="40" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
          </svg>
          <h1 class="headline-title">All quiet</h1>
          <p class="headline-desc">Your {state.repos.length} repos are up to date.</p>
        </div>

        <button
          class="accordion-btn"
          onclick={() => (showUpToDate = !showUpToDate)}
          aria-expanded={showUpToDate}
        >
          <svg class="chevron" class:open={showUpToDate} width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M9 6l6 6-6 6"></path>
          </svg>
          <span class="accordion-label">Up to date</span>
          <span class="accordion-count">{state.repos.length} repo{state.repos.length === 1 ? "" : "s"}</span>
        </button>

        {#if showUpToDate}
          <div class="cards-list">
            {#each state.repos as r (r.path)}
              <RepoCard repo={r} onStateRefresh={loadState} />
            {/each}
          </div>
        {/if}
      {/if}
    </div>

    <Footer
      hasNews={newsRepos.length > 0}
      onAddFolder={handleAddFolder}
      onMarkAllSeen={handleMarkAllSeen}
    />
  {/if}
</main>

<style>
  :global(body) {
    margin: 0;
    font-family: "Ubuntu", system-ui, -apple-system, sans-serif;
    color: #1b1b1f;
    background: #f6f6f4;
    user-select: none;
  }
  .app-root {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;
    box-sizing: border-box;
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
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .headline-row {
    display: flex;
    align-items: center;
    gap: 10px;
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
  .cards-list {
    display: flex;
    flex-direction: column;
    gap: 12px;
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
  .btn-primary {
    height: 40px;
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
</style>
