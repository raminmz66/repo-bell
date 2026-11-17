<script lang="ts">
  import { tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { UpdateView } from "./types";
  import { checkedAt } from "./util";
  import appIcon from "../../src-tauri/icons/128x128.png?no-inline";

  export let update: UpdateView;
  export let now: number;
  export let aboutRequest = 0;

  type Action = "check" | "install";
  interface Line {
    text: string;
    tone: "normal" | "ready" | "error";
    alert: boolean;
    /** The end of `text` that must stay on one line. */
    keep?: string;
  }
  interface Btn {
    label: string;
    primary: boolean;
    action: Action;
    disabled: boolean;
  }

  const ERRORS = {
    offline: { text: "Can't reach GitHub to check for updates", tone: "error" },
    download_failed: { text: "The download stopped. Check your connection and try again.", tone: "error" },
    bad_download: { text: "The download didn't pass the safety check, so it wasn't installed", tone: "error" },
    not_installed: { text: "Update not installed. Nothing was changed.", tone: "normal" },
  } as const;

  let section: HTMLElement;
  let heading: HTMLElement;
  let button: HTMLButtonElement | undefined;

  $: version = update.available?.version ?? "";
  $: downloading = update.status === "downloading";
  $: doneMb = Math.floor(update.downloaded / 1e6);
  $: totalMb = update.total ? Math.ceil(update.total / 1e6) : null;
  $: sizeText = totalMb === null ? `${doneMb} MB` : `${doneMb} of ${totalMb} MB`;
  $: pct = update.total ? Math.min(100, (update.downloaded / update.total) * 100) : 0;

  $: line = lineFor(update, version, sizeText, now);
  $: btn = buttonFor(update);
  $: waiting = btn?.action === "install" && update.busy;
  $: notes = parseNotes(update.available?.notes ?? "");
  // Runs before the page updates, so it still sees the button the user pressed.
  $: keepFocus(btn);

  /** A bullet ("- " or "* ") starts an item; any other line continues the item before it. */
  function parseNotes(text: string): string[] {
    const items: string[] = [];
    for (const raw of text.split("\n")) {
      const l = raw.trim();
      if (l === "") continue;
      if (/^[-*] /.test(l)) items.push(l.slice(2).trim());
      else if (items.length > 0) items[items.length - 1] += ` ${l}`;
      else items.push(l);
    }
    return items;
  }

  /** When the button the user is on goes away or turns off, the keyboard focus goes to the heading. */
  async function keepFocus(next: Btn | null) {
    if (!button || document.activeElement !== button || (next && !next.disabled)) return;
    await tick();
    heading?.focus({ preventScroll: true });
  }

  function lineFor(u: UpdateView, v: string, size: string, nowMs: number): Line | null {
    if (!u.can_update) return { text: "This copy was built from source, so it doesn't update itself.", tone: "normal", alert: false };
    if (u.error) return { ...ERRORS[u.error], alert: true };
    switch (u.status) {
      case "checking":
        return { text: "Checking for updates…", tone: "normal", alert: false };
      case "up_to_date":
        if (u.checked_at === null) return { text: "Up to date", tone: "normal", alert: false };
        const checked = `Checked ${checkedAt(u.checked_at, nowMs)}`;
        return { text: `Up to date. ${checked}`, tone: "normal", alert: false, keep: checked };
      case "ready":
        return { text: `Version ${v} is ready`, tone: "ready", alert: false };
      case "downloading":
        return { text: `Downloading ${v}… ${size}`, tone: "normal", alert: false };
      case "installing":
        return {
          text: u.deb
            ? "Enter your password in the Ubuntu window to finish. RepoBell restarts after."
            : `Installing ${v}… RepoBell restarts after.`,
          tone: "normal",
          alert: false,
        };
      default:
        return null;
    }
  }

  function buttonFor(u: UpdateView): Btn | null {
    if (!u.can_update) return null;
    if (u.error) {
      const action = u.error === "offline" ? "check" : "install";
      return { label: "Try again", primary: u.error === "not_installed", action, disabled: action === "install" && u.busy };
    }
    switch (u.status) {
      case "ready":
        return { label: "Install", primary: true, action: "install", disabled: u.busy };
      case "downloading":
      case "installing":
        return null;
      default:
        return { label: "Check for updates", primary: false, action: "check", disabled: u.status === "checking" };
    }
  }

  // The new state arrives through `state-updated`, so the result is not needed here.
  function run(action: Action) {
    invoke(action === "check" ? "check_for_updates" : "install_update").catch(() => {});
  }

  function open(target: "source" | "issues") {
    invoke("open_link", { target }).catch(() => {});
  }

  async function jump() {
    await tick();
    section?.scrollIntoView?.({ block: "start" });
    heading?.focus({ preventScroll: true });
  }

  $: if (aboutRequest > 0) jump();
</script>

<section class="section" aria-labelledby="about-title" bind:this={section}>
  <h2 class="section-title" id="about-title" tabindex="-1" bind:this={heading}>About</h2>

  <div class="about-card">
    <div class="about-top">
      <img class="icon" src={appIcon} alt="" width="40" height="40" />
      <div class="who">
        <div class="ver">RepoBell {update.current}</div>
        {#if line}
          <!-- A new element for each line, so a screen reader announces the alert. -->
          {#key line.text}
            <p class="status" class:ready={line.tone === "ready"} class:error={line.tone === "error"} role={line.alert ? "alert" : "status"}>{#if line.keep}{line.text.slice(0, -line.keep.length)}<span class="nowrap">{line.keep}</span>{:else}{line.text}{/if}</p>
          {/key}
        {/if}
      </div>
      {#if btn}
        <button
          class={btn.primary ? "btn-primary" : "btn-quiet"}
          disabled={btn.disabled}
          bind:this={button}
          aria-describedby={waiting ? "install-wait" : undefined}
          onclick={() => run(btn.action)}
        >{btn.label}</button>
      {/if}
    </div>

    {#if waiting}
      <p class="wait" id="install-wait">Wait for the repo action to finish.</p>
    {/if}

    {#if downloading}
      <div
        class="bar"
        role="progressbar"
        aria-label="Download progress"
        aria-valuemin="0"
        aria-valuemax={totalMb ?? undefined}
        aria-valuenow={doneMb}
        aria-valuetext={sizeText}
      >
        <div class="fill" style:width="{pct}%"></div>
      </div>
    {/if}

    {#if update.available && notes.length > 0}
      <div class="notes">
        <p class="notes-title">What's new in {version}</p>
        <ul>
          {#each notes as note}
            <li>{note}</li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="links">
      <button class="link" onclick={() => open("source")}>Source code</button>
      <button class="link" onclick={() => open("issues")}>Report a problem</button>
      <span class="license">MIT license</span>
    </div>
  </div>
</section>

<style>
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
  /* Only a script focuses this heading, so no ring. */
  .section-title:focus,
  .section-title:focus-visible {
    outline: none;
  }
  .about-card {
    padding: 12px;
    background: #ffffff;
    border: 1px solid #e3e3e0;
    border-radius: 10px;
  }
  .about-top {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 36px;
  }
  .icon {
    flex-shrink: 0;
    border-radius: 10px;
  }
  .who {
    flex: 1;
    min-width: 0;
  }
  .ver {
    font-size: 14px;
    font-weight: 700;
    color: #1b1b1f;
  }
  .status {
    margin: 2px 0 0;
    font-size: 13px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .nowrap {
    white-space: nowrap;
  }
  .status.ready {
    color: #1f5fd1;
  }
  .status.error {
    color: #a12a20;
  }
  .btn-primary,
  .btn-quiet {
    flex-shrink: 0;
    height: 36px;
    padding: 0 14px;
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
  .btn-primary:hover:not(:disabled) {
    background: #174aa6;
  }
  .btn-quiet {
    border: 1px solid #c9c9c4;
    background: #ffffff;
    color: #1b1b1f;
  }
  .btn-quiet:hover:not(:disabled) {
    background: #f3f3f1;
  }
  .btn-primary:disabled,
  .btn-quiet:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .btn-primary:focus-visible,
  .btn-quiet:focus-visible,
  .link:focus-visible {
    outline: 2px solid #1f5fd1;
    outline-offset: 2px;
  }
  .wait {
    margin: 8px 0 0;
    text-align: right;
    font-size: 12px;
    line-height: 1.4;
    color: #5b5b66;
  }
  .bar {
    margin-top: 8px;
    height: 4px;
    border-radius: 2px;
    background: #eeeeec;
    overflow: hidden;
  }
  .fill {
    height: 100%;
    background: #5b5b66;
    border-radius: 2px;
    transition: width 0.15s ease;
  }
  .notes {
    margin-top: 10px;
    padding: 10px 12px;
    background: #f6f6f4;
    border-radius: 8px;
    font-size: 13px;
    line-height: 1.4;
    color: #3d3d45;
  }
  .notes-title {
    margin: 0 0 2px;
    font-weight: 700;
    color: #1b1b1f;
  }
  .notes ul {
    margin: 0;
    padding-left: 18px;
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0 14px;
    margin-top: 10px;
    padding-top: 6px;
    border-top: 1px solid #e3e3e0;
    font-size: 13px;
  }
  .link {
    min-height: 32px;
    padding: 0;
    border: none;
    background: transparent;
    color: #1f5fd1;
    font: inherit;
    text-decoration: underline;
    text-underline-offset: 2px;
    cursor: pointer;
  }
  .link:hover {
    color: #174aa6;
  }
  .license {
    color: #5b5b66;
  }
  @media (prefers-reduced-motion: reduce) {
    .fill {
      transition: none;
    }
  }
</style>
