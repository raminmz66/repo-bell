<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Header from "./Header.svelte";
  import AboutSection from "./AboutSection.svelte";
  import type { AppStateView, Settings, RepoCardView } from "./types";
  import { errText, plural, tildePath } from "./util";

  export let state: AppStateView;
  export let onBack: () => void;
  export let onStateRefresh: () => void;
  export let now: number = Date.now();
  export let aboutRequest = 0;

  const SAVED_BUT = "Your settings were saved, but";

  let localSettings: Settings = JSON.parse(JSON.stringify(state.settings));
  let prevSettingsJson = JSON.stringify(state.settings);
  let isSaving = false;
  let hasPendingSave = false;
  let saveError: string | null = null;
  let autostartError: string | null = null;
  let confirmFolder: string | null = null;
  let openMenu: string | null = null;

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
      if (eds && eds.length > 0) availableEditors = eds;
    } catch {
      // The list stays at "System default app", which always works.
    }
  });

  async function persistSettings() {
    if (isSaving) {
      hasPendingSave = true;
      return;
    }
    isSaving = true;
    saveError = null;
    try {
      do {
        hasPendingSave = false;
        const payload = JSON.parse(JSON.stringify(localSettings));
        try {
          const updated = await invoke<AppStateView>("save_settings", { settings: payload });
          autostartError = null;
          if (updated?.settings) prevSettingsJson = JSON.stringify(updated.settings);
        } catch (e) {
          const msg = errText(e);
          if (msg.startsWith(SAVED_BUT)) {
            // Everything was saved except "start at login", so keep the other changes.
            autostartError = msg;
            prevSettingsJson = JSON.stringify(payload);
          } else {
            // Nothing was saved: show the saved values again.
            saveError = msg;
            localSettings = JSON.parse(prevSettingsJson);
            hasPendingSave = false;
          }
        }
      } while (hasPendingSave);
      onStateRefresh();
    } finally {
      isSaving = false;
    }
  }

  async function handleAddFolder() {
    try {
      const picked = await invoke<string | null>("pick_folder");
      if (picked && !localSettings.folders.includes(picked)) {
        localSettings = { ...localSettings, folders: [...localSettings.folders, picked] };
        await persistSettings();
      }
    } catch (e) {
      saveError = errText(e);
    }
  }

  async function removeFolder(folder: string) {
    confirmFolder = null;
    localSettings = { ...localSettings, folders: localSettings.folders.filter((f) => f !== folder) };
    await persistSettings();
  }

  async function setWatched(repoPath: string, watched: boolean) {
    openMenu = null;
    localSettings = {
      ...localSettings,
      ignored: watched
        ? localSettings.ignored.filter((p) => p !== repoPath)
        : [...localSettings.ignored.filter((p) => p !== repoPath), repoPath],
    };
    await persistSettings();
  }

  async function setNotify(repoPath: string, on: boolean) {
    localSettings = {
      ...localSettings,
      muted: on
        ? localSettings.muted.filter((p) => p !== repoPath)
        : [...localSettings.muted.filter((p) => p !== repoPath), repoPath],
    };
    await persistSettings();
  }

  function reposInFolder(folder: string): RepoCardView[] {
    return state.repos.filter((r) => r.folder === folder);
  }

  function missing(folder: string): boolean {
    return state.folder_errors.some((f) => f.path === folder);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== "Escape") return;
    if (openMenu) openMenu = null;
    else if (confirmFolder) confirmFolder = null;
    else onBack();
  }

  function onWindowClick(e: MouseEvent) {
    if (openMenu && !(e.target as HTMLElement).closest?.(".menu-wrap")) openMenu = null;
  }

  $: editorValue = localSettings.editor ?? "System default app";
  $: editorOptions = availableEditors.includes(editorValue) ? availableEditors : [...availableEditors, editorValue];
</script>

<svelte:window onkeydown={onKey} onclick={onWindowClick} />

<div class="settings-container">
  <Header title="Settings" showBack={true} {onBack} />

  <div class="settings-body">
    {#if saveError}
      <div class="banner" role="alert">
        <p>{saveError}</p>
        <button class="banner-close" onclick={() => (saveError = null)}>Dismiss</button>
      </div>
    {/if}

    <section class="section" aria-labelledby="folders-title">
      <div>
        <h2 class="section-title" id="folders-title">Folders and repos</h2>
        <p class="section-desc">
          RepoBell watches the repos inside these folders. A repo with notifications off still shows its news in RepoBell.
        </p>
      </div>

      {#each localSettings.folders as folder (folder)}
        {@const repos = reposInFolder(folder)}
        <div class="folder-group">
          <div class="folder-header">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#5B5B66" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
              <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"></path>
            </svg>
            <span class="folder-path" title={folder}><bdi>{tildePath(folder)}</bdi></span>
            <button class="btn-remove-folder" aria-label={`Remove folder ${folder}`} onclick={() => (confirmFolder = folder)}>Remove</button>
          </div>

          {#if confirmFolder === folder}
            <div class="confirm-row" role="group" aria-label="Confirm removing folder">
              <p>RepoBell will stop watching all repos in this folder. Your files aren't touched.</p>
              <div class="confirm-actions">
                <button class="btn-text" onclick={() => (confirmFolder = null)}>Cancel</button>
                <button class="btn-danger" onclick={() => removeFolder(folder)}>Remove folder</button>
              </div>
            </div>
          {/if}

          <ul class="repo-list">
            {#each repos as r (r.path)}
              {@const isWatched = !localSettings.ignored.includes(r.path)}
              {@const notify = !localSettings.muted.includes(r.path)}
              <li class="repo-item" class:repo-ignored={!isWatched}>
                <span class="repo-info">
                  <span class="repo-name">{r.name}</span>
                  {#if !isWatched}
                    <span class="repo-note">Not watched</span>
                  {:else if !notify}
                    <span class="repo-note">Notifications off</span>
                  {/if}
                </span>

                <input
                  type="checkbox"
                  role="switch"
                  class="switch"
                  checked={notify}
                  disabled={!isWatched}
                  aria-label={`Notifications for ${r.name}`}
                  onchange={() => setNotify(r.path, !notify)}
                />

                <span class="menu-wrap">
                  <button
                    class="menu-btn"
                    class:open={openMenu === r.path}
                    aria-label={`More options for ${r.name}`}
                    aria-haspopup="menu"
                    aria-expanded={openMenu === r.path}
                    onclick={() => (openMenu = openMenu === r.path ? null : r.path)}
                  >
                    <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                      <circle cx="5" cy="12" r="1.8"></circle><circle cx="12" cy="12" r="1.8"></circle><circle cx="19" cy="12" r="1.8"></circle>
                    </svg>
                  </button>
                  {#if openMenu === r.path}
                    <ul class="menu" role="menu" aria-label={`${r.name} options`}>
                      <li role="none">
                        <button class="menu-item" role="menuitem" onclick={() => setWatched(r.path, !isWatched)}>
                          {isWatched ? "Stop watching" : "Watch again"}
                        </button>
                      </li>
                    </ul>
                  {/if}
                </span>
              </li>
            {:else}
              <li class="repo-empty">
                {missing(folder) ? "Folder not found. It may have been moved or deleted." : "No repos found in this folder."}
              </li>
            {/each}
          </ul>
        </div>
      {:else}
        <p class="section-desc">No folders yet. Add one to start watching repos.</p>
      {/each}

      <button class="btn-add-folder" onclick={handleAddFolder}>
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12 5v14M5 12h14"></path>
        </svg>
        Add folder
      </button>
    </section>

    <section class="section" aria-labelledby="editor-title">
      <h2 class="section-title" id="editor-title">Editor</h2>
      <label for="editor" class="field-label">Open files in</label>
      <select
        id="editor"
        class="select-input"
        value={editorValue}
        onchange={(e) => {
          localSettings = { ...localSettings, editor: e.currentTarget.value };
          persistSettings();
        }}
      >
        {#each editorOptions as ed}
          <option value={ed}>{ed}</option>
        {/each}
      </select>
      <p class="field-hint">These are the editors found on this computer. If yours is missing, choose System default app.</p>
    </section>

    <section class="section" aria-labelledby="checking-title">
      <h2 class="section-title" id="checking-title">Checking</h2>
      <label for="interval" class="field-label">Check for news</label>
      <select
        id="interval"
        class="select-input"
        value={localSettings.check_minutes}
        onchange={(e) => {
          localSettings = { ...localSettings, check_minutes: Number(e.currentTarget.value) };
          persistSettings();
        }}
      >
        <option value={1}>Every minute</option>
        <option value={5}>Every 5 minutes</option>
        <option value={15}>Every 15 minutes</option>
        <option value={30}>Every 30 minutes</option>
        <option value={60}>Every hour</option>
      </select>
    </section>

    <section class="section">
      <div class="row-between">
        <div>
          <label class="section-title" for="autostart">Start at login</label>
          <p class="section-desc" id="autostart-desc">Keep RepoBell running in the background when you reboot.</p>
        </div>
        <input
          id="autostart"
          type="checkbox"
          role="switch"
          class="switch"
          aria-describedby="autostart-desc"
          checked={localSettings.start_at_login}
          onchange={(e) => {
            localSettings = { ...localSettings, start_at_login: e.currentTarget.checked };
            persistSettings();
          }}
        />
      </div>
      {#if autostartError}
        <p class="msg-error" role="alert">{autostartError}</p>
      {/if}
    </section>

    <AboutSection update={state.update} {now} {aboutRequest} />
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
    margin: -4px 0 -8px;
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
  .section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .section-title {
    display: block;
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
  }
  .folder-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 8px 8px 14px;
    background: #fafaf8;
    border-bottom: 1px solid #e3e3e0;
    border-radius: 10px 10px 0 0;
  }
  .folder-header svg {
    flex-shrink: 0;
  }
  .folder-path {
    flex-grow: 1;
    min-width: 0;
    overflow: hidden;
    /* rtl puts the "..." at the start, so the folder name at the end stays visible; <bdi> keeps the path itself reading left to right */
    direction: rtl;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 14px;
    font-family: "Ubuntu Mono", ui-monospace, monospace;
  }
  .btn-remove-folder {
    height: 40px;
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
  .confirm-row {
    padding: 10px 14px;
    background: #fbf3f2;
    border-bottom: 1px solid #e8b9b4;
  }
  .confirm-row p {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .confirm-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
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
  .btn-danger {
    height: 38px;
    padding: 0 14px;
    border: none;
    border-radius: 8px;
    background: #a12a20;
    color: #ffffff;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
  }
  .btn-danger:hover {
    background: #85221a;
  }
  .repo-list {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .repo-item {
    display: flex;
    align-items: center;
    gap: 6px;
    min-height: 52px;
    padding: 0 6px 0 14px;
    border-bottom: 1px solid #efefec;
    position: relative;
  }
  .repo-item:last-child {
    border-bottom: none;
  }
  .repo-info {
    flex-grow: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .repo-name {
    font-size: 14px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .repo-note {
    font-size: 12px;
    color: #5b5b66;
  }
  .repo-empty {
    padding: 14px;
    font-size: 13px;
    color: #5b5b66;
  }
  .switch {
    appearance: none;
    -webkit-appearance: none;
    margin: 0;
    width: 44px;
    height: 26px;
    border-radius: 13px;
    background: #8a8a94;
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
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .switch:focus-visible {
    outline: 2px solid #1f5fd1;
    outline-offset: 2px;
  }
  .menu-wrap {
    position: relative;
    flex-shrink: 0;
  }
  .menu-btn {
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
  .menu-btn:hover,
  .menu-btn.open {
    background: #eeeeec;
    color: #3d3d45;
  }
  .menu {
    position: absolute;
    top: 44px;
    right: 0;
    z-index: 2;
    margin: 0;
    padding: 6px;
    list-style: none;
    width: 180px;
    background: #ffffff;
    border: 1px solid #d9d9d5;
    border-radius: 10px;
    box-shadow: 0 6px 18px rgba(0, 0, 0, 0.12);
  }
  .menu-item {
    width: 100%;
    height: 40px;
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: #1b1b1f;
    font-size: 14px;
    text-align: left;
    cursor: pointer;
  }
  .menu-item:hover {
    background: #f3f3f1;
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
    gap: 12px;
  }
  .msg-error {
    margin: 0;
    font-size: 13px;
    line-height: 1.4;
    color: #a12a20;
  }
  @media (prefers-reduced-motion: reduce) {
    .switch::after {
      transition: none;
    }
  }
</style>
