<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { SuggestedFolder, Settings } from "./types";

  export let onFinish: () => void;

  let step = 1;
  let suggested: SuggestedFolder[] = [];
  let selectedRepos: Set<string> = new Set();
  let startAtLogin = true;
  let isSaving = false;

  onMount(async () => {
    try {
      suggested = await invoke<SuggestedFolder[]>("discover_suggested_folders");
      for (const group of suggested) {
        for (const repo of group.repos) {
          selectedRepos.add(`${group.path}/${repo}`);
        }
      }
      selectedRepos = selectedRepos;
    } catch {
      // Fallback
    }
  });

  function toggleRepo(fullPath: string) {
    if (selectedRepos.has(fullPath)) {
      selectedRepos.delete(fullPath);
    } else {
      selectedRepos.add(fullPath);
    }
    selectedRepos = selectedRepos;
  }

  async function handleAddFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked) {
        // Add as a suggested group
        suggested.push({
          path: picked,
          display: picked,
          repos: [],
        });
        suggested = suggested;
      }
    } catch (e) {
      alert(`Could not add folder: ${e}`);
    }
  }

  async function finishWizard() {
    isSaving = true;
    const foldersToWatch = Array.from(new Set(suggested.map((s) => s.path)));

    const ignored: string[] = [];
    for (const group of suggested) {
      for (const repo of group.repos) {
        const full = `${group.path}/${repo}`;
        if (!selectedRepos.has(full)) {
          ignored.push(full);
        }
      }
    }

    const settings: Settings = {
      folders: foldersToWatch,
      ignored,
      muted: [],
      editor: null,
      check_minutes: 5,
      start_at_login: startAtLogin,
    };

    try {
      await invoke("save_settings", { settings });
      onFinish();
    } catch (e) {
      alert(`Error saving setup: ${e}`);
    } finally {
      isSaving = false;
    }
  }
</script>

<div class="welcome-box">
  <header class="welcome-header">Welcome to RepoBell</header>

  {#if step === 1}
    <div class="step-content text-center">
      <svg width="56" height="56" viewBox="0 0 24 24" fill="none" stroke="#1F5FD1" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
        <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
      </svg>
      <h1 class="welcome-h1">Know when your team changes a repo</h1>
      <p class="welcome-desc">
        RepoBell checks the repos on this computer and tells you what's new on the main version. It never changes anything unless you press Update.
      </p>
    </div>
    <footer class="welcome-footer">
      <span class="step-label">Step 1 of 3</span>
      <button class="btn-primary" onclick={() => (step = 2)}>Find my repos</button>
    </footer>
  {:else if step === 2}
    <div class="step-content list-content">
      <div>
        <h1 class="welcome-h1-left">
          Found {selectedRepos.size} repo{selectedRepos.size === 1 ? "" : "s"} in {suggested.length} folder{suggested.length === 1 ? "" : "s"}
        </h1>
        <p class="welcome-desc-left">
          RepoBell looked in your usual project folders. Untick any you don't want to watch, or add another folder. You can change this later in Settings.
        </p>
      </div>

      <div class="folder-selection-box">
        {#each suggested as group}
          <div class="group-header">{group.display}</div>
          {#each group.repos as repo}
            {@const fullPath = `${group.path}/${repo}`}
            <label class="repo-checkbox-label">
              <input
                type="checkbox"
                class="checkbox"
                checked={selectedRepos.has(fullPath)}
                onchange={() => toggleRepo(fullPath)}
              />
              {repo}
            </label>
          {/each}
        {/each}
      </div>

      <button class="btn-text-icon" onclick={handleAddFolder}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 5v14M5 12h14"></path>
        </svg>
        Add another folder
      </button>
    </div>
    <footer class="welcome-footer">
      <span class="step-label">Step 2 of 3</span>
      <button class="btn-primary" onclick={() => (step = 3)}>
        Watch {selectedRepos.size} repo{selectedRepos.size === 1 ? "" : "s"}
      </button>
    </footer>
  {:else if step === 3}
    <div class="step-content text-center">
      <div class="tray-demo-bar">
        <span class="tray-demo-bell">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#F2F2F2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-label="RepoBell bell">
            <path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"></path>
            <path d="M13.7 21a2 2 0 0 1-3.4 0"></path>
          </svg>
        </span>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#F2F2F2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M5 12.5a10 10 0 0 1 14 0M8.5 16a5 5 0 0 1 7 0"></path>
        </svg>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#F2F2F2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M11 5L6 9H3v6h3l5 4z"></path>
        </svg>
      </div>

      <h1 class="welcome-h1">The bell is in your top bar</h1>
      <p class="welcome-desc">
        Click it to see what's new. It gets a dot when there is news. This window opens from "Open RepoBell" in that menu.
      </p>

      <label class="autostart-toggle-label">
        <input type="checkbox" class="checkbox" bind:checked={startAtLogin} />
        Start RepoBell when I log in
      </label>
    </div>
    <footer class="welcome-footer">
      <span class="step-label">Step 3 of 3</span>
      <button class="btn-primary" disabled={isSaving} onclick={finishWizard}>
        {isSaving ? "Saving..." : "Done"}
      </button>
    </footer>
  {/if}
</div>

<style>
  .welcome-box {
    display: flex;
    flex-direction: column;
    height: 100vh;
    max-height: 100vh;
    overflow: hidden;
    background: #f6f6f4;
  }
  .welcome-header {
    height: 52px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #ebebe8;
    border-bottom: 1px solid #d9d9d5;
    font-size: 15px;
    font-weight: 700;
  }
  .step-content {
    flex-grow: 1;
    overflow-y: auto;
    padding: 32px 20px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .text-center {
    align-items: center;
    justify-content: center;
    text-align: center;
  }
  .list-content {
    padding: 20px 16px;
    gap: 12px;
  }
  .welcome-h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 700;
  }
  .welcome-h1-left {
    margin: 0;
    font-size: 18px;
    font-weight: 700;
  }
  .welcome-desc {
    margin: 0;
    max-width: 340px;
    font-size: 14px;
    line-height: 1.5;
    color: #3d3d45;
  }
  .welcome-desc-left {
    margin: 4px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .welcome-footer {
    height: 72px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    border-top: 1px solid #e3e3e0;
    background: #ffffff;
  }
  .step-label {
    font-size: 13px;
    color: #5b5b66;
  }
  .btn-primary {
    height: 44px;
    padding: 0 18px;
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
  .folder-selection-box {
    margin: 0;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    background: #ffffff;
    overflow: hidden;
  }
  .group-header {
    padding: 10px 14px 6px;
    font-size: 12px;
    font-family: monospace;
    color: #5b5b66;
    background: #fafaf8;
    border-top: 1px solid #efecec;
  }
  .group-header:first-child {
    border-top: none;
  }
  .repo-checkbox-label {
    min-height: 44px;
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 0 14px;
    font-size: 14px;
    cursor: pointer;
    border-bottom: 1px solid #f2f2f0;
  }
  .repo-checkbox-label:last-child {
    border-bottom: none;
  }
  .checkbox {
    width: 18px;
    height: 18px;
    margin: 0;
    accent-color: #1f5fd1;
  }
  .btn-text-icon {
    align-self: flex-start;
    height: 40px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    font-size: 14px;
    cursor: pointer;
  }
  .tray-demo-bar {
    width: 280px;
    height: 36px;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    padding: 0 12px;
    border-radius: 8px;
    background: #1d1d1d;
  }
  .tray-demo-bell {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 26px;
    border-radius: 6px;
    outline: 2px solid #8db4f5;
    outline-offset: 2px;
  }
  .autostart-toggle-label {
    min-height: 44px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 14px;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    background: #ffffff;
    font-size: 14px;
    cursor: pointer;
  }
</style>
