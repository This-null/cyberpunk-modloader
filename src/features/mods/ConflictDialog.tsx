import { useT } from "../../i18n";
import { useMods } from "../../stores/mods";

export function ConflictDialog() {
  const t = useT();
  const pending = useMods((s) => s.pending);
  const busy = useMods((s) => s.busy);
  const enable = useMods((s) => s.enable);
  const cancel = useMods((s) => s.cancelPending);

  if (!pending) return null;

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-full w-full max-w-lg flex-col gap-4 overflow-hidden rounded-lg border border-line bg-surface p-6">
        <h2 className="text-lg font-semibold">{t("conflict.title")}</h2>
        <p>{t("conflict.body", { mod: pending.name })}</p>
        <ul className="max-h-56 overflow-auto rounded bg-surface-2 p-3 text-xs">
          {pending.conflicts.map((c) => (
            <li key={c.file} className="py-1">
              <span className="break-all">{c.file}</span>
              <span className="text-muted">
                {" "}
                · {c.owner ? t("conflict.byMod", { mod: c.owner }) : t("conflict.existing")}
              </span>
            </li>
          ))}
        </ul>
        <div className="flex justify-end gap-2">
          <button onClick={cancel} className="rounded border border-line px-4 py-2 hover:bg-surface-2">
            {t("action.cancel")}
          </button>
          <button
            disabled={busy}
            onClick={() => (pending.retry ? pending.retry() : enable(pending.name, true))}
            className="rounded bg-danger px-4 py-2 font-medium text-white"
          >
            {t("action.overwrite")}
          </button>
        </div>
      </div>
    </div>
  );
}
