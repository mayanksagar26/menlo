/**
 * The light behind the glass.
 *
 * Three radial pools drift on long, offset loops so the window never looks static,
 * with a vignette over them to keep the edges dark enough for the hairlines to read.
 * Purely decorative: it sits under everything and is never hit-tested.
 *
 * Two rules keep this smooth. The pools are soft-edged gradients already, so they
 * carry no `filter: blur()` — blurring a half-window element every frame is what
 * made this stutter, and it bought nothing a wider gradient stop could not. And each
 * pool is promoted to its own layer, so drifting one never repaints the glass
 * stacked above it.
 */
export function Ambient() {
  return (
    <div
      className="pointer-events-none absolute inset-0 overflow-hidden"
      aria-hidden
      style={{ contain: "strict" }}
    >
      <div
        className="absolute"
        style={{
          left: "-10%",
          top: "-18%",
          width: "70%",
          height: "80%",
          background: "radial-gradient(circle, var(--color-pool-blue) 0%, transparent 68%)",
          animation: "menlo-orb-a 19s ease-in-out infinite",
          willChange: "transform",
        }}
      />
      <div
        className="absolute"
        style={{
          right: "-14%",
          top: "6%",
          width: "62%",
          height: "72%",
          background: "radial-gradient(circle, var(--color-pool-magenta) 0%, transparent 70%)",
          animation: "menlo-orb-b 23s ease-in-out infinite",
          willChange: "transform",
        }}
      />
      <div
        className="absolute"
        style={{
          left: "18%",
          bottom: "-26%",
          width: "66%",
          height: "76%",
          background: "radial-gradient(circle, var(--color-pool-teal) 0%, transparent 70%)",
          animation: "menlo-orb-c 27s ease-in-out infinite",
          willChange: "transform",
        }}
      />
      <div className="absolute inset-0" style={{ boxShadow: "inset 0 0 90px rgba(0,0,0,0.55)" }} />
    </div>
  );
}
