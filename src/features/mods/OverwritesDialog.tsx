import { useState } from "react";
import { ChevronDown, ChevronRight, PowerOff, ShieldAlert, X } from "lucide-react";
import { useT } from "../../i18n";
import type { Overwrite } from "../../lib/overwrites";
import { useMods } from "../../stores/mods";

export function OverwritesDialog({ items, locked, onClose }: { items: Overwrite[]; locked: boolean; onClose: () => void }) {
  const t = useT();
  const disable = useMods((s) => s.disable);
  const [open, setOpen] = useState<string | null>(null);

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-2xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <div className="flex items-center justify-between gap-3">
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <ShieldAlert size={18} className="text-accent" />
            {t("overwrites.title")}
          </h2>
          <button onClick={onClose} aria-label={t("action.close")} className="rounded p-1 text-muted hover:text-ink">
            <X size={18} />
          </button>
        </div>
        <p className="text-muted">{t("overwrites.hint")}</p>
        {items.length === 0 && <p className="text-ok">{t("overwrites.none")}</p>}
        <ul className="flex flex-col gap-2">
          {items.map((o) => {
            const key = `${o.by}|${o.owner ?? ""}`;
            const expanded = open === key;
            return (
              <li key={key} className="rounded-md border border-line bg-surface-2 p-3">
                <div className="flex flex-wrap items-center gap-3">
                  <button onClick={() => setOpen(expanded ? null : key)} aria-expanded={expanded} className="flex min-w-0 flex-1 items-center gap-2 text-left">
                    {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
                    <span className="min-w-0 break-words">
                      {o.owner
                        ? t("overwrites.row", { by: o.by, count: o.files.length, owner: o.owner })
                        : t("overwrites.rowGame", { by: o.by, count: o.files.length })}
                    </span>
                  </button>
                  <button disabled={locked} onClick={() => void disable(o.by)} className="btn btn-sm btn-danger">
                    <PowerOff size={14} />
                    {t("overwrites.disable", { by: o.by })}
                  </button>
                </div>
                {expanded && (
                  <ul className="mt-2 max-h-48 overflow-auto rounded bg-surface p-2 font-mono text-xs text-muted">
                    {o.files.slice(0, 200).map((f) => (
                      <li key={f} className="break-all">
                        {f}
                      </li>
                    ))}
                    {o.files.length > 200 && <li>…</li>}
                  </ul>
                )}
              </li>
            );
          })}
        </ul>
        <div className="flex justify-end">
          <button onClick={onClose} className="btn">
            <X size={15} />
            {t("action.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
