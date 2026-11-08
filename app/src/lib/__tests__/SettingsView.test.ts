import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import SettingsView from "../SettingsView.svelte";
import type { AppStateView } from "../types";

function createMockAppState(): AppStateView {
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
  };
}

describe("SettingsView component button interactions", () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it("clicking Back button invokes onBack callback", async () => {
    const handleBack = vi.fn();
    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: handleBack,
      onStateRefresh: vi.fn(),
    });

    const backBtn = screen.getByRole("button", { name: "Back" });
    await fireEvent.click(backBtn);
    expect(handleBack).toHaveBeenCalledTimes(1);
  });

  it("toggles watch switch and persists ignored state", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const onStateRefresh = vi.fn();
    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh,
    });

    const watchSwitch = screen.getByRole("switch", { name: "Watch repo-one" });
    expect(watchSwitch).toBeChecked();

    // Toggle off
    await fireEvent.click(watchSwitch);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.ignored).toContain("/home/user/projects/repo-one");
      expect(onStateRefresh).toHaveBeenCalled();
    });
  });

  it("toggles alerts bell button and persists muted state", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh: vi.fn(),
    });

    const bellBtn = screen.getByRole("button", { name: "Toggle alerts for repo-one" });
    expect(screen.getByText("Alerts")).toBeInTheDocument();

    // Click to mute
    await fireEvent.click(bellBtn);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.muted).toContain("/home/user/projects/repo-one");
    });
  });

  it("clicking Remove folder removes it from settings", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh: vi.fn(),
    });

    const removeBtn = screen.getByRole("button", { name: "Remove" });
    await fireEvent.click(removeBtn);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.folders).not.toContain("/home/user/projects");
    });
  });

  it("clicking Add folder invokes pick_folder and saves new folder", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "pick_folder") return "/home/user/new-folder";
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh: vi.fn(),
    });

    const addFolderBtn = screen.getByRole("button", { name: /Add folder/i });
    await fireEvent.click(addFolderBtn);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.folders).toContain("/home/user/new-folder");
    });
  });

  it("changing check interval saves setting", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh: vi.fn(),
    });

    const intervalSelect = screen.getByLabelText("Check for news");
    await fireEvent.change(intervalSelect, { target: { value: "15" } });

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.check_minutes).toBe(15);
    });
  });

  it("toggling Start at login switch saves setting", async () => {
    let savedSettings: any = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "detect_editors") return ["VS Code"];
      if (cmd === "save_settings") {
        savedSettings = args.settings;
        return {
          ...createMockAppState(),
          settings: args.settings,
        };
      }
    });

    const state = createMockAppState();

    render(SettingsView, {
      state,
      onBack: vi.fn(),
      onStateRefresh: vi.fn(),
    });

    const switches = screen.getAllByRole("switch");
    // The second switch is start at login
    const loginSwitch = switches[switches.length - 1];
    expect(loginSwitch).toBeChecked();

    await fireEvent.click(loginSwitch);

    await waitFor(() => {
      expect(savedSettings).not.toBeNull();
      expect(savedSettings.start_at_login).toBe(false);
    });
  });
});
