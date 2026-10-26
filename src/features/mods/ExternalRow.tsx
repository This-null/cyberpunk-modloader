import type { CSSProperties } from "react";
import clsx from "clsx";
import { useT, type MessageKey } from "../../i18n";
import type { ExternalMod } from "../../lib/ipc";

interface Props {
  mod: ExternalMod;
  disabled: boolean;
  onImport: () => void;
  version?: string | null;
  selected: boolean;
  onSelect: (mods: { ctrl: boolean; shift: boolean }) => void;
  index?: number;
}

export function ExternalRow({ mod, disabled, onImport, version, selected, onSelect, index = 0 }: Props) {
  const t = useT();

  return (
    <li
      className={clsx(
        "anim-rise flex items-center gap-4 rounded-lg border border-dashed bg-surface px-4 py-3",
        selected ? "border-accent" : "border-line",
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
      <span className="h-2.5 w-2.5 shrink-0 rounded-full bg-accent" aria-hidden />
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <span className="truncate font-medium">{mod.name}</span>
          {version && <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">{version}</span>}
        </div>
        <div className="text-xs text-muted">
          {t(`kind.${mod.kind}` as MessageKey)} · {t("mods.files", { count: mod.files.length })}
        </div>
      </div>
      <button disabled={disabled} onClick={onImport} className="rounded border border-line px-4 py-1.5 hover:bg-surface-2">
        {t("action.import")}
      </button>
    </li>
  );
}
