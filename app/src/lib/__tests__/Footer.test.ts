import { describe, it, expect, vi } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import Footer from "../Footer.svelte";

describe("Footer component button interactions", () => {
  it("clicking Add folder button triggers onAddFolder callback", async () => {
    const handleAddFolder = vi.fn();
    const handleMarkAllSeen = vi.fn();

    render(Footer, {
      onAddFolder: handleAddFolder,
      onMarkAllSeen: handleMarkAllSeen,
      hasNews: false,
    });

    const addBtn = screen.getByRole("button", { name: /Add folder/i });
    await fireEvent.click(addBtn);
    expect(handleAddFolder).toHaveBeenCalledTimes(1);
  });

  it("disables Mark all as seen button when hasNews is false", async () => {
    const handleAddFolder = vi.fn();
    const handleMarkAllSeen = vi.fn();

    render(Footer, {
      onAddFolder: handleAddFolder,
      onMarkAllSeen: handleMarkAllSeen,
      hasNews: false,
    });

    const markSeenBtn = screen.getByRole("button", { name: "Mark all as seen" });
    expect(markSeenBtn).toBeDisabled();
    await fireEvent.click(markSeenBtn);
    expect(handleMarkAllSeen).not.toHaveBeenCalled();
  });

  it("enables and triggers Mark all as seen button when hasNews is true", async () => {
    const handleAddFolder = vi.fn();
    const handleMarkAllSeen = vi.fn();

    render(Footer, {
      onAddFolder: handleAddFolder,
      onMarkAllSeen: handleMarkAllSeen,
      hasNews: true,
    });

    const markSeenBtn = screen.getByRole("button", { name: "Mark all as seen" });
    expect(markSeenBtn).toBeEnabled();
    await fireEvent.click(markSeenBtn);
    expect(handleMarkAllSeen).toHaveBeenCalledTimes(1);
  });
});
