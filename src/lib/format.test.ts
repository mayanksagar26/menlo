import { describe, expect, it } from "vitest";
import { formatBytes, formatConfidence, plural, relativeTime, slugify, tildePath } from "./format";

describe("formatBytes", () => {
  it("keeps bytes exact below 1 KB", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("uses one decimal below ten and none above", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(184320)).toBe("180 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });

  it("climbs units", () => {
    expect(formatBytes(3 * 1024 ** 3)).toBe("3.0 GB");
    expect(formatBytes(2 * 1024 ** 4)).toBe("2.0 TB");
  });
});

describe("formatConfidence", () => {
  it("is always two decimals, so the column does not jitter", () => {
    expect(formatConfidence(1)).toBe("1.00");
    expect(formatConfidence(0.9312)).toBe("0.93");
    expect(formatConfidence(0)).toBe("0.00");
  });
});

describe("tildePath", () => {
  it("abbreviates the home directory", () => {
    expect(tildePath("/Users/ada/Downloads")).toBe("~/Downloads");
  });

  it("leaves other paths alone", () => {
    expect(tildePath("/Volumes/Archive/2026")).toBe("/Volumes/Archive/2026");
  });
});

describe("relativeTime", () => {
  const now = new Date("2026-09-08T12:00:00Z");
  const ago = (ms: number) => new Date(now.getTime() - ms).toISOString();

  it("describes recent moments", () => {
    expect(relativeTime(ago(5_000), now)).toBe("just now");
    expect(relativeTime(ago(60_000), now)).toBe("a minute ago");
    expect(relativeTime(ago(10 * 60_000), now)).toBe("10 minutes ago");
    expect(relativeTime(ago(3 * 3_600_000), now)).toBe("3 hours ago");
    expect(relativeTime(ago(26 * 3_600_000), now)).toBe("1 day ago");
  });

  it("falls back to a date past a month", () => {
    expect(relativeTime(ago(90 * 86_400_000), now)).toMatch(/2026/);
  });
});

describe("plural", () => {
  it("agrees with its number", () => {
    expect(plural(1, "file")).toBe("1 file");
    expect(plural(0, "file")).toBe("0 files");
    expect(plural(3, "entry", "entries")).toBe("3 entries");
  });
});

describe("slugify", () => {
  it("produces keys the Rust validator accepts", () => {
    expect(slugify("Tax Documents")).toBe("tax-documents");
    expect(slugify("  Photos!! 2026  ")).toBe("photos-2026");
    expect(slugify("Fotografías")).toBe("fotografias");
    expect(slugify("Ünïcode")).toBe("unicode");
  });

  it("never emits a leading or trailing dash", () => {
    expect(slugify("---a---")).toBe("a");
  });
});
