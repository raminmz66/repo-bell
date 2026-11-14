import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import RepoCard from "../RepoCard.svelte";
import { makeRepo, makeFeed } from "../../test-helpers";
import { shortTimeAgo } from "../util";
import type { PlanView, UpdateResult, OldBranch } from "../types";

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
    it("news row: badge with hover, no Behind", () => {
      render(RepoCard, { repo: newsRepo({ behind: 4, new_count: 3 }) });
      expect(screen.getByText("3 new")).toHaveAttribute("title", "Your team made 3 changes since you last looked");
      expect(screen.queryByText(/Behind by/)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Update" })).toBeInTheDocument();
    });

    it("has no news badge when only behind", () => {
      render(RepoCard, { repo: makeRepo({ behind: 2 }) });
      expect(screen.queryByText(/\bnew\b/)).not.toBeInTheDocument();
      expect(screen.getByText("Behind by 2")).toHaveAttribute("title", "The team made 2 changes you don't have yet");
    });

    describe("news line 2", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const by = (...names: string[]) =>
        names.map((n, i) => ({ author: n, title: `T${i}`, sha: `s${i}`, timestamp: t - i, mine: n === "me" }));
      const withChanges = (names: string[], files = ["a.ts", "b.ts"]) =>
        newsRepo({ feed: makeFeed({ total_changes: names.length, changes: by(...names), files_changed: files }) });

      it("names authors and files", () => {
        render(RepoCard, { repo: withChanges(["Sara", "Amine"]) });
        expect(screen.getByText("Sara, Amine · 2 files")).toBeInTheDocument();
      });

      it("says You for the user's own change", () => {
        render(RepoCard, { repo: withChanges(["me", "Amine"]) });
        expect(screen.getByText(/^You, Amine/)).toBeInTheDocument();
      });

      it("says 1 file for one, and leaves files out when there are none", () => {
        const { unmount } = render(RepoCard, { repo: withChanges(["Sara", "Amine"], ["a.ts"]) });
        expect(screen.getByText("Sara, Amine · 1 file")).toBeInTheDocument();
        unmount();
        render(RepoCard, { repo: withChanges(["Sara", "Amine"], []) });
        expect(screen.getByText("Sara, Amine")).toBeInTheDocument();
      });

      it("cuts to 2 names and counts the others", () => {
        const { unmount } = render(RepoCard, { repo: withChanges(["Sara", "Amine", "Bo", "Cy"]) });
        expect(screen.getByText("Sara, Amine and 2 others · 2 files")).toBeInTheDocument();
        unmount();
        render(RepoCard, { repo: withChanges(["Sara", "Amine", "Bo"]) });
        expect(screen.getByText("Sara, Amine and 1 other · 2 files")).toBeInTheDocument();
      });
    });

    it("shows Ahead with hover text and Unsaved", () => {
      render(RepoCard, { repo: newsRepo({ ahead: 1, unsaved: true }) });
      expect(screen.getByText("Ahead by 1")).toHaveAttribute("title", "You have 1 change the team doesn't have yet");
      expect(screen.getByText("Unsaved")).toHaveAttribute("title", "You edited files that aren't saved as a change yet");
    });

    it("shows the newest change time in short form", () => {
      render(RepoCard, { repo: newsRepo() });
      expect(screen.getByText("5 min")).toHaveAttribute("title", "Newest change 5 min ago");
      const now = Date.now();
      expect(shortTimeAgo(now / 1000 - 10, now)).toBe("now");
      expect(shortTimeAgo(now / 1000 - 3 * 3600, now)).toBe("3 h");
      expect(shortTimeAgo(null, now)).toBe("");
    });

    it("up-to-date row has no button and no status text", () => {
      const { container } = render(RepoCard, { repo: makeRepo() });
      expect(screen.getByText("my-repo")).toBeInTheDocument();
      expect(screen.queryByRole("button")).not.toBeInTheDocument();
      expect(screen.queryByText("Up to date")).not.toBeInTheDocument();
      expect(container.querySelector(".compact")).toBeNull();
    });

    it("a row that can't open has no toggle", () => {
      const { container, unmount } = render(RepoCard, {
        repo: makeRepo({ error: "Can't reach the server.", error_kind: "offline" }),
      });
      expect(container.querySelector("[aria-expanded]")).toBeNull();
      expect(container.querySelector('[role="button"]')).toBeNull();
      expect(screen.getByText("my-repo").closest("h3")).not.toBeNull();
      unmount();
      const second = render(RepoCard, { repo: makeRepo() });
      expect(second.container.querySelector("[aria-expanded]")).toBeNull();
      expect(second.container.querySelector('[role="button"]')).toBeNull();
      expect(screen.getByText("my-repo").closest("h3")).not.toBeNull();
    });

    it("old branches flag opens the list without toggling the row", async () => {
      const onToggle = vi.fn();
      const t = Math.floor(Date.now() / 1000) - 3 * 86400;
      const old = [
        { name: "a", sha: "1", merged: true, last_commit_time: t },
        { name: "b", sha: "2", merged: false, last_commit_time: t },
      ];
      mockIPC(() => {});
      render(RepoCard, { repo: makeRepo({ feed: makeFeed(), old_branches: old }), onToggle });
      const flag = screen.getByRole("button", { name: "2 old branches" });
      await fireEvent.click(flag);
      expect(flag).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByText("Merged into main")).toBeInTheDocument();
      expect(onToggle).not.toHaveBeenCalled();
    });

    it("only the toggle toggles the row", async () => {
      const onToggle = vi.fn();
      mockIPC((cmd) => { if (cmd === "preview_update") return fastForward; });
      render(RepoCard, { repo: newsRepo(), expanded: true, onToggle });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click(screen.getByText("Fix login timeout"));
      await fireEvent.click(await screen.findByRole("button", { name: "Cancel" }));
      expect(onToggle).not.toHaveBeenCalled();
      await fireEvent.click(screen.getByRole("button", { name: /my-repo/ }));
      expect(onToggle).toHaveBeenCalledTimes(1);
    });

    it("follows the expanded prop", () => {
      const { unmount } = render(RepoCard, { repo: newsRepo() });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("Fix login timeout")).not.toBeInTheDocument();
      unmount();
      render(RepoCard, { repo: newsRepo(), expanded: true });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "true");
      expect(screen.getByText("Fix login timeout")).toBeInTheDocument();
    });

    it("uses a real disclosure button inside the heading, with no div role=button", async () => {
      const onToggle = vi.fn();
      const { container, rerender } = render(RepoCard, { repo: newsRepo(), expanded: true, onToggle });
      expect(container.querySelector('[role="button"]')).toBeNull();
      const toggle = screen.getByRole("button", { name: /my-repo/ });
      expect(toggle.closest("h3")).not.toBeNull();
      expect(screen.getByText("Fix login timeout")).toBeInTheDocument();
      await fireEvent.click(toggle);
      expect(onToggle).toHaveBeenCalledTimes(1);
      await rerender({ expanded: false });
      expect(screen.queryByText("Fix login timeout")).not.toBeInTheDocument();
    });

    it("scrolls into view and flashes when highlighted", async () => {
      const scroll = vi.fn();
      Element.prototype.scrollIntoView = scroll;
      const { rerender, container } = render(RepoCard, { repo: makeRepo({ feed: makeFeed() }), highlight: false });
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "false");
      await rerender({ highlight: true });
      await waitFor(() => expect(scroll).toHaveBeenCalled());
      expect(screen.getByRole("button", { name: /my-repo/ })).toHaveAttribute("aria-expanded", "false");
      expect(container.querySelector(".row")).toHaveClass("highlight");
    });
  });

  describe("feed", () => {
    it("shows change titles but never commit hashes", () => {
      const { container } = render(RepoCard, { repo: newsRepo(), expanded: true });
      expect(screen.getByText("Fix login timeout")).toBeInTheDocument();
      expect(container.textContent).not.toContain("abc1234");
      expect(container.textContent).not.toMatch(/\b[0-9a-f]{7,40}\b/);
    });

    it('shows at most 3 titles and then "N more · N files changed"', () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = ["One", "Two", "Three", "Four", "Five"].map((title, i) => ({
        author: "Sara", title, sha: `aaaaaaa${i}`, timestamp: t - i, mine: false,
      }));
      render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 8, changes }) }), expanded: true });
      expect(screen.getByText("One")).toBeInTheDocument();
      expect(screen.getByText("Three")).toBeInTheDocument();
      expect(screen.queryByText("Four")).not.toBeInTheDocument();
      const more = screen.getByText("5 more");
      expect(more.parentElement?.textContent?.replace(/\s+/g, " ").trim()).toBe("5 more · 2 files changed");
    });

    it("shows no 'N more' when everything fits", () => {
      render(RepoCard, { repo: newsRepo(), expanded: true });
      expect(screen.queryByText(/\d+ more/)).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /2 files changed/ })).toBeInTheDocument();
    });

    it("has no summary line or day labels", () => {
      render(RepoCard, { repo: newsRepo(), expanded: true });
      expect(screen.queryByText("Alex added 2 changes to the main version")).not.toBeInTheDocument();
      expect(screen.queryByText("Today")).not.toBeInTheDocument();
      expect(screen.queryByText("Yesterday")).not.toBeInTheDocument();
    });

    it("prefixes the author only when titles have more than one author", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = [
        { author: "Sara", title: "A", sha: "1", timestamp: t, mine: false },
        { author: "Ali", title: "B", sha: "2", timestamp: t - 5, mine: false },
        { author: "me", title: "C", sha: "3", timestamp: t - 9, mine: true },
      ];
      const { unmount } = render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 3, changes }) }), expanded: true });
      expect(screen.getByText("Sara: A")).toBeInTheDocument();
      expect(screen.getByText("Ali: B")).toBeInTheDocument();
      expect(screen.getByText("You: C")).toBeInTheDocument();
      expect(screen.queryByText("Sara")).not.toBeInTheDocument();
      unmount();
      render(RepoCard, { repo: newsRepo(), expanded: true });
      expect(screen.getByText("Fix login timeout")).toBeInTheDocument();
      expect(screen.queryByText(/^Alex:/)).not.toBeInTheDocument();
    });

    it("lists authors and titles newest first, with each author once", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = [
        { author: "Ali", title: "Middle", sha: "1", timestamp: t - 10, mine: false },
        { author: "Sara", title: "Oldest", sha: "2", timestamp: t - 30, mine: false },
        { author: "Sara", title: "Newest", sha: "3", timestamp: t, mine: false },
      ];
      const repo = newsRepo({ feed: makeFeed({ total_changes: 3, changes, files_changed: [] }) });
      const { container } = render(RepoCard, { repo, expanded: true });
      expect(screen.getByText("Sara, Ali")).toBeInTheDocument();
      const titles = [...container.querySelectorAll(".change-item")].map((li) => li.textContent);
      expect(titles).toEqual(["Sara: Newest", "Ali: Middle", "Sara: Oldest"]);
    });

    it("puts a dot between 'N more' and the files toggle only when both show", () => {
      const t = Math.floor(Date.now() / 1000) - 100;
      const changes = ["One", "Two", "Three"].map((title, i) => ({ author: "Sara", title, sha: `a${i}`, timestamp: t - i, mine: false }));
      const meta = (c: HTMLElement) => c.querySelector(".feed-meta")!.textContent!.replace(/\s+/g, " ").trim();
      const onlyMore = render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 5, changes, files_changed: [] }) }), expanded: true });
      expect(meta(onlyMore.container)).toBe("2 more");
      onlyMore.unmount();
      const onlyFiles = render(RepoCard, { repo: newsRepo({ feed: makeFeed({ total_changes: 3, changes, files_changed: ["a.ts", "b.ts"] }) }), expanded: true });
      expect(meta(onlyFiles.container)).toBe("2 files changed");
    });

    it("lists files and opens one with { repo, file }", async () => {
      let args: any = null;
      mockIPC((cmd, a) => { if (cmd === "open_editor") args = a; });
      render(RepoCard, { repo: newsRepo(), expanded: true });
      const toggle = screen.getByRole("button", { name: /2 files changed/ });
      expect(toggle).toHaveAttribute("aria-expanded", "false");
      await fireEvent.click(toggle);
      await fireEvent.click(screen.getByRole("button", { name: /src\/api\.ts/ }));
      expect(args).toEqual({ repo: PATH, file: "src/api.ts" });
    });

    it("shows a failed editor open inline", async () => {
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockIPC((cmd) => { if (cmd === "open_editor") throw "The file is outside the repo."; });
      render(RepoCard, { repo: newsRepo(), expanded: true });
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

    const stashMsg = "Updated, but your unsaved changes were put aside safely. Ask a developer to bring them back.";
    async function runStashFlow(props: Record<string, unknown>, status: Partial<UpdateResult> = { status: "edits_in_stash", message: stashMsg, stash: "stash@{0}", stash_command: "git stash pop" }) {
      mockIPC((cmd) => {
        if (cmd === "preview_update") return setAside;
        if (cmd === "update_repo") return result(status);
      });
      const view = render(RepoCard, { repo: newsRepo({ unsaved: true }), ...props });
      await fireEvent.click(screen.getByRole("button", { name: "Update" }));
      await fireEvent.click(await screen.findByRole("button", { name: "Set aside and update" }));
      return view;
    }

    it("keeps panel state across a remount", async () => {
      const panels = new Map();
      const { unmount } = await runStashFlow({ panels });
      await screen.findByText(stashMsg);
      unmount();
      render(RepoCard, { repo: newsRepo({ unsaved: true }), panels });
      expect(screen.getByText(stashMsg)).toBeInTheDocument();
    });

    it("reports when an alert panel shows", async () => {
      const onAlertChange = vi.fn();
      const { unmount } = await runStashFlow({ onAlertChange });
      await screen.findByText(stashMsg);
      expect(onAlertChange).toHaveBeenLastCalledWith(PATH, true);
      await fireEvent.click(screen.getByRole("button", { name: "Close" }));
      expect(onAlertChange).toHaveBeenLastCalledWith(PATH, false);
      unmount();
      const second = vi.fn();
      await runStashFlow({ onAlertChange: second }, {});
      await screen.findByText("Updated. 2 changes downloaded.");
      expect(second).toHaveBeenLastCalledWith(PATH, false);
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

  describe("old branches", () => {
    const t = Math.floor(Date.now() / 1000) - 3 * 86400;
    const merged: OldBranch = { name: "feature/login", sha: "aaa111", merged: true, last_commit_time: t };
    const squashed: OldBranch = { name: "fix-typo", sha: "bbb222", merged: false, last_commit_time: t };
    const cleanupRepo = (o = {}) => makeRepo({ old_branches: [merged, squashed], ...o });

    it('shows "2 old branches" line and no news badge', () => {
      render(RepoCard, { repo: cleanupRepo() });
      const line = screen.getByRole("button", { name: "2 old branches" });
      expect(line).toHaveAttribute("title", "Your team deleted these branches on the server. You can delete your copies.");
      expect(screen.queryByText(/\bnew\b/)).not.toBeInTheDocument();
    });

    it("says 1 old branch for one", () => {
      render(RepoCard, { repo: makeRepo({ old_branches: [merged] }) });
      expect(screen.getByRole("button", { name: "1 old branch" })).toBeInTheDocument();
    });

    it("hides the line when the card has an error", () => {
      render(RepoCard, { repo: cleanupRepo({ error: "Can't reach the server", error_kind: "offline" }) });
      expect(screen.queryByText(/old branch/)).not.toBeInTheDocument();
    });

    it("opening the list asks for a fresh server answer", async () => {
      let args: any = null;
      mockIPC((cmd, a) => { if (cmd === "refresh_old_branches") args = a; });
      render(RepoCard, { repo: cleanupRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "2 old branches" }));
      await waitFor(() => expect(args).toEqual({ path: PATH }));
    });

    it("merged rows start ticked and may-not-be-merged rows start unticked", async () => {
      mockIPC(() => {});
      render(RepoCard, { repo: cleanupRepo() });
      await fireEvent.click(screen.getByRole("button", { name: "2 old branches" }));
      expect(screen.getByText("Merged into main")).toBeInTheDocument();
      expect(screen.getByText("May not be merged")).toBeInTheDocument();
      expect(screen.getByText(/Tick only the ones you're done with/)).toBeInTheDocument();
      expect(screen.getByRole("checkbox", { name: "feature/login" })).toBeChecked();
      expect(screen.getByRole("checkbox", { name: "fix-typo" })).not.toBeChecked();
      expect(screen.getAllByText("last change 3 d ago")).toHaveLength(2);
    });

    it("shows a group only when it has rows", async () => {
      mockIPC(() => {});
      render(RepoCard, { repo: makeRepo({ old_branches: [merged] }) });
      await fireEvent.click(screen.getByRole("button", { name: "1 old branch" }));
      expect(screen.queryByText("May not be merged")).not.toBeInTheDocument();
    });

    it("delete sends only the ticked branches with their sha", async () => {
      let args: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "delete_old_branches") { args = a; return { status: "deleted", message: "Deleted 2 branches. You can undo it.", detail: null }; }
      });
      const onStateRefresh = vi.fn();
      render(RepoCard, { repo: cleanupRepo(), onStateRefresh });
      await fireEvent.click(screen.getByRole("button", { name: "2 old branches" }));
      await fireEvent.click(screen.getByRole("checkbox", { name: "fix-typo" }));
      await fireEvent.click(screen.getByRole("button", { name: "Delete 2 branches" }));
      await waitFor(() => expect(onStateRefresh).toHaveBeenCalled());
      expect(args).toEqual({
        path: PATH,
        branches: [{ name: "feature/login", sha: "aaa111" }, { name: "fix-typo", sha: "bbb222" }],
      });
    });

    it("delete button is disabled when nothing is ticked", async () => {
      mockIPC(() => {});
      render(RepoCard, { repo: makeRepo({ old_branches: [squashed] }) });
      await fireEvent.click(screen.getByRole("button", { name: "1 old branch" }));
      expect(screen.getByRole("button", { name: "Delete branches" })).toBeDisabled();
    });

    it("undo banner shows when can_undo_cleanup and calls undo_cleanup", async () => {
      let args: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "undo_cleanup") { args = a; return { status: "undone", message: "Brought back 2 branches.", detail: null }; }
      });
      const onStateRefresh = vi.fn();
      render(RepoCard, { repo: makeRepo({ can_undo_cleanup: true }), onStateRefresh });
      expect(screen.getByText("RepoBell deleted branches here. You can undo it.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Undo branch delete" }));
      await waitFor(() => expect(onStateRefresh).toHaveBeenCalled());
      expect(args).toEqual({ path: PATH });
    });

    it("a ticked branch that moved is unticked and not sent", async () => {
      let args: any = null;
      mockIPC((cmd, a) => { if (cmd === "delete_old_branches") args = a; return { status: "deleted", message: "ok", detail: null }; });
      const { rerender } = render(RepoCard, { repo: cleanupRepo() });
      const line = screen.getByRole("button", { name: "2 old branches" });
      await fireEvent.click(line);
      expect(line).toHaveAttribute("aria-controls");
      await fireEvent.click(screen.getByRole("checkbox", { name: "fix-typo" }));
      expect(screen.getByRole("checkbox", { name: "fix-typo" })).toBeChecked();
      await rerender({ repo: cleanupRepo({ old_branches: [merged, { ...squashed, sha: "ccc333" }] }) });
      expect(screen.getByRole("checkbox", { name: "fix-typo" })).not.toBeChecked();
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      await waitFor(() => expect(args).not.toBeNull());
      expect(args.branches).toEqual([{ name: "feature/login", sha: "aaa111" }]);
    });

    it("a ticked branch that moved is unticked and not sent", async () => {
      let args: any = null;
      mockIPC((cmd, a) => { if (cmd === "delete_old_branches") args = a; return { status: "deleted", message: "ok", detail: null }; });
      const { rerender } = render(RepoCard, { repo: cleanupRepo() });
      const line = screen.getByRole("button", { name: "2 old branches" });
      await fireEvent.click(line);
      expect(line).toHaveAttribute("aria-controls");
      await fireEvent.click(screen.getByRole("checkbox", { name: "fix-typo" }));
      expect(screen.getByRole("checkbox", { name: "fix-typo" })).toBeChecked();
      await rerender({ repo: cleanupRepo({ old_branches: [merged, { ...squashed, sha: "ccc333" }] }) });
      expect(screen.getByRole("checkbox", { name: "fix-typo" })).not.toBeChecked();
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      await waitFor(() => expect(args).not.toBeNull());
      expect(args.branches).toEqual([{ name: "feature/login", sha: "aaa111" }]);
    });

    it("slash names render and are sent unchanged", async () => {
      let args: any = null;
      mockIPC((cmd, a) => {
        if (cmd === "delete_old_branches") { args = a; return { status: "deleted", message: "Deleted 1 branch. You can undo it.", detail: null }; }
      });
      render(RepoCard, { repo: makeRepo({ old_branches: [merged] }) });
      await fireEvent.click(screen.getByRole("button", { name: "1 old branch" }));
      expect(screen.getByText("feature/login")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      await waitFor(() => expect(args).not.toBeNull());
      expect(args.branches).toEqual([{ name: "feature/login", sha: "aaa111" }]);
    });
  });

  describe("side branch", () => {
    const side = () => makeRepo({ on_main: false, branch: "feature-x", behind: 3, ahead: 1, main_behind: 3 });

    it("names the branch and offers Update main instead of Update", () => {
      render(RepoCard, { repo: side() });
      expect(screen.getByText("· main 3 behind")).toBeInTheDocument();
      const hint = "Refreshes main in the background. Your files stay as they are.";
      expect(screen.getByRole("button", { name: "Update main" })).toHaveAttribute("title", hint);
      expect(screen.queryByRole("button", { name: "Update" })).not.toBeInTheDocument();
      expect(screen.queryByText(hint)).not.toBeInTheDocument();
      // the side branch's own behind/ahead numbers would mislead here
      expect(screen.queryByText(/Behind by/)).not.toBeInTheDocument();
      expect(screen.queryByText(/Ahead by/)).not.toBeInTheDocument();
    });

    it("cuts only the branch name: it has the full name on hover and the status sits in its own element", () => {
      render(RepoCard, { repo: makeRepo({ on_main: false, branch: "feature/a-very-long-branch-name", main_behind: 2 }) });
      const br = screen.getByText("feature/a-very-long-branch-name");
      expect(br).toHaveAttribute("title", "feature/a-very-long-branch-name");
      expect(br).toHaveClass("br");
      const st = screen.getByText("· main 2 behind");
      expect(st).toHaveClass("st");
      expect(st).not.toBe(br);
      expect(br.parentElement).toBe(st.parentElement);
    });

    it("a plain side-branch row has no branch flag", () => {
      const { container } = render(RepoCard, { repo: makeRepo({ on_main: false, branch: "feature-x", main_behind: 2 }) });
      expect(container.querySelector(".flag-branch")).toBeNull();
    });

    it("shows a quiet note instead of the button when main has nothing new", () => {
      render(RepoCard, { repo: makeRepo({ on_main: false, branch: "feature-x", main_behind: 0 }) });
      expect(screen.queryByRole("button", { name: "Update main" })).not.toBeInTheDocument();
      expect(screen.getByText("· main up to date")).toBeInTheDocument();
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
      render(RepoCard, { repo: makeRepo({ on_main: false, branch: null, main_behind: 3 }) });
      expect(screen.getByText("Not on a branch")).toBeInTheDocument();
      expect(screen.getByText("· main 3 behind")).toBeInTheDocument();
    });

    it("news row on a side branch", () => {
      render(RepoCard, { repo: newsRepo({ on_main: false, branch: "feature-x", main_behind: 2 }) });
      expect(screen.getByRole("button", { name: "Update main" })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: "Update" })).not.toBeInTheDocument();
      expect(screen.getByText("feature-x")).toHaveAttribute("title", "feature-x");
      expect(screen.getByText(/^Alex · 2 files$/)).toBeInTheDocument();
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

    it("shows the reason, no Can't check badge and Try again that checks", async () => {
      const onTryAgain = vi.fn();
      render(RepoCard, { repo: err(), onTryAgain });
      expect(screen.queryByText("Can't check")).not.toBeInTheDocument();
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
