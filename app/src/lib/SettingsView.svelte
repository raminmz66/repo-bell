<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Header from "./Header.svelte";
  import type { AppStateView, Settings, RepoCardView } from "./types";

  export let state: AppStateView;
  export let onBack: () => void;
  export let onStateRefresh: () => void;

  let localSettings: Settings = JSON.parse(JSON.stringify(state.settings));
  let prevSettingsJson = JSON.stringify(state.settings);
  let isSaving = false;
  let hasPendingSave = false;

  $: {
    const currentJson = JSON.stringify(state.settings);
    if (!isSaving && !hasPendingSave && currentJson !== prevSettingsJson) {
      prevSettingsJson = currentJson;
      localSettings = JSON.parse(currentJson);
    }
  }

  let availableEditors: string[] = ["System default app"];

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
    if (isSaving) {
      hasPendingSave = true;
      return;
    }
    isSaving = true;
    try {
      do {
        hasPendingSave = false;
        const payload = JSON.parse(JSON.stringify(localSettings));
        const updated = await invoke<AppStateView>("save_settings", { settings: payload });
        if (updated?.settings) {
          prevSettingsJson = JSON.stringify(updated.settings);
        }
      } while (hasPendingSave);
      onStateRefresh();
    } catch (e) {
      alert(`Could not save settings: ${e}`);
    } finally {
      isSaving = false;
    }
  }

  async function handleAddFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && !localSettings.folders.includes(picked)) {
        localSettings = {
          ...localSettings,
          folders: [...localSettings.folders, picked],
        };
        await persistSettings();
      }
    } catch (e) {
      alert(`Could not add folder: ${e}`);
    }
  }

  async function removeFolder(folder: string) {
    localSettings = {
      ...localSettings,
      folders: localSettings.folders.filter((f) => f !== folder),
    };
    await persistSettings();
  }

  async function toggleWatch(repoPath: string) {
    const isIgnored = localSettings.ignored.includes(repoPath);
    localSettings = {
      ...localSettings,
      ignored: isIgnored
        ? localSettings.ignored.filter((p) => p !== repoPath)
        : [...localSettings.ignored, repoPath],
    };
    await persistSettings();
  }

  async function toggleMute(repoPath: string) {
    const isMuted = localSettings.muted.includes(repoPath);
    localSettings = {
      ...localSettings,
      muted: isMuted
        ? localSettings.muted.filter((p) => p !== repoPath)
        : [...localSettings.muted, repoPath],
    };
    await persistSettings();
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
          Turn off a switch to stop watching that repo. Click the bell to mute desktop alerts.
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
              {@const isWatched = !localSettings.ignored.includes(r.path)}
              {@const isMuted = localSettings.muted.includes(r.path)}
              <li class="repo-item" class:repo-ignored={!isWatched}>
                <div class="repo-info">
                  <span class="repo-name">{r.name}</span>
                  {#if !isWatched}
                    <span class="status-badge status-stopped">Stopped (Ignored)</span>
                  {:else if isMuted}
                    <span class="status-badge status-muted">Watched · Alerts muted</span>
                  {:else}
                    <span class="status-badge status-active">Watched · Alerts on</span>
                  {/if}
                </div>

                <div class="repo-actions">
                  {#if isWatched}
                    <button
                      type="button"
                      class="bell-btn"
                      class:muted={isMuted}
                      title={isMuted ? "Desktop alerts muted. Click to turn on alerts." : "Desktop alerts on. Click to mute alerts."}
                      onclick={() => toggleMute(r.path)}
                      aria-label={`Toggle alerts for ${r.name}`}
                    >
                      {#if isMuted}
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                          <path d="M13.73 21a2 2 0 0 1-3.46 0"></path>
                          <path d="M18.63 13A17.89 17.89 0 0 1 18 8"></path>
                          <path d="M6.26 6.26A5.86 5.86 0 0 0 6 8c0 7-3 9-3 9h14"></path>
                          <path d="M1 1l22 22"></path>
                        </svg>
                        <span>Muted</span>
                      {:else}
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9"></path>
                          <path d="M13.73 21a2 2 0 0 1-3.46 0"></path>
                        </svg>
                        <span>Alerts</span>
                      {/if}
                    </button>
                  {/if}

                  <input
                    type="checkbox"
                    role="switch"
                    class="switch"
                    checked={isWatched}
                    title={isWatched ? "Watching (Click to stop watching)" : "Stopped (Click to watch)"}
                    aria-label={`Watch ${r.name}`}
                    onchange={() => toggleWatch(r.path)}
                  />
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
    gap: 12px;
    min-height: 52px;
    padding: 8px 14px;
    border-bottom: 1px solid #efecec;
    position: relative;
    transition: opacity 0.15s ease;
  }
  .repo-item:last-child {
    border-bottom: none;
  }
  .repo-item.repo-ignored {
    opacity: 0.6;
    background: #fafaf8;
  }
  .repo-info {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .repo-name {
    font-size: 14px;
    font-weight: 600;
  }
  .status-badge {
    font-size: 12px;
    font-weight: 500;
  }
  .status-active {
    color: #1e6b34;
  }
  .status-muted {
    color: #55555c;
  }
  .status-stopped {
    color: #55555c;
  }
  .repo-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  .bell-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    height: 30px;
    padding: 0 10px;
    border: 1px solid #dcdcd8;
    border-radius: 6px;
    background: #ffffff;
    color: #2e2e33;
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s ease, border-color 0.15s ease;
  }
  .bell-btn:hover {
    background: #f0f0ee;
  }
  .bell-btn.muted {
    background: #f6f6f4;
    color: #66666e;
    border-color: #e0e0dc;
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
