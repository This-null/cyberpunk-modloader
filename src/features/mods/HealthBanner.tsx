import { useEffect, useState } from "react";
import { ChevronDown, ChevronRight, HeartPulse, RefreshCw } from "lucide-react";
import { useT } from "../../i18n";
import { ipc, type ModHealth } from "../../lib/ipc";
import { useMods } from "../../stores/mods";

export function HealthBanner({ locked }: { locked: boolean }) {
  const t = useT();
  const mods = useMods((s) => s.mods);
  const enable = useMods((s) => s.enable);
  const disable = useMods((s) => s.disable);
  const [issues, setIssues] = useState<ModHealth[]>([]);
  const [open, setOpen] = useState(false);

  useEffect(() => {
    let alive = true;
    const run = () =>
      ipc
        .modHealth()
        .then((h) => alive && setIssues(h))
        .catch(() => {});
    run();
    window.addEventListener("focus", run);
    return () => {
      alive = false;
      window.removeEventListener("focus", run);
    };
  }, [mods]);

  if (issues.length === 0) return null;

  async function repair(name: string) {
    await disable(name);
    await enable(name);
  }

  return (
    <div className="anim-rise flex flex-col gap-2 rounded-lg border border-danger/50 bg-danger/10 p-3">
      <button onClick={() => setOpen(!open)} aria-expanded={open} className="flex items-center gap-3 text-left">
        <HeartPulse size={16} className="shrink-0 text-danger" />
        <span className="min-w-0 flex-1 text-ink">{t("health.text", { count: issues.length })}</span>
        {open ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
      </button>
      {open && (
        <ul className="flex flex-col gap-2">
          {issues.map((h) => (
            <li key={h.name} className="rounded-md border border-line bg-surface p-3">
              <div className="flex flex-wrap items-center gap-3">
                <span className="min-w-0 flex-1 break-words font-medium">{h.name}</span>
                {h.enabled && h.missing_installed.length > 0 && h.missing_library === 0 && (
                  <button disabled={locked} onClick={() => void repair(h.name)} className="btn btn-sm">
                    <RefreshCw size={14} />
                    {t("health.repair")}
                  </button>
                )}
              </div>
              {h.missing_installed.length > 0 && (
                <>
                  <p className="mt-1 text-xs text-muted">{t("health.missingGame", { count: h.missing_installed.length })}</p>
                  <ul className="mt-1 max-h-32 overflow-auto font-mono text-xs text-muted">
                    {h.missing_installed.slice(0, 20).map((f) => (
                      <li key={f} className="break-all">
                        {f}
                      </li>
                    ))}
                  </ul>
                </>
              )}
              {h.missing_library > 0 && <p className="mt-1 text-xs text-danger">{t("health.missingLibrary", { count: h.missing_library })}</p>}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
