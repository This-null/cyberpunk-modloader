import { Search, TriangleAlert } from "lucide-react";
import { useState } from "react";
import { useT } from "../../i18n";
import { ipc, toAppError, type Suspect } from "../../lib/ipc";
import { useErrors } from "../../stores/errors";
import { useMods } from "../../stores/mods";

export function CrashAnalysis() {
  const t = useT();
  const [found, setFound] = useState<Suspect[] | null>(null);
  const [running, setRunning] = useState(false);
  const busy = useMods((s) => s.busy);

  async function run() {
    setRunning(true);
    try {
      setFound(await ipc.crashAnalyze());
    } catch (e) {
      setFound(null);
      useErrors.getState().report(toAppError(e));
    } finally {
      setRunning(false);
    }
  }

  async function off(name: string) {
    await useMods.getState().disable(name);
    await run();
  }

  return (
    <section className="flex flex-col gap-3 rounded-lg border border-line bg-surface p-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-col">
          <h2 className="font-semibold">{t("crash.title")}</h2>
          <span className="text-xs text-muted">{t("crash.hint")}</span>
        </div>
        <button onClick={() => void run()} disabled={running} className="btn disabled:opacity-60">
          <Search size={15} />
          {running ? t("crash.running") : t("crash.run")}
        </button>
      </div>
      {found !== null && found.length === 0 && <p className="text-muted">{t("crash.none")}</p>}
      {found !== null && found.length > 0 && (
        <ul className="flex flex-col gap-2">
          {found.map((s) => (
            <li key={s.name} className="flex flex-col gap-2 rounded border border-line bg-surface-2 p-3">
              <div className="flex flex-wrap items-center gap-2">
                <TriangleAlert size={15} className={s.kind === "config_var" ? "text-danger" : "text-accent"} />
                <span className="font-medium">{s.name}</span>
                <span className="rounded border border-line px-1.5 py-0.5 text-xs text-muted">
                  {s.kind === "config_var" ? t("crash.kindConfig") : t("crash.kindPath")}
                </span>
                {!s.managed && <span className="rounded border border-line px-1.5 py-0.5 text-xs text-muted">{t("crash.unmanaged")}</span>}
                {s.managed && s.enabled && (
                  <button disabled={busy} onClick={() => void off(s.name)} className="btn btn-sm ml-auto disabled:opacity-60">
                    {t("action.disable")}
                  </button>
                )}
              </div>
              <ul className="flex flex-col gap-1 font-mono text-xs text-muted">
                {s.evidence.map((line, i) => (
                  <li key={i} className="break-words">
                    {line}
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
