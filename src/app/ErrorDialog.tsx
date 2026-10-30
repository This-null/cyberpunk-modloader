import { useEffect, useState } from "react";
import { Copy, Download, ExternalLink, X } from "lucide-react";
import { activeLocale, useLocale, useT } from "../i18n";
import { Dropdown } from "../components/Dropdown";
import { ipc, type LogFile } from "../lib/ipc";
import { errorMessageKey, useErrors } from "../stores/errors";

export function ErrorDialog() {
  const t = useT();
  const err = useErrors((s) => s.current);
  const dismiss = useErrors((s) => s.dismiss);
  const choice = useLocale((s) => s.choice);
  const [note, setNote] = useState("");
  const [report, setReport] = useState("");
  const [status, setStatus] = useState("");
  const [logFiles, setLogFiles] = useState<LogFile[]>([]);
  const [logId, setLogId] = useState<string | null>(null);

  useEffect(() => {
    setNote("");
    setStatus("");
    setLogId(null);
    if (err) ipc.logsScan().then(setLogFiles).catch(() => setLogFiles([]));
  }, [err]);

  useEffect(() => {
    if (!err) return;
    let alive = true;
    ipc
      .buildErrorReport(err.technical, note, activeLocale(choice), logId)
      .then((r) => alive && setReport(r))
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [err, note, choice, logId]);

  if (!err) return null;

  if (err.code === "no_files") {
    const note = err.technical.split("\n---\n")[1]?.trim() ?? "";
    const links = [...new Set((note.match(/https:\/\/(?:www\.)?nexusmods\.com\/[^\s"')>]+/gi) ?? []).map((u) => u.replace(/[.,;:]+$/, "")))];
    return (
      <div className="fixed inset-0 z-50 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
        <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
          <h2 className="text-lg font-semibold">{t("error.noFilesTitle")}</h2>
          <p className="text-muted">{t("err.no_files")}</p>
          {note && (
            <div className="flex flex-col gap-1">
              <span className="text-xs text-muted">{t("error.authorNote")}</span>
              <pre className="select-text whitespace-pre-wrap break-words rounded border border-line bg-surface-2 p-3 text-sm">{note}</pre>
            </div>
          )}
          <div className="flex flex-wrap justify-end gap-2">
            {links.slice(0, 3).map((u) => (
              <button key={u} onClick={() => void ipc.openUrl(u)} className="btn btn-primary">
                <ExternalLink size={15} />
                {t("error.openLink")}
              </button>
            ))}
            <button onClick={dismiss} className="btn">
              <X size={15} />
              {t("action.close")}
            </button>
          </div>
        </div>
      </div>
    );
  }

  const key = errorMessageKey(err.code);

  async function save() {
    try {
      const path = await ipc.saveErrorReport(report);
      await ipc.openFolder(path);
      setStatus(t("error.saved", { path }));
    } catch (e) {
      setStatus(String(e));
    }
  }

  async function copy() {
    await navigator.clipboard.writeText(report);
    setStatus(t("error.copied"));
  }

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <h2 className="text-lg font-semibold">{t("error.title")}</h2>
        <p>{key ? t(key) : t("error.title")}</p>
        <details className="text-muted">
          <summary className="cursor-pointer">{t("error.details")}</summary>
          <pre className="mt-2 whitespace-pre-wrap break-words rounded bg-surface-2 p-3 text-xs">{err.technical}</pre>
        </details>
        <label className="flex flex-col gap-1">
          <span className="text-muted">{t("error.note")}</span>
          <textarea
            value={note}
            onChange={(e) => setNote(e.target.value)}
            rows={2}
            className="resize-none rounded border border-line bg-surface-2 p-2"
          />
        </label>
        <label className="flex flex-col gap-1">
          <span className="text-muted">{t("error.addLog")}</span>
          <Dropdown
            value={logId ?? ""}
            onChange={(v) => setLogId(v || null)}
            ariaLabel={t("error.addLog")}
            options={[{ value: "", label: t("error.noLog") }, ...logFiles.slice(0, 40).map((f) => ({ value: f.id, label: `${f.source} · ${f.name}` }))]}
          />
        </label>
        <details className="text-muted">
          <summary className="cursor-pointer">{t("error.preview")}</summary>
          <pre className="mt-2 max-h-48 overflow-auto whitespace-pre-wrap break-words rounded bg-surface-2 p-3 text-xs">{report}</pre>
        </details>
        <p className="text-muted">{t("error.hint")}</p>
        {status && <p className="text-ok">{status}</p>}
        <div className="flex flex-wrap justify-end gap-2">
          <button onClick={copy} className="btn">
            <Copy size={15} />
            {t("error.copy")}
          </button>
          <button onClick={save} className="btn btn-primary">
            <Download size={15} />
            {t("error.save")}
          </button>
          <button onClick={dismiss} className="btn">
            <X size={15} />
            {t("action.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
