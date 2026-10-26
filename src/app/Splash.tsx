import { useEffect, useState } from "react";
import { Logo } from "../brand/Logo";
import { motionReduced, useSettings } from "../stores/settings";

const reduced = () => motionReduced(useSettings.getState().motion);

export function Splash() {
  const [phase, setPhase] = useState<"show" | "leave" | "gone">(reduced() ? "gone" : "show");

  useEffect(() => {
    if (phase === "gone") return;
    const leave = setTimeout(() => setPhase("leave"), 1300);
    const gone = setTimeout(() => setPhase("gone"), 1650);
    return () => {
      clearTimeout(leave);
      clearTimeout(gone);
    };
  }, [phase === "gone"]);

  if (phase === "gone") return null;
  return (
    <div
      aria-hidden
      className={phase === "leave" ? "splash splash-out" : "splash"}
    >
      <div className="flex flex-col items-center gap-4">
        <Logo size={96} animated className="text-accent" />
        <span className="splash-name text-lg font-semibold tracking-[0.2em] text-ink">CP2077 MOD LAUNCHER</span>
      </div>
    </div>
  );
}
