import { getCurrentWindow } from "@tauri-apps/api/window";
import { Minus, Square, X } from "lucide-react";
import { Logo } from "../brand/Logo";
import { useT } from "../i18n";

export function TitleBar() {
  const t = useT();
  const win = getCurrentWindow();
  const btn = "grid h-10 w-12 place-items-center text-muted hover:bg-surface-2 hover:text-ink";

  return (
    <div data-tauri-drag-region className="flex h-10 shrink-0 items-center justify-between border-b border-line bg-surface">
      <div data-tauri-drag-region className="flex items-center gap-2 pl-4 text-sm font-medium tracking-wide">
        <Logo size={18} animated className="pointer-events-none text-accent" />
        {t("app.name")}
      </div>
      <div className="flex">
        <button className={btn} aria-label={t("window.minimize")} onClick={() => win.minimize()}>
          <Minus size={16} />
        </button>
        <button className={btn} aria-label={t("window.maximize")} onClick={() => win.toggleMaximize()}>
          <Square size={13} />
        </button>
        <button className={`${btn} hover:!bg-danger hover:!text-white`} aria-label={t("window.close")} onClick={() => win.close()}>
          <X size={16} />
        </button>
      </div>
    </div>
  );
}
