import { useEffect } from "react";
import { Banner } from "./components/Banner";
import { History } from "./screens/History";
import { PlanReview } from "./screens/PlanReview";
import { Settings } from "./screens/Settings";
import { Setup } from "./screens/Setup";
import { useStore } from "./store";

export default function App() {
  const screen = useStore((s) => s.screen);
  const ready = useStore((s) => s.ready);
  const boot = useStore((s) => s.boot);

  useEffect(() => {
    void boot();
  }, [boot]);

  if (!ready) return <div className="h-full" />;

  return (
    <div className="flex h-full flex-col">
      <Banner />
      <div className="min-h-0 flex-1">
        {screen === "setup" && <Setup />}
        {screen === "review" && <PlanReview />}
        {screen === "history" && <History />}
        {screen === "settings" && <Settings />}
      </div>
    </div>
  );
}
