import { useRef, useState } from "react";
import { DOCS, MODELS } from "../lib/copy";
import { Page, RailRow } from "./Page";

/**
 * How it works — the five stages, then who decides.
 *
 * Sections other than the one in view sit back at 55% opacity, so the page reads as a
 * single explanation you are moving through rather than competing blocks.
 */
export function HowItWorks() {
  const [active, setActive] = useState(0);
  const sections = useRef<(HTMLElement | null)[]>([]);
  // The five stages, then one more section for the working models.
  const models = DOCS.length;

  function go(i: number) {
    setActive(i);
    sections.current[i]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }

  return (
    <Page
      title="How it works"
      rail={
        <>
          {DOCS.map((d, i) => (
            <RailRow key={d.num} active={active === i} onClick={() => go(i)}>
              <span className="text-[11px] text-ink-60">{d.num}</span>
              <span className="text-[12.5px]">{d.title}</span>
            </RailRow>
          ))}
          <RailRow active={active === models} onClick={() => go(models)}>
            <span className="text-[11px] text-ink-60">06</span>
            <span className="text-[12.5px]">Working models</span>
          </RailRow>
        </>
      }
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

        <section
          ref={(el) => {
            sections.current[models] = el;
          }}
          className="max-w-[560px] pb-4 transition-opacity duration-[340ms]"
          style={{
            opacity: active === models ? 1 : 0.55,
            transitionTimingFunction: "var(--ease-menlo)",
          }}
          onMouseEnter={() => setActive(models)}
        >
          <p className="eyebrow m-0">06 · Working models</p>
          <p className="mt-3 mb-0 text-[12.5px] leading-[1.7] text-ink-70">
            Who decides where a file goes, once your rules have had their turn. Menlo files
            most of a folder before any of this: the rules you wrote and the types it knows.
            A model is only ever asked about what is left, and it never touches your files —
            it reads text and answers with a folder Menlo offered it.
          </p>

          <div className="mt-5 flex flex-col gap-2">
            {MODELS.map((m) => (
              <article key={m.value} className="glass rounded-[14px] p-3.5">
                <p className="m-0 flex items-center gap-2 text-[13.5px] font-medium tracking-[-0.01em] text-ink-86">
                  {m.title}
                  <span className="rounded-full border border-hair px-2 py-[1px] text-[10px] font-normal text-ink-60">
                    {m.when}
                  </span>
                </p>
                <p className="mt-1.5 mb-0 text-[11.5px] leading-[1.6] text-ink-66">{m.body}</p>
                <p className="mt-2 mb-0 text-[11.5px] leading-[1.6] text-ink-60">{m.gains}</p>
                {m.link && (
                  <a
                    href={m.link.href}
                    target="_blank"
                    rel="noreferrer"
                    className="mt-2 inline-block text-[11px] text-ink-70 underline decoration-hair underline-offset-2 hover:text-ink"
                  >
                    {m.link.label} ↗
                  </a>
                )}
              </article>
            ))}
          </div>

          <p className="mt-4 mb-0 text-[11px] leading-[1.6] text-ink-60">
            Change this any time in Settings → Working model. Whatever you pick, every landing
            is still shown before a file moves, and every run can still be undone.
          </p>
        </section>
      </div>
    </Page>
  );
}
