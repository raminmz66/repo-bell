<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { SuggestedFolder, Settings } from "./types";
  import { errText, plural } from "./util";

  export let onFinish: () => void;

  const SAVED_BUT = "Your settings were saved, but";

  let step = 1;
  let scanning = true;
  let scanError: string | null = null;
  let suggested: SuggestedFolder[] = [];
  // Folders the user adds by hand, each scanned as soon as it is added.
  let addedFolders: SuggestedFolder[] = [];
  let addingFolder = false;
  // "enabled" | "disabled" | "unknown": can the bell show in the top bar?
  let tray = "unknown";
  let selectedRepos: Set<string> = new Set();
  let startAtLogin = true;
  let isSaving = false;
  let saveError: string | null = null;
  let savedWithWarning = false;

  onMount(async () => {
    try {
      suggested = await invoke<SuggestedFolder[]>("discover_suggested_folders");
      for (const group of suggested) {
        for (const repo of group.repos) selectedRepos.add(repo);
      }
      selectedRepos = selectedRepos;
    } catch (e) {
      scanError = errText(e);
    } finally {
      scanning = false;
    }
  });

  $: groups = [...suggested, ...addedFolders];
  $: found = groups.reduce((n, g) => n + g.repos.length, 0);
  $: canWatch = selectedRepos.size > 0;

  // Repo name as shown in the list: its path below the folder (nested repos keep their parent).
  const label = (g: SuggestedFolder, repo: string) =>
    repo.startsWith(g.path + "/") ? repo.slice(g.path.length + 1) : repo;

  async function checkTray() {
    try {
      tray = await invoke<string>("check_tray");
    } catch {
      tray = "unknown";
    }
  }

  function toggleRepo(fullPath: string) {
    if (selectedRepos.has(fullPath)) selectedRepos.delete(fullPath);
    else selectedRepos.add(fullPath);
    selectedRepos = selectedRepos;
  }

  async function handleAddFolder() {
    saveError = null;
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && !groups.some((g) => g.path === picked)) {
        addingFolder = true;
        const group = await invoke<SuggestedFolder>("scan_folder", { path: picked });
        addedFolders = [...addedFolders, group];
        for (const repo of group.repos) selectedRepos.add(repo);
        selectedRepos = selectedRepos;
      }
    } catch (e) {
      saveError = errText(e);
    } finally {
      addingFolder = false;
    }
  }

  async function finishWizard() {
    if (savedWithWarning) {
      onFinish();
      return;
    }
    isSaving = true;
    saveError = null;

    // A folder whose repos were all unticked isn't watched at all.
    const kept = groups.filter((g) => g.repos.some((r) => selectedRepos.has(r)));
    const ignored = kept.flatMap((g) => g.repos.filter((r) => !selectedRepos.has(r)));

    const settings: Settings = {
      folders: kept.map((g) => g.path),
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
      const msg = errText(e);
      if (msg.startsWith(SAVED_BUT)) {
        // The folders are saved; only "start at login" failed. Say so, then let the user go on.
        saveError = msg;
        savedWithWarning = true;
      } else {
        saveError = msg;
      }
    } finally {
      isSaving = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === "Escape" && step > 1 && !isSaving && !savedWithWarning) step -= 1;
  }
</script>

<svelte:window onkeydown={onKey} />

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
      {#if scanning}
        <div class="scan-state" role="status">
          <span class="spinner" aria-hidden="true"></span>
          <h1 class="welcome-h1-left">Looking for your repos…</h1>
          <p class="welcome-desc-left">This takes a few seconds.</p>
        </div>
      {:else}
        <div>
          {#if found > 0}
            <h1 class="welcome-h1-left">
              Found {found} {plural(found, "repo")} in {groups.length} {plural(groups.length, "folder")}
            </h1>
            <p class="welcome-desc-left">
              RepoBell looked in your usual project folders. Untick any you don't want to watch, or add another folder. You can change this later in Settings.
            </p>
          {:else}
            <h1 class="welcome-h1-left">No repos found yet</h1>
            <p class="welcome-desc-left">
              {scanError ?? "RepoBell looked in your usual project folders and found none."} Add the folder where you keep your repos.
            </p>
          {/if}
        </div>

        {#if groups.length > 0}
          <fieldset class="folder-selection-box">
            <legend class="visually-hidden">Repos to watch</legend>
            {#each groups as group}
              <div class="group-header">{group.display}</div>
              {#if group.repos.length === 0}
                <p class="added-note">No repos in this folder.</p>
              {/if}
              {#each group.repos as repo}
                <label class="repo-checkbox-label">
                  <input type="checkbox" class="checkbox" checked={selectedRepos.has(repo)} onchange={() => toggleRepo(repo)} />
                  {label(group, repo)}
                </label>
              {/each}
            {/each}
          </fieldset>
        {/if}

        {#if saveError && step === 2}
          <p class="msg-error" role="alert">{saveError}</p>
        {/if}

        <button class="btn-text-icon" disabled={addingFolder} onclick={handleAddFolder}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            <path d="M12 5v14M5 12h14"></path>
          </svg>
          {groups.length > 0 ? "Add another folder" : "Add a folder"}
        </button>
      {/if}
    </div>
    <footer class="welcome-footer">
      <div class="footer-left">
        <button class="btn-back" onclick={() => (step = 1)}>Back</button>
        <span class="step-label">Step 2 of 3</span>
      </div>
      <button class="btn-primary" disabled={scanning || addingFolder || !canWatch} onclick={() => { step = 3; checkTray(); }}>
        {#if selectedRepos.size > 0}
          Watch {selectedRepos.size} {plural(selectedRepos.size, "repo")}
        {:else}
          Watch repos
        {/if}
      </button>
    </footer>
  {:else if step === 3}
    <div class="step-content text-center">
      <div class="tray-demo-bar">
        <span class="tray-demo-bell">
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#F2F2F2" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" role="img" aria-label="RepoBell bell">
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

      {#if tray === "disabled"}
        <h1 class="welcome-h1">The bell can't show in your top bar</h1>
        <p class="welcome-desc">
          Your desktop needs the "Ubuntu AppIndicators" extension. Turn it on in the Extensions app, then press Check again.
        </p>
        <p class="welcome-desc">RepoBell still works without it. Open it from the app menu, and notifications still arrive.</p>
        <button class="btn-text-icon" onclick={checkTray}>Check again</button>
      {:else}
        <h1 class="welcome-h1">The bell is in your top bar</h1>
        <p class="welcome-desc">
          Click it to see what's new. It gets a dot when there is news. This window opens from "Open RepoBell" in that menu.
        </p>

        <details class="bell-help">
          <summary>Can't see the bell?</summary>
          <p>
            Your desktop needs the "Ubuntu AppIndicators" extension. Turn it on in the Extensions app, then sign out and back in.
          </p>
          <p>RepoBell still works without it. Open it from the app menu, and notifications still arrive.</p>
        </details>
      {/if}

      <label class="autostart-toggle-label">
        <input type="checkbox" class="checkbox" bind:checked={startAtLogin} disabled={savedWithWarning} />
        Start RepoBell when I log in
      </label>

      {#if saveError}
        <p class="msg-error" role="alert">{saveError}</p>
      {/if}
    </div>
    <footer class="welcome-footer">
      <div class="footer-left">
        {#if !savedWithWarning}<button class="btn-back" disabled={isSaving} onclick={() => (step = 2)}>Back</button>{/if}
        <span class="step-label">Step 3 of 3</span>
      </div>
      <button class="btn-primary" disabled={isSaving} onclick={finishWizard}>
        {isSaving ? "Saving…" : savedWithWarning ? "Continue" : tray === "disabled" ? "Continue without the bell" : "Done"}
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
  .scan-state {
    flex-grow: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    text-align: center;
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
  .footer-left {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .step-label {
    font-size: 13px;
    color: #5b5b66;
  }
  .btn-back {
    height: 40px;
    padding: 0 12px;
    border: none;
    border-radius: 8px;
    background: transparent;
    color: #3d3d45;
    font-size: 14px;
    cursor: pointer;
  }
  .btn-back:hover:not(:disabled) {
    background: #f3f3f1;
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
  .btn-primary:hover:not(:disabled) {
    background: #174aa6;
  }
  .btn-primary:disabled {
    background: #8aa7de;
    cursor: default;
  }
  .folder-selection-box {
    margin: 0;
    padding: 0;
    min-width: 0;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
    background: #ffffff;
    min-height: 132px;
    overflow-y: auto;
    overflow-x: hidden;
  }
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
  }
  .group-header {
    padding: 10px 14px 6px;
    font-size: 12px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
    color: #5b5b66;
    background: #fafaf8;
    border-top: 1px solid #efecec;
  }
  .group-header:first-of-type {
    border-top: none;
  }
  .added-note {
    margin: 0;
    padding: 10px 14px;
    font-size: 13px;
    color: #5b5b66;
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
  .btn-text-icon:hover {
    background: #eeeeec;
  }
  .msg-error {
    margin: 0;
    font-size: 13px;
    line-height: 1.4;
    color: #a12a20;
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
  .bell-help {
    max-width: 340px;
    font-size: 13px;
    line-height: 1.4;
    color: #3d3d45;
    text-align: left;
  }
  .bell-help summary {
    cursor: pointer;
    color: #1f5fd1;
    padding: 6px 0;
  }
  .bell-help p {
    margin: 4px 0 0;
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
  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation-duration: 3s;
    }
  }
</style>
