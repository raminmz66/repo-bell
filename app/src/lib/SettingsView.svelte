<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Header from "./Header.svelte";
  import type { AppStateView, Settings, RepoCardView } from "./types";

  export let state: AppStateView;
  export let onBack: () => void;
  export let onStateRefresh: () => void;

  let localSettings: Settings = JSON.parse(JSON.stringify(state.settings));
  let availableEditors: string[] = ["System default app"];
  let openMenuRepo: string | null = null;

  onMount(async () => {
    try {
      const eds = await invoke<string[]>("detect_editors");
      if (eds && eds.length > 0) {
        availableEditors = eds;
      }
    } catch {
      // Fallback
    }
  });

  async function persistSettings() {
    try {
      await invoke("save_settings", { settings: localSettings });
      onStateRefresh();
    } catch (e) {
      alert(`Could not save settings: ${e}`);
    }
  }

  async function handleAddFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && !localSettings.folders.includes(picked)) {
        localSettings.folders.push(picked);
        await persistSettings();
      }
    } catch (e) {
      alert(`Could not add folder: ${e}`);
    }
  }

  async function removeFolder(folder: string) {
    localSettings.folders = localSettings.folders.filter((f) => f !== folder);
    await persistSettings();
  }

  async function toggleMute(repoPath: string) {
    if (localSettings.muted.includes(repoPath)) {
      localSettings.muted = localSettings.muted.filter((p) => p !== repoPath);
    } else {
      localSettings.muted.push(repoPath);
    }
    await persistSettings();
  }

  async function stopWatching(repoPath: string) {
    openMenuRepo = null;
    if (!localSettings.ignored.includes(repoPath)) {
      localSettings.ignored.push(repoPath);
      await persistSettings();
    }
  }

  function reposInFolder(folder: string): RepoCardView[] {
    return state.repos.filter((r) => r.folder === folder || r.path.startsWith(folder));
  }
</script>

<div class="settings-container">
  <Header title="Settings" showBack={true} {onBack} />

  <div class="settings-body">
    <!-- Folders and repos -->
    <section class="section">
      <div>
        <h2 class="section-title">Folders and repos</h2>
        <p class="section-desc">
          RepoBell watches the repos inside these folders. A repo with notifications off still shows its news in RepoBell.
        </p>
      </div>

      {#each localSettings.folders as folder}
        <div class="folder-group">
          <div class="folder-header">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
            </svg>
            <span class="folder-path">{folder}</span>
            <button class="btn-remove-folder" onclick={() => removeFolder(folder)}>Remove</button>
          </div>

          <ul class="repo-list">
            {#each reposInFolder(folder) as r}
              <li class="repo-item">
                <div class="repo-info">
                  <span class="repo-name">{r.name}</span>
                  {#if localSettings.muted.includes(r.path)}
                    <span class="muted-label">Notifications off</span>
                  {/if}
                </div>

                <input
                  type="checkbox"
                  role="switch"
                  class="switch"
                  checked={!localSettings.muted.includes(r.path)}
                  aria-label={`Notifications for ${r.name}`}
                  onchange={() => toggleMute(r.path)}
                />

                <div class="menu-container">
                  <button
                    aria-label={`More options for ${r.name}`}
                    class="btn-more"
                    onclick={() => (openMenuRepo = openMenuRepo === r.path ? null : r.path)}
                  >
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                      <circle cx="5" cy="12" r="1.8"></circle>
                      <circle cx="12" cy="12" r="1.8"></circle>
                      <circle cx="19" cy="12" r="1.8"></circle>
                    </svg>
                  </button>

                  {#if openMenuRepo === r.path}
                    <ul class="dropdown-menu" role="menu">
                      <li role="none">
                        <button role="menuitem" class="menu-item" onclick={() => stopWatching(r.path)}>
                          Stop watching
                        </button>
                      </li>
                    </ul>
                  {/if}
                </div>
              </li>
            {/each}
          </ul>
        </div>
      {/each}

      <button class="btn-add-folder" onclick={handleAddFolder}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 5v14M5 12h14"></path>
        </svg>
        Add folder
      </button>
    </section>

    <!-- Editor -->
    <section class="section">
      <h2 class="section-title">Editor</h2>
      <label for="editor" class="field-label">Open conflicting files in</label>
      <select id="editor" class="select-input" bind:value={localSettings.editor} onchange={persistSettings}>
        {#each availableEditors as ed}
          <option value={ed}>{ed}</option>
        {/each}
      </select>
      <p class="field-hint">These are the editors found on this computer. If yours is missing, choose System default app.</p>
    </section>

    <!-- Checking -->
    <section class="section">
      <h2 class="section-title">Checking</h2>
      <label for="interval" class="field-label">Check for news</label>
      <select id="interval" class="select-input" bind:value={localSettings.check_minutes} onchange={persistSettings}>
        <option value={1}>Every minute</option>
        <option value={5}>Every 5 minutes</option>
        <option value={15}>Every 15 minutes</option>
        <option value={30}>Every 30 minutes</option>
        <option value={60}>Every hour</option>
      </select>
    </section>

    <!-- Start at login -->
    <section class="section">
      <div class="row-between">
        <div>
          <h2 class="section-title">Start at login</h2>
          <p class="section-desc">Keep RepoBell running in the background when you reboot.</p>
        </div>
        <input
          type="checkbox"
          role="switch"
          class="switch"
          bind:checked={localSettings.start_at_login}
          onchange={persistSettings}
        />
      </div>
    </section>
  </div>
</div>

<style>
  .settings-container {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;
    background: #f6f6f4;
  }
  .settings-body {
    flex-grow: 1;
    overflow-y: auto;
    padding: 18px 16px;
    display: flex;
    flex-direction: column;
    gap: 22px;
  }
  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .section-title {
    margin: 0;
    font-size: 15px;
    font-weight: 700;
  }
  .section-desc {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .folder-group {
    background: #ffffff;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    overflow: hidden;
  }
  .folder-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 14px;
    background: #fafaf8;
    border-bottom: 1px solid #e3e3e0;
  }
  .folder-path {
    flex-grow: 1;
    font-size: 14px;
    font-family: monospace;
  }
  .btn-remove-folder {
    height: 36px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    font-size: 14px;
    cursor: pointer;
  }
  .btn-remove-folder:hover {
    background: #ebebeb;
  }
  .repo-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .repo-item {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 52px;
    padding: 0 6px 0 14px;
    border-bottom: 1px solid #efecec;
    position: relative;
  }
  .repo-item:last-child {
    border-bottom: none;
  }
  .repo-info {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .repo-name {
    font-size: 14px;
  }
  .muted-label {
    font-size: 12px;
    color: #5b5b66;
  }
  .switch {
    appearance: none;
    -webkit-appearance: none;
    margin: 0;
    width: 44px;
    height: 26px;
    border-radius: 13px;
    background: #c9c9c4;
    position: relative;
    cursor: pointer;
    flex-shrink: 0;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.25);
    transition: left 0.15s ease;
  }
  .switch:checked {
    background: #1f5fd1;
  }
  .switch:checked::after {
    left: 21px;
  }
  .menu-container {
    position: relative;
  }
  .btn-more {
    width: 40px;
    height: 40px;
    display: flex;
    align-items: center;
    justify-content: center;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #5b5b66;
    cursor: pointer;
  }
  .btn-more:hover {
    background: #f0f0ee;
  }
  .dropdown-menu {
    position: absolute;
    top: 44px;
    right: 0;
    z-index: 10;
    margin: 0;
    padding: 6px;
    list-style: none;
    width: 160px;
    background: #ffffff;
    border: 1px solid #d9d9d5;
    border-radius: 10px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
  }
  .menu-item {
    width: 100%;
    height: 38px;
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    background: #f3f3f1;
    color: #1b1b1f;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .btn-add-folder {
    align-self: flex-start;
    height: 40px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 14px;
    border: 1px solid #c9c9c4;
    border-radius: 8px;
    background: #ffffff;
    color: #1b1b1f;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .field-label {
    font-size: 13px;
    color: #5b5b66;
  }
  .field-hint {
    margin: 0;
    font-size: 12px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .select-input {
    height: 44px;
    padding: 0 12px;
    border: 1px solid #c9c9c4;
    border-radius: 8px;
    background: #ffffff;
    color: #1b1b1f;
    font-size: 14px;
  }
  .row-between {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
</style>
