import { useEffect, useState } from "react";
import { RefreshCw, X } from "lucide-react";
import { useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { useNav } from "../../stores/nav";

const KEY = "cp2077-game-version";

function remembered(): string | null {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function remember(version: string) {
  try {
    localStorage.setItem(KEY, version);
  } catch {
    // the notice then shows once per launch
  }
}

export function GameUpdateBanner() {
  const t = useT();
  const [change, setChange] = useState<{ from: string; to: string } | null>(null);

  useEffect(() => {
    let alive = true;
    ipc
      .gameVersion()
      .then((now) => {
        if (!alive || !now) return;
        const before = remembered();
        if (before === null) remember(now);
        else if (before !== now) setChange({ from: before, to: now });
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, []);

  if (!change) return null;
  return (
    <div className="anim-rise flex flex-wrap items-center gap-3 rounded-lg border border-accent/50 bg-accent/10 p-3">
      <RefreshCw size={16} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 text-ink">{t("gameupd.text", { from: change.from, to: change.to })}</span>
      <button className="btn btn-sm" onClick={() => useNav.getState().setPage("profile")}>
        {t("gameupd.check")}
      </button>
      <button
        className="btn btn-sm"
        onClick={() => {
          remember(change.to);
          setChange(null);
        }}
      >
        <X size={14} />
        {t("action.close")}
      </button>
    </div>
  );
}
