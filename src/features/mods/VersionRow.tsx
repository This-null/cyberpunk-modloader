import { useState } from "react";
import clsx from "clsx";
import { useT } from "../../i18n";
import type { Relation } from "../../lib/groups";
import type { ModRecord } from "../../lib/ipc";
import { useMods } from "../../stores/mods";

interface Props {
  mod: ModRecord;
  relation: Relation;
  disabled: boolean;
  onEnable: () => void;
  onDisable: () => void;
  onUninstall: () => void;
  selected: boolean;
  onSelect: (mods: { ctrl: boolean; shift: boolean }) => void;
}

export function VersionRow({ mod, relation, disabled, onEnable, onDisable, onUninstall, selected, onSelect }: Props) {
  const t = useT();
  const [armed, setArmed] = useState(false);
  const openDetail = useMods((s) => s.openDetail);
  const small = "rounded border border-line px-3 py-1 text-sm hover:bg-surface";

  return (
    <li
      className={clsx(
        "flex flex-wrap items-center gap-3 rounded-md border bg-surface-2 px-3 py-2",
        selected ? "border-accent" : "border-line",
      )}
      onClickCapture={(e) => {
        if (e.ctrlKey || e.metaKey) {
          e.preventDefault();
          e.stopPropagation();
          onSelect({ ctrl: true, shift: e.shiftKey });
        }
      }}
    >
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
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <span className="truncate font-medium">{mod.name}</span>
        {mod.version && <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">{mod.version}</span>}
        {relation === "older" && <span className="shrink-0 text-xs text-muted">{t("mods.older")}</span>}
        {relation === "newer" && <span className="shrink-0 text-xs font-medium text-accent">{t("mods.newer")}</span>}
        <span className="shrink-0 text-xs text-muted">{mod.enabled ? t("mods.enabled") : t("mods.disabled")}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <button disabled={disabled} onClick={mod.enabled ? onDisable : onEnable} className={clsx(small, !mod.enabled && "bg-accent font-medium text-accent-ink hover:bg-accent")}>
          {mod.enabled ? t("action.disable") : t("action.enable")}
        </button>
        <button onClick={() => openDetail(mod.name)} className={small}>
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
              className="rounded bg-danger px-3 py-1 text-sm font-medium text-white"
            >
              {t("action.uninstall")}
            </button>
            <button onClick={() => setArmed(false)} className={small}>
              {t("action.cancel")}
            </button>
          </>
        ) : (
          <button disabled={disabled} onClick={() => setArmed(true)} className={`${small} text-muted hover:text-ink`}>
            {t("action.uninstall")}
          </button>
        )}
      </div>
    </li>
  );
}
