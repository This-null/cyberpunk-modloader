import { useTranslated } from "../../lib/translate";
import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { ChevronDown, ChevronRight, Info, Power, PowerOff, RefreshCw, Trash2, TriangleAlert } from "lucide-react";
import clsx from "clsx";
import { CoverPlaceholder } from "../../brand/CoverPlaceholder";
import { useT, type MessageKey } from "../../i18n";
import type { ModRecord } from "../../lib/ipc";
import { kindsOf } from "../../lib/kinds";
import { ipc } from "../../lib/ipc";
import { useMods } from "../../stores/mods";
import { useNexus } from "../../stores/nexus";
import { percent } from "../../lib/nightcoop";
import { useGameInfo } from "../../stores/gameInfo";
import { downloadPageUrl, pickFile } from "../../lib/nexusPage";
import { useModUpdates } from "../../stores/modUpdates";
import { useNexusBrowse } from "../../stores/nexusBrowse";
import { useDependents } from "./useDependents";

interface Props {
  mod: ModRecord;
  disabled: boolean;
  onEnable: () => void;
  onDisable: () => void;
  onUninstall: () => void;
  selected: boolean;
  onSelect: (mods: { ctrl: boolean; shift: boolean }) => void;
  index?: number;
  others?: ReactNode;
  othersCount?: number;
  expanded?: boolean;
  onToggle?: () => void;
}

export function ModCard({
  mod,
  disabled,
  onEnable,
  onDisable,
  onUninstall,
  selected,
  onSelect,
  index = 0,
  others,
  othersCount = 0,
  expanded = false,
  onToggle,
}: Props) {
  const t = useT();
  const description = useTranslated(mod.description);
  const [armed, setArmed] = useState(false);
  const neededBy = useDependents(mod, armed);
  const image = useMods((s) => s.images[mod.name]);
  const loadImage = useMods((s) => s.loadImage);
  const openDetail = useMods((s) => s.openDetail);
  const item = useNexus((s) => s.items[mod.name]);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);
  const running = useModUpdates((s) => s.running);
  const updateOne = useModUpdates((s) => s.updateOne);
  const patchTime = useGameInfo((s) => s.patchTime);
  const progress = useNexusBrowse((s) => s.progress);
  const chain = useNexusBrowse((s) => s.chain);
  const pct = progress ? percent(progress.done, progress.total) : 0;
  const updateLabel = chain ? `${Math.min(chain.done + 1, chain.total)}/${chain.total} · ${pct}%` : `${pct}%`;

  async function openDownloadPage(modId: number, fallback: string) {
    const file = await ipc.nexusFiles(modId).then(pickFile).catch(() => null);
    void ipc.openUrl(file ? downloadPageUrl(modId, file.file_id) : fallback);
  }

  useEffect(() => {
    void loadImage(mod.name);
  }, [mod.name, mod.image, loadImage]);

  return (
    <li
      className={clsx(
        "anim-rise rounded-lg border border-l-[3px] bg-surface p-4 transition-opacity",
        selected ? "border-accent" : "border-line",
        mod.enabled ? "border-l-ok" : "border-l-line opacity-90 hover:opacity-100",
      )}
      style={{ "--i": Math.min(index, 8) } as CSSProperties}
      onClickCapture={(e) => {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          e.stopPropagation();
          onSelect({ ctrl: true, shift: e.shiftKey });
        }
      }}
    >
      <div className="flex gap-5">
      <div className="flex min-w-0 flex-1 flex-col gap-2">
        <div className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={selected}
            onChange={() => {}}
            onClick={(e) => {
              e.stopPropagation();
              onSelect({ ctrl: true, shift: e.shiftKey });
            }}
            aria-label={t("mods.select", { name: mod.name })}
            className="shrink-0"
          />
          <button onClick={() => openDetail(mod.name)} className="truncate text-left text-base font-semibold hover:underline">
            {mod.name}
          </button>
          {mod.version && (
            <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">{mod.version}</span>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
          <span
            className={clsx(
              "inline-flex items-center gap-1.5 rounded-full px-2 py-0.5 font-medium",
              mod.enabled ? "bg-ok/15 text-ok" : "bg-surface-2 text-muted",
            )}
          >
            <span
              className={clsx(
                "h-1.5 w-1.5 rounded-full bg-current",
                mod.enabled && "shadow-[0_0_8px_1px_color-mix(in_srgb,var(--ok)_70%,transparent)]",
              )}
              aria-hidden
            />
            {mod.enabled ? t("mods.enabled") : t("mods.disabled")}
          </span>
          <span>{t("mods.files", { count: mod.files.length })}</span>
          {kindsOf(mod.files).map((k) => (
            <span key={k} className="rounded bg-surface-2 px-1.5 py-0.5">
              {t(`kind.${k}` as MessageKey)}
            </span>
          ))}
        </div>
        <p className="line-clamp-3 text-muted">{description || t("mods.noDescription")}</p>
        <div className="mt-auto flex flex-wrap items-center gap-2 pt-2">
          <button disabled={disabled} onClick={mod.enabled ? onDisable : onEnable} className={clsx("btn", mod.enabled ? "btn-danger" : "btn-ok")}>
            {mod.enabled ? <PowerOff size={15} /> : <Power size={15} />}
            {mod.enabled ? t("action.disable") : t("action.enable")}
          </button>
          {item?.updated != null &&
            patchTime != null &&
            item.updated < patchTime &&
            !item.update &&
            kindsOf(mod.files).some((k) => k === "cet" || k === "redscript" || k === "red4ext") && (
              <span className="inline-flex items-center gap-1.5 rounded border border-line px-2 py-1 text-xs text-muted" title={t("mods.staleHint")}>
                <TriangleAlert size={13} />
                {t("mods.staleChip")}
              </span>
            )}
          {(item?.error === "removed" || item?.error === "not_found") && (
            <span className="rounded border border-line px-2 py-1 text-xs text-muted" title={t("mods.removedHint")}>
              {t("mods.removedOnNexus")}
            </span>
          )}
          {item?.update && item.latest && (
            <button
              disabled={premium && (disabled || running !== null)}
              onClick={() => (premium ? void updateOne(mod.name) : void openDownloadPage(item.mod_id, item.page_url))}
              title={premium ? t("modupd.install") : t("nexus.openOnNexus")}
              className="btn btn-primary"
            >
              <RefreshCw size={14} className={running === mod.name ? "animate-spin" : undefined} />
              {running === mod.name ? t("modupd.progress", { percent: updateLabel }) : t("modupd.update", { version: item.latest })}
            </button>
          )}
          <button onClick={() => openDetail(mod.name)} className="btn">
            <Info size={15} />
            {t("action.details")}
          </button>
          {armed ? (
            <>
              <button
                disabled={disabled}
                onClick={() => {
                  setArmed(false);
                  onUninstall();
                }}
                className="btn btn-danger-solid"
              >
                <Trash2 size={15} />
                <Trash2 size={15} />
              {t("action.uninstall")}
              </button>
              <button onClick={() => setArmed(false)} className="btn">
                {t("action.cancel")}
              </button>
            </>
          ) : (
            <button
              disabled={disabled}
              onClick={() => setArmed(true)}
              className="btn btn-danger opacity-80 hover:opacity-100"
            >
              <Trash2 size={15} />
              {t("action.uninstall")}
            </button>
          )}
        </div>
        {armed && neededBy.length > 0 && (
          <p className="anim-rise rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-xs text-danger">
            {t("mods.neededBy", { names: neededBy.join(", ") })}
          </p>
        )}
      </div>
      <div className="grid h-[117px] w-[208px] shrink-0 place-items-center overflow-hidden rounded-md bg-surface-2">
        {image ? (
          <img src={image} alt="" className="h-full w-full object-cover" draggable={false} />
        ) : (
          <CoverPlaceholder name={mod.name} />
        )}
      </div>
      </div>
      {othersCount > 0 && (
        <div className="mt-3 border-t border-line pt-3">
          <button onClick={onToggle} aria-expanded={expanded} className="flex items-center gap-2 text-muted hover:text-ink">
            {expanded ? <ChevronDown size={16} /> : <ChevronRight size={16} />}
            {t("mods.otherVersions", { count: othersCount })}
          </button>
          {expanded && <ul className="anim-rise mt-2 flex flex-col gap-2">{others}</ul>}
        </div>
      )}
    </li>
  );
}
