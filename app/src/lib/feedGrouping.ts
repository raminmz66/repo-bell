import type { ChangeView, DayGroup, AuthorDayGroup } from "./types";

export function formatDayLabel(date: Date, now: Date = new Date()): string {
  const dYear = date.getFullYear();
  const dMonth = date.getMonth();
  const dDate = date.getDate();

  const nowYear = now.getFullYear();
  const nowMonth = now.getMonth();
  const nowDate = now.getDate();

  if (dYear === nowYear && dMonth === nowMonth && dDate === nowDate) {
    return "Today";
  }

  const yDay = new Date(nowYear, nowMonth, nowDate - 1);
  if (dYear === yDay.getFullYear() && dMonth === yDay.getMonth() && dDate === yDay.getDate()) {
    return "Yesterday";
  }

  const monthNames = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun",
    "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"
  ];
  const m = monthNames[dMonth];
  if (dYear === nowYear) {
    return `${m} ${dDate}`;
  }
  return `${m} ${dDate}, ${dYear}`;
}

export function groupChangesByPersonAndDay(changes: ChangeView[]): DayGroup[] {
  if (!changes || changes.length === 0) return [];

  const sorted = [...changes].sort((a, b) => b.timestamp - a.timestamp);
  const daysMap = new Map<string, { label: string; date: Date; changes: ChangeView[] }>();
  const now = new Date();

  for (const change of sorted) {
    const d = new Date(change.timestamp * 1000);
    const year = d.getFullYear();
    const month = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    const key = `${year}-${month}-${day}`;

    if (!daysMap.has(key)) {
      daysMap.set(key, {
        label: formatDayLabel(d, now),
        date: d,
        changes: [],
      });
    }
    daysMap.get(key)!.changes.push(change);
  }

  const result: DayGroup[] = [];

  for (const [dayKey, dayData] of daysMap.entries()) {
    const authorMap = new Map<string, { mine: boolean; changes: ChangeView[] }>();
    for (const change of dayData.changes) {
      if (!authorMap.has(change.author)) {
        authorMap.set(change.author, {
          mine: change.mine,
          changes: [],
        });
      }
      authorMap.get(change.author)!.changes.push(change);
    }

    const authors: AuthorDayGroup[] = [];
    for (const [author, data] of authorMap.entries()) {
      authors.push({
        author,
        mine: data.mine,
        changes: data.changes,
      });
    }

    result.push({
      dayKey,
      dayLabel: dayData.label,
      authors,
    });
  }

  return result;
}
