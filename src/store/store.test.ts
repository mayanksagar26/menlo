import { describe, expect, it } from "vitest";
import { planStats } from "./index";
import type { Plan, PlanEntry } from "../lib/types";

function entry(over: Partial<PlanEntry>): PlanEntry {
  return {
    id: "f_000",
    name: "a.pdf",
    ext: "pdf",
    size_bytes: 1,
    action: "move",
    destination_key: "documents",
    rename_to: null,
    confidence: 1,
    reason: "",
    suggest_rule: null,
    resolved_by: "deterministic",
    included: true,
    overridden: false,
    ...over,
  };
}

function plan(entries: PlanEntry[]): Plan {
  return {
    batch_id: "b",
    source: "/tmp",
    created_at: new Date().toISOString(),
    entries,
    skipped: [],
    truncated: false,
    prompt_version: null,
  };
}

describe("planStats", () => {
  it("is empty without a plan", () => {
    expect(planStats(null)).toEqual({
      total: 0,
      willMove: 0,
      withoutAi: 0,
      needsReview: 0,
      excluded: 0,
    });
  });

  it("counts only included moves as pending work", () => {
    const s = planStats(
      plan([
        entry({ id: "a" }),
        entry({ id: "b", included: false }),
        entry({ id: "c", action: "needs_review", destination_key: null, included: false }),
      ]),
    );
    expect(s.total).toBe(3);
    expect(s.willMove).toBe(1);
    expect(s.needsReview).toBe(1);
    expect(s.excluded).toBe(2);
  });

  it("counts AI-free resolutions across both deterministic tiers", () => {
    const s = planStats(
      plan([
        entry({ id: "a", resolved_by: "deterministic" }),
        entry({ id: "b", resolved_by: "learned" }),
        entry({ id: "c", resolved_by: "llm" }),
      ]),
    );
    expect(s.withoutAi).toBe(2);
  });

  it("does not count an excluded file as AI-free work avoided", () => {
    // The counter is about classification, not execution, so an excluded but
    // rule-matched file still counts.
    const s = planStats(plan([entry({ included: false })]));
    expect(s.withoutAi).toBe(1);
    expect(s.willMove).toBe(0);
  });
});
