import { useRef, useState } from "react";
import { DOCS } from "../lib/copy";
import { Page, RailRow } from "./Page";

/**
 * How it works — the five stages, explained once.
 *
 * Sections other than the one in view sit back at 55% opacity, so the page reads as
 * a single explanation you are moving through rather than five competing blocks.
 */
export function HowItWorks() {
  const [active, setActive] = useState(0);
  const sections = useRef<(HTMLElement | null)[]>([]);

  function go(i: number) {
    setActive(i);
    sections.current[i]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  return (
    <Page
      title="How it works"
      rail={DOCS.map((d, i) => (
        <RailRow key={d.num} active={active === i} onClick={() => go(i)}>
          <span className="text-[11px] text-ink-60">{d.num}</span>
          <span className="text-[12.5px]">{d.title}</span>
        </RailRow>
      ))}
    >
      <div className="flex flex-col gap-8">
        {DOCS.map((d, i) => (
          <section
            key={d.num}
            ref={(el) => {
              sections.current[i] = el;
            }}
            className="max-w-[560px] transition-opacity duration-[340ms]"
            style={{ opacity: active === i ? 1 : 0.55, transitionTimingFunction: "var(--ease-menlo)" }}
            onMouseEnter={() => setActive(i)}
          >
            <p className="eyebrow m-0">
              {d.num} · {d.title}
            </p>
            <p className="mt-3 mb-0 text-[12.5px] leading-[1.7] text-ink-70">{d.body}</p>
            <div className="mt-4 flex flex-wrap items-center gap-2">
              {d.flow.map((chip, j) => (
                <span key={chip} className="flex items-center gap-2">
                  {j > 0 && <span className="text-[11px] text-ink-60">→</span>}
                  <span className="glass rounded-full px-3 py-1 text-[11px] text-ink-86">{chip}</span>
                </span>
              ))}
            </div>
          </section>
        ))}
      </div>
    </Page>
  );
}
