import { motion } from "framer-motion";
import { formatBytes, formatConfidence } from "../lib/format";
import type { Destination, PlanEntry } from "../lib/types";

interface Props {
  entry: PlanEntry;
  destinations: Destination[];
  index: number;
  flying: boolean;
  /** Pixel offset to the destination tile, for the flight animation (§5). */
  flightTo: { x: number; y: number } | null;
  reducedMotion: boolean;
  onChange: (patch: Partial<PlanEntry>) => void;
}

/** Where a decision came from. The user is entitled to know what they are trusting. */
function Provenance({ entry }: { entry: PlanEntry }) {
  const map = {
    deterministic: { label: "rule", title: "Matched a built-in or user rule — no AI involved" },
    learned: { label: "learned", title: "A rule Menlo learned from a decision you approved" },
    llm: { label: "AI", title: "Classified by your local CLI" },
  } as const;
  const { label, title } = map[entry.resolved_by];
  return (
    <span
      title={title}
      className={`rounded-[3px] border px-1.5 py-px text-[10px] uppercase tracking-wide ${
        entry.resolved_by === "llm"
          ? "border-warn/40 text-warn"
          : "border-rule text-mute"
      }`}
    >
      {label}
    </span>
  );
}

export function FileRow({
  entry,
  destinations,
  index,
  flying,
  flightTo,
  reducedMotion,
  onChange,
}: Props) {
  const needsReview = entry.action === "needs_review";
  const dimmed = !entry.included;

  // §5: 320ms, ease-out-quart, 40ms stagger, at most 12 in flight. Under reduced
  // motion this becomes a cross-fade with no travel.
  const delay = flying ? Math.min(index, 11) * 0.04 : 0;
  const animate =
    flying && entry.included
      ? reducedMotion || !flightTo
        ? { opacity: 0 }
        : { x: flightTo.x, y: flightTo.y, opacity: 0, scale: 0.9 }
      : { x: 0, y: 0, opacity: 1, scale: 1 };

  return (
    <motion.li
      layout={!flying}
      animate={animate}
      transition={{ duration: reducedMotion ? 0.15 : 0.32, ease: [0.25, 1, 0.5, 1], delay }}
      className={`grid grid-cols-[20px_minmax(0,1fr)_auto] items-start gap-x-3 border-b border-rule px-3 py-2.5 last:border-b-0 ${
        dimmed ? "opacity-45" : ""
      }`}
    >
      <input
        type="checkbox"
        checked={entry.included}
        disabled={!entry.destination_key}
        onChange={(e) => onChange({ included: e.target.checked })}
        aria-label={`Include ${entry.name}`}
        className="mt-0.5 h-3.5 w-3.5 accent-oxblood disabled:opacity-30"
      />

      <div className="min-w-0">
        <div className="flex items-baseline gap-2">
          <span className="mono selectable truncate text-ink" title={entry.name}>
            {entry.name}
          </span>
          <span className="mono shrink-0 text-mute">{formatBytes(entry.size_bytes)}</span>
        </div>
        <div className="mt-1 flex flex-wrap items-center gap-x-2 gap-y-1">
          {/* A file nothing matched has no provenance to report; the badge would lie. */}
          {!needsReview && <Provenance entry={entry} />}
          <span className={`min-w-0 ${needsReview ? "text-mute italic" : "text-ink-soft"}`}>
            {entry.reason}
          </span>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <span
          className="mono w-9 text-right text-mute"
          title="Confidence"
          aria-label={`Confidence ${formatConfidence(entry.confidence)}`}
        >
          {formatConfidence(entry.confidence)}
        </span>
        <select
          value={entry.destination_key ?? ""}
          onChange={(e) => {
            const key = e.target.value || null;
            onChange({
              destination_key: key,
              // Choosing a destination by hand is an approval, and it suppresses
              // learning from the original suggestion (task 12).
              overridden: true,
              action: key ? "move" : "needs_review",
              included: key !== null,
              confidence: key ? 1 : 0,
              reason: key ? "You chose this." : "No rule matched.",
            });
          }}
          aria-label={`Destination for ${entry.name}`}
          className="field h-7 w-[132px] py-0"
        >
          <option value="">— leave in place —</option>
          {destinations.map((d) => (
            <option key={d.key} value={d.key}>
              {d.label}
            </option>
          ))}
        </select>
      </div>
    </motion.li>
  );
}
