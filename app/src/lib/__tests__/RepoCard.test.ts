import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import RepoCard from "../RepoCard.svelte";
import { makeRepo, makeFeed } from "../../test-helpers";
import type { PlanView, UpdateResult } from "../types";

const PATH = "/home/user/projects/my-repo";

function newsRepo(overrides = {}) {
  return makeRepo({ behind: 2, new_count: 2, feed: makeFeed(), ...overrides });
}

const fastForward: PlanView = { kind: "fast_forward", downloads: 2, yours: 0, set_aside: false, message: "This will download 2 changes" };
const setAside: PlanView = { kind: "fast_forward", downloads: 2, yours: 0, set_aside: true, message: "This will download 2 changes. Your unsaved changes are set aside and put back" };
const combine: PlanView = { kind: "combine", downloads: 2, yours: 1, set_aside: false, message: "This will download 2 changes and combine them with your 1 change" };

function result(overrides: Partial<UpdateResult>): UpdateResult {
  return { status: "updated", message: "Updated. 2 changes downloaded.", downloads: 2, conflict_files: [], plan: null, stash: null, stash_command: null, ...overrides };
}

describe("RepoCard", () => {
  beforeEach(() => clearMocks());
  afterEach(() => clearMocks());

  describe("header and badges", () => {
    it("shows the news count as the badge and Behind as a quiet status next to Update", () => {
      render(RepoCard, { repo: newsRepo({ behind: 4, new_count: 3 }) });
      const badge = screen.getByText("3 new");
      expect(badge).toBeInTheDocument();
      const behind = screen.getByText("Behind by 4");
      expect(behind).toHaveAttribute("title", "The team made 4 changes you don't have yet");
      // the two numbers are different elements and the badge never says "Behind"
      expect(badge).not.toBe(behind);
      expect(badge.textContent).not.toMatch(/behind/i);
      expect(screen.getByRole("button", { name: "Update" })).toBeInTheDocument();
    });

    it("has no news badge when only behind", () => {
      render(RepoCard, { repo: makeRepo({ behind: 2 }) });
      expect(screen.queryByText(/\bnew\b/)).not.toBeInTheDocument();
      expect(screen.getByText("Behind by 2")).toBeInTheDocument();
    });

    it("shows Ahead with hover text and Has unsaved changes", () => {
      render(RepoCard, { repo: newsRepo({ ahead: 1, unsaved: true }) });
      expect(screen.getByText("Ahead by 1")).toHaveAttribute("title", "You have 1 change the team doesn't have yet");
      expect(screen.getByText("Has unsaved changes")).toBeInTheDocument();
    });

    it("renders an up-to-date repo as a compact row without a disclosure button", () => {
      const { container } = render(RepoCard, { repo: makeRepo() });
      expect(screen.getByText("my-repo")).toBeInTheDocument();
      expect(screen.getByText("Up to date")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      expect(container.querySelector(".repo-card")).toHaveClass("compact");
    });

    it("starts a news card expanded and a quiet one with own changes collapsed", () => {
      const { unmount } = render(RepoCard, { repo: newsRepo() });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "true");
      unmount();
      render(RepoCard, { repo: makeRepo({ feed: makeFeed() }) });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "false");
    });

    it("uses a real disclosure button inside the heading, with no div role=button", async () => {
      const { container } = render(RepoCard, { repo: newsRepo() });
      expect(container.querySelector('[role="button"]')).toBeNull();
      const toggle = screen.getByRole("button", { name: /my-repo/ });
      expect(toggle.closest("h2")).not.toBeNull();
      expect(screen.getByText("Alex added 2 changes to the main version")).toBeInTheDocument();
      await fireEvent.click(toggle);
      expect(toggle).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("Alex added 2 changes to the main version")).not.toBeInTheDocument();
      await fireEvent.click(toggle);
      expect(screen.getByText("Alex added 2 changes to the main version")).toBeInTheDocument();
    });

    it("expands and scrolls into view when highlighted", async () => {
      const scroll = vi.fn();
      Element.prototype.scrollIntoView = scroll;
      const { rerender, container } = render(RepoCard, { repo: makeRepo({ feed: makeFeed() }), highlight: false });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "false");
      await rerender({ highlight: true });
      await waitFor(() => expect(scroll).toHaveBeenCalled());
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "true");
      expect(container.querySelector(".repo-card")).toHaveClass("highlight");
    });
  });

  describe("feed", () => {
    it("shows change titles but never commit hashes", () => {
      const { container } = render(RepoCard, { repo: newsRepo() });
      expect(screen.getByText("Fix login timeout")).toBeInTheDocument();
      expect(container.textContent).not.toContain("abc1234");
      expect(container.textContent).not.toMatch(/\b[0-9a-f]{7,40}\b/);
    });

    it("shows at most 3 titles and then 'and N more'", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = ["One", "Two", "Three", "Four", "Five"].map((title, i) => ({
        author: "Sara", title, sha: `aaaaaaa${i}`, timestamp: t - i, mine: false,
      }));
      render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 8, changes }) }) });
      expect(screen.getByText("One")).toBeInTheDocument();
      expect(screen.getByText("Three")).toBeInTheDocument();
      expect(screen.queryByText("Four")).not.toBeInTheDocument();
      expect(screen.getByText("and 5 more")).toBeInTheDocument();
    });

    it("shows no 'and N more' when everything fits", () => {
      render(RepoCard, { repo: newsRepo() });
      expect(screen.queryByText(/and \d+ more/)).not.toBeInTheDocument();
    });

    it("groups by person when several people changed things", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = [
        { author: "Sara", title: "A", sha: "1", timestamp: t, mine: false },
        { author: "Ali", title: "B", sha: "2", timestamp: t - 5, mine: false },
        { author: "me", title: "C", sha: "3", timestamp: t - 9, mine: true },
      ];
      render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 3, changes }) }) });
      expect(screen.getByText("Sara")).toBeInTheDocument();
      expect(screen.getByText("Ali")).toBeInTheDocument();
      expect(screen.getByText("You")).toBeInTheDocument();
    });

    it("lists files and opens one with { repo, file }", async () => {
      let args: any = null;
      mockIPC((cmd, a) => { if (cmd === "open_editor") args = a; });
      render(RepoCard, { repo: newsRepo() });
      const toggle = screen.getByRole("button", { name: /2 files changed/ });
      expect(toggle).toHaveAttribute("aria-expanded", "false");
      await fireEvent.click(toggle);
      await fireEvent.click(screen.getByRole("button", { name: /src\/api\.ts/ }));
      expect(args).toEqual({ repo: PATH, file: "src/api.ts" });
    });

    it("shows a failed editor open inline", async () => {
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockIPC((cmd) => { if (cmd === "open_editor") throw "The file is outside the repo."; });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: /2 files changed/ }));
      await fireEvent.click(screen.getByRole("button", { name: /README\.md/ }));
      await waitFor(() => expect(screen.getByRole("alert")).toHaveTextContent("The file is outside the repo."));
      expect(alertSpy).not.toHaveBeenCalled();
    });
  });

  describe("update flow", () => {
    it("previews, asks, then updates with the same plan", async () => {
      let updateArgs: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "preview_update") return fastForward;
        if (cmd === "update_repo") { updateArgs = a; return result({}); }
      });
      const onStateRefresh = vi.fn();
      render(RepoCard, { repo: newsRepo(), onStateRefresh });

      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await waitFor(() => expect(screen.getByText("This will download 2 changes.")).toBeInTheDocument());
      expect(updateArgs).toBeNull();

      await fireEvent.click(screen.getAllByRole("button", { name: "Update" }).slice(-1)[0]);
      await waitFor(() => expect(updateArgs).toEqual({ path: PATH, expected: fastForward }));
      await waitFor(() => expect(screen.getByText("Updated. 2 changes downloaded.")).toBeInTheDocument());
      expect(onStateRefresh).toHaveBeenCalled();
    });

    it("cancel runs nothing", async () => {
      const calls: string[] = [];
      mockIPC((cmd) => { calls.push(cmd); if (cmd === "preview_update") return fastForward; });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
      expect(calls).toEqual(["preview_update"]);
      expect(screen.queryByText("This will download 2 changes.")).not.toBeInTheDocument();
    });

    it("asks before setting unsaved changes aside", async () => {
      let sent: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "preview_update") return setAside;
        if (cmd === "update_repo") { sent = a; return result({}); }
      });
      render(RepoCard, { repo: newsRepo({ unsaved: true }) });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      expect(await screen.findByText("You have unsaved changes.")).toBeInTheDocument();
      expect(screen.getByText("Set them aside, update, then bring them back?")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Set aside and update" }));
      await waitFor(() => expect(sent.expected).toEqual(setAside));
    });

    it("asks before combining your work with the team's", async () => {
      let sent: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "preview_update") return combine;
        if (cmd === "update_repo") { sent = a; return result({}); }
      });
      render(RepoCard, { repo: newsRepo({ ahead: 1 }) });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      expect(await screen.findByText("Your work and the team's work both changed.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Update and combine" }));
      await waitFor(() => expect(sent.expected).toEqual(combine));
    });

    it("shows the new preview and asks again when the plan changed", async () => {
      let n = 0;
      mockIPC((cmd) => {
        if (cmd === "preview_update") return fastForward;
        if (cmd === "update_repo") {
          n++;
          return n === 1
            ? result({ status: "plan_changed", message: "This repo changed while you were looking. Check the new preview.", plan: combine })
            : result({ message: "Updated and combined." });
        }
      });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click((await screen.findAllByRole("button", { name: "Update" })).slice(-1)[0]);
      expect(await screen.findByText("This repo changed while you were looking. Check the new preview.")).toBeInTheDocument();
      expect(screen.getByText("This will download 2 changes and combine them with your 1 change.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Update and combine" }));
      await waitFor(() => expect(screen.getByText("Updated and combined.")).toBeInTheDocument());
    });

    it("shows blocked previews as a message with no run button", async () => {
      mockIPC((cmd) => {
        if (cmd === "preview_update") return { kind: "in_progress", downloads: 0, yours: 0, set_aside: false, message: "Another update is half-finished in this repo. Ask a developer to finish it first" };
      });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      expect(await screen.findByText(/half-finished/)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "OK" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Cancel" })).not.toBeInTheDocument();
    });

    it("lists conflict files and opens them with { repo, file }", async () => {
      let opened: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "preview_update") return fastForward;
        if (cmd === "update_repo") return result({ status: "conflict", message: "Update stopped. Nothing was changed.", conflict_files: ["src/api.ts"] });
        if (cmd === "open_editor") opened = a;
      });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click((await screen.findAllByRole("button", { name: "Update" })).slice(-1)[0]);
      expect(await screen.findByText("Update stopped. Nothing was changed.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Open in editor" }));
      expect(opened).toEqual({ repo: PATH, file: "src/api.ts" });
      await fireEvent.click(screen.getByRole("button", { name: "Close" }));
      expect(screen.queryByText("Update stopped. Nothing was changed.")).not.toBeInTheDocument();
    });

    it("shows the stash message prominently", async () => {
      const msg = "Updated, but your unsaved changes were put aside safely. Ask a developer to bring them back.";
      mockIPC((cmd) => {
        if (cmd === "preview_update") return setAside;
        if (cmd === "update_repo") return result({ status: "edits_in_stash", message: msg, stash: "stash@{0}", stash_command: "git stash pop" });
      });
      render(RepoCard, { repo: newsRepo({ unsaved: true }) });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click(await screen.findByRole("button", { name: "Set aside and update" }));
      const alert = await screen.findByRole("alert");
      expect(alert).toHaveTextContent(msg);
      // Git's own name for the stash sits behind Details
      expect(screen.getByText("Details")).toBeInTheDocument();
      expect(alert).toHaveTextContent("stash@{0}");
      expect(alert).toHaveTextContent("git stash pop");
      expect(screen.getByText(msg).textContent).not.toMatch(/git|`/);
    });

    it("Escape closes the preview and focus goes back to Update", async () => {
      mockIPC((cmd) => { if (cmd === "preview_update") return fastForward; });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      const box = await screen.findByRole("group", { name: "Update preview" });
      await fireEvent.keyDown(box, { key: "Escape" });
      await waitFor(() => expect(screen.queryByRole("group", { name: "Update preview" })).not.toBeInTheDocument());
      await waitFor(() => expect(screen.getByRole("button", { name: "Update" })).toHaveFocus());
    });

    it("Cancel and a finished update also put focus back on Update", async () => {
      mockIPC((cmd) => {
        if (cmd === "preview_update") return fastForward;
        if (cmd === "update_repo") return result({ status: "up_to_date", message: "Already up to date." });
      });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
      await waitFor(() => expect(screen.getByRole("button", { name: "Update" })).toHaveFocus());
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click((await screen.findAllByRole("button", { name: "Update" })).slice(-1)[0]);
      await screen.findByText("Already up to date.");
      await waitFor(() => expect(screen.getByRole("button", { name: "Update" })).toHaveFocus());
    });

    it("shows a rejected update inline instead of an alert dialog", async () => {
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockIPC((cmd) => {
        if (cmd === "preview_update") return fastForward;
        if (cmd === "update_repo") throw "RepoBell isn't watching that repo.";
      });
      render(RepoCard, { repo: newsRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click((await screen.findAllByRole("button", { name: "Update" })).slice(-1)[0]);
      expect(await screen.findByText("RepoBell isn't watching that repo.")).toBeInTheDocument();
      expect(alertSpy).not.toHaveBeenCalled();
    });
  });

  describe("undo", () => {
    it("shows Undo only when the backend says it's possible", () => {
      const { unmount } = render(RepoCard, { repo: newsRepo({ can_undo: false }) });
      expect(screen.queryByRole("button", { name: "Undo" })).not.toBeInTheDocument();
      unmount();
      render(RepoCard, { repo: newsRepo({ can_undo: true }) });
      expect(screen.getByRole("button", { name: "Undo" })).toBeInTheDocument();
    });

    it("calls undo_repo with only the path", async () => {
      let args: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "undo_repo") { args = a; return { status: "undone", message: "Undone. The repo is back where it was.", detail: null }; }
      });
      const onStateRefresh = vi.fn();
      render(RepoCard, { repo: newsRepo({ can_undo: true }), onStateRefresh });
      await fireEvent.click(screen.getByRole("button", { name: "Undo" }));
      await waitFor(() => expect(onStateRefresh).toHaveBeenCalled());
      expect(args).toEqual({ path: PATH });
    });

    it("explains when undo is no longer possible", async () => {
      mockIPC((cmd) => {
        if (cmd === "undo_repo") return { status: "not_possible", message: "Undo isn't possible because the repo changed since the update. Nothing was changed.", detail: null };
      });
      render(RepoCard, { repo: newsRepo({ can_undo: true }) });
      await fireEvent.click(screen.getByRole("button", { name: "Undo" }));
      expect(await screen.findByText(/Undo isn't possible/)).toBeInTheDocument();
    });
  });

  describe("side branch", () => {
    const side = () => makeRepo({ on_main: false, branch: "feature-x", behind: 3, ahead: 1, main_behind: 3 });

    it("names the branch and offers Update main instead of Update", () => {
      render(RepoCard, { repo: side() });
      expect(screen.getByText("You're on feature-x")).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Update main" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Update" })).not.toBeInTheDocument();
      expect(screen.getByText("Refreshes the main version in the background. Your files stay as they are.")).toBeInTheDocument();
      // the side branch's own behind/ahead numbers would mislead here
      expect(screen.queryByText(/Behind by/)).not.toBeInTheDocument();
      expect(screen.queryByText(/Ahead by/)).not.toBeInTheDocument();
    });

    it("shows a quiet note instead of the button when main has nothing new", () => {
      render(RepoCard, { repo: makeRepo({ on_main: false, branch: "feature-x", main_behind: 0 }) });
      expect(screen.queryByRole("button", { name: "Update main" })).not.toBeInTheDocument();
      expect(screen.getByText("Main is up to date")).toBeInTheDocument();
      expect(screen.queryByText(/Refreshes the main version/)).not.toBeInTheDocument();
    });

    it("asks the window to refresh after Update main", async () => {
      const onStateRefresh = vi.fn();
      mockIPC((cmd) => {
        if (cmd === "update_main") return { status: "updated", message: "Main is up to date now. Your files and branch weren't touched.", detail: null };
      });
      render(RepoCard, { repo: side(), onStateRefresh });
      await fireEvent.click(screen.getByRole("button", { name: "Update main" }));
      await waitFor(() => expect(onStateRefresh).toHaveBeenCalled());
    });

    it("says so when HEAD isn't on a branch", () => {
      render(RepoCard, { repo: makeRepo({ on_main: false, branch: null }) });
      expect(screen.getByText("You're not on a branch")).toBeInTheDocument();
    });

    it("calls update_main and shows its message", async () => {
      let args: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "update_main") { args = a; return { status: "updated", message: "Main is up to date now. Your files and branch weren't touched.", detail: null }; }
      });
      render(RepoCard, { repo: side() });
      await fireEvent.click(screen.getByRole("button", { name: "Update main" }));
      expect(await screen.findByText(/Main is up to date now/)).toBeInTheDocument();
      expect(args).toEqual({ path: PATH });
    });

    it("keeps Git's own words behind a Details link", async () => {
      mockIPC((cmd) => {
        if (cmd === "update_main") return { status: "failed", message: "Couldn't update main. Ask a developer to take a look.", detail: "fatal: not possible to fast-forward" };
      });
      render(RepoCard, { repo: side() });
      await fireEvent.click(screen.getByRole("button", { name: "Update main" }));
      expect(await screen.findByText(/Couldn't update main/)).toBeInTheDocument();
      expect(screen.queryByText(/fatal:/)).not.toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Details" }));
      expect(screen.getByText(/fatal: not possible/)).toBeInTheDocument();
    });

    it("shows the main-has-changes message", async () => {
      mockIPC((cmd) => {
        if (cmd === "update_main") return { status: "main_has_changes", message: "Your main has changes the team doesn't have yet, so it can't be moved forward. Ask a developer to take a look.", detail: null };
      });
      render(RepoCard, { repo: side() });
      await fireEvent.click(screen.getByRole("button", { name: "Update main" }));
      expect(await screen.findByText(/Ask a developer to take a look/)).toBeInTheDocument();
    });
  });

  describe("error card", () => {
    const err = (o = {}) => makeRepo({ error: "Git can't log in to the server. Your login may have expired.", error_kind: "login", ...o });

    it("shows the reason, a Can't check badge and Try again that checks", async () => {
      const onTryAgain = vi.fn();
      render(RepoCard, { repo: err(), onTryAgain });
      expect(screen.getByText("Can't check")).toBeInTheDocument();
      expect(screen.getByText(/can't log in/)).toBeInTheDocument();
      expect(screen.queryByText(/new$/)).not.toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Try again" }));
      expect(onTryAgain).toHaveBeenCalledTimes(1);
    });

    it("disables Try again while a check runs", () => {
      render(RepoCard, { repo: err(), checking: true });
      expect(screen.getByRole("button", { name: "Checking…" })).toBeDisabled();
    });

    it("Remove stops watching a repo whose folder is gone", async () => {
      const onStopWatching = vi.fn().mockResolvedValue(undefined);
      render(RepoCard, { repo: err({ error: "Folder not found. It may have been moved or deleted.", error_kind: "folder_missing" }), onStopWatching });
      expect(screen.queryByRole("button", { name: "Try again" })).not.toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Remove" }));
      expect(onStopWatching).toHaveBeenCalledWith(PATH);
    });

    it("shows a failed Remove inline", async () => {
      const onStopWatching = vi.fn().mockRejectedValue("Couldn't save your settings: disk full");
      render(RepoCard, { repo: err({ error_kind: "not_a_repo", error: "This folder is no longer a Git repo." }), onStopWatching });
      await fireEvent.click(screen.getByRole("button", { name: "Remove" }));
      expect(await screen.findByText(/disk full/)).toBeInTheDocument();
    });
  });
});
