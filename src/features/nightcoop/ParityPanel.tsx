import { Check, Copy } from "lucide-react";
import { useState } from "react";
import { useT, type MessageKey } from "../../i18n";
import { ipc, toAppError, type ParityReport } from "../../lib/ipc";

const DISCORD_LIMIT = 1900;

function headerLabel(key: string): MessageKey {
  if (key === "game") return "parity.game";
  if (key === "nc") return "parity.nc";
  if (key === "protocol") return "parity.protocol";
  return "parity.framework";
}

function List({ title, items }: { title: string; items: string[] }) {
  if (items.length === 0) return null;
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs font-medium uppercase tracking-wide text-muted">
        {title} ({items.length})
      </span>
      <ul className="flex max-h-40 list-disc flex-col gap-0.5 overflow-auto pl-5 pr-2 text-ink">
        {items.map((n) => (
          <li key={n}>{n}</li>
        ))}
      </ul>
    </div>
  );
}

export function ParityPanel() {
  const t = useT();
  const [pasted, setPasted] = useState("");
  const [report, setReport] = useState<ParityReport | null>(null);
  const [error, setError] = useState(false);
  const [copied, setCopied] = useState(false);
  const [long, setLong] = useState(false);
  const [busy, setBusy] = useState(false);

  async function copy() {
    setBusy(true);
    try {
      const code = await ipc.parityCode();
      await navigator.clipboard.writeText(code);
      setLong(code.length > DISCORD_LIMIT);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } finally {
      setBusy(false);
    }
  }

  async function compare() {
    setBusy(true);
    setError(false);
    try {
      setReport(await ipc.parityCompare(pasted));
    } catch (e) {
      setReport(null);
      setError(toAppError(e).code === "unmappable");
    } finally {
      setBusy(false);
    }
  }

  return (
    <details className="text-muted">
      <summary className="cursor-pointer">{t("parity.title")}</summary>
      <div className="mt-2 flex flex-col gap-3 text-ink">
        <p className="text-xs text-muted">{t("parity.hint")}</p>
        <div className="flex flex-wrap items-center gap-3">
          <button disabled={busy} onClick={() => void copy()} className="btn btn-sm disabled:opacity-60">
            {copied ? <Check size={14} /> : <Copy size={14} />}
            {copied ? t("parity.copied") : t("parity.make")}
          </button>
          {long && <span className="text-xs text-danger">{t("parity.long")}</span>}
        </div>
        <textarea
          value={pasted}
          onChange={(e) => setPasted(e.target.value)}
          placeholder={t("parity.paste")}
          rows={3}
          className="field w-full resize-none font-mono text-xs"
        />
        <div className="flex items-center gap-3">
          <button disabled={busy || pasted.trim() === ""} onClick={() => void compare()} className="btn btn-sm btn-primary disabled:opacity-60">
            {t("parity.compare")}
          </button>
          {error && <span className="text-xs text-danger">{t("parity.invalid")}</span>}
        </div>
        {report && (
          <div className="anim-rise flex flex-col gap-3 rounded-md bg-surface-2/70 p-3">
            {report.same ? (
              <p className="flex items-center gap-2 text-ok">
                <Check size={15} />
                {t("parity.same")}
              </p>
            ) : (
              <>
                {report.header.length > 0 && (
                  <ul className="flex flex-col gap-1">
                    {report.header.map((d) => (
                      <li key={d.key} className="text-danger">
                        {t(headerLabel(d.key))}
                        {d.key.startsWith("fw:") ? ` ${d.key.slice(3)}` : ""}: {d.mine || "-"} / {d.theirs || "-"}
                      </li>
                    ))}
                  </ul>
                )}
                <List title={t("parity.onlyMine")} items={report.only_mine} />
                <List title={t("parity.onlyTheirs")} items={report.only_theirs} />
                <List title={t("parity.versions")} items={report.versions.map((v) => `${v.name}: ${v.mine} / ${v.theirs}`)} />
                <List title={t("parity.files")} items={report.files} />
              </>
            )}
          </div>
        )}
      </div>
    </details>
  );
}
