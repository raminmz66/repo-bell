import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen, waitFor } from "@testing-library/svelte";
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
      makeRepo({ path: `${P}/web-app`, name: "web-app", behind: 4, new_count: 3, feed: makeFeed({ author_summary: "Sara added 3 changes to the main version" }) }),
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
      expect(screen.getByText("Behind by 4")).toBeInTheDocument();
      expect(screen.queryByText("4 new")).not.toBeInTheDocument();
    });

    it("keeps errors in their own section, not among the news", async () => {
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); });
      render(App);
      const section = await screen.findByRole("heading", { level: 2, name: "1 repo can't be checked" });
      const docs = screen.getByRole("heading", { level: 2, name: "docs" });
      const web = screen.getByRole("heading", { level: 2, name: /^web-app/ });
      // news cards come first, the error section after them
      expect(web.compareDocumentPosition(section) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      expect(section.compareDocumentPosition(docs) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
      // an errored repo has no news badge
      expect(docs.closest("article")!.textContent).not.toMatch(/\d+ new/);
      expect(screen.getByText("Can't check")).toBeInTheDocument();
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
      const row = screen.getByText("quiet-one").closest("article")!;
      expect(row).toHaveClass("compact");
      expect(row).toHaveTextContent("Up to date");
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
      expect(screen.getByText("You're on feature-x")).toBeInTheDocument();
      expect(screen.getByText(/Nothing new in your 2 repos/)).toBeInTheDocument();
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
    it("Mark all as seen calls mark_all_seen and shows the cleared state", async () => {
      mockIPC((cmd) => {
        if (cmd === "get_app_state") return newsState();
        if (cmd === "mark_all_seen") return makeState();
      });
      render(App);
      await fireEvent.click(await screen.findByRole("button", { name: "Mark all as seen" }));
      expect(await screen.findByText("All quiet")).toBeInTheDocument();
    });

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

  describe("focus-repo event", () => {
    it("scrolls to and highlights that card", async () => {
      const scroll = vi.fn();
      Element.prototype.scrollIntoView = scroll;
      mockIPC((cmd) => { if (cmd === "get_app_state") return newsState(); }, { shouldMockEvents: true });
      render(App);
      await screen.findByText("3 new");
      await emit("focus-repo", `${P}/api`);
      await waitFor(() => expect(scroll).toHaveBeenCalled());
      const card = document.querySelector(`[data-repo-path="${P}/api"]`)!;
      expect(card).toHaveClass("highlight");
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
});
