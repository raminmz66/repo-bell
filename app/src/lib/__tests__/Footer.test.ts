import { describe, it, expect, vi } from "vitest";
import { render, fireEvent, screen } from "@testing-library/svelte";
import Footer from "../Footer.svelte";

describe("Footer", () => {
  it("shows only Add folder and calls onAddFolder", async () => {
    const handleAddFolder = vi.fn();
    render(Footer, { onAddFolder: handleAddFolder });

    expect(screen.getAllByRole("button")).toHaveLength(1);
    await fireEvent.click(screen.getByRole("button", { name: /Add folder/i }));
    expect(handleAddFolder).toHaveBeenCalledTimes(1);
  });
});
