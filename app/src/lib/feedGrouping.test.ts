import { describe, it, expect } from "vitest";
import { groupChangesByPersonAndDay } from "./feedGrouping";
import type { ChangeView } from "./types";

describe("groupChangesByPersonAndDay", () => {
  it("returns empty array for empty changes", () => {
    const result = groupChangesByPersonAndDay([]);
    expect(result).toEqual([]);
  });

  it("groups commits on the same day by author", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    const changes: ChangeView[] = [
      {
        author: "Sarah Chen",
        title: "feat: add user profile page",
        sha: "abc1234",
        timestamp: nowSec - 100,
        mine: false,
      },
      {
        author: "Sarah Chen",
        title: "fix: navbar alignment",
        sha: "def5678",
        timestamp: nowSec - 200,
        mine: false,
      },
      {
        author: "Alex Rivera",
        title: "docs: update readme",
        sha: "9876543",
        timestamp: nowSec - 300,
        mine: false,
      },
    ];

    const grouped = groupChangesByPersonAndDay(changes);
    expect(grouped.length).toBe(1);
    expect(grouped[0].dayLabel).toBe("Today");
    expect(grouped[0].authors.length).toBe(2);

    expect(grouped[0].authors[0].author).toBe("Sarah Chen");
    expect(grouped[0].authors[0].changes.length).toBe(2);

    expect(grouped[0].authors[1].author).toBe("Alex Rivera");
    expect(grouped[0].authors[1].changes.length).toBe(1);
  });

  it("groups commits across multiple days (Today, Yesterday, Older)", () => {
    const now = new Date();
    const todaySec = Math.floor(now.getTime() / 1000);

    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);
    const yesterdaySec = Math.floor(yesterday.getTime() / 1000);

    const fourDaysAgo = new Date(now);
    fourDaysAgo.setDate(now.getDate() - 4);
    const fourDaysAgoSec = Math.floor(fourDaysAgo.getTime() / 1000);

    const changes: ChangeView[] = [
      {
        author: "Sarah Chen",
        title: "feat: today commit",
        sha: "aaa1111",
        timestamp: todaySec,
        mine: false,
      },
      {
        author: "Alex Rivera",
        title: "fix: yesterday commit",
        sha: "bbb2222",
        timestamp: yesterdaySec,
        mine: false,
      },
      {
        author: "Sarah Chen",
        title: "refactor: old commit",
        sha: "ccc3333",
        timestamp: fourDaysAgoSec,
        mine: false,
      },
    ];

    const grouped = groupChangesByPersonAndDay(changes);
    expect(grouped.length).toBe(3);
    expect(grouped[0].dayLabel).toBe("Today");
    expect(grouped[1].dayLabel).toBe("Yesterday");
    expect(grouped[2].dayLabel).not.toBe("Today");
    expect(grouped[2].dayLabel).not.toBe("Yesterday");
  });

  it("correctly identifies user's own commits", () => {
    const nowSec = Math.floor(Date.now() / 1000);
    const changes: ChangeView[] = [
      {
        author: "Ramin",
        title: "feat: my commit",
        sha: "1112223",
        timestamp: nowSec,
        mine: true,
      },
    ];

    const grouped = groupChangesByPersonAndDay(changes);
    expect(grouped.length).toBe(1);
    expect(grouped[0].authors[0].mine).toBe(true);
    expect(grouped[0].authors[0].author).toBe("Ramin");
  });
});
