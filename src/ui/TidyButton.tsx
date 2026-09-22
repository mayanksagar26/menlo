import { useState } from "react";
import broom from "../assets/broom.png";

/**
 * "Let's tidy up" — the one button the whole home screen exists to present.
 *
 * It is the only surface with a gradient fill and a drop shadow, and a specular band
 * crosses it once every 20s so it reads as glass catching the light rather than as a
 * button that is animating at you. Hovering it also answers the question people
 * actually have before clicking: what is this about to do to my files?
 */
export function TidyButton({ onClick }: { onClick: () => void }) {
  const [hover, setHover] = useState(false);

  return (
    <div className="relative inline-flex flex-col items-center">
      <div
        role="tooltip"
        className="glass-pop pointer-events-none absolute bottom-[calc(100%+12px)] w-[280px] rounded-[14px] px-3.5 py-3 text-[11.5px] leading-[1.6] text-ink-70 transition-all duration-200"
        style={{
          opacity: hover ? 1 : 0,
          transform: hover ? "translateY(0)" : "translateY(4px)",
          transitionTimingFunction: "var(--ease-menlo)",
        }}
      >
        Menlo only moves files into folders you picked. Nothing is renamed, nothing is
        deleted, and every run can be sent back.
      </div>

      <button
        type="button"
        onClick={onClick}
        onMouseEnter={() => setHover(true)}
        onMouseLeave={() => setHover(false)}
        className="relative inline-flex h-[46px] items-center gap-2.5 overflow-hidden rounded-full px-5 transition-transform duration-200"
        style={{
          background:
            "linear-gradient(168deg, rgba(255,255,255,0.13), rgba(255,255,255,0.05) 52%, rgba(255,255,255,0.09))",
          border: `1px solid rgba(255,255,255,${hover ? 0.42 : 0.22})`,
          boxShadow: "inset 0 1px 0 rgba(255,255,255,0.30), 0 12px 30px rgba(0,0,0,0.45)",
          backdropFilter: "blur(30px) saturate(170%)",
          WebkitBackdropFilter: "blur(30px) saturate(170%)",
          transform: hover ? "translateY(-1px)" : "none",
          transitionTimingFunction: "var(--ease-menlo)",
        }}
      >
        {/* The glint: a diagonal band that crosses the face corner to corner. */}
        <span
          aria-hidden
          className="pointer-events-none absolute top-1/2 left-1/2 h-[300%] w-[58%] -translate-x-1/2 -translate-y-1/2"
          style={{
            background:
              "linear-gradient(90deg, transparent, rgba(255,255,255,0.55), transparent)",
            filter: "blur(2px)",
            animation: "menlo-glint 20s linear infinite",
          }}
        />
        <img src={broom} alt="" width={22} height={22} style={{ filter: "invert(1)" }} />
        <span className="text-[14.5px] font-light text-ink">Let's tidy up</span>
      </button>
    </div>
  );
}
