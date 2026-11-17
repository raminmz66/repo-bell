import { describe, it, expect, vi } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import Footer from "../Footer.svelte";
import { makeUpdate } from "../../test-helpers";

describe("Footer", () => {
  it("shows only Add folder and calls onAddFolder", async () => {
    const handleAddFolder = vi.fn();
    render(Footer, { onAddFolder: handleAddFolder });

    expect(screen.getAllByRole("button")).toHaveLength(1);
    await fireEvent.click(screen.getByRole("button", { name: /Add folder/i }));
    expect(handleAddFolder).toHaveBeenCalledTimes(1);
  });

  it("shows the ready button and calls onOpenAbout", async () => {
    const onOpenAbout = vi.fn();
    const update = makeUpdate({ status: "ready", available: { version: "0.2.0", notes: "" } });
    render(Footer, { onAddFolder: vi.fn(), update, onOpenAbout });

    await fireEvent.click(screen.getByRole("button", { name: "RepoBell 0.2.0 is ready" }));
    expect(onOpenAbout).toHaveBeenCalledTimes(1);
  });

  it("no button when the copy can't update", () => {
    const update = makeUpdate({ status: "ready", can_update: false, available: { version: "0.2.0", notes: "" } });
    render(Footer, { onAddFolder: vi.fn(), update });

    expect(screen.getAllByRole("button")).toHaveLength(1);
  });
});
