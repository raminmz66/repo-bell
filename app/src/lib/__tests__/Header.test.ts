import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import { mockIPC, clearMocks } from "@tauri-apps/api/mocks";
import Header from "../Header.svelte";

describe("Header component button interactions", () => {
  beforeEach(() => {
    clearMocks();
  });

  afterEach(() => {
    clearMocks();
  });

  it("renders title correctly", () => {
    render(Header, { title: "Custom Title" });
    expect(screen.getByText("Custom Title")).toBeInTheDocument();
  });

  it("clicking Close window button invokes hide_window", async () => {
    let hideInvoked = false;
    mockIPC((cmd) => {
      if (cmd === "hide_window") {
        hideInvoked = true;
        return;
      }
    });

    render(Header, { title: "RepoBell" });
    const closeBtn = screen.getByRole("button", { name: "Close window" });
    await fireEvent.click(closeBtn);

    expect(hideInvoked).toBe(true);
  });

  it("clicking Check now button triggers onCheckNow callback", async () => {
    const handleCheckNow = vi.fn();
    render(Header, {
      title: "RepoBell",
      onCheckNow: handleCheckNow,
      isChecking: false,
    });

    const checkBtn = screen.getByRole("button", { name: "Check now" });
    await fireEvent.click(checkBtn);
    expect(handleCheckNow).toHaveBeenCalledTimes(1);
  });

  it("disables Check now button while isChecking is true", async () => {
    const handleCheckNow = vi.fn();
    render(Header, {
      title: "RepoBell",
      onCheckNow: handleCheckNow,
      isChecking: true,
    });

    const checkBtn = screen.getByRole("button", { name: "Check now" });
    expect(checkBtn).toBeDisabled();
    await fireEvent.click(checkBtn);
    expect(handleCheckNow).not.toHaveBeenCalled();
  });

  it("clicking Settings button triggers onOpenSettings callback", async () => {
    const handleOpenSettings = vi.fn();
    render(Header, {
      title: "RepoBell",
      onOpenSettings: handleOpenSettings,
    });

    const settingsBtn = screen.getByRole("button", { name: "Settings" });
    await fireEvent.click(settingsBtn);
    expect(handleOpenSettings).toHaveBeenCalledTimes(1);
  });

  it("shows Back button when showBack is true and triggers onBack callback", async () => {
    const handleBack = vi.fn();
    render(Header, {
      title: "Settings",
      showBack: true,
      onBack: handleBack,
    });

    const backBtn = screen.getByRole("button", { name: "Back" });
    await fireEvent.click(backBtn);
    expect(handleBack).toHaveBeenCalledTimes(1);
  });
});
