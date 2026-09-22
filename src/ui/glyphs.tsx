/**
 * Glyphs, drawn inline.
 *
 * The prototype built these out of CSS boxes and text characters. Inline SVG holds
 * the shapes at any size and inherits `currentColor`, so a row only has to set its
 * own ink and the glyph follows. Nothing here has its own colour.
 */

/** The broad kind of a file, for the label on its glyph. */
export type Bucket = "doc" | "img" | "code" | "vid" | "aud" | "arc";

const BUCKET_EXTS: Record<Bucket, string[]> = {
  doc: ["pdf", "doc", "docx", "txt", "md", "rtf", "pages", "xls", "xlsx", "csv", "ppt", "pptx", "key"],
  img: ["jpg", "jpeg", "png", "gif", "webp", "heic", "tiff", "tif", "bmp", "svg", "avif"],
  code: ["rs", "ts", "tsx", "js", "jsx", "py", "go", "rb", "java", "swift", "c", "cpp", "sh", "sql", "json", "css", "html"],
  vid: ["mp4", "mov", "mkv", "avi", "webm", "m4v"],
  aud: ["mp3", "m4a", "wav", "flac", "aac", "ogg", "aiff"],
  arc: ["zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "dmg", "pkg"],
};

export function bucketOf(ext: string): Bucket | null {
  const e = ext.toLowerCase();
  for (const [bucket, exts] of Object.entries(BUCKET_EXTS) as [Bucket, string[]][]) {
    if (exts.includes(e)) return bucket;
  }
  return null;
}

interface GlyphProps {
  size?: number;
  className?: string;
}

export function FolderGlyph({ size = 15, className, dashed = false }: GlyphProps & { dashed?: boolean }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      aria-hidden
      className={className}
    >
      <path
        d="M1.5 4.2c0-.66.54-1.2 1.2-1.2h3.1c.4 0 .78.2 1 .54l.5.76h6c.66 0 1.2.54 1.2 1.2v6.3c0 .66-.54 1.2-1.2 1.2H2.7c-.66 0-1.2-.54-1.2-1.2V4.2Z"
        stroke="currentColor"
        strokeWidth="1.1"
        strokeDasharray={dashed ? "2.2 2" : undefined}
      />
    </svg>
  );
}

/**
 * The folder-set pill's glyph: one mini folder per source, fanned out. Spacing and
 * rotation widen on hover, which is the whole animation — `open` drives it.
 */
export function StackedFolders({ count, open }: { count: number; open: boolean }) {
  const shown = Math.min(Math.max(count, 1), 3);
  const gap = open ? 15 : 7;
  return (
    <span
      className="relative inline-block shrink-0"
      style={{ width: 13 + (shown - 1) * gap, height: 15 }}
      aria-hidden
    >
      {Array.from({ length: shown }, (_, i) => (
        <span
          key={i}
          className="absolute top-0 left-0 transition-transform duration-[280ms]"
          style={{
            transform: `translateX(${i * gap}px) rotate(${open ? (i - 1) * 4 : 0}deg)`,
            opacity: 1 - i * 0.22,
            transitionTimingFunction: "var(--ease-menlo)",
          }}
        >
          <FolderGlyph size={14} />
        </span>
      ))}
    </span>
  );
}

const FILE_LABEL: Record<Bucket, string> = {
  doc: "PDF",
  img: "IMG",
  code: "SRC",
  vid: "MOV",
  aud: "AUD",
  arc: "ZIP",
};

/** A page with a folded corner, labelled by what kind of file it is. */
export function FileGlyph({ bucket, size = 16 }: { bucket: Bucket | null; size?: number }) {
  return (
    <svg width={size} height={size * 1.22} viewBox="0 0 16 20" fill="none" aria-hidden>
      <path
        d="M2.5 2.1c0-.6.5-1.1 1.1-1.1h6.1L14 5.1v12.8c0 .6-.5 1.1-1.1 1.1H3.6c-.6 0-1.1-.5-1.1-1.1V2.1Z"
        stroke="currentColor"
        strokeWidth="1"
      />
      <path d="M9.6 1.2v3.6c0 .4.3.7.7.7h3.5" stroke="currentColor" strokeWidth="1" />
      {bucket && (
        <text
          x="8"
          y="15.4"
          textAnchor="middle"
          fontSize="4.6"
          letterSpacing="0.06em"
          fill="currentColor"
        >
          {FILE_LABEL[bucket]}
        </text>
      )}
    </svg>
  );
}

export function Chevron({ open, size = 10 }: { open?: boolean; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 10 10"
      fill="none"
      aria-hidden
      className="transition-transform duration-[220ms]"
      style={{
        transform: open ? "rotate(90deg)" : "none",
        transitionTimingFunction: "var(--ease-menlo)",
      }}
    >
      <path d="M3.4 1.6 6.9 5l-3.5 3.4" stroke="currentColor" strokeWidth="1.2" />
    </svg>
  );
}

export function Check({ size = 9 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 10 10" fill="none" aria-hidden>
      <path d="M1.8 5.2 4 7.4l4.2-5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

const DRIFT: { bucket: Bucket | null; left: string; top: string; size: number; rot: number; dur: number; opacity: number }[] = [
  // Kept clear of the centre column, where the greeting and the stat bar sit: these
  // are meant to be felt at the edge of vision, not read over the headline.
  { bucket: "doc", left: "5%", top: "26%", size: 30, rot: -8, dur: 9.5, opacity: 0.3 },
  { bucket: "img", left: "11%", top: "64%", size: 24, rot: 11, dur: 12.5, opacity: 0.24 },
  { bucket: "vid", left: "87%", top: "20%", size: 28, rot: 6, dur: 10.5, opacity: 0.26 },
  { bucket: "arc", left: "92%", top: "62%", size: 22, rot: -12, dur: 8, opacity: 0.22 },
  { bucket: "aud", left: "81%", top: "84%", size: 20, rot: 9, dur: 11.5, opacity: 0.24 },
  { bucket: "code", left: "14%", top: "10%", size: 22, rot: -5, dur: 7.5, opacity: 0.22 },
  { bucket: null, left: "89%", top: "42%", size: 26, rot: 14, dur: 12, opacity: 0.18 },
];

/** File-type glyphs drifting behind the home screen. Decoration; never hit-tested. */
export function DriftField() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden text-ink" aria-hidden>
      {DRIFT.map((g, i) => (
        <span
          key={i}
          className="absolute"
          style={{
            left: g.left,
            top: g.top,
            opacity: g.opacity,
            ["--rot" as string]: `${g.rot}deg`,
            animation: `menlo-drift ${g.dur}s ease-in-out ${i * 0.7}s infinite`,
          }}
        >
          <FileGlyph bucket={g.bucket} size={g.size} />
        </span>
      ))}
    </div>
  );
}

// ── Menu icons ───────────────────────────────────────────────────────────────
// One per place outside the flow, drawn on the same 16px grid and stroke as the
// folder glyph so the profile menu and the Settings rail read as one family.

type IconProps = { size?: number };

function Icon({ size = 15, children }: IconProps & { children: React.ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 16 16"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.1"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
    >
      {children}
    </svg>
  );
}

/** Rules memory: lines of writing. */
export function RulesIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3.5 3.5h9M3.5 6.5h9M3.5 9.5h6M3.5 12.5h4" />
    </Icon>
  );
}

/** Runs: a clock turning back. */
export function RunsIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M2.8 8a5.2 5.2 0 1 0 1.5-3.7" />
      <path d="M2.4 2.6v2.2h2.2" />
      <path d="M8 5.2V8l1.9 1.3" />
    </Icon>
  );
}

/** Folder sets: two folders, stacked. */
export function SetsIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3.8 3.2h2.4l.8 1.1h4.9c.5 0 .9.4.9.9" />
      <path d="M1.8 6.1c0-.5.4-.9.9-.9h2.9l.9 1.2h6.1c.5 0 .9.4.9.9v4.8c0 .5-.4.9-.9.9H2.7c-.5 0-.9-.4-.9-.9V6.1Z" />
    </Icon>
  );
}

/** How it works: an open book. */
export function DocsIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M8 4.2C6.6 3.2 4.6 3 2.5 3.4v8.8c2.1-.4 4.1-.2 5.5.8 1.4-1 3.4-1.2 5.5-.8V3.4C11.4 3 9.4 3.2 8 4.2Z" />
      <path d="M8 4.2v8.8" />
    </Icon>
  );
}

/** Settings: sliders, rather than a gear — these are choices, not machinery. */
export function SettingsIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3 4.5h10M3 8h10M3 11.5h10" />
      <circle cx="10.5" cy="4.5" r="1.3" fill="var(--color-desk)" />
      <circle cx="5.5" cy="8" r="1.3" fill="var(--color-desk)" />
      <circle cx="9" cy="11.5" r="1.3" fill="var(--color-desk)" />
    </Icon>
  );
}

/** Profile, for the Settings rail. */
export function ProfileIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="5.6" r="2.4" />
      <path d="M3.2 13c.6-2.3 2.5-3.6 4.8-3.6s4.2 1.3 4.8 3.6" />
    </Icon>
  );
}

/** Working model, for the Settings rail: a chip. */
export function ModelIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="4" y="4" width="8" height="8" rx="1.6" />
      <path d="M6.5 2v2M9.5 2v2M6.5 12v2M9.5 12v2M2 6.5h2M2 9.5h2M12 6.5h2M12 9.5h2" />
    </Icon>
  );
}

/** Running, for the Settings rail: a play mark. */
export function RunningIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="8" r="5.6" />
      <path d="M6.8 5.9v4.2L10.2 8 6.8 5.9Z" />
    </Icon>
  );
}

/** About, for the Settings rail. */
export function AboutIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="8" cy="8" r="5.6" />
      <path d="M8 7.4v3.4M8 5.2v.1" />
    </Icon>
  );
}

/** Plus, for "Add folder". */
export function PlusIcon(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M8 3.5v9M3.5 8h9" />
    </Icon>
  );
}

