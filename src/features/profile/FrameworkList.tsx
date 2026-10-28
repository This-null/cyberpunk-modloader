import { useEffect, useState } from "react";
import { AlertTriangle, Check, X } from "lucide-react";
import { useT } from "../../i18n";
import { FRAMEWORKS, neededFrameworks } from "../../lib/frameworks";
import { frameworkOutdated, useFrameworks } from "../../stores/frameworks";
import { useMods } from "../../stores/mods";
import { useNav } from "../../stores/nav";
import { useNexus } from "../../stores/nexus";
import { percent } from "../../lib/nightcoop";
import { useNexusBrowse } from "../../stores/nexusBrowse";

export function FrameworkList() {
  const t = useT();
  const present = useFrameworks((s) => s.present);
  const versions = useFrameworks((s) => s.versions);
  const latest = useFrameworks((s) => s.latest);
  const oldLoader = useFrameworks((s) => s.oldLoader);
  const owned = useFrameworks((s) => s.owned);
  const plan = useFrameworks((s) => s.plan);
  const mods = useMods((s) => s.mods);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);
  const connected = useNexus((s) => s.status?.connected ?? false);
  const installing = useNexusBrowse((s) => s.installing);
  const progress = useNexusBrowse((s) => s.progress);
  const chain = useNexusBrowse((s) => s.chain);
  const busy = useFrameworks((s) => s.busy);
  const pct = progress ? percent(progress.done, progress.total) : 0;
  const needed = neededFrameworks(mods);
  const [armed, setArmed] = useState<string | null>(null);

  useEffect(() => {
    if (connected) void useFrameworks.getState().lookup();
  }, [connected]);

  useEffect(() => {
    const refresh = () => void useFrameworks.getState().load();
    refresh();
    const id = setInterval(refresh, 3000);
    window.addEventListener("focus", refresh);
    return () => {
      clearInterval(id);
      window.removeEventListener("focus", refresh);
    };
  }, []);

  return (
    <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
      <h2 className="text-base font-semibold">{t("fw.title")}</h2>
      <p className="text-xs text-muted">{t("fw.hint")}</p>
      {oldLoader && (
        <p className="flex items-start gap-2 rounded-md border border-danger/50 bg-danger/10 p-3 text-xs text-danger">
          <AlertTriangle size={16} className="mt-0.5 shrink-0" />
          {t("fw.oldLoader")}
        </p>
      )}
      {owned && (
        <p className="anim-rise flex flex-wrap items-center gap-3 rounded-md border border-line bg-surface-2 p-3 text-xs">
          <span className="min-w-0 flex-1">
            {t("fw.owned", { name: FRAMEWORKS.find((f) => f.id === owned.id)?.name ?? owned.id, mod: owned.mod })}
          </span>
          <button className="btn btn-sm" onClick={() => useNav.getState().setPage("mods")}>
            {t("fw.goMods")}
          </button>
        </p>
      )}
      {armed && plan.length > 0 && (
        <p className="anim-rise rounded-md border border-danger/50 bg-danger/10 p-3 text-xs text-danger">
          {t("fw.willDelete", { name: FRAMEWORKS.find((f) => f.id === armed)?.name ?? armed, paths: plan.join(", ") })}
        </p>
      )}
      <ul className="grid grid-cols-[repeat(auto-fill,minmax(17rem,1fr))] gap-2">
        {FRAMEWORKS.map((fw) => {
          const on = present[fw.id];
          const users = needed[fw.id]?.length ?? 0;
          const info = latest[fw.id];
          const outdated = frameworkOutdated(fw.id, versions, latest);
          const canInstall = premium && info && (!on || outdated);
          return (
            <li key={fw.id} className="flex items-center gap-2 rounded-md bg-surface-2 px-3 py-2">
              {on ? <Check size={16} className="shrink-0 text-ok" /> : <X size={16} className={users > 0 ? "shrink-0 text-danger" : "shrink-0 text-muted"} />}
              <span className="min-w-0 flex-1">
                <span className="block truncate">{fw.name}</span>
                <span className={outdated ? "text-xs text-accent" : on ? "text-xs text-ok" : users > 0 ? "text-xs text-danger" : "text-xs text-muted"}>
                  {on
                    ? outdated
                      ? t("fw.outdated", { have: versions[fw.id] ?? "", latest: info?.version ?? "" })
                      : versions[fw.id]
                        ? t("fw.okVersion", { version: versions[fw.id] ?? "" })
                        : t("fw.ok")
                    : t("fw.no")}
                </span>
              </span>
              {on &&
                (armed === fw.id ? (
                  <>
                    <button
                      className="btn btn-sm btn-danger-solid"
                      onClick={() => {
                        setArmed(null);
                        void useFrameworks.getState().remove(fw.id);
                      }}
                    >
                      {t("fw.removeConfirm")}
                    </button>
                    <button className="btn btn-sm" onClick={() => setArmed(null)}>
                      {t("action.cancel")}
                    </button>
                  </>
                ) : (
                  <button className="btn btn-sm btn-danger" disabled={busy !== null} onClick={() => void useFrameworks.getState().arm(fw.id).then(() => setArmed(fw.id))}>
                    {t("fw.remove")}
                  </button>
                ))}
              {(canInstall || busy === fw.id) && (
                <button
                  disabled={installing !== null || busy !== null}
                  onClick={() => void useFrameworks.getState().install(fw.id)}
                  className="btn btn-sm btn-primary"
                >
                  {busy === fw.id
                    ? chain
                      ? `${Math.min(chain.done + 1, chain.total)}/${chain.total} · ${pct}%`
                      : t("nexusPage.installingPercent", { percent: pct })
                    : on
                      ? t("fw.update")
                      : t("fw.install")}
                </button>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
