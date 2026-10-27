import { useEffect, useMemo, useRef, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Download, Upload } from "lucide-react";
import { useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import {
  addKeys,
  extKey,
  matchesQuery,
  modKey,
  namesWith,
  pruneSelection,
  rangeKeys,
  toggleKey,
} from "../../lib/selection";
import { useMods } from "../../stores/mods";
import { useNightcoop } from "../../stores/nightcoop";
import { useSettings } from "../../stores/settings";
import { NightCoopCard } from "../nightcoop/NightCoopCard";
import { ConflictDialog } from "./ConflictDialog";
import { ExternalRow } from "./ExternalRow";
import { groupMods, relation } from "../../lib/groups";
import { ModCard } from "./ModCard";
import { ProfileDialog } from "./ProfileDialog";
import { useProfile } from "../../stores/profile";
import { VersionRow } from "./VersionRow";
import { ModDetails } from "./ModDetails";

function typing(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el) return false;
  return el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable;
}

export function ModsScreen() {
  const t = useT();
  const mods = useMods((s) => s.mods);
  const external = useMods((s) => s.external);
  const selected = useMods((s) => s.selected);
  const setSelected = useMods((s) => s.setSelected);
  const importExternal = useMods((s) => s.importExternal);
  const exportAll = useMods((s) => s.exportAll);
  const exportSome = useMods((s) => s.exportSome);
  const bulkEnable = useMods((s) => s.bulkEnable);
  const bulkDisable = useMods((s) => s.bulkDisable);
  const bulkUninstall = useMods((s) => s.bulkUninstall);
  const bulkImport = useMods((s) => s.bulkImport);
  const detail = useMods((s) => s.detail);
  const pending = useMods((s) => s.pending);
  const detected = useNightcoop((s) => s.detected);
  const notice = useMods((s) => s.notice);
  const clearNotice = useMods((s) => s.clearNotice);
  const busy = useMods((s) => s.busy);
  const skipped = useMods((s) => s.skipped);
  const clearSkipped = useMods((s) => s.clearSkipped);
  const install = useMods((s) => s.install);
  const enable = useMods((s) => s.enable);
  const disable = useMods((s) => s.disable);
  const uninstall = useMods((s) => s.uninstall);
  const gameDir = useSettings((s) => s.gameDir);
  const [running, setRunning] = useState(false);
  const [over, setOver] = useState(false);
  const [query, setQuery] = useState("");
  const [confirming, setConfirming] = useState(false);
  const [openGroups, setOpenGroups] = useState<Set<string>>(() => new Set());
  const lastKey = useRef<string | null>(null);

  useEffect(() => {
    let alive = true;
    const tick = () =>
      ipc
        .gameRunning()
        .then((r) => alive && setRunning(r))
        .catch(() => {});
    tick();
    const id = setInterval(tick, 3000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent((e) => {
      const p = e.payload;
      if (p.type === "enter" || p.type === "over") {
        setOver(true);
      } else if (p.type === "drop") {
        setOver(false);
        const zips = p.paths.filter((x) => /\.(zip|rar|7z)$/i.test(x));
        if (zips.length > 0 && !useMods.getState().busy) void install(zips);
      } else {
        setOver(false);
      }
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [install]);

  const shownMods = useMemo(
    () => mods.filter((m) => matchesQuery([m.name, m.description, m.version], query)),
    [mods, query],
  );
  const shownExternal = useMemo(
    () => external.filter((m) => matchesQuery([m.name, m.kind], query)),
    [external, query],
  );
  const groups = useMemo(() => groupMods(shownMods), [shownMods]);
  const order = useMemo(
    () => [
      ...groups.flatMap((g) => [modKey(g.primary.name), ...(openGroups.has(g.key) ? g.others.map((o) => modKey(o.name)) : [])]),
      ...shownExternal.map((m) => extKey(m.name)),
    ],
    [groups, openGroups, shownExternal],
  );
  const allKeys = useMemo(
    () => [...mods.map((m) => modKey(m.name)), ...external.map((m) => extKey(m.name))],
    [mods, external],
  );

  useEffect(() => {
    const kept = pruneSelection(selected, order);
    if (kept.length !== selected.length) setSelected(kept);
  }, [order, selected, setSelected]);

  useEffect(() => {
    setConfirming(false);
  }, [selected]);

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (typing(e.target) || detail || pending) return;
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setSelected(order);
      } else if (e.key === "Escape") {
        setSelected([]);
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [order, detail, pending, setSelected]);

  function onSelect(key: string, shift: boolean) {
    if (shift && lastKey.current) {
      setSelected(addKeys(selected, rangeKeys(order, lastKey.current, key)));
    } else {
      setSelected(toggleKey(selected, key));
    }
    lastKey.current = key;
  }

  async function pick() {
    const chosen = await open({ multiple: true, filters: [{ name: "Mods", extensions: ["zip", "rar", "7z"] }] });
    if (!chosen) return;
    await install(Array.isArray(chosen) ? chosen : [chosen]);
  }

  async function pack(names?: string[]) {
    const dest = await save({ defaultPath: "mod-pack.zip", filters: [{ name: "ZIP", extensions: ["zip"] }] });
    if (!dest) return;
    if (names) await exportSome(names, dest);
    else await exportAll(dest);
  }

  const locked = busy || running || !gameDir;
  const selectedNames = namesWith(selected, "m:");
  const selectedExternal = namesWith(selected, "e:");
  const selectedMods = mods.filter((m) => selectedNames.includes(m.name));
  const toEnable = selectedMods.filter((m) => !m.enabled).map((m) => m.name);
  const toDisable = selectedMods.filter((m) => m.enabled).map((m) => m.name);
  const nothingShown = shownMods.length === 0 && shownExternal.length === 0 && allKeys.length > 0;
  const btn = "rounded border border-line px-3 py-1.5 hover:bg-surface-2";

  return (
    <div className="relative flex h-full flex-col gap-4">
      <ProfileDialog names={selectedNames.length > 0 ? selectedNames : mods.map((m) => m.name)} />
      <div className="flex items-center justify-between">
        <h1 className="text-xl font-semibold">{t("mods.title")}</h1>
        <div className="flex items-center gap-2">
          <button
            disabled={busy || mods.length === 0}
            onClick={() => void useProfile.getState().open("share", selectedNames.length > 0 ? selectedNames : mods.map((m) => m.name))}
            className="flex items-center gap-2 rounded border border-line px-4 py-2 hover:bg-surface-2"
          >
            {t("profile.share")}
          </button>
          <button
            disabled={busy || !gameDir}
            onClick={() => void useProfile.getState().open("import", [])}
            className="flex items-center gap-2 rounded border border-line px-4 py-2 hover:bg-surface-2"
          >
            {t("profile.import")}
          </button>
          <button
            disabled={busy || mods.length === 0}
            onClick={() => void pack()}
            className="flex items-center gap-2 rounded border border-line px-4 py-2 hover:bg-surface-2"
          >
            <Upload size={16} />
            {t("action.exportAll")}
          </button>
          <button
            disabled={busy || !gameDir}
            onClick={pick}
            className="flex items-center gap-2 rounded bg-accent px-4 py-2 font-medium text-accent-ink"
          >
            <Download size={16} />
            {t("mods.install")}
          </button>
        </div>
      </div>

      {allKeys.length > 0 && (
        <div className="flex flex-col gap-2">
          <div className="flex flex-wrap items-center gap-2">
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={t("mods.search")}
              aria-label={t("mods.search")}
              className="min-w-48 flex-1 rounded border border-line bg-surface px-3 py-2"
            />
            <button className={btn} disabled={order.length === 0} onClick={() => setSelected(order)}>
              {t("mods.selectAll")}
            </button>
            <button className={btn} disabled={selected.length === 0} onClick={() => setSelected([])}>
              {t("mods.clearSelection")}
            </button>
          </div>
          {selected.length === 0 ? (
            <p className="text-xs text-muted">{t("mods.selectHint")}</p>
          ) : (
            <div className="anim-rise flex flex-wrap items-center gap-2 rounded-lg border border-accent bg-surface px-3 py-2">
              <span className="mr-2 font-medium">{t("mods.selected", { count: selected.length })}</span>
              <button className={btn} disabled={locked || toEnable.length === 0} onClick={() => void bulkEnable(toEnable)}>
                {t("action.enable")}
              </button>
              <button className={btn} disabled={locked || toDisable.length === 0} onClick={() => void bulkDisable(toDisable)}>
                {t("action.disable")}
              </button>
              <button className={btn} disabled={busy || selectedNames.length === 0} onClick={() => void pack(selectedNames)}>
                {t("action.export")}
              </button>
              <button className={btn} disabled={busy || selectedExternal.length === 0} onClick={() => void bulkImport(selectedExternal)}>
                {t("action.import")}
              </button>
              {confirming ? (
                <>
                  <button
                    disabled={locked}
                    onClick={() => {
                      setConfirming(false);
                      void bulkUninstall(selectedNames);
                    }}
                    className="rounded bg-danger px-3 py-1.5 font-medium text-white"
                  >
                    {t("mods.confirmUninstall", { count: selectedNames.length })}
                  </button>
                  <button className={btn} onClick={() => setConfirming(false)}>
                    {t("action.cancel")}
                  </button>
                </>
              ) : (
                <button
                  className={`${btn} text-muted hover:text-ink`}
                  disabled={locked || selectedNames.length === 0}
                  onClick={() => setConfirming(true)}
                >
                  {t("action.uninstall")}
                </button>
              )}
            </div>
          )}
        </div>
      )}

      {notice && (
        <p className="anim-rise flex items-center justify-between gap-3 rounded border border-line bg-surface p-3 text-ok">
          <span className="break-all">{t("export.saved", { path: notice })}</span>
          <button onClick={clearNotice} className="shrink-0 text-muted hover:text-ink">
            {t("action.close")}
          </button>
        </p>
      )}
      {skipped.length > 0 && (
        <p className="anim-rise flex items-center justify-between gap-3 rounded border border-line bg-surface p-3">
          <span className="min-w-0 break-words">
            {t("mods.skipped", { count: skipped.length, names: skipped.slice(0, 6).join(", ") + (skipped.length > 6 ? "..." : "") })}
          </span>
          <button onClick={clearSkipped} className="shrink-0 text-muted hover:text-ink">
            {t("action.close")}
          </button>
        </p>
      )}
      {!gameDir && <p className="rounded border border-line bg-surface p-3 text-muted">{t("mods.noGame")}</p>}
      {running && <p className="rounded border border-danger bg-surface p-3 text-danger">{t("err.game_running")}</p>}

      <div className="flex min-h-0 flex-1 flex-col gap-6 overflow-auto pr-1">
        <NightCoopCard locked={locked} />
        {allKeys.length === 0 && (
          <div className="grid flex-1 place-items-center rounded-lg border border-dashed border-line p-8 text-center text-muted">
            {t("mods.empty")}
          </div>
        )}
        {nothingShown && <p className="text-muted">{t("mods.noResults")}</p>}
        {groups.length > 0 && (
          <ul className="flex flex-col gap-2">
            {groups.map((g, i) => (
              <ModCard
                key={g.key}
                mod={g.primary}
                disabled={locked}
                onEnable={() => enable(g.primary.name)}
                onDisable={() => disable(g.primary.name)}
                onUninstall={() => uninstall(g.primary.name)}
                selected={selected.includes(modKey(g.primary.name))}
                onSelect={({ shift }) => onSelect(modKey(g.primary.name), shift)}
                index={i}
                othersCount={g.others.length}
                expanded={openGroups.has(g.key)}
                onToggle={() =>
                  setOpenGroups((prev) => {
                    const next = new Set(prev);
                    if (!next.delete(g.key)) next.add(g.key);
                    return next;
                  })
                }
                others={g.others.map((o) => (
                  <VersionRow
                    key={o.name}
                    mod={o}
                    relation={relation(o.version, g.primary.version)}
                    disabled={locked}
                    onEnable={() => enable(o.name)}
                    onDisable={() => disable(o.name)}
                    onUninstall={() => uninstall(o.name)}
                    selected={selected.includes(modKey(o.name))}
                    onSelect={({ shift }) => onSelect(modKey(o.name), shift)}
                  />
                ))}
              />
            ))}
          </ul>
        )}
        {shownExternal.length > 0 && (
          <section className="flex flex-col gap-2">
            <h2 className="font-medium">{t("mods.external")}</h2>
            <p className="text-muted">{t("mods.externalHint")}</p>
            <ul className="flex flex-col gap-2">
              {shownExternal.map((m, i) => (
                <ExternalRow
                  key={m.name}
                  mod={m}
                  disabled={busy}
                  onImport={() => importExternal(m.name)}
                  version={m.kind === "red4ext" && m.name.toLowerCase() === "nightcoop" ? detected?.version : null}
                  selected={selected.includes(extKey(m.name))}
                  onSelect={({ shift }) => onSelect(extKey(m.name), shift)}
                  index={groups.length + i}
                />
              ))}
            </ul>
          </section>
        )}
      </div>

      {over && (
        <div className="pointer-events-none absolute inset-0 grid place-items-center rounded-lg border-2 border-dashed border-accent bg-bg/80 text-lg">
          {t("mods.drop")}
        </div>
      )}
      <ConflictDialog />
      <ModDetails />
    </div>
  );
}

