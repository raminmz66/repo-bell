import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import AboutSection from "../AboutSection.svelte";
import { makeUpdate } from "../../test-helpers";
import type { UpdateView } from "../types";

const now = new Date(2026, 10, 17, 15, 0).getTime();
const checked = new Date(2026, 10, 17, 9, 12).getTime() / 1000;
const ready = { version: "0.2.0", notes: "- Shows the version\n\n* Faster checks" };

function mount(overrides: Partial<UpdateView>, extra: Record<string, any> = {}) {
  const calls: { cmd: string; args: any }[] = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
  });
  render(AboutSection, { update: makeUpdate(overrides), now, ...extra });
  return calls;
}

const PASSWORD = "Enter your password in the Ubuntu window to finish. RepoBell restarts after.";

const rows: { name: string; update: Partial<UpdateView>; line: string | null; button: string | null; disabled?: boolean; alert?: boolean }[] = [
  { name: "idle", update: {}, line: null, button: "Check for updates" },
  { name: "checking", update: { status: "checking" }, line: "Checking for updates…", button: "Check for updates", disabled: true },
  { name: "up to date", update: { status: "up_to_date", checked_at: checked }, line: "Up to date. Checked today at 09:12", button: "Check for updates" },
  { name: "ready", update: { status: "ready", available: ready }, line: "Version 0.2.0 is ready", button: "Install" },
  { name: "downloading", update: { status: "downloading", available: ready, downloaded: 4_200_000, total: 8_100_000 }, line: "Downloading 0.2.0… 4 of 9 MB", button: null },
  { name: "installing deb", update: { status: "installing", available: ready, deb: true }, line: PASSWORD, button: null },
  { name: "installing appimage", update: { status: "installing", available: ready, deb: false }, line: "Installing 0.2.0… RepoBell restarts after.", button: null },
  { name: "offline", update: { error: "offline" }, line: "Can't reach GitHub to check for updates", button: "Try again", alert: true },
  { name: "download failed", update: { status: "ready", available: ready, error: "download_failed" }, line: "The download stopped. Check your connection and try again.", button: "Try again", alert: true },
  { name: "bad download", update: { status: "ready", available: ready, error: "bad_download" }, line: "The download didn't pass the safety check, so it wasn't installed", button: "Try again", alert: true },
  { name: "not installed", update: { status: "ready", available: ready, error: "not_installed" }, line: "Update not installed. Nothing was changed.", button: "Try again", alert: true },
];

describe("AboutSection", () => {
  beforeEach(() => clearMocks());
  afterEach(() => clearMocks());

  it.each(rows)("$name: line and button", ({ update, line, button, disabled, alert }) => {
    mount(update);
    if (line) {
      const el = screen.getByText(line);
      if (alert) expect(el).toHaveAttribute("role", "alert");
      else expect(el).not.toHaveAttribute("role", "alert");
    }
    if (button) {
      const b = screen.getByRole("button", { name: button });
      if (disabled) expect(b).toBeDisabled();
      else expect(b).toBeEnabled();
    } else {
      expect(screen.queryByRole("button", { name: /Check for updates|Install|Try again/ })).toBeNull();
    }
  });

  it.each(rows)("$name: always shows the version", ({ update }) => {
    mount(update);
    expect(screen.getByText("RepoBell 0.1.0")).toBeInTheDocument();
  });

  it("idle shows no status line", () => {
    const { container } = render(AboutSection, { update: makeUpdate({}), now });
    expect(container.querySelector(".status")).toBeNull();
  });

  it("download numbers count only while downloading", () => {
    mount({ status: "ready", available: ready, error: "not_installed", downloaded: 5_000_000, total: 9_000_000 });
    expect(screen.queryByRole("progressbar")).toBeNull();
    expect(screen.queryByText(/MB/)).toBeNull();
  });

  it("try again after an install error waits for a repo action", () => {
    mount({ status: "ready", available: ready, error: "bad_download", busy: true });
    const again = screen.getByRole("button", { name: "Try again" });
    expect(again).toBeDisabled();
    const id = again.getAttribute("aria-describedby")!;
    expect(document.getElementById(id)).toHaveTextContent("Wait for the repo action to finish.");
  });

  it("notes are split into items and only shown when not empty", () => {
    mount({ status: "ready", available: ready });
    expect(screen.getByText("What's new in 0.2.0")).toBeInTheDocument();
    expect(screen.getAllByRole("listitem").map((li) => li.textContent)).toEqual(["Shows the version", "Faster checks"]);
  });

  it("hides the notes box when the notes are empty", () => {
    mount({ status: "ready", available: { version: "0.2.0", notes: "  \n" } });
    expect(screen.queryByText(/What's new/)).toBeNull();
  });

  it("notes are plain text", () => {
    const { container } = render(AboutSection, {
      update: makeUpdate({ status: "ready", available: { version: "0.2.0", notes: "<b>bold</b>" } }),
      now,
    });
    expect(screen.getByRole("listitem")).toHaveTextContent("<b>bold</b>");
    expect(container.querySelector("b")).toBeNull();
  });

  it("install waits for a repo action", () => {
    mount({ status: "ready", available: ready, busy: true });
    const install = screen.getByRole("button", { name: "Install" });
    expect(install).toBeDisabled();
    const id = install.getAttribute("aria-describedby")!;
    expect(document.getElementById(id)).toHaveTextContent("Wait for the repo action to finish.");
  });

  it("install calls the backend", async () => {
    const calls = mount({ status: "ready", available: ready });
    await fireEvent.click(screen.getByRole("button", { name: "Install" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toContain("install_update"));
  });

  it("check for updates calls the backend", async () => {
    const calls = mount({});
    await fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toContain("check_for_updates"));
  });

  it("try again repeats the step that failed", async () => {
    const calls = mount({ error: "offline" });
    await fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["check_for_updates"]));
  });

  it("try again after a bad download installs again", async () => {
    const calls = mount({ status: "ready", available: ready, error: "bad_download" });
    await fireEvent.click(screen.getByRole("button", { name: "Try again" }));
    await waitFor(() => expect(calls.map((c) => c.cmd)).toEqual(["install_update"]));
  });

  it("download bar without a total", () => {
    mount({ status: "downloading", available: ready, downloaded: 4_200_000, total: null });
    expect(screen.getByText("Downloading 0.2.0… 4 MB")).toBeInTheDocument();
    const bar = screen.getByRole("progressbar", { name: "Download progress" });
    expect(bar).not.toHaveAttribute("aria-valuemax");
    expect(bar).toHaveAttribute("aria-valuetext", "4 MB");
  });

  it("download bar with a total", () => {
    mount({ status: "downloading", available: ready, downloaded: 4_200_000, total: 8_100_000 });
    const bar = screen.getByRole("progressbar", { name: "Download progress" });
    expect(bar).toHaveAttribute("aria-valuemax", "9");
    expect(bar).toHaveAttribute("aria-valuenow", "4");
    expect(bar).toHaveAttribute("aria-valuetext", "4 of 9 MB");
  });

  it("links open through the backend", async () => {
    const calls = mount({});
    await fireEvent.click(screen.getByRole("button", { name: "Source code" }));
    await fireEvent.click(screen.getByRole("button", { name: "Report a problem" }));
    await waitFor(() =>
      expect(calls.filter((c) => c.cmd === "open_link").map((c) => c.args)).toEqual([{ target: "source" }, { target: "issues" }]),
    );
    expect(screen.getByText("MIT license")).toBeInTheDocument();
  });

  it("built from source: no button, ignores the status", () => {
    mount({ can_update: false, status: "checking" });
    expect(screen.getByText("This copy was built from source, so it doesn't update itself.")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /Check for updates|Install|Try again/ })).toBeNull();
  });

  it("jumps to About when asked", async () => {
    const scroll = vi.fn();
    Element.prototype.scrollIntoView = scroll;
    mount({}, { aboutRequest: 1 });
    await waitFor(() => expect(scroll).toHaveBeenCalled());
    expect(document.activeElement).toBe(screen.getByRole("heading", { name: "About" }));
  });
});
