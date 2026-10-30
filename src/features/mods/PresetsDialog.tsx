import { useState } from "react";
import clsx from "clsx";
import { Check, Pencil, Trash2 } from "lucide-react";
import { useT } from "../../i18n";
import { useMods } from "../../stores/mods";
import { sameSet, useMyPresets } from "../../stores/presets";

const btn = "btn disabled:opacity-60";
const primary = "btn btn-primary disabled:opacity-60";

export function PresetsDialog() {
  const t = useT();
  const open = useMyPresets((s) => s.open);
  const list = useMyPresets((s) => s.list);
  const report = useMyPresets((s) => s.report);
  const applied = useMyPresets((s) => s.applied);
  const { close, saveCurrent, rename, remove, apply } = useMyPresets.getState();
  const mods = useMods((s) => s.mods);
  const busy = useMods((s) => s.busy);
  const [name, setName] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [confirm, setConfirm] = useState<string | null>(null);

  if (!open) return null;
  const enabled = mods.filter((m) => m.enabled).map((m) => m.name);

  async function save() {
    if (await saveCurrent(name)) setName("");
  }

  async function finishRename(old: string) {
    if (draft.trim() !== "" && draft.trim() !== old) await rename(old, draft);
    setEditing(null);
  }

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">{t("presets.title")}</h2>
          <button onClick={close} className="text-muted hover:text-ink">{t("action.close")}</button>
        </div>
        <p className="text-xs text-muted">{t("presets.hint")}</p>

        {list.length === 0 ? (
          <p className="text-muted">{t("presets.empty")}</p>
        ) : (
          <ul className="flex flex-col gap-2">
            {list.map((p) => {
              const active = sameSet(p.mods.filter((m) => mods.some((x) => x.name === m)), enabled) && p.mods.length > 0;
              return (
                <li key={p.name} className={clsx("flex flex-wrap items-center gap-2 rounded-md border px-3 py-2", active ? "border-accent/50 bg-accent/10" : "border-line bg-surface-2")}>
                  <div className="min-w-0 flex-1">
                    {editing === p.name ? (
                      <input
                        autoFocus
                        value={draft}
                        maxLength={60}
                        onChange={(e) => setDraft(e.target.value)}
                        onBlur={() => void finishRename(p.name)}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") void finishRename(p.name);
                          if (e.key === "Escape") setEditing(null);
                        }}
                        className="w-full rounded border border-line bg-surface px-2 py-1"
                      />
                    ) : (
                      <span className="flex items-center gap-2 font-medium">
                        {p.name}
                        {active && (
                          <span className="flex items-center gap-1 rounded bg-accent px-1.5 py-0.5 text-xs text-accent-ink">
                            <Check size={12} />
                            {t("presets.active")}
                          </span>
                        )}
                      </span>
                    )}
                    <span className="text-xs text-muted">{t("presets.count", { count: p.mods.length })}</span>
                  </div>
                  {confirm === p.name ? (
                    <>
                      <button onClick={() => { void remove(p.name); setConfirm(null); }} className="btn btn-danger-solid">
                        {t("presets.confirmDelete")}
                      </button>
                      <button onClick={() => setConfirm(null)} className={btn}>{t("action.cancel")}</button>
                    </>
                  ) : (
                    <>
                      <button disabled={busy || active} onClick={() => void apply(p.name)} className={primary}>{t("presets.switch")}</button>
                      <button aria-label={t("presets.rename")} title={t("presets.rename")} onClick={() => { setEditing(p.name); setDraft(p.name); }} className={btn}>
                        <Pencil size={14} />
                      </button>
                      <button aria-label={t("presets.delete")} title={t("presets.delete")} onClick={() => setConfirm(p.name)} className={btn}>
                        <Trash2 size={14} />
                      </button>
                    </>
                  )}
                </li>
              );
            })}
          </ul>
        )}

        {report && applied && (
          <div className="anim-rise flex flex-col gap-1 rounded-md border border-line bg-surface-2 p-3 text-xs">
            <span className="text-ok">{t("presets.applied", { name: applied, on: report.enabled.length, off: report.disabled.length })}</span>
            {report.failed.length > 0 && <span className="text-danger">{t("presets.failed", { names: report.failed.map((f) => f[0]).join(", ") })}</span>}
            {report.missing.length > 0 && <span className="text-muted">{t("presets.missing", { names: report.missing.join(", ") })}</span>}
          </div>
        )}

        <div className="flex flex-col gap-2 border-t border-line pt-4">
          <span className="text-xs text-muted">{t("presets.saveCurrent", { count: enabled.length })}</span>
          <div className="flex items-center gap-2">
            <input
              value={name}
              maxLength={60}
              onChange={(e) => setName(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && void save()}
              placeholder={t("presets.namePlaceholder")}
              className="min-w-0 flex-1 rounded border border-line bg-surface-2 px-3 py-2"
            />
            <button disabled={name.trim() === "" || enabled.length === 0} onClick={() => void save()} className={primary}>
              {t("presets.save")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
