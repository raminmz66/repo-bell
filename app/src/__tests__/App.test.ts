import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor, within } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import App from "../App.svelte";
import { makeRepo, makeFeed, makeState } from "../test-helpers";

const P = "/home/user/projects";

function newsState() {
  return makeState({
    headline: "2 repos have news, 1 can't be checked",
    news_repos: 2,
    news_changes: 5,
    error_repos: 1,
    repos: [
      makeRepo({ path: `${P}/web-app`, name: "web-app", behind: 4, new_count: 3, feed: makeFeed() }),
      makeRepo({ path: `${P}/api`, name: "api", behind: 2, new_count: 2, feed: makeFeed() }),
      makeRepo({ path: `${P}/docs`, name: "docs", error: "Git can't log in to the server. Your login may have expired.", error_kind: "login" }),
      makeRepo({ path: `${P}/quiet-one`, name: "quiet-one" }),
    ],
  });
}

describe("App", () => {
  beforeEach(() => clearMocks());
  afterEach(() => {
    clearMocks();
    vi.restoreAllMocks();
  });

  describe("navigation", () => {
    it("opens Settings, goes back with Back, and closes Settings with Escape", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState();
        if (cmd === "detect_editors") return ["VS Code"];
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
      expect(await screen.findByText("Folders and repos")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Back" }));
      const gear = await screen.findByRole("button", { name: "Settings" });
      await waitFor(() => expect(gear).toHaveFocus());

      await fireEvent.click(screen.getByRole("button", { name: "Settings" }));
      await screen.findByText("Folders and repos");
      await fireEvent.keyDown(window, { key: "Escape" });
      await screen.findByRole("button", { name: "Settings" });
    });
  });

  describe("loading", () => {
    it("shows an error with Try again when the state can't load, then recovers", async () => {
      let fail = true;
      mockIPC((cmd) => {
        if (cmd === "get_app_state") {
          if (fail) throw "RepoBell can't read its settings.";
          return makeState();
        }
      });
      render(App);
      expect(await screen.findByText("RepoBell can't read its settings.")).toBeInTheDocument();
      expect(screen.queryByText(/Loading RepoBell/)).not.toBeInTheDocument();
      fail = false;
      await fireEvent.click(screen.getByRole("button", { name: "Try again" }));
      expect(await screen.findByText("All quiet")).toBeInTheDocument();
    });
  });

  describe("news and errors", () => {
    it("uses the backend headline and shows the news count, not the behind count, on the badge", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      expect(await screen.findByRole("heading", { level: 1, name: "2 repos have news, 1 can't be checked" })).toBeInTheDocument();
      expect(screen.getByText("3 new")).toBeInTheDocument();
      expect(screen.queryByText("Behind by 4")).not.toBeInTheDocument();
      expect(screen.queryByText("4 new")).not.toBeInTheDocument();
    });

    it("puts Can't check first", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      const label = await screen.findByRole("heading", { level: 2, name: /^Can't check/ });
      const news = screen.getByRole("heading", { level: 2, name: /^News/ });
      const docs = screen.getByRole("heading", { level: 3, name: "docs" });
      expect(label.compareDocumentPosition(news) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(label.compareDocumentPosition(docs) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(screen.getAllByText("Can't check")).toHaveLength(1); // the label, no badge
      expect(docs.closest("li")!.textContent).not.toMatch(/\d+ new/);
    });

    it("shows only errors as the headline when there is no news", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({
            headline: "1 repo can't be checked",
            error_repos: 1,
            repos: [makeRepo({ name: "docs", error: "Can't reach the server.", error_kind: "offline" })],
          });
      });
      render(App);
      expect(await screen.findByRole("heading", { level: 1, name: "1 repo can't be checked" })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { level: 2, name: "1 repo can't be checked" })).not.toBeInTheDocument();
      expect(screen.getByRole("heading", { level: 2, name: /^Can't check/ })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /^News/ })).not.toBeInTheDocument();
    });

    it("Try again on an error card runs check_now", async () => {
      let checks = 0;
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return newsState();
        if (cmd === "check_now") { checks++; return newsState(); }
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Try again" }));
      await waitFor(() => expect(checks).toBe(1));
    });

    it("uses the crossed bell when there is only trouble and the plain bell when there is news", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({
            headline: "1 repo can't be checked",
            error_repos: 1,
            repos: [makeRepo({ path: `${P}/old`, name: "old", error: "Git can't log in to the server. Your login may have expired.", error_kind: "login" })],
          });
      });
      const { unmount } = render(App);
      await screen.findByRole("heading", { level: 1, name: "1 repo can't be checked" });
      expect(document.querySelector(".headline-row svg")).toHaveAttribute("data-icon", "bell-off");
      unmount();
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      await screen.findByText("3 new");
      expect(document.querySelector(".headline-row svg")).toHaveAttribute("data-icon", "bell");
    });

    it("Remove on a repo whose folder is gone stops watching it", async () => {
      let saved: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "get_app_state")
          return makeState({
            error_repos: 1,
            repos: [makeRepo({ path: `${P}/old`, name: "old", error: "Folder not found. It may have been moved or deleted.", error_kind: "folder_missing" })],
          });
        if (cmd === "save_settings") { saved = a.settings; return makeState({ settings: a.settings, repos: [] }); }
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Remove" }));
      await waitFor(() => expect(saved.ignored).toContain(`${P}/old`));
    });

    it("shows a Folder not found card for a missing watched folder and removes it", async () => {
      let removed: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "get_app_state")
          return makeState({
            folder_errors: [{ path: "/home/user/gone", message: "Folder not found. It may have been moved or deleted." }],
          });
        if (cmd === "remove_folder") { removed = a; return makeState({ folder_errors: [] }); }
      });
      render(App);
      expect(await screen.findByText("Folder not found. It may have been moved or deleted.")).toBeInTheDocument();
      expect(screen.getByRole("heading", { level: 1, name: "1 folder not found" })).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Remove" }));
      await waitFor(() => expect(removed).toEqual({ path: "/home/user/gone" }));
      await waitFor(() => expect(screen.queryByText(/Folder not found/)).not.toBeInTheDocument());
    });

    it("shows a failed Remove on the card, not in a dialog", async () => {
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ folder_errors: [{ path: "/home/user/gone", message: "Folder not found." }] });
        if (cmd === "remove_folder") throw "Couldn't save your settings: disk full";
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Remove" }));
      expect(await screen.findByText(/disk full/)).toBeInTheDocument();
      expect(alertSpy).not.toHaveBeenCalled();
    });

    it("Mark all as seen on the News label calls mark_all_seen and shows the cleared state", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return newsState();
        if (cmd === "mark_all_seen") return makeState();
      });
      render(App);
      const section = (await screen.findByRole("heading", { level: 2, name: /^News/ })).closest("section")!;
      expect(within(document.querySelector("footer")!).queryByRole("button", { name: "Mark all as seen" })).toBeNull();
      await fireEvent.click(within(section).getByRole("button", { name: "Mark all as seen" }));
      expect(await screen.findByText("All quiet")).toBeInTheDocument();
    });

    it("puts focus on Check now after Mark all as seen", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return newsState();
        if (cmd === "mark_all_seen") return makeState();
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Mark all as seen" }));
      await screen.findByText("All quiet");
      await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Check now" })));
    });

    it("shows a failed Mark all as seen in the window banner", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ repos: [makeRepo({ behind: 2, new_count: 2, feed: makeFeed() })] });
        if (cmd === "mark_all_seen") throw "Couldn't save. Try again.";
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Mark all as seen" }));
      expect(await screen.findByRole("alert")).toHaveTextContent("Couldn't save. Try again.");
    });
  });

  describe("groups", () => {
    const hint = "Update main refreshes main. Your files stay as they are.";
    const newsRepo = (o = {}) => makeRepo({ path: `${P}/web-app`, name: "web-app", behind: 2, new_count: 2, feed: makeFeed(), ...o });

    it("renders groups in order with counts", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({
            folder_errors: [{ path: "/home/user/gone", message: "Folder not found." }],
            repos: [
              makeRepo({ path: `${P}/docs`, name: "docs", error: "Can't reach the server.", error_kind: "offline" }),
              newsRepo(),
              makeRepo({ path: `${P}/behind`, name: "behind", behind: 2 }),
              makeRepo({ path: `${P}/side`, name: "side", on_main: false, branch: "feature-x" }),
              makeRepo({ path: `${P}/plain`, name: "plain" }),
            ],
          });
      });
      render(App);
      await screen.findByRole("button", { name: /Up to date 1 repo/ });
      const labels = screen.getAllByRole("heading", { level: 2 }).map((h) => h.textContent!.replace(/\s+/g, " ").trim());
      expect(labels).toEqual(["Can't check 2", "News 1", "On main 1", "On a branch 1"]);
    });

    it("leaves out On main and On a branch when empty", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return makeState({ repos: [newsRepo()] }); });
      render(App);
      await screen.findByRole("heading", { level: 2, name: /^News/ });
      expect(screen.queryByRole("heading", { name: /^On main/ })).not.toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /^On a branch/ })).not.toBeInTheDocument();
    });

    it("shows the side-branch hint once", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({
            repos: [
              makeRepo({ path: `${P}/a`, name: "a", on_main: false, branch: "x", main_behind: 2 }),
              makeRepo({ path: `${P}/b`, name: "b", on_main: false, branch: "y", main_behind: 2 }),
            ],
          });
      });
      render(App);
      await screen.findByRole("heading", { level: 2, name: /^On a branch/ });
      expect(screen.getAllByText(hint)).toHaveLength(1);
    });

    it("sorts On a branch: behind main first, then unsaved, then the rest", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({
            repos: [
              makeRepo({ path: `${P}/quiet`, name: "quiet", on_main: false, branch: "x" }),
              makeRepo({ path: `${P}/behind`, name: "behind", on_main: false, branch: "y", main_behind: 2 }),
              makeRepo({ path: `${P}/unsaved`, name: "unsaved", on_main: false, branch: "z", unsaved: true }),
            ],
          });
      });
      render(App);
      const section = (await screen.findByRole("heading", { level: 2, name: /^On a branch/ })).closest("section")!;
      const names = [...section.querySelectorAll("h3")].map((h) => h.textContent!.trim());
      expect(names).toEqual(["behind", "unsaved", "quiet"]);
    });

    it("subline says when it last checked", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      expect(await screen.findByText("Checked 2 min ago")).toBeInTheDocument();
      expect(screen.queryByText(/Since you last looked/)).not.toBeInTheDocument();
    });

    it("All quiet comes before the On a branch group", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({ repos: [makeRepo({ name: "side", on_main: false, branch: "feature-x", main_behind: 2 }), makeRepo({ path: `${P}/b`, name: "b" })] });
      });
      render(App);
      const quiet = await screen.findByText(/Nothing new in your 2 repos/);
      const group = screen.getByRole("heading", { name: /^On a branch/ });
      expect(quiet.compareDocumentPosition(group) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    });

    it("a news repo on a side branch sits in News", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ repos: [newsRepo({ on_main: false, branch: "feature-x", main_behind: 2 })] });
      });
      render(App);
      const section = (await screen.findByRole("heading", { level: 2, name: /^News/ })).closest("section")!;
      expect(within(section).getByRole("button", { name: "Update main" })).toBeInTheDocument();
      expect(within(section).getByText("feature-x")).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /^On a branch/ })).not.toBeInTheDocument();
    });

    it("a row with a stash alert stays out of Up to date until closed", async () => {
      const msg = "Updated, but your unsaved changes were put aside safely. Ask a developer to bring them back.";
      let updated = false;
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({ repos: [updated ? newsRepo({ new_count: 0, behind: 0, feed: null }) : newsRepo({ unsaved: true })] });
        if (cmd === "preview_update")
          return { kind: "fast_forward", downloads: 2, yours: 0, set_aside: true, message: "This will download 2 changes. Your unsaved changes are set aside and put back" };
        if (cmd === "update_repo") {
          updated = true;
          return { status: "edits_in_stash", message: msg, downloads: 2, conflict_files: [], plan: null, stash: "stash@{0}", stash_command: "git stash pop" };
        }
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Update" }));
      await fireEvent.click(await screen.findByRole("button", { name: "Set aside and update" }));
      const section = (await screen.findByRole("heading", { level: 2, name: /^On main/ })).closest("section")!;
      expect(await within(section).findByText(msg)).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Up to date/ })).not.toBeInTheDocument();
      await fireEvent.click(within(section).getByRole("button", { name: "Close" }));
      expect(await screen.findByRole("button", { name: /Up to date 1 repo/ })).toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /^On main/ })).not.toBeInTheDocument();
    });

    it("keeps a running action's result when the page switches from All quiet to the news view", async () => {
      const merged = { name: "old-one", sha: "aaa111", merged: true, last_commit_time: 1_700_000_000 };
      const rowA = () => makeRepo({ path: `${P}/a`, name: "a", old_branches: [merged] });
      let finish: (v: unknown) => void = () => {};
      mockIPC(
        (cmd) => {
          if (cmd === "get_app_state") return makeState({ repos: [rowA()] });
          if (cmd === "delete_old_branches") return new Promise((r) => (finish = r));
        },
        { shouldMockEvents: true },
      );
      render(App);
      // every repo is plain, so the page shows the centered All quiet
      expect(await screen.findByRole("heading", { level: 1, name: "All quiet" })).toBeInTheDocument();
      await fireEvent.click(await screen.findByRole("button", { name: /Up to date 1 repo/ }));
      await fireEvent.click(await screen.findByRole("button", { name: /1 old branch/ }));
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      expect(await screen.findByRole("button", { name: "Deleting…" })).toBeDisabled();
      // another repo gains news: the page switches to the news view
      await emit("state-updated", makeState({ repos: [rowA(), newsRepo({ path: `${P}/b`, name: "b" })] }));
      await screen.findByRole("heading", { level: 2, name: /^News/ });
      expect(screen.getByRole("button", { name: "Deleting…" })).toBeDisabled();
      finish({ status: "failed", message: "Couldn't delete the branch.", detail: null });
      expect(await screen.findByText("Couldn't delete the branch.")).toBeInTheDocument();
    });

    it("forgets a row's old notice when its repo is stopped and later comes back", async () => {
      const merged = { name: "old-one", sha: "aaa111", merged: true, last_commit_time: 1_700_000_000 };
      const row = (o = {}) => makeRepo({ path: `${P}/web-app`, name: "web-app", old_branches: [merged], ...o });
      mockIPC(
        (cmd) => {
          if (cmd === "get_app_state") return makeState({ repos: [row()] });
          if (cmd === "delete_old_branches") return { status: "failed", message: "Couldn't delete the branch.", detail: null };
          if (cmd === "save_settings") return makeState({ repos: [row({ ignored: true })] });
        },
        { shouldMockEvents: true },
      );
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: /Up to date 1 repo/ }));
      await fireEvent.click(await screen.findByRole("button", { name: /1 old branch/ }));
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      expect(await screen.findByText("Couldn't delete the branch.")).toBeInTheDocument();
      await emit("state-updated", makeState({ repos: [row({ error: "The folder is gone.", error_kind: "folder_missing" })] }));
      await fireEvent.click(await screen.findByRole("button", { name: "Remove" }));
      await waitFor(() => expect(screen.queryByText("web-app")).not.toBeInTheDocument());
      await emit("state-updated", makeState({ repos: [row()] }));
      expect(await screen.findByText("web-app")).toBeInTheDocument();
      expect(screen.queryByText("Couldn't delete the branch.")).not.toBeInTheDocument();
      expect(screen.queryByRole("heading", { name: /^On main/ })).not.toBeInTheDocument();
    });

    it("keeps a row in Up to date while its branch delete runs, then shows the result", async () => {
      const merged = { name: "old-one", sha: "aaa111", merged: true, last_commit_time: 1_700_000_000 };
      let finish: (v: unknown) => void = () => {};
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ repos: [makeRepo({ path: `${P}/web-app`, name: "web-app", old_branches: [merged] })] });
        if (cmd === "delete_old_branches") return new Promise((r) => (finish = r));
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: /Up to date 1 repo/ }));
      await fireEvent.click(await screen.findByRole("button", { name: /1 old branch/ }));
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      const deleting = await screen.findByRole("button", { name: "Deleting…" });
      expect(deleting).toBeDisabled();
      expect(screen.queryByRole("heading", { name: /^On main/ })).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: /Up to date 1 repo/ })).toBeInTheDocument();
      finish({ status: "failed", message: "Couldn't delete the branch.", detail: null });
      expect(await screen.findByText("Couldn't delete the branch.")).toBeInTheDocument();
      // a warn notice is an alert, so the row now sits under On main
      const section = screen.getByRole("heading", { level: 2, name: /^On main/ }).closest("section")!;
      expect(within(section).getByText("Couldn't delete the branch.")).toBeInTheDocument();
      // trying again from On main keeps the row there until the second delete ends
      await fireEvent.click(within(section).getByRole("button", { name: /1 old branch/ }));
      await fireEvent.click(await within(section).findByRole("button", { name: "Delete 1 branch" }));
      expect(await screen.findByRole("button", { name: "Deleting…" })).toBeDisabled();
      expect(screen.getByRole("heading", { level: 2, name: /^On main/ }).closest("section")).toContainElement(screen.getByRole("button", { name: "Deleting…" }));
      finish({ status: "failed", message: "Still couldn't delete.", detail: null });
      expect(await screen.findByText("Still couldn't delete.")).toBeInTheDocument();
    });

    it("unpins a row whose action was running when the Up to date list closed", async () => {
      const merged = { name: "old-one", sha: "aaa111", merged: true, last_commit_time: 1_700_000_000 };
      const row = (o = {}) => makeRepo({ path: `${P}/web-app`, name: "web-app", old_branches: [merged], ...o });
      mockIPC(
        (cmd) => {
          if (cmd === "get_app_state") return makeState({ repos: [row()] });
          if (cmd === "delete_old_branches") return new Promise(() => {});
        },
        { shouldMockEvents: true },
      );
      render(App);
      const accordion = await screen.findByRole("button", { name: /Up to date 1 repo/ });
      await fireEvent.click(accordion);
      await fireEvent.click(await screen.findByRole("button", { name: /1 old branch/ }));
      await fireEvent.click(screen.getByRole("button", { name: "Delete 1 branch" }));
      await screen.findByRole("button", { name: "Deleting…" });
      await fireEvent.click(accordion);
      await emit("state-updated", makeState({ repos: [row({ behind: 2, new_count: 2, feed: makeFeed() })] }));
      const news = await screen.findByRole("heading", { level: 2, name: /^News/ });
      expect(within(news.closest("section")!).getByText("web-app")).toBeInTheDocument();
    });

    it("keeps an On main row in place while its update runs, even if a check makes it plain", async () => {
      const row = (o = {}) => makeRepo({ path: `${P}/web-app`, name: "web-app", ...o });
      let finish: (v: unknown) => void = () => {};
      mockIPC(
        (cmd) => {
          if (cmd === "get_app_state") return makeState({ repos: [row({ behind: 2 })] });
          if (cmd === "preview_update") return new Promise((r) => (finish = r));
        },
        { shouldMockEvents: true },
      );
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Update" }));
      await emit("state-updated", makeState({ repos: [row()] }));
      await screen.findByText("All quiet");
      expect(screen.getByRole("heading", { level: 2, name: /^On main/ })).toBeInTheDocument();
      expect(screen.queryByRole("button", { name: /Up to date/ })).not.toBeInTheDocument();
      finish({ kind: "fast_forward", downloads: 2, yours: 0, set_aside: false, message: "This will download 2 changes" });
      expect(await screen.findByRole("group", { name: "Update preview" })).toBeInTheDocument();
    });
  });

  describe("up to date", () => {
    it("tucks plain up-to-date repos into an accordion of compact rows", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      const accordion = await screen.findByRole("button", { name: /Up to date 1 repo/ });
      expect(accordion).toHaveAttribute("aria-expanded", "false");
      expect(screen.queryByText("quiet-one")).not.toBeInTheDocument();
      await fireEvent.click(accordion);
      expect(accordion).toHaveAttribute("aria-expanded", "true");
      const row = document.querySelector(`[data-repo-path="${P}/quiet-one"]`)!;
      expect(row).not.toHaveClass("compact");
      expect(row).not.toHaveTextContent("Up to date");
      expect(row.querySelector("button")).toBeNull(); // no disclosure, no Update
    });

    it("shows All quiet with the last-checked time", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return makeState({ last_checked: Math.floor(Date.now() / 1000) - 120 }); });
      render(App);
      expect(await screen.findByText("All quiet")).toBeInTheDocument();
      expect(screen.getByText(/Your 1 repo is up to date\./)).toBeInTheDocument();
      expect(screen.getByText(/Last checked 2 min ago\./)).toBeInTheDocument();
    });

    it("keeps repos with something to act on visible, outside the accordion", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state")
          return makeState({ repos: [makeRepo({ name: "side", on_main: false, branch: "feature-x", main_behind: 2 }), makeRepo({ path: `${P}/b`, name: "b" })] });
      });
      render(App);
      expect(await screen.findByRole("button", { name: "Update main" })).toBeInTheDocument();
      expect(screen.getByText("feature-x")).toBeInTheDocument();
      expect(screen.getByText("· main 2 behind")).toBeInTheDocument();
      expect(screen.getByText(/Nothing new in your 2 repos/)).toBeInTheDocument();
      const title = screen.getByRole("heading", { level: 1, name: "All quiet" });
      const row = title.closest(".headline-row")!;
      expect(row).not.toBeNull();
      expect(row).toHaveTextContent("Nothing new in your 2 repos · Checked 2 min ago");
      expect(row.querySelector("svg")).toHaveAttribute("data-icon", "bell");
    });

    it("keeps the big centered All quiet when every repo is plain", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return makeState(); });
      render(App);
      const title = await screen.findByRole("heading", { level: 1, name: "All quiet" });
      expect(title.closest(".headline-row")).toBeNull();
      expect(title.closest(".empty-center")).not.toBeNull();
      expect(screen.getByText(/Your 1 repo is up to date\./)).toBeInTheDocument();
    });
  });

  describe("states", () => {
    it("offline: shows the message, last checked and retries with check_now", async () => {
      let called = false;
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ offline: true, last_checked: Math.floor(Date.now() / 1000) - 3600 });
        if (cmd === "check_now") { called = true; return makeState(); }
      });
      render(App);
      expect(await screen.findByText("Can't check your repos")).toBeInTheDocument();
      expect(screen.getByText(/last checked 1 h ago/)).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Try again" }));
      await waitFor(() => expect(called).toBe(true));
      expect(await screen.findByText("All quiet")).toBeInTheDocument();
    });

    it("first check at startup says Checking, not All quiet", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return makeState({ checking: true, last_checked: null }); });
      render(App);
      expect(await screen.findByText("Checking your repos…")).toBeInTheDocument();
      expect(screen.queryByText("All quiet")).not.toBeInTheDocument();
    });

    it("check_now that returns checking:true keeps the spinner until state-updated", async () => {
      mockIPC(
        (cmd) => {
          if (cmd === "get_app_state") return makeState();
          if (cmd === "check_now") return makeState({ checking: true });
        },
        { shouldMockEvents: true },
      );
      render(App);
      const btn = await screen.findByRole("button", { name: "Check now" });
      await fireEvent.click(btn);
      await waitFor(() => expect(btn).toBeDisabled());
      await emit("state-updated", makeState({ checking: false }));
      await waitFor(() => expect(btn).not.toBeDisabled());
    });

    it("no repos: offers Add folder and saves the picked folder", async () => {
      let saved: any = null;
      mockIPC((cmd, a: any) => {
        if (cmd === "get_app_state") return makeState({ repos: [] });
        if (cmd === "pick_folder") return "/home/user/picked";
        if (cmd === "save_settings") { saved = a.settings; return makeState({ settings: a.settings }); }
      });
      render(App);
      expect(await screen.findByText("No repos found yet. Add a folder to start watching.")).toBeInTheDocument();
      await fireEvent.click(screen.getAllByRole("button", { name: /Add folder/i })[0]);
      await waitFor(() => expect(saved.folders).toContain("/home/user/picked"));
    });

    it("all repos stopped: text and button both point to Settings", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") {
          return makeState({ repos: [makeRepo({ path: `${P}/a`, name: "a", ignored: true })] });
        }
        if (cmd === "detect_editors") return ["VS Code"];
      });
      render(App);
      expect(await screen.findByText("You're not watching any repos. Turn one on in Settings.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
      expect(await screen.findByText("Folders and repos")).toBeInTheDocument();
    });

    it("all repos stopped: text and button both point to Settings", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") {
          return makeState({ repos: [makeRepo({ path: `${P}/a`, name: "a", ignored: true })] });
        }
        if (cmd === "detect_editors") return ["VS Code"];
      });
      render(App);
      expect(await screen.findByText("You're not watching any repos. Turn one on in Settings.")).toBeInTheDocument();
      await fireEvent.click(screen.getByRole("button", { name: "Open Settings" }));
      expect(await screen.findByText("Folders and repos")).toBeInTheDocument();
    });

    it("shows first-run setup while first_run is set", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ first_run: true, repos: [] });
        if (cmd === "discover_suggested_folders") return [];
      });
      render(App);
      expect(await screen.findByText("Welcome to RepoBell")).toBeInTheDocument();
    });
  });

  describe("footer actions", () => {
    it("shows a failed action inline and never opens an alert dialog", async () => {
      const alertSpy = vi.spyOn(window, "alert").mockImplementation(() => {});
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ repos: [] });
        if (cmd === "pick_folder") throw "zenity isn't installed.";
      });
      render(App);
      await fireEvent.click((await screen.findAllByRole("button", { name: /Add folder/i }))[0]);
      expect(await screen.findByRole("alert")).toHaveTextContent("zenity isn't installed.");
      expect(alertSpy).not.toHaveBeenCalled();
    });
  });

  describe("open and closed rows", () => {
    const news = (n: number) =>
      makeState({
        repos: Array.from({ length: n }, (_, i) =>
          makeRepo({ path: `${P}/n${i}`, name: `n${i}`, behind: 2, new_count: 2, feed: makeFeed() }),
        ),
      });
    const toggle = (name: string) => screen.getByRole("button", { name: new RegExp(`^${name}\\b`) });
    const boot = async (state = news(3)) => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return state; }, { shouldMockEvents: true });
      render(App);
      await screen.findByRole("heading", { level: 2, name: /^News/ });
    };

    it("opens news rows when 2 or fewer have news", async () => {
      await boot(news(2));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "true");
      expect(toggle("n1")).toHaveAttribute("aria-expanded", "true");
    });

    it("keeps news rows closed when 3 or more have news", async () => {
      await boot(news(3));
      for (const n of ["n0", "n1", "n2"]) expect(toggle(n)).toHaveAttribute("aria-expanded", "false");
    });

    it("a row the user opened stays open on state-updated", async () => {
      await boot(news(3));
      await fireEvent.click(toggle("n0"));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "true");
      await emit("state-updated", news(3));
      await waitFor(() => expect(toggle("n0")).toHaveAttribute("aria-expanded", "true"));
    });

    it("a closed row stays closed on state-updated when 2 or fewer have news", async () => {
      await boot(news(2));
      await fireEvent.click(toggle("n0"));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "false");
      await emit("state-updated", news(2));
      await waitFor(() => expect(toggle("n1")).toHaveAttribute("aria-expanded", "true"));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "false");
    });

    it("a repo entering News opens only while News has 2 or fewer", async () => {
      await boot(news(1));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "true");
      await emit("state-updated", news(2));
      await waitFor(() => expect(toggle("n1")).toHaveAttribute("aria-expanded", "true"));
      await emit("state-updated", news(3));
      await waitFor(() => expect(toggle("n2")).toBeInTheDocument());
      expect(toggle("n2")).toHaveAttribute("aria-expanded", "false");
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "true");
      expect(toggle("n1")).toHaveAttribute("aria-expanded", "true");
    });

    it("a row that leaves News keeps its open state", async () => {
      await boot(news(3));
      await fireEvent.click(toggle("n0"));
      const next = news(3);
      next.repos[0] = makeRepo({ path: `${P}/n0`, name: "n0", behind: 2, new_count: 0, feed: makeFeed() });
      await emit("state-updated", next);
      const onMain = await screen.findByRole("heading", { level: 2, name: /^On main/ });
      const btn = within(onMain.closest("section")!).getByRole("button", { name: /^n0\b/ });
      expect(btn).toHaveAttribute("aria-expanded", "true");
    });

    it("window-shown opens both rows when 2 or fewer have news", async () => {
      await boot(news(2));
      await fireEvent.click(toggle("n0"));
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "false");
      await emit("window-shown");
      await waitFor(() => expect(toggle("n0")).toHaveAttribute("aria-expanded", "true"));
      expect(toggle("n1")).toHaveAttribute("aria-expanded", "true");
    });

    it("window-shown re-applies the rule", async () => {
      const state = news(3);
      state.repos.push(
        makeRepo({ path: `${P}/m`, name: "m", behind: 2, feed: makeFeed() }),
        makeRepo({ path: `${P}/plain`, name: "plain" }),
      );
      await boot(state);
      await fireEvent.click(toggle("n0"));
      await fireEvent.click(toggle("m"));
      const accordion = screen.getByRole("button", { name: /Up to date 1 repo/ });
      await fireEvent.click(accordion);
      expect(toggle("n0")).toHaveAttribute("aria-expanded", "true");
      expect(toggle("m")).toHaveAttribute("aria-expanded", "true");
      await emit("window-shown");
      await waitFor(() => expect(toggle("n0")).toHaveAttribute("aria-expanded", "false"));
      expect(toggle("m")).toHaveAttribute("aria-expanded", "false");
      expect(accordion).toHaveAttribute("aria-expanded", "true");
    });
  });

  describe("focus-repo event", () => {
    it("focus jump opens, scrolls and flashes the row", async () => {
      const scroll = vi.fn();
      Element.prototype.scrollIntoView = scroll;
      const state = makeState({
        repos: [0, 1, 2].map((i) => makeRepo({ path: `${P}/n${i}`, name: `n${i}`, behind: 2, new_count: 2, feed: makeFeed() })),
      });
      mockIPC((cmd) => { if (cmd === "get_app_state") return state; }, { shouldMockEvents: true });
      render(App);
      await screen.findByRole("heading", { level: 2, name: /^News/ });
      await emit("focus-repo", `${P}/n1`);
      await waitFor(() => expect(scroll).toHaveBeenCalled());
      const card = document.querySelector(`[data-repo-path="${P}/n1"]`)! as HTMLElement;
      expect(card).toHaveClass("highlight");
      expect(within(card).getByRole("button", { name: /^n1\b/ })).toHaveAttribute("aria-expanded", "true");
    });

    it("focus jump to a row that can't open still scrolls and flashes", async () => {
      const scroll = vi.fn();
      Element.prototype.scrollIntoView = scroll;
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); }, { shouldMockEvents: true });
      render(App);
      await screen.findByText("3 new");
      await emit("focus-repo", `${P}/docs`);
      await waitFor(() => expect(scroll).toHaveBeenCalled());
      const card = document.querySelector(`[data-repo-path="${P}/docs"]`)!;
      expect(card).toHaveClass("highlight");
      expect(card.querySelector("[aria-expanded]")).toBeNull();
    });

    it("opens the Up to date list when the repo is in it", async () => {
      Element.prototype.scrollIntoView = vi.fn();
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); }, { shouldMockEvents: true });
      render(App);
      await screen.findByText("3 new");
      await emit("focus-repo", `${P}/quiet-one`);
      await waitFor(() => expect(screen.getByText("quiet-one")).toBeInTheDocument());
    });
  });

  describe("check progress", () => {
    const progress = { done: 6, total: 15, current: "alpha" };
    const strip = () => document.querySelector(".check-strip");
    const sideBranch = makeRepo({ path: `${P}/side`, name: "side", on_main: false, branch: "feature-x", main_behind: 2 });
    const plain = makeRepo({ path: `${P}/plain`, name: "plain" });

    it.each([
      ["news view", () => ({ ...newsState(), progress })],
      ["compact All quiet", () => makeState({ progress, repos: [sideBranch, plain] })],
      ["big All quiet", () => makeState({ progress })],
      ["offline", () => makeState({ offline: true, progress })],
      ["first run", () => makeState({ checking: true, last_checked: null, progress })],
    ])("shows the strip in the %s", async (_name, make) => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return make(); });
      render(App);
      const bar = await screen.findByRole("progressbar", { name: "Check progress" });
      expect(strip()!.textContent).toContain("Checking alpha");
      expect(strip()!.textContent).toContain("6 of 15");
      expect(bar).toHaveAttribute("aria-valuemin", "0");
      expect(bar).toHaveAttribute("aria-valuemax", "15");
      expect(bar).toHaveAttribute("aria-valuenow", "6");
      expect(bar).toHaveAttribute("aria-valuetext", "6 of 15");
      expect((strip()!.querySelector(".strip-fill") as HTMLElement).style.width).toBe("40%");
    });

    it("shows the strip from a state-updated event and removes it when progress is null", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return makeState(); }, { shouldMockEvents: true });
      render(App);
      await screen.findByRole("heading", { level: 1, name: "All quiet" });
      expect(strip()).toBeNull();
      await emit("state-updated", makeState({ checking: true, progress }));
      await screen.findByRole("progressbar", { name: "Check progress" });
      await emit("state-updated", makeState({ progress: null }));
      await waitFor(() => expect(strip()).toBeNull());
    });

    it("no strip in Settings", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return makeState({ progress });
        if (cmd === "detect_editors") return ["VS Code"];
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Settings" }));
      await screen.findByText("Folders and repos");
      expect(screen.queryByRole("progressbar")).toBeNull();
    });
  });
});
