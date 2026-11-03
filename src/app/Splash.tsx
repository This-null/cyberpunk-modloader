import { useEffect, useState } from "react";
import { Logo } from "../brand/Logo";
import { motionReduced, useSettings } from "../stores/settings";

const reduced = () => motionReduced(useSettings.getState().motion);

export function Splash() {
  const [phase, setPhase] = useState<"show" | "leave" | "gone">(reduced() ? "gone" : "show");

  useEffect(() => {
    if (phase === "gone") return;
    const leave = setTimeout(() => setPhase("leave"), 2700);
    const gone = setTimeout(() => setPhase("gone"), 3000);
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
      <div className="relative z-10 flex flex-col items-center gap-5">
        <div className="splash-logo">
          <Logo size={104} animated className="text-accent" />
        </div>
        <span className="flex text-lg font-semibold tracking-[0.2em] text-ink">
          {[..."CP2077 MOD LAUNCHER"].map((c, i) => (
            <span key={i} className="splash-letter" style={{ animationDelay: `${500 + i * 35}ms` }}>
              {c === " " ? " " : c}
            </span>
          ))}
        </span>
        <div className="splash-bar">
          <span />
        </div>
      </div>
    </div>
  );
}
