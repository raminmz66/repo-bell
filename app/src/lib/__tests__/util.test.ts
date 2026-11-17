import { describe, it, expect } from "vitest";
import { checkedAt } from "../util";

const s = (d: Date) => d.getTime() / 1000;

describe("checkedAt", () => {
  it("says today for the same calendar day", () => {
    const now = new Date(2026, 10, 17, 15, 0).getTime();
    expect(checkedAt(s(new Date(2026, 10, 17, 9, 12)), now)).toBe("today at 09:12");
  });

  it("says yesterday for the day before, also across a month", () => {
    const now = new Date(2026, 10, 17, 15, 0).getTime();
    expect(checkedAt(s(new Date(2026, 10, 16, 23, 59)), now)).toBe("yesterday at 23:59");
    const first = new Date(2026, 11, 1, 8, 0).getTime();
    expect(checkedAt(s(new Date(2026, 10, 30, 22, 5)), first)).toBe("yesterday at 22:05");
  });

  it("gives the date for older times", () => {
    const now = new Date(2026, 10, 17, 15, 0).getTime();
    expect(checkedAt(s(new Date(2026, 10, 14, 9, 12)), now)).toBe("on 14 Nov at 09:12");
  });
});
