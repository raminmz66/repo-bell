import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import RepoCard from "../RepoCard.svelte";
import type { RepoCardView } from "../types";

function createMockRepo(overrides: Partial<RepoCardView> = {}): RepoCardView {
  return {
    path: "/home/user/projects/my-repo",
    name: "my-repo",
    folder: "/home/user/projects",
    main_branch: "main",
    error: null,
    behind: 2,
    ahead: 0,
    unsaved: false,
    muted: false,
    ignored: false,
    feed: {
      total_changes: 2,
      author_summary: "Alex pushed 2 changes",
      newest_time: Math.floor(Date.now() / 1000) - 300,
      changes: [
        {
          author: "Alex",
          title: "Fix bug in api",
          sha: "abc123456789",
          timestamp: Math.floor(Date.now() / 1000) - 300,
          mine: false,
        },
      ],
      files_changed: ["src/api.ts", "README.md"],
    },
    undo_head: null,
    ...overrides,
  };
}

describe("RepoCard component button interactions", () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it("toggles expand and collapse when header is clicked", async () => {
    const repo = createMockRepo();
    render(RepoCard, { repo, onStateRefresh: vi.fn() });

    const header = screen.getByRole("button", { name: /my-repo/i });
    expect(header).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Alex pushed 2 changes")).toBeInTheDocument();

    // Click header to collapse
    await fireEvent.click(header);
    expect(header).toHaveAttribute("aria-expanded", "false");
    expect(screen.queryByText("Alex pushed 2 changes")).not.toBeInTheDocument();

    // Click header to re-expand
    await fireEvent.click(header);
    expect(header).toHaveAttribute("aria-expanded", "true");
    expect(screen.getByText("Alex pushed 2 changes")).toBeInTheDocument();
  });

  it("clicking Update executes update_repo when clean", async () => {
    let updatePath: string | null = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "update_repo") {
        updatePath = args.path;
        return {
          success: true,
          downloads: 2,
          undo_head: "old_head_sha",
          conflict_files: [],
          message: "Updated. 2 changes downloaded.",
        };
      }
    });

    const onStateRefresh = vi.fn();
    const repo = createMockRepo();
    render(RepoCard, { repo, onStateRefresh });

    const updateBtn = screen.getByRole("button", { name: "Update" });
    await fireEvent.click(updateBtn);

    await waitFor(() => {
      expect(updatePath).toBe("/home/user/projects/my-repo");
      expect(onStateRefresh).toHaveBeenCalledTimes(1);
    });

    // Shows Undo banner
    expect(screen.getByText("Updated. 2 changes downloaded.")).toBeInTheDocument();
  });

  it("handles unsaved prompt cancel and confirm", async () => {
    let updateCalled = false;
    mockIPC((cmd) => {
      if (cmd === "update_repo") {
        updateCalled = true;
        return {
          success: true,
          downloads: 2,
          undo_head: "head_sha",
          conflict_files: [],
          message: "Updated successfully",
        };
      }
    });

    const repo = createMockRepo({ unsaved: true });
    render(RepoCard, { repo, onStateRefresh: vi.fn() });

    const updateBtn = screen.getByRole("button", { name: "Update" });
    await fireEvent.click(updateBtn);

    // Prompt appears
    expect(screen.getByText("You have unsaved changes.")).toBeInTheDocument();

    // Click Cancel
    const cancelBtn = screen.getByRole("button", { name: "Cancel" });
    await fireEvent.click(cancelBtn);
    expect(screen.queryByText("You have unsaved changes.")).not.toBeInTheDocument();
    expect(updateCalled).toBe(false);

    // Re-open and confirm "Set aside and update"
    await fireEvent.click(screen.getByRole("button", { name: "Update" }));
    const confirmBtn = screen.getByRole("button", { name: "Set aside and update" });
    await fireEvent.click(confirmBtn);

    await waitFor(() => {
      expect(updateCalled).toBe(true);
    });
  });

  it("handles combine prompt cancel and confirm", async () => {
    let updateCalled = false;
    mockIPC((cmd) => {
      if (cmd === "update_repo") {
        updateCalled = true;
        return {
          success: true,
          downloads: 2,
          undo_head: "head_sha",
          conflict_files: [],
          message: "Updated and combined",
        };
      }
    });

    const repo = createMockRepo({ ahead: 1 });
    render(RepoCard, { repo, onStateRefresh: vi.fn() });

    const updateBtn = screen.getByRole("button", { name: "Update" });
    await fireEvent.click(updateBtn);

    expect(screen.getByText("Your work and the team's work both changed.")).toBeInTheDocument();

    // Click Cancel
    const cancelBtn = screen.getByRole("button", { name: "Cancel" });
    await fireEvent.click(cancelBtn);
    expect(screen.queryByText("Your work and the team's work both changed.")).not.toBeInTheDocument();

    // Re-open and confirm "Update and combine"
    await fireEvent.click(screen.getByRole("button", { name: "Update" }));
    const combineBtn = screen.getByRole("button", { name: "Update and combine" });
    await fireEvent.click(combineBtn);

    await waitFor(() => {
      expect(updateCalled).toBe(true);
    });
  });

  it("handles conflict state, open editor, and close conflict alert", async () => {
    let openedFile: string | null = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "update_repo") {
        return {
          success: false,
          downloads: 0,
          undo_head: null,
          conflict_files: ["src/api.ts"],
          message: "Conflict",
        };
      }
      if (cmd === "open_editor") {
        openedFile = args.filePath;
        return;
      }
    });

    const repo = createMockRepo();
    render(RepoCard, { repo, onStateRefresh: vi.fn() });

    await fireEvent.click(screen.getByRole("button", { name: "Update" }));

    await waitFor(() => {
      expect(screen.getByText("Update stopped. Nothing was changed.")).toBeInTheDocument();
      expect(screen.getByText("src/api.ts")).toBeInTheDocument();
    });

    // Click "Open in editor"
    const openEditorBtn = screen.getByRole("button", { name: "Open in editor" });
    await fireEvent.click(openEditorBtn);
    expect(openedFile).toBe("/home/user/projects/my-repo/src/api.ts");

    // Click "Close"
    const closeBtn = screen.getByRole("button", { name: "Close" });
    await fireEvent.click(closeBtn);
    expect(screen.queryByText("Update stopped. Nothing was changed.")).not.toBeInTheDocument();
  });

  it("handles undo button click", async () => {
    let undoHeadArg: string | null = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "undo_repo") {
        undoHeadArg = args.undoHead;
        return true;
      }
    });

    const onStateRefresh = vi.fn();
    // Simulate repo that has undo_head and update message
    const repo = createMockRepo({ undo_head: "prev_commit_sha" });
    render(RepoCard, { repo, onStateRefresh });

    // Directly set updateMessage by simulating a successful update first
    mockIPC((cmd) => {
      if (cmd === "update_repo") {
        return {
          success: true,
          downloads: 1,
          undo_head: "prev_commit_sha",
          conflict_files: [],
          message: "Updated successfully",
        };
      }
      if (cmd === "undo_repo") {
        undoHeadArg = "prev_commit_sha";
        return true;
      }
    });

    await fireEvent.click(screen.getByRole("button", { name: "Update" }));

    await waitFor(() => {
      expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    });

    const undoBtn = screen.getByRole("button", { name: "Undo" });
    await fireEvent.click(undoBtn);

    await waitFor(() => {
      expect(undoHeadArg).toBe("prev_commit_sha");
      expect(onStateRefresh).toHaveBeenCalled();
    });
  });

  it("toggles files changed list and clicks individual file links to open editor", async () => {
    let openedFile: string | null = null;
    mockIPC((cmd, args: any) => {
      if (cmd === "open_editor") {
        openedFile = args.filePath;
        return;
      }
    });

    const repo = createMockRepo();
    render(RepoCard, { repo, onStateRefresh: vi.fn() });

    const filesToggleBtn = screen.getByRole("button", { name: /2 files changed/i });
    expect(filesToggleBtn).toHaveAttribute("aria-expanded", "false");

    // Click files toggle to expand
    await fireEvent.click(filesToggleBtn);
    expect(filesToggleBtn).toHaveAttribute("aria-expanded", "true");

    const fileLink = screen.getByRole("button", { name: /src\/api\.ts/i });
    await fireEvent.click(fileLink);
    expect(openedFile).toBe("/home/user/projects/my-repo/src/api.ts");
  });
});
