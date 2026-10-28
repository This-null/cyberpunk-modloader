import { useState } from "react";
import clsx from "clsx";
import { useT } from "../../i18n";
import type { Relation } from "../../lib/groups";
import type { ModRecord } from "../../lib/ipc";
import { useMods } from "../../stores/mods";
import { useDependents } from "./useDependents";

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
  const neededBy = useDependents(mod, armed);
  const openDetail = useMods((s) => s.openDetail);
  const small = "btn btn-sm";

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
        className="shrink-0"
      />
      <span
        className={clsx(
          "h-3 w-3 shrink-0 rounded-full",
          mod.enabled ? "bg-ok shadow-[0_0_10px_2px_color-mix(in_srgb,var(--ok)_60%,transparent)]" : "border-2 border-muted bg-transparent",
        )}
        aria-hidden
      />
      <div className="flex min-w-0 flex-1 items-center gap-2">
        <span className="truncate font-medium">{mod.name}</span>
        {mod.version && <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">{mod.version}</span>}
        {relation === "older" && <span className="shrink-0 text-xs text-muted">{t("mods.older")}</span>}
        {relation === "newer" && <span className="shrink-0 text-xs font-medium text-accent">{t("mods.newer")}</span>}
        <span className="shrink-0 text-xs text-muted">{mod.enabled ? t("mods.enabled") : t("mods.disabled")}</span>
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <button disabled={disabled} onClick={mod.enabled ? onDisable : onEnable} className={clsx("btn btn-sm", mod.enabled ? "btn-danger" : "btn-ok")}>
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
              className="btn btn-sm btn-danger-solid"
            >
              {t("action.uninstall")}
            </button>
            <button onClick={() => setArmed(false)} className={small}>
              {t("action.cancel")}
            </button>
          </>
        ) : (
          <button disabled={disabled} onClick={() => setArmed(true)} className="btn btn-sm btn-danger opacity-80 hover:opacity-100">
            {t("action.uninstall")}
          </button>
        )}
      </div>
      {armed && neededBy.length > 0 && (
        <p className="anim-rise basis-full rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-xs text-danger">
          {t("mods.neededBy", { names: neededBy.join(", ") })}
        </p>
      )}
    </li>
  );
}
