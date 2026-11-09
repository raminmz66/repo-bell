import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import WelcomeView from "../WelcomeView.svelte";

const FOUND = [{ path: "/home/user/projects", display: "~/projects", repos: ["/home/user/projects/repo-alpha", "/home/user/projects/repo-beta"] }];

function mount(opts: { found?: any; saveError?: string; discoverDelay?: boolean; tray?: string; scanned?: any } = {}) {
  let saved: any = null;
  let release: (v: any) => void = () => {};
  mockIPC((cmd, a: any) => {
    if (cmd === "discover_suggested_folders") {
      if (opts.discoverDelay) return new Promise((r) => (release = r));
      return opts.found ?? FOUND;
    }
    if (cmd === "pick_folder") return "/home/user/extra";
    if (cmd === "scan_folder")
      return opts.scanned ?? { path: a.path, display: a.path, repos: [`${a.path}/nested/app`] };
    if (cmd === "check_tray") return opts.tray ?? "enabled";
    if (cmd === "save_settings") {
      saved = a.settings;
      if (opts.saveError) throw opts.saveError;
    }
  });
  const onFinish = vi.fn();
  render(WelcomeView, { onFinish });
  return { onFinish, saved: () => saved, release: (v: any) => release(v) };
}

async function toStep2() {
  await fireEvent.click(screen.getByRole("button", { name: "Find my repos" }));
}

describe("WelcomeView", () => {
  beforeEach(() => clearMocks());
  afterEach(() => {
    clearMocks();
    vi.restoreAllMocks();
  });

  it("walks through the steps and saves the choices", async () => {
    const m = mount();
    expect(screen.getByText("Step 1 of 3")).toBeInTheDocument();
    await toStep2();
    expect(await screen.findByText("Found 2 repos in 1 folder")).toBeInTheDocument();

    await fireEvent.click(screen.getByLabelText("repo-alpha"));
    // the headline counts what was found, the button counts what is ticked
    expect(screen.getByText("Found 2 repos in 1 folder")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Watch 1 repo" }));

    expect(screen.getByText("Step 3 of 3")).toBeInTheDocument();
    const login = screen.getByLabelText("Start RepoBell when I log in");
    expect(login).toBeChecked();
    await fireEvent.click(login);
    await fireEvent.click(screen.getByRole("button", { name: "Done" }));

    await waitFor(() => expect(m.onFinish).toHaveBeenCalledTimes(1));
    expect(m.saved().start_at_login).toBe(false);
    expect(m.saved().ignored).toEqual(["/home/user/projects/repo-alpha"]);
    expect(m.saved().folders).toEqual(["/home/user/projects"]);
  });

  it("shows a loading state while scanning and disables Watch", async () => {
    const m = mount({ discoverDelay: true });
    await toStep2();
    expect(screen.getByText("Looking for your repos…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Watch/ })).toBeDisabled();
    m.release(FOUND);
    expect(await screen.findByText("Found 2 repos in 1 folder")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Watch 2 repos" })).toBeEnabled();
  });

  it("shows an empty state and disables Watch when nothing was found", async () => {
    mount({ found: [] });
    await toStep2();
    expect(await screen.findByText("No repos found yet")).toBeInTheDocument();
    expect(screen.queryByText(/Watch 0 repos/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Watch repos" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Add a folder" })).toBeInTheDocument();
  });

  it("disables Watch when every repo is unticked", async () => {
    mount({ found: [{ path: "/p", display: "~/p", repos: ["/p/only"] }] });
    await toStep2();
    await fireEvent.click(await screen.findByLabelText("only"));
    expect(screen.queryByText(/Watch 0 repos/)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Watch repos" })).toBeDisabled();
  });

  it("an added folder shows up, enables Watch and is saved", async () => {
    const m = mount({ found: [] });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Add a folder" }));
    expect(await screen.findByText("/home/user/extra")).toBeInTheDocument();
    // scanned at once; a nested repo keeps its parent folder in the label
    expect(await screen.findByLabelText("nested/app")).toBeChecked();
    await fireEvent.click(screen.getByRole("button", { name: "Watch 1 repo" }));
    await fireEvent.click(screen.getByRole("button", { name: "Done" }));
    await waitFor(() => expect(m.saved().folders).toEqual(["/home/user/extra"]));
    expect(m.saved().ignored).toEqual([]);
  });

  it("an added folder with no repos says so", async () => {
    mount({ found: [], scanned: { path: "/home/user/extra", display: "/home/user/extra", repos: [] } });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Add a folder" }));
    expect(await screen.findByText("No repos in this folder.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Watch repos" })).toBeDisabled();
  });

  it("with the bell blocked, step 3 explains the extension and continues without it", async () => {
    const m = mount({ tray: "disabled" });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    expect(await screen.findByText("The bell can't show in your top bar")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Check again" })).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Continue without the bell" }));
    await waitFor(() => expect(m.onFinish).toHaveBeenCalledTimes(1));
  });

  it("with the bell working, step 3 confirms it", async () => {
    mount({ tray: "enabled" });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    expect(await screen.findByText("The bell is in your top bar")).toBeInTheDocument();
  });

  it("does not watch a folder whose repos were all unticked", async () => {
    const m = mount({
      found: [
        { path: "/a", display: "~/a", repos: ["/a/one"] },
        { path: "/b", display: "~/b", repos: ["/b/two"] },
      ],
    });
    await toStep2();
    await fireEvent.click(await screen.findByLabelText("one"));
    await fireEvent.click(screen.getByRole("button", { name: "Watch 1 repo" }));
    await fireEvent.click(screen.getByRole("button", { name: "Done" }));
    await waitFor(() => expect(m.saved().folders).toEqual(["/b"]));
    expect(m.saved().ignored).toEqual([]);
  });

  it("Back goes up one step, and so does Escape", async () => {
    mount();
    await toStep2();
    await screen.findByText("Step 2 of 3");
    await fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByText("Step 1 of 3")).toBeInTheDocument();

    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    expect(screen.getByText("Step 3 of 3")).toBeInTheDocument();
    await fireEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(screen.getByText("Step 2 of 3")).toBeInTheDocument();
    await fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.getByText("Step 1 of 3")).toBeInTheDocument();
  });

  it("step 3 has the 'Can't see the bell?' help", async () => {
    mount();
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    expect(screen.getByText("Can't see the bell?")).toBeInTheDocument();
    expect(screen.getByText(/Ubuntu AppIndicators/)).toBeInTheDocument();
  });

  it("shows a save error inline and lets the user try again", async () => {
    const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
    const m = mount({ saveError: "Couldn't save your settings: disk full" });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    await fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(await screen.findByRole("alert")).toHaveTextContent("disk full");
    expect(m.onFinish).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Done" })).toBeEnabled();
    expect(alertSpy).not.toHaveBeenCalled();
  });

  it("when only start-at-login failed, says so and lets the user continue", async () => {
    const m = mount({ saveError: `Your settings were saved, but "start at login" couldn't be changed: no permission` });
    await toStep2();
    await fireEvent.click(await screen.findByRole("button", { name: "Watch 2 repos" }));
    await fireEvent.click(screen.getByRole("button", { name: "Done" }));
    expect(await screen.findByText(/couldn't be changed/)).toBeInTheDocument();
    expect(m.onFinish).not.toHaveBeenCalled();
    await fireEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(m.onFinish).toHaveBeenCalledTimes(1);
  });
});
