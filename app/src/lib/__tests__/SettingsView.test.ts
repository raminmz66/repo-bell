import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor, within } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import SettingsView from "../SettingsView.svelte";
import { makeRepo, makeState } from "../../test-helpers";

const P = "/home/user/projects";

function twoFolders() {
  return makeState({
    settings: {
      folders: [P, "/home/user/design"],
      ignored: [`${P}/old`],
      muted: [`${P}/api`],
      editor: "VS Code",
      check_minutes: 5,
      start_at_login: true,
    },
    repos: [
      makeRepo({ path: `${P}/web`, name: "web", folder: P }),
      makeRepo({ path: `${P}/api`, name: "api", folder: P, muted: true }),
      makeRepo({ path: `${P}/old`, name: "old", folder: P, ignored: true }),
      makeRepo({ path: "/home/user/design/ds", name: "ds", folder: "/home/user/design" }),
    ],
  });
}

function mount(state = twoFolders(), extra: Record<string, any> = {}) {
  let saved: any = null;
  mockIPC((cmd, a: any) => {
    if (cmd === "detect_editors") return ["VS Code", "System default app"];
    if (cmd === "save_settings") {
      if (extra.saveError) throw extra.saveError;
      saved = a.settings;
      return { ...state, settings: a.settings };
    }
    if (cmd === "pick_folder") return "/home/user/new-folder";
  });
  const onBack = vi.fn();
  const onStateRefresh = vi.fn();
  render(SettingsView, { state, onBack, onStateRefresh });
  return { onBack, onStateRefresh, saved: () => saved };
}

describe("SettingsView", () => {
  beforeEach(() => clearMocks());
  afterEach(() => {
    clearMocks();
    vi.restoreAllMocks();
  });

  it("clicking Back calls onBack and Escape does too", async () => {
    const { onBack } = mount();
    await fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(onBack).toHaveBeenCalledTimes(1);
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(onBack).toHaveBeenCalledTimes(2);
  });

  it("shows ~ for the home folder and keeps the full path in the tooltip", () => {
    mount();
    const path = document.querySelector(".folder-path")!;
    expect(path).toHaveTextContent("~/projects");
    expect(path).not.toHaveTextContent("/home/user");
    expect(path).toHaveAttribute("title", P);
  });

  it("groups repos under their folder", () => {
    mount();
    const groups = document.querySelectorAll(".folder-group");
    expect(groups).toHaveLength(2);
    expect(within(groups[0] as HTMLElement).getByText("web")).toBeInTheDocument();
    expect(within(groups[1] as HTMLElement).getByText("ds")).toBeInTheDocument();
    expect(within(groups[1] as HTMLElement).queryByText("web")).not.toBeInTheDocument();
  });

  it("shows the muted state once: a switch plus a short note, no second control", () => {
    mount();
    expect(screen.getByRole("switch", { name: "Notifications for web" })).toBeChecked();
    expect(screen.getByRole("switch", { name: "Notifications for api" })).not.toBeChecked();
    expect(screen.getAllByText("Notifications off")).toHaveLength(1);
    expect(screen.queryByText(/muted/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/Watched/)).not.toBeInTheDocument();
  });

  it("turning a notify switch off saves the repo as muted", async () => {
    const m = mount();
    await fireEvent.click(screen.getByRole("switch", { name: "Notifications for web" }));
    await waitFor(() => expect(m.saved().muted).toContain(`${P}/web`));
    await waitFor(() => expect(m.onStateRefresh).toHaveBeenCalled());
  });

  it("labels an unwatched repo 'Not watched' and never 'Stopped (Ignored)'", () => {
    mount();
    expect(screen.getByText("Not watched")).toBeInTheDocument();
    expect(screen.queryByText(/Stopped|Ignored/)).not.toBeInTheDocument();
    expect(screen.getByRole("switch", { name: "Notifications for old" })).toBeDisabled();
  });

  it("the repo menu has Stop watching, which saves the repo as ignored", async () => {
    const m = mount();
    const btn = screen.getByRole("button", { name: "More options for web" });
    expect(btn).toHaveAttribute("aria-expanded", "false");
    await fireEvent.click(btn);
    expect(btn).toHaveAttribute("aria-expanded", "true");
    await fireEvent.click(screen.getByRole("menuitem", { name: "Stop watching" }));
    await waitFor(() => expect(m.saved().ignored).toContain(`${P}/web`));
  });

  it("the menu for a stopped repo offers Watch again", async () => {
    const m = mount();
    await fireEvent.click(screen.getByRole("button", { name: "More options for old" }));
    await fireEvent.click(screen.getByRole("menuitem", { name: "Watch again" }));
    await waitFor(() => expect(m.saved().ignored).not.toContain(`${P}/old`));
  });

  it("Escape closes an open menu first, without leaving Settings", async () => {
    const { onBack } = mount();
    await fireEvent.click(screen.getByRole("button", { name: "More options for web" }));
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("menu")).not.toBeInTheDocument();
    expect(onBack).not.toHaveBeenCalled();
  });

  it("removing a folder asks first, inline, then saves", async () => {
    const dialogs = vi.spyOn(window, "confirm").mockImplementation(() => true);
    const m = mount();
    await fireEvent.click(screen.getByRole("button", { name: `Remove folder ${P}` }));
    expect(m.saved()).toBeNull();
    expect(screen.getByText(/stop watching all repos in this folder/)).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Remove folder" }));
    await waitFor(() => expect(m.saved().folders).toEqual(["/home/user/design"]));
    expect(dialogs).not.toHaveBeenCalled();
  });

  it("cancelling the folder confirm keeps the folder", async () => {
    const m = mount();
    await fireEvent.click(screen.getByRole("button", { name: `Remove folder ${P}` }));
    await fireEvent.click(screen.getByRole("button", { name: "Cancel" }));
    expect(screen.queryByText(/stop watching all repos/)).not.toBeInTheDocument();
    expect(m.saved()).toBeNull();
  });

  it("Add folder saves the picked folder", async () => {
    const m = mount();
    await fireEvent.click(screen.getByRole("button", { name: /Add folder/ }));
    await waitFor(() => expect(m.saved().folders).toContain("/home/user/new-folder"));
  });

  it("saves the check interval and the editor", async () => {
    const m = mount();
    await fireEvent.change(screen.getByLabelText("Check for news"), { target: { value: "15" } });
    await waitFor(() => expect(m.saved().check_minutes).toBe(15));
    await fireEvent.change(screen.getByLabelText("Open files in"), { target: { value: "System default app" } });
    await waitFor(() => expect(m.saved().editor).toBe("System default app"));
  });

  it("Start at login has a label and saves", async () => {
    const m = mount();
    const sw = screen.getByRole("switch", { name: "Start at login" });
    expect(sw).toBeChecked();
    await fireEvent.click(sw);
    await waitFor(() => expect(m.saved().start_at_login).toBe(false));
  });

  it("shows a failed save inline, puts the old value back and opens no dialog", async () => {
    const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
    mount(twoFolders(), { saveError: "Couldn't save your settings: disk full" });
    await fireEvent.click(screen.getByRole("switch", { name: "Notifications for web" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("disk full");
    expect(screen.getByRole("switch", { name: "Notifications for web" })).toBeChecked();
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it("shows the start-at-login problem next to that switch and keeps the other changes", async () => {
    const state = twoFolders();
    mockIPC((cmd, a: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") throw `Your settings were saved, but "start at login" couldn't be changed: no permission`;
    });
    const { rerender } = render(SettingsView, { state, onBack: vi.fn(), onStateRefresh: vi.fn() });
    await fireEvent.click(screen.getByRole("switch", { name: "Notifications for web" }));
    expect(await screen.findByText(/start at login" couldn't be changed/)).toBeInTheDocument();
    // the backend applied the settings and says so with a state-updated event
    await rerender({ state: { ...state, settings: { ...state.settings, muted: [...state.settings.muted, `${P}/web`] } } });
    expect(screen.getByRole("switch", { name: "Notifications for web" })).not.toBeChecked();
    expect(screen.getByText(/start at login" couldn't be changed/)).toBeInTheDocument();
  });

  it("says when a watched folder is missing", () => {
    const s = twoFolders();
    s.folder_errors = [{ path: "/home/user/design", message: "Folder not found. It may have been moved or deleted." }];
    s.repos = s.repos.filter((r) => r.name !== "ds");
    mount(s);
    expect(screen.getByText("Folder not found. It may have been moved or deleted.")).toBeInTheDocument();
  });
});
