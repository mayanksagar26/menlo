import { useEffect, useState } from "react";
import { useApp } from "../store/app";
import { fileCount, useUi } from "../store/ui";
import { Eyebrow, Modal } from "../ui/primitives";
import { ShuffleField, StepList } from "./Thinking";

/** The phases `plan_run` goes through, in order. */
const STEPS = [
  "Reading your folders and rules",
  "Matching each file to a folder",
  "Finding clearer names",
  "Checking for files already filed",
];

/**
 * Between Shape and the move: Menlo working out where everything goes.
 *
 * The steps are the real phases of `plan_run`. They tick on a timer because the call
 * reports nothing until it is done, and the sheet closes when the plan arrives, not
 * when the ticking ends. There is no Cancel: planning is one short call, and a button
 * that could not actually stop it would be worse than none.
 */
export function SortingSheet() {
  const view = useApp((s) => s.view);
  const ui = useUi();
  const [step, setStep] = useState(0);

  useEffect(() => {
    const t = setInterval(() => setStep((s) => Math.min(s + 1, STEPS.length - 1)), 420);
    return () => clearInterval(t);
  }, []);

  const n = fileCount(view, ui);

  return (
    <Modal onClose={() => {}} width={440} labelledBy="sorting-title">
      <div className="flex flex-col gap-5 p-6">
        <div>
          <Eyebrow>Sorting</Eyebrow>
          <h2
            id="sorting-title"
            className="mt-2.5 mb-0 text-[20px] leading-[1.25] font-medium tracking-[-0.02em] text-ink"
          >
            Working out where {n} {n === 1 ? "file belongs" : "files belong"}
          </h2>
        </div>
        <ShuffleField />
        <StepList steps={STEPS} active={step} />
      </div>
    </Modal>
  );
}
