import type { ReactNode } from "react";
import { useUi } from "../store/ui";
import { Choose } from "./Choose";
import { Move } from "./Move";
import { Receipt } from "./Receipt";
import { Shape } from "./Shape";

/**
 * The stage track.
 *
 * The four stages are one 400%-tall column that slides. It is deliberately not a
 * router: the slide is what tells the user the stages are one continuous pass rather
 * than four separate screens, and keeping them all mounted is what lets a stage keep
 * its scroll position when the user steps back.
 *
 * Mounted is not the same as present, though. A stage that has slid out of view is
 * `inert` — out of the tab order and the accessibility tree — so the keyboard and a
 * screen reader only ever reach the stage that is on screen.
 */
export function Flow() {
  const step = useUi((s) => s.step);

  return (
    <div className="h-full overflow-hidden">
      <div
        className="h-[400%] transition-transform duration-[900ms]"
        style={{
          transform: `translateY(${-25 * step}%)`,
          transitionTimingFunction: "var(--ease-menlo)",
        }}
      >
        <Stage on={step === 0}>
          <Choose />
        </Stage>
        <Stage on={step === 1}>
          <Shape />
        </Stage>
        <Stage on={step === 2}>
          <Move />
        </Stage>
        <Stage on={step === 3}>
          <Receipt />
        </Stage>
      </div>
    </div>
  );
}

function Stage({ on, children }: { on: boolean; children: ReactNode }) {
  return (
    <div className="h-1/4" inert={!on} aria-hidden={!on}>
      {children}
    </div>
  );
}
