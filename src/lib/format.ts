/** Presentation helpers. Kept dependency-free and unit-tested. */

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  // One decimal below 10 so "1.4 MB" reads better than "1 MB", none above.
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[unit]}`;
}

/** Confidence is a figure, so §5 says it is set in Menlo. Two decimals, always. */
export function formatConfidence(c: number): string {
  return c.toFixed(2);
}

/** `/Users/x/Downloads` → `~/Downloads`. */
export function tildePath(path: string, home?: string): string {
  const h = home ?? guessHome(path);
  return h && path.startsWith(h) ? `~${path.slice(h.length)}` : path;
}

function guessHome(path: string): string | null {
  const m = path.match(/^(\/Users\/[^/]+)/);
  return m ? m[1] : null;
}

export function relativeTime(iso: string, now: Date = new Date()): string {
  const then = new Date(iso);
  const seconds = Math.round((now.getTime() - then.getTime()) / 1000);
  if (seconds < 45) return "just now";
  if (seconds < 90) return "a minute ago";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} minutes ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} ${hours === 1 ? "hour" : "hours"} ago`;
  const days = Math.round(hours / 24);
  if (days < 30) return `${days} ${days === 1 ? "day" : "days"} ago`;
  return then.toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/** "1 file" / "24 files" — small thing, but the app says these constantly. */
export function plural(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** A key suggestion derived from a folder name: "Tax Documents" → "tax-documents". */
export function slugify(input: string): string {
  return input
    // Strip diacritics first, so "Fotografías" becomes "fotografias" rather than
    // "fotograf-as". The Rust validator only accepts [A-Za-z0-9_-].
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 40);
}
