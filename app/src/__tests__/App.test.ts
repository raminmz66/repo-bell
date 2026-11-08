import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import App from "../App.svelte";
import type { AppStateView } from "../lib/types";

function createMockAppState(overrides: Partial<AppStateView> = {}): AppStateView {
  return {
    first_run: false,
    offline: false,
    last_checked: 1000,
    settings: {
      folders: ["/home/user/projects"],
      ignored: [],
      muted: [],
      editor: "VS Code",
      check_minutes: 5,
      start_at_login: true,
    },
    repos: [
      {
        path: "/home/user/projects/repo-one",
        name: "repo-one",
        folder: "/home/user/projects",
        main_branch: "main",
        error: null,
        behind: 0,
        ahead: 0,
        unsaved: false,
        muted: false,
        ignored: false,
        feed: null,
        undo_head: null,
      },
    ],
    ...overrides,
  };
}

describe("App root view and button interactions", () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it("navigates to Settings when gear icon is clicked, and back when Back is clicked", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_app_state") return createMockAppState();
      if (cmd === "detect_editors") return ["VS Code"];
    });

    render(App);

    // Wait for state to load and header to display
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    });

    // Click Settings button
    const settingsBtn = screen.getByRole("button", { name: "Settings" });
    await fireEvent.click(settingsBtn);

    // Should now show Settings header with Back button
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Back" })).toBeInTheDocument();
      expect(screen.getByText("Folders and repos")).toBeInTheDocument();
    });

    // Click Back button
    const backBtn = screen.getByRole("button", { name: "Back" });
    await fireEvent.click(backBtn);

    // Back to main view
    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Settings" })).toBeInTheDocument();
    });
  });

  it("renders offline state and allows retrying via Try again button", async () => {
    let checkNowCalled = false;
    mockIPC((cmd) => {
      if (cmd === "get_app_state") {
        return createMockAppState({ offline: true });
      }
      if (cmd === "check_now") {
        checkNowCalled = true;
        return createMockAppState({ offline: false });
      }
    });

    render(App);

    await waitFor(() => {
      expect(screen.getByText("Can't check your repos")).toBeInTheDocument();
    });

    const tryAgainBtn = screen.getByRole("button", { name: "Try again" });
    await fireEvent.click(tryAgainBtn);

    await waitFor(() => {
      expect(checkNowCalled).toBe(true);
      expect(screen.getByText("All quiet")).toBeInTheDocument();
    });
  });

  it("renders empty repos state and allows adding a folder", async () => {
    let pickFolderCalled = false;
    let savedFolders: string[] = [];

    mockIPC((cmd, args: any) => {
      if (cmd === "get_app_state") {
        return createMockAppState({
          repos: [],
          settings: {
            folders: [],
            ignored: [],
            muted: [],
            editor: null,
            check_minutes: 5,
            start_at_login: true,
          },
        });
      }
      if (cmd === "pick_folder") {
        pickFolderCalled = true;
        return "/home/user/picked";
      }
      if (cmd === "save_settings") {
        savedFolders = args.settings.folders;
        return createMockAppState({
          settings: args.settings,
        });
      }
    });

    render(App);

    await waitFor(() => {
      expect(screen.getByText("No repos yet. Add a folder to start watching.")).toBeInTheDocument();
    });

    const addFolderBtns = screen.getAllByRole("button", { name: /Add folder/i });
    await fireEvent.click(addFolderBtns[0]);

    await waitFor(() => {
      expect(pickFolderCalled).toBe(true);
      expect(savedFolders).toContain("/home/user/picked");
    });
  });

  it("toggles the Up to date accordion dropdown", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_app_state") return createMockAppState();
    });

    render(App);

    await waitFor(() => {
      expect(screen.getByText("All quiet")).toBeInTheDocument();
    });

    const accordionBtn = screen.getByRole("button", { name: /Up to date 1 repo/i });
    expect(accordionBtn).toHaveAttribute("aria-expanded", "false");

    // Click to expand
    await fireEvent.click(accordionBtn);
    expect(accordionBtn).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("repo-one")).toBeInTheDocument();

    // Click to collapse
    await fireEvent.click(accordionBtn);
    expect(accordionBtn).toHaveAttribute("aria-expanded", "false");
  });
});
