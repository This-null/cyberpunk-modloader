import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Download, Package, X } from "lucide-react";
import { useT } from "../i18n";
import { ipc, type NxmLink } from "../lib/ipc";
import { formatSize, percent } from "../lib/nightcoop";
import { useIncoming } from "../stores/incoming";
import { useUpdate } from "../stores/update";
import { useMods } from "../stores/mods";
import { useNexus } from "../stores/nexus";

export function useIncomingEvents() {
  useEffect(() => {
    const receive = useIncoming.getState().receive;
    void ipc.takePendingNxm().then((l) => l && void receive(l)).catch(() => {});
    const un = listen<NxmLink>("nxm-link", (e) => void receive(e.payload));
    const onFocus = () => void useIncoming.getState().check();
    window.addEventListener("focus", onFocus);
    return () => {
      void un.then((f) => f());
      window.removeEventListener("focus", onFocus);
    };
  }, []);
}

export function IncomingDialog() {
  const t = useT();
  const link = useIncoming((s) => s.link);
  const name = useIncoming((s) => s.name);
  const expired = useIncoming((s) => s.expired);
  const installing = useIncoming((s) => s.installing);
  const progress = useIncoming((s) => s.progress);
  const confirm = useIncoming((s) => s.confirmLink);
  const cancel = useIncoming((s) => s.cancelLink);
  const connected = useNexus((s) => s.status?.connected ?? false);
  const busy = useMods((s) => s.busy);

  if (!link) return null;
  const pct = progress ? percent(progress.done, progress.total) : 0;

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex w-full max-w-md flex-col gap-4 rounded-lg border border-line bg-surface p-6">
        <h2 className="text-lg font-semibold">{t("incoming.title")}</h2>
        <p className="text-muted">{t("incoming.body", { name: name || `#${link.mod_id}` })}</p>
        {expired && <p className="text-danger">{t("incoming.expired")}</p>}
        {!connected && <p className="text-danger">{t("incoming.needKey")}</p>}
        {installing && (
          <div className="flex flex-col gap-1">
            <div className="h-2 overflow-hidden rounded bg-surface-2">
              <div className="h-full bg-accent transition-[width]" style={{ width: `${pct}%` }} />
            </div>
            <span className="text-xs text-muted">{t("nightcoop.downloading", { percent: pct })}</span>
          </div>
        )}
        <div className="flex justify-end gap-2">
          <button disabled={installing} onClick={cancel} className="btn">
            {t("action.cancel")}
          </button>
          <button
            disabled={installing || expired || !connected || busy}
            onClick={() => void confirm()}
            className="btn btn-primary disabled:opacity-60"
          >
            {t("incoming.install")}
          </button>
        </div>
      </div>
    </div>
  );
}

export function DownloadsBanner() {
  const t = useT();
  const found = useIncoming((s) => s.found);
  const dismiss = useIncoming((s) => s.dismiss);
  const install = useIncoming((s) => s.installFound);
  const busy = useMods((s) => s.busy);
  const first = found[0];
  if (!first) return null;
  return (
    <div className="anim-rise flex items-center gap-3 border-b border-accent/40 bg-accent/10 px-4 py-2">
      <Download size={16} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 truncate">
        {t("incoming.found", { name: first.name, size: formatSize(first.size) })}
        {found.length > 1 && <span className="text-muted"> {t("incoming.more", { count: found.length - 1 })}</span>}
      </span>
      <button
        disabled={busy}
        onClick={() => void install(first.path)}
        className="btn btn-sm btn-primary disabled:opacity-60"
      >
        {t("incoming.install")}
      </button>
      <button onClick={() => dismiss(first.path)} aria-label={t("action.close")} className="rounded p-1 text-muted hover:text-ink">
        <X size={16} />
      </button>
    </div>
  );
}

export function UpdateBanner() {
  const t = useT();
  const status = useUpdate((s) => s.status);
  const version = useUpdate((s) => s.version);
  const percent = useUpdate((s) => s.percent);
  const hidden = useUpdate((s) => s.hidden);
  const install = useUpdate((s) => s.install);
  const hide = useUpdate((s) => s.hide);
  const busy = useMods((s) => s.busy);
  if (hidden || (status !== "available" && status !== "downloading")) return null;
  return (
    <div className="anim-rise flex items-center gap-3 border-b border-accent/40 bg-accent/10 px-4 py-2">
      <Download size={16} className="shrink-0 text-accent" />
      <span className="min-w-0 flex-1 truncate">
        {status === "downloading" ? t("update.downloading", { percent }) : t("update.available", { version })}
      </span>
      {status === "available" && (
        <>
          <button
            disabled={busy}
            onClick={() => void install()}
            className="btn btn-sm btn-primary disabled:opacity-60"
          >
            {t("update.now")}
          </button>
          <button onClick={hide} className="btn btn-sm">
            {t("update.later")}
          </button>
        </>
      )}
    </div>
  );
}

export function ExportBanner() {
  const t = useT();
  const progress = useMods((s) => s.exporting);
  if (!progress) return null;
  const pct = progress.total > 0 ? Math.round((progress.done / progress.total) * 100) : 0;
  return (
    <div className="anim-rise border-b border-accent/40 bg-accent/10 px-4 py-2">
      <div className="flex items-center gap-3">
        <Package size={16} className="shrink-0 text-accent" />
        <span className="min-w-0 flex-1 truncate">{t("export.packing", { percent: pct })}</span>
      </div>
      <div className="mt-1.5 h-1.5 overflow-hidden rounded-full bg-surface-2">
        <div className="h-full rounded-full bg-accent transition-[width] duration-200" style={{ width: `${pct}%` }} />
      </div>
    </div>
  );
}
