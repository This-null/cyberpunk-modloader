import { Check, Link2, X } from "lucide-react";
import { useEffect, useState } from "react";
import clsx from "clsx";
import { useT } from "../../i18n";
import { ipc, type ModRecord } from "../../lib/ipc";
import { bestMatch, type Grade } from "../../lib/nexusMatch";
import { cleanName } from "../../lib/nexusMatch";
import { useMods } from "../../stores/mods";
import { useNexus } from "../../stores/nexus";

interface Suggestion {
  mod: string;
  title: string;
  modId: number;
  grade: Grade;
  on: boolean;
}

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function LinkSuggestions({ targets, onClose }: { targets: ModRecord[]; onClose: () => void }) {
  const t = useT();
  const [done, setDone] = useState(0);
  const [found, setFound] = useState<Suggestion[]>([]);
  const [missed, setMissed] = useState<string[]>([]);
  const [searching, setSearching] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    let alive = true;
    (async () => {
      for (const mod of targets) {
        if (!alive) return;
        const query = cleanName(mod.name);
        let matched = false;
        if (query !== "") {
          try {
            const page = await ipc.nexusSearch({ query, categories: [], hide_adult: false, sort: "downloads", tag: null, offset: 0, count: 10 });
            const hit = bestMatch(mod.name, page.items);
            if (hit && alive) {
              matched = true;
              setFound((list) => [...list, { mod: mod.name, title: hit.item.name ?? "", modId: hit.item.mod_id, grade: hit.grade, on: hit.grade === "exact" }]);
            }
          } catch {
            // one failed search should not stop the rest
          }
          await sleep(200);
        }
        if (alive && !matched) setMissed((list) => [...list, mod.name]);
        if (alive) setDone((n) => n + 1);
      }
      if (alive) setSearching(false);
    })();
    return () => {
      alive = false;
    };
  }, [targets]);

  const chosen = found.filter((s) => s.on);

  async function apply() {
    setSaving(true);
    for (const s of chosen) {
      await ipc.nexusLink(s.mod, `https://www.nexusmods.com/cyberpunk2077/mods/${s.modId}`).catch(() => null);
    }
    await useMods.getState().refresh();
    void useNexus.getState().check();
    onClose();
  }

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-2xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <div className="flex items-center justify-between gap-3">
          <h2 className="flex items-center gap-2 text-lg font-semibold">
            <Link2 size={18} className="text-accent" />
            {t("link.title")}
          </h2>
          <button onClick={onClose} aria-label={t("action.close")} className="rounded p-1 text-muted hover:text-ink">
            <X size={18} />
          </button>
        </div>
        <p className="text-muted">{t("link.hint")}</p>
        {searching && <p className="text-xs text-muted">{t("link.searching", { done, total: targets.length })}</p>}
        {!searching && found.length === 0 && <p className="text-muted">{t("link.none")}</p>}
        <ul className="flex flex-col gap-2">
          {found.map((s) => (
            <li key={s.mod}>
              <label className={clsx("flex cursor-pointer items-center gap-3 rounded-md border bg-surface-2 p-3", s.on ? "border-accent/60" : "border-line")}>
                <input type="checkbox" checked={s.on} onChange={() => setFound((l) => l.map((x) => (x.mod === s.mod ? { ...x, on: !x.on } : x)))} />
                <span className="min-w-0 flex-1">
                  <span className="block truncate font-medium">{s.mod}</span>
                  <span className="block truncate text-xs text-muted">
                    {s.title} · #{s.modId}
                  </span>
                </span>
                <span className={clsx("shrink-0 rounded border px-1.5 py-0.5 text-xs", s.grade === "exact" ? "border-ok text-ok" : "border-line text-muted")}>
                  {s.grade === "exact" ? t("link.exact") : t("link.likely")}
                </span>
              </label>
            </li>
          ))}
        </ul>
        {missed.length > 0 && (
          <div className="flex flex-col gap-2">
            <span className="text-xs font-medium uppercase tracking-wide text-muted">
              {t("link.missed", { count: missed.length })}
            </span>
            <ul className="flex flex-col gap-2">
              {missed.map((name) => (
                <li key={name} className="flex items-center justify-between gap-3 rounded-md border border-line bg-surface-2 p-3">
                  <span className="min-w-0 truncate">{name}</span>
                  <button
                    className="btn btn-sm shrink-0"
                    onClick={() => {
                      useMods.getState().openDetail(name);
                      onClose();
                    }}
                  >
                    {t("link.manual")}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
        <div className="flex items-center justify-end gap-2">
          <button onClick={onClose} className="btn">
            {t("action.cancel")}
          </button>
          <button disabled={saving || chosen.length === 0} onClick={() => void apply()} className="btn btn-primary disabled:opacity-60">
            <Check size={15} />
            {t("link.apply", { count: chosen.length })}
          </button>
        </div>
      </div>
    </div>
  );
}
