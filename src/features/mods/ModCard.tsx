import { useEffect, useState, type CSSProperties, type ReactNode } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import clsx from "clsx";
import { Logo } from "../../brand/Logo";
import { useT, type MessageKey } from "../../i18n";
import type { ModRecord } from "../../lib/ipc";
import { kindsOf } from "../../lib/kinds";
import { ipc } from "../../lib/ipc";
import { useMods } from "../../stores/mods";
import { useNexus } from "../../stores/nexus";

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
  const [armed, setArmed] = useState(false);
  const image = useMods((s) => s.images[mod.name]);
  const loadImage = useMods((s) => s.loadImage);
  const openDetail = useMods((s) => s.openDetail);
  const item = useNexus((s) => s.items[mod.name]);

  useEffect(() => {
    void loadImage(mod.name);
  }, [mod.name, mod.image, loadImage]);

  return (
    <li
      className={clsx("anim-rise rounded-lg border bg-surface p-4", selected ? "border-accent" : "border-line")}
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
            className="h-4 w-4 shrink-0 cursor-pointer accent-[var(--accent)]"
          />
          <span className={clsx("h-2.5 w-2.5 shrink-0 rounded-full", mod.enabled ? "bg-ok" : "bg-muted")} aria-hidden />
          <button onClick={() => openDetail(mod.name)} className="truncate text-left text-base font-semibold hover:underline">
            {mod.name}
          </button>
          {mod.version && (
            <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">{mod.version}</span>
          )}
          {item?.update && item.latest && (
            <button
              onClick={() => void ipc.openUrl(item.page_url)}
              title={t("nexus.openOnNexus")}
              className="shrink-0 rounded bg-accent px-1.5 py-0.5 text-xs font-medium text-accent-ink"
            >
              {t("nexus.updateTo", { version: item.latest })}
            </button>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5 text-xs text-muted">
          <span>{mod.enabled ? t("mods.enabled") : t("mods.disabled")}</span>
          <span aria-hidden>·</span>
          <span>{t("mods.files", { count: mod.files.length })}</span>
          {kindsOf(mod.files).map((k) => (
            <span key={k} className="rounded bg-surface-2 px-1.5 py-0.5">
              {t(`kind.${k}` as MessageKey)}
            </span>
          ))}
        </div>
        <p className="line-clamp-3 text-muted">{mod.description || t("mods.noDescription")}</p>
        <div className="mt-auto flex flex-wrap items-center gap-2 pt-2">
          <button
            disabled={disabled}
            onClick={mod.enabled ? onDisable : onEnable}
            className={clsx(
              "rounded px-4 py-1.5",
              mod.enabled ? "border border-line hover:bg-surface-2" : "bg-accent font-medium text-accent-ink",
            )}
          >
            {mod.enabled ? t("action.disable") : t("action.enable")}
          </button>
          <button onClick={() => openDetail(mod.name)} className="rounded border border-line px-3 py-1.5 hover:bg-surface-2">
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
                className="rounded bg-danger px-3 py-1.5 font-medium text-white"
              >
                {t("action.uninstall")}
              </button>
              <button onClick={() => setArmed(false)} className="rounded border border-line px-3 py-1.5 hover:bg-surface-2">
                {t("action.cancel")}
              </button>
            </>
          ) : (
            <button
              disabled={disabled}
              onClick={() => setArmed(true)}
              className="rounded border border-line px-3 py-1.5 text-muted hover:bg-surface-2 hover:text-ink"
            >
              {t("action.uninstall")}
            </button>
          )}
        </div>
      </div>
      <div className="grid h-[117px] w-[208px] shrink-0 place-items-center overflow-hidden rounded-md bg-surface-2">
        {image ? (
          <img src={image} alt="" className="h-full w-full object-cover" draggable={false} />
        ) : (
          <Logo size={44} className="text-line" />
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
