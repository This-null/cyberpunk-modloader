import clsx from "clsx";
import { useT } from "../../i18n";
import { displayOrder } from "../../lib/fomod";
import { useFomod } from "../../stores/fomod";

export function FomodDialog() {
  const t = useT();
  const open = useFomod((s) => s.open);
  const name = useFomod((s) => s.name);
  const view = useFomod((s) => s.view);
  const selection = useFomod((s) => s.selection);
  const evaluation = useFomod((s) => s.evaluation);
  const working = useFomod((s) => s.working);
  const { choose, install, cancel } = useFomod.getState();
  if (!open || !view || !evaluation) return null;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/60 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-full w-full max-w-2xl flex-col gap-4 rounded-lg border border-line bg-surface p-6">
        <div>
          <h2 className="text-lg font-semibold">{t("fomod.title", { name })}</h2>
          <p className="mt-1 text-xs text-muted">{t("fomod.hint")}</p>
        </div>
        <div className="flex min-h-0 flex-1 flex-col gap-5 overflow-auto pr-1">
          {displayOrder(view.steps.map((s) => s.name), view.order).map((si) => {
            const step = view.steps[si];
            return evaluation.visible[si] === false ? null : (
              <section key={si} className="flex flex-col gap-3">
                {view.steps.length > 1 && <h3 className="text-sm font-semibold text-muted">{step.name}</h3>}
                {displayOrder(step.groups.map((g) => g.name), step.order).map((gi) => {
                  const group = step.groups[gi];
                  const radio = ["selectexactlyone", "selectatmostone"].includes(group.kind.toLowerCase());
                  return (
                    <div key={gi} className="flex flex-col gap-2">
                      <span className="font-medium">{group.name}</span>
                      {displayOrder(group.plugins.map((p) => p.name), group.order).map((pi) => {
                        const plugin = group.plugins[pi];
                        const type = (evaluation.types[si]?.[gi]?.[pi] ?? "").toLowerCase();
                        const on = selection[si]?.[gi]?.includes(pi) ?? false;
                        const locked = type === "required" || type === "notusable" || group.kind.toLowerCase() === "selectall";
                        return (
                          <button
                            key={pi}
                            disabled={locked}
                            onClick={() => void choose(si, gi, pi)}
                            aria-pressed={on}
                            className={clsx(
                              "flex items-start gap-3 rounded-md border px-3 py-2 text-left transition-colors disabled:cursor-default",
                              on ? "border-accent bg-accent/10" : "border-line bg-surface-2 hover:border-accent/50",
                              type === "notusable" && "opacity-50",
                            )}
                          >
                            <span
                              className={clsx(
                                "mt-1 grid h-4 w-4 shrink-0 place-items-center border-2",
                                radio ? "rounded-full" : "rounded",
                                on ? "border-accent bg-accent" : "border-muted",
                              )}
                            >
                              {on && <span className={clsx("bg-accent-ink", radio ? "h-1.5 w-1.5 rounded-full" : "h-2 w-2 rounded-sm")} />}
                            </span>
                            <span className="min-w-0 flex-1">
                              <span className="flex flex-wrap items-center gap-2">
                                <span className="font-medium">{plugin.name}</span>
                                {type === "recommended" && <span className="rounded bg-ok/15 px-1.5 py-0.5 text-xs text-ok">{t("fomod.recommended")}</span>}
                                {type === "required" && <span className="rounded bg-surface px-1.5 py-0.5 text-xs text-muted">{t("fomod.required")}</span>}
                                {type === "notusable" && <span className="rounded bg-surface px-1.5 py-0.5 text-xs text-muted">{t("fomod.unavailable")}</span>}
                              </span>
                              {plugin.description && <span className="mt-0.5 line-clamp-3 block whitespace-pre-line text-xs text-muted">{plugin.description}</span>}
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  );
                })}
              </section>
            );
          })}
        </div>
        <div className="flex justify-end gap-2">
          <button className="btn" disabled={working} onClick={() => void cancel()}>
            {t("action.cancel")}
          </button>
          <button className="btn btn-primary" disabled={working} onClick={() => void install()}>
            {working ? t("fomod.installing") : t("fomod.install")}
          </button>
        </div>
      </div>
    </div>
  );
}
