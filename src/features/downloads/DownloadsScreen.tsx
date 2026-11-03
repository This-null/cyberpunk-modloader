import { AlertTriangle, Check, Download, PackageOpen, X } from "lucide-react";
import { useEffect, useState } from "react";
import clsx from "clsx";
import { activeLocale, useLocale, useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { formatSize, percent } from "../../lib/nightcoop";
import { isActive, useWebDownload, type DownloadItem } from "../../stores/webDownload";

function ago(at: number, locale: string): string {
  const mins = Math.round((Date.now() - at) / 60000);
  const rtf = new Intl.RelativeTimeFormat(locale, { numeric: "auto" });
  if (mins < 1) return rtf.format(0, "second");
  if (mins < 60) return rtf.format(-mins, "minute");
  if (mins < 60 * 24) return rtf.format(-Math.round(mins / 60), "hour");
  return rtf.format(-Math.round(mins / 1440), "day");
}

function Row({ item }: { item: DownloadItem }) {
  const t = useT();
  const locale = activeLocale(useLocale((s) => s.choice));
  const dismiss = useWebDownload((s) => s.dismiss);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (item.phase !== "downloading") return;
    const id = setInterval(() => setNow(Date.now()), 5000);
    return () => clearInterval(id);
  }, [item.phase]);
  const stalled = item.phase === "downloading" && now - item.moved > 20000;
  const pct = item.total > 0 ? percent(item.done, item.total) : null;
  const finished = item.phase === "done";
  const failed = item.phase === "error" || item.phase === "cancelled";
  const status =
    item.phase === "downloading"
      ? [pct === null ? formatSize(item.done) : `${formatSize(item.done)} / ${formatSize(item.total)}`, pct === null ? "" : `${pct}%`].filter(Boolean).join(" · ")
      : item.phase === "installing"
        ? t("downloads.installing")
        : finished
          ? t("downloads.done")
          : item.phase === "cancelled"
            ? t("downloads.cancelled")
            : (item.message ?? t("downloads.failed"));
  const Icon = finished ? Check : failed ? AlertTriangle : item.phase === "installing" ? PackageOpen : Download;

  return (
    <li className="anim-rise flex flex-col gap-2 rounded-lg border border-line bg-surface p-4">
      <div className="flex items-center gap-3">
        <Icon size={18} className={clsx("shrink-0", finished ? "text-ok" : failed ? "text-danger" : "text-accent")} />
        <span className="min-w-0 flex-1 truncate font-medium">{item.name}</span>
        <button
          aria-label={item.phase === "downloading" ? t("downloads.cancel") : t("downloads.dismiss")}
          title={item.phase === "downloading" ? t("downloads.cancel") : t("downloads.dismiss")}
          disabled={item.phase === "installing"}
          onClick={() => (item.phase === "downloading" ? void ipc.cancelWebDownload(item.id) : dismiss(item.id))}
          className="btn btn-sm disabled:opacity-40"
        >
          <X size={14} />
        </button>
      </div>
      <span className={clsx("text-xs", failed ? "text-danger" : "text-muted")}>
        {status}
        {!isActive(item) && ` · ${ago(item.at, locale)}`}
      </span>
      {stalled && <span className="text-xs text-danger">{t("downloads.stalled")}</span>}
      {isActive(item) && (
        <div className="h-1.5 overflow-hidden bg-surface-2">
          <div
            className={clsx("h-full bg-accent transition-[width] duration-300", (pct === null || item.phase === "installing") && "w-full animate-pulse")}
            style={pct === null || item.phase === "installing" ? undefined : { width: `${pct}%` }}
          />
        </div>
      )}
    </li>
  );
}

export function DownloadsScreen() {
  const t = useT();
  const items = useWebDownload((s) => s.items);
  const clear = useWebDownload((s) => s.clearFinished);
  const finished = items.some((i) => !isActive(i));

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-4">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-xl font-semibold">{t("nav.downloads")}</h1>
        {finished && (
          <button onClick={clear} className="btn">
            {t("downloads.clear")}
          </button>
        )}
      </div>
      {items.length === 0 ? (
        <p className="rounded-lg border border-dashed border-line p-8 text-center text-muted">{t("downloads.empty")}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {items.map((i) => (
            <Row key={i.id} item={i} />
          ))}
        </ul>
      )}
    </div>
  );
}
