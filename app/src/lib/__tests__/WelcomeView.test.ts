import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import WelcomeView from "../WelcomeView.svelte";

describe("WelcomeView component button interactions", () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it("walks through step 1, step 2, and step 3 finishing with save_settings", async () => {
    let savedSettings: any = null;
    let pickedFolderRequested = false;

    mockIPC((cmd, args: any) => {
      if (cmd === "discover_suggested_folders") {
        return [
          {
            path: "/home/user/projects",
            display: "~/projects",
            repos: ["repo-alpha", "repo-beta"],
          },
        ];
      }
      if (cmd === "pick_folder") {
        pickedFolderRequested = true;
        return "/home/user/extra";
      }
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return;
      }
    });

    const handleFinish = vi.fn();
    render(WelcomeView, { onFinish: handleFinish });

    // Step 1: Click "Find my repos"
    expect(screen.getByText("Step 1 of 3")).toBeInTheDocument();
    const findBtn = screen.getByRole("button", { name: "Find my repos" });
    await fireEvent.click(findBtn);

    // Step 2: Rendered with suggested repos
    await waitFor(() => {
      expect(screen.getByText("Step 2 of 3")).toBeInTheDocument();
    });

    // Checkboxes for discovered repos
    const alphaCheckbox = screen.getByLabelText("repo-alpha");
    expect(alphaCheckbox).toBeChecked();

    // Untick repo-alpha
    await fireEvent.click(alphaCheckbox);
    expect(alphaCheckbox).not.toBeChecked();

    // Click "Add another folder"
    const addFolderBtn = screen.getByRole("button", { name: /Add another folder/i });
    await fireEvent.click(addFolderBtn);
    expect(pickedFolderRequested).toBe(true);

    // Click "Watch 1 repo" (since alpha was unticked and beta is still checked)
    const watchBtn = screen.getByRole("button", { name: /Watch 1 repo/i });
    await fireEvent.click(watchBtn);

    // Step 3: Autostart toggle and "Done" button
    await waitFor(() => {
      expect(screen.getByText("Step 3 of 3")).toBeInTheDocument();
    });

    const loginCheckbox = screen.getByLabelText("Start RepoBell when I log in");
    expect(loginCheckbox).toBeChecked();
    // Toggle login autostart off
    await fireEvent.click(loginCheckbox);
    expect(loginCheckbox).not.toBeChecked();

    // Click "Done"
    const doneBtn = screen.getByRole("button", { name: "Done" });
    await fireEvent.click(doneBtn);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(handleFinish).toHaveBeenCalledTimes(1);
    });

    // Verify saved settings contents
    expect(savedSettings.start_at_login).toBe(false);
    expect(savedSettings.ignored).toContain("/home/user/projects/repo-alpha");
    expect(savedSettings.folders).toContain("/home/user/projects");
    expect(savedSettings.folders).toContain("/home/user/extra");
  });
});
