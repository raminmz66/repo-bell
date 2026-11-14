/** "just now", "5 min ago", "2 h ago", "3 d ago". `now` is in milliseconds. */
export function timeAgo(timestamp: number | null, now: number = Date.now()): string {
  if (!timestamp) return "";
  const seconds = Math.floor(now / 1000 - timestamp);
  if (seconds < 60) return "just now";
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  return `${Math.floor(hours / 24)} d ago`;
}

/** `timeAgo` without " ago", and "now" for "just now": "5 min", "2 h", "3 d". */
export function shortTimeAgo(timestamp: number | null, now: number = Date.now()): string {
  return timeAgo(timestamp, now).replace(" ago", "").replace("just now", "now");
}

/** The plain-words text of a rejected IPC call. */
export function errText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Something went wrong. Try again.";
}

/** Adds a closing full stop when a one-line message has none. */
export function sentence(text: string): string {
  return /[.!?]$/.test(text) ? text : `${text}.`;
}

export function plural(n: number, one: string, many: string = `${one}s`): string {
  return n === 1 ? one : many;
}

/** `/home/ana/projects` -> `~/projects`. The window can't know the home folder, so this uses the Linux layout. */
export function tildePath(path: string): string {
  return path.replace(/^\/home\/[^/]+(?=\/|$)/, "~").replace(/^\/root(?=\/|$)/, "~");
}

/** Last part of a path, for a folder name. */
export function baseName(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}
