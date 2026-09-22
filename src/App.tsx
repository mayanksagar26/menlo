import { useEffect } from "react";
import { Flow } from "./flow/Flow";
import { HowItWorks } from "./pages/HowItWorks";
import { RulesMemory } from "./pages/RulesMemory";
import { Runs } from "./pages/Runs";
import { SettingsPage } from "./pages/SettingsPage";
import { Home } from "./screens/Home";
import { DuplicatesSheet } from "./sheets/DuplicatesSheet";
import { FolderSets } from "./sheets/FolderSets";
import { MovePreview } from "./sheets/MovePreview";
import { RulePopup } from "./sheets/RulePopup";
import { SortingSheet } from "./sheets/SortingSheet";
import { SuggestSheet } from "./sheets/SuggestSheets";
import { useApp } from "./store/app";
import { CUSTOM, useUi } from "./store/ui";
import { Ambient } from "./ui/Ambient";
import { BottomBar } from "./ui/BottomBar";
import { ProfileMenu } from "./ui/ProfileMenu";
import { TopBar } from "./ui/TopBar";

/**
 * The window.
 *
 * Everything is one surface: home and the four stages swap in the middle, the pages
 * and sheets open over the whole window, and the two bars never move. There is no
 * router — position in the flow is state (`store/ui.ts`), because the design treats
 * the stages as one pass rather than four destinations.
 */
export default function App() {
  const ui = useUi();
  const view = useApp((s) => s.view);
  const error = useApp((s) => s.error);
  const boot = useApp((s) => s.boot);
  const dismissError = useApp((s) => s.dismissError);

  useEffect(() => {
    void boot();
  }, [boot]);

  // Arrive on home with the first pinned set already picked, so the greeting and the
  // figures describe something rather than nothing.
  useEffect(() => {
    if (!view || ui.preset !== CUSTOM || Object.keys(ui.src).length) return;
    const first = view.folder_sets.find((s) => s.pinned) ?? view.folder_sets[0];
    if (first) ui.chooseSet(first);
  }, [view, ui]);

  return (
    <div
      className="relative flex h-full flex-col overflow-hidden"
      style={{ background: "linear-gradient(180deg,#0C0E13 0%,#07080B 100%)" }}
    >
      <Ambient />

      <TopBar />

      <main className="relative z-10 min-h-0 flex-1">{view && (ui.home ? <Home /> : <Flow />)}</main>

      <BottomBar />

      {error && (
        <div
          role="alert"
          className="glass-pop anim-rise absolute bottom-[86px] left-1/2 z-[60] flex max-w-[560px] -translate-x-1/2 items-start gap-3 rounded-[12px] px-3.5 py-2.5"
        >
          <p className="selectable m-0 flex-1 text-[12px] leading-[1.5] text-ink-86">{error}</p>
          <button
            type="button"
            onClick={dismissError}
            aria-label="Dismiss"
            className="text-[12px] text-ink-60 hover:text-ink"
          >
            ✕
          </button>
        </div>
      )}

      {ui.profileOpen && <ProfileMenu />}

      {/* Pages and sheets cover the whole window, bars included: while one is open it
          is the only thing to act on. */}
      {ui.page === "docs" && <HowItWorks />}
      {ui.page === "rules" && <RulesMemory />}
      {ui.page === "runs" && <Runs />}
      {ui.page === "settings" && <SettingsPage />}

      {ui.setsOpen && <FolderSets />}
      {ui.planning && <SortingSheet />}
      {ui.dupesOpen && <DuplicatesSheet />}
      {ui.confirmOpen && <MovePreview />}
      {ui.suggest.open && <SuggestSheet />}
      {ui.ruleFor && <RulePopup destKey={ui.ruleFor} />}
    </div>
  );
}
