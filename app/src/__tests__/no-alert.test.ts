/// <reference types="vite/client" />
import { describe, it, expect } from "vitest";

// Dialogs from alert() are not allowed: every message belongs on the card or screen it is about.
const sources = import.meta.glob(["../**/*.svelte", "../**/*.ts", "!../**/*.test.ts", "!../test-*.ts"], {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

describe("no alert dialogs", () => {
  it("scans the app source", () => {
    expect(Object.keys(sources).length).toBeGreaterThan(5);
  });

  it("never calls alert(), confirm() or prompt()", () => {
    const hits = Object.entries(sources)
      .filter(([, text]) => /(^|[^\w.])(window\.)?(alert|confirm|prompt)\s*\(/m.test(text))
      .map(([file]) => file);
    expect(hits).toEqual([]);
  });
});
