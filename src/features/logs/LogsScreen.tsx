import { useEffect, useMemo, useRef, useState } from "react";
import clsx from "clsx";
import { Copy, ExternalLink, FolderOpen, RefreshCw, Trash2 } from "lucide-react";
import { activeLocale, useLocale, useT } from "../../i18n";
import { ipc, type LogFile } from "../../lib/ipc";
import {
  countLevels,
  filterLines,
  formatBytes,
  MAX_RENDERED,
  nextIndex,
  parseLines,
  type Filter,
} from "../../lib/logs";
import { useLogs } from "../../stores/logs";

function Section({
  title,
  items,
  selected,
  onPick,
  when,
  crashLabel,
}: {
  title: string;
  items: LogFile[];
  selected: string | null;
  onPick: (id: string) => void;
  when: (secs: number) => string;
  crashLabel: string;
}) {
  if (items.length === 0) return null;
  return (
    <div className="flex flex-col gap-1">
      <h3 className="px-2 pt-2 text-xs font-medium uppercase tracking-wide text-muted">{title}</h3>
      {items.map((f) => (
        <button
          key={f.id}
          onClick={() => onPick(f.id)}
          aria-current={selected === f.id}
          className={clsx(
            "flex flex-col gap-0.5 rounded px-2 py-1.5 text-left hover:bg-surface-2",
            selected === f.id && "bg-surface-2",
          )}
        >
          <span className="flex items-center gap-2">
            <span className="truncate text-sm">{f.name}</span>
            {f.crash && <span className="shrink-0 rounded bg-danger px-1 text-[10px] font-medium text-white">{crashLabel}</span>}
          </span>
          <span className="truncate text-xs text-muted">
            {f.source} · {formatBytes(f.size)} · {when(f.modified)}
          </span>
        </button>
      ))}
    </div>
  );
}

export function LogsScreen() {
  const t = useT();
  const choice = useLocale((s) => s.choice);
  const files = useLogs((s) => s.files);
  const summary = useLogs((s) => s.summary);
  const selected = useLogs((s) => s.selected);
  const text = useLogs((s) => s.text);
  const start = useLogs((s) => s.start);
  const size = useLogs((s) => s.size);
  const end = useLogs((s) => s.end);
  const loading = useLogs((s) => s.loading);
  const follow = useLogs((s) => s.follow);
  const loadList = useLogs((s) => s.loadList);
  const select = useLogs((s) => s.select);
  const earlier = useLogs((s) => s.earlier);
  const refresh = useLogs((s) => s.refresh);
  const poll = useLogs((s) => s.poll);
  const setFollow = useLogs((s) => s.setFollow);
  const clear = useLogs((s) => s.clear);
  const [asking, setAsking] = useState(false);
  const [freed, setFreed] = useState<number | null>(null);

  const [filter, setFilter] = useState<Filter>("all");
  const [query, setQuery] = useState("");
  const [copied, setCopied] = useState(false);
  const [active, setActive] = useState(-1);
  const scroller = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void loadList();
  }, [loadList]);

  useEffect(() => {
    if (!follow || !selected) return;
    const id = setInterval(() => void poll(), 2000);
    return () => clearInterval(id);
  }, [follow, selected, poll]);

  const lines = useMemo(() => parseLines(text), [text]);
  const counts = useMemo(() => countLevels(lines), [lines]);
  const filtered = useMemo(() => filterLines(lines, filter, query), [lines, filter, query]);
  const shown = filtered.length > MAX_RENDERED ? filtered.slice(-MAX_RENDERED) : filtered;

  useEffect(() => {
    setActive(-1);
  }, [selected, filter, query]);

  const current = files.find((f) => f.id === selected);

  useEffect(() => {
    const el = scroller.current;
    if (el && (follow || !loading)) el.scrollTop = current?.crash && !follow ? 0 : el.scrollHeight;
  }, [selected, loading, follow, shown.length, current?.crash]);

  const when = (secs: number) => new Date(secs * 1000).toLocaleString(activeLocale(choice));
  const pick = (id: string) => void select(id);

  function goNext() {
    const i = nextIndex(
      shown.map((l) => l.level),
      active,
    );
    if (i < 0) return;
    setActive(i);
    scroller.current?.querySelector(`[data-i="${i}"]`)?.scrollIntoView({ block: "center" });
  }

  async function copy() {
    await navigator.clipboard.writeText(shown.map((l) => l.text).join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  const total = files.reduce((n, f) => n + f.size, 0);

  async function wipe() {
    setAsking(false);
    const done = await clear();
    if (done) setFreed(done.freed);
  }

  const chip = "rounded border px-3 py-1.5";
  const crashLabel = t("logs.crashBadge");

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex flex-col">
          <h1 className="text-xl font-semibold">{t("logs.title")}</h1>
          <span className="text-xs text-muted">
            {freed !== null && files.length === 0
              ? t("logs.cleared", { size: formatBytes(freed) })
              : t("logs.total", { count: files.length, size: formatBytes(total) })}
          </span>
        </div>
        {files.length > 0 &&
          (asking ? (
            <div className="anim-pop flex items-center gap-2">
              <span className="text-muted">{t("logs.clearAsk", { count: files.length, size: formatBytes(total) })}</span>
              <button onClick={() => void wipe()} className="btn btn-danger-solid hover:opacity-90">
                {t("logs.clearYes")}
              </button>
              <button onClick={() => setAsking(false)} className="btn">
                {t("action.cancel")}
              </button>
            </div>
          ) : (
            <button onClick={() => { setFreed(null); setAsking(true); }} className="btn">
              <Trash2 size={15} />
              {t("logs.clear")}
            </button>
          ))}
      </div>

      {summary ? (
        <section className="flex flex-col gap-2 rounded-lg border border-danger bg-surface p-4">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <h2 className="font-semibold text-danger">{t("logs.lastCrash")}</h2>
              <span className="text-xs text-muted">
                {summary.source} · {summary.name} · {when(summary.modified)}
              </span>
            </div>
            <button onClick={() => pick(summary.id)} className="btn">
              {t("logs.openLog")}
            </button>
          </div>
          <pre className="max-h-32 select-text overflow-auto whitespace-pre-wrap break-words rounded bg-surface-2 p-3 font-mono text-xs">
            {summary.excerpt}
          </pre>
        </section>
      ) : (
        <p className="text-muted">{t("logs.noCrash")}</p>
      )}

      {files.length === 0 ? (
        <div className="grid flex-1 place-items-center rounded-lg border border-dashed border-line p-8 text-center text-muted">
          {t("logs.empty")}
        </div>
      ) : (
        <div className="flex min-h-0 flex-1 gap-4">
          <div className="flex w-72 shrink-0 flex-col gap-1 overflow-auto rounded-lg border border-line bg-surface p-2">
            <Section title={t("logs.crashes")} items={files.filter((f) => f.crash)} selected={selected} onPick={pick} when={when} crashLabel={crashLabel} />
            <Section title={t("logs.loaders")} items={files.filter((f) => !f.crash && f.group === "loader")} selected={selected} onPick={pick} when={when} crashLabel={crashLabel} />
            <Section title={t("logs.mods")} items={files.filter((f) => !f.crash && f.group === "mod")} selected={selected} onPick={pick} when={when} crashLabel={crashLabel} />
            <Section title={t("logs.launcher")} items={files.filter((f) => !f.crash && f.group === "launcher")} selected={selected} onPick={pick} when={when} crashLabel={crashLabel} />
          </div>

          <div className="flex min-w-0 flex-1 flex-col gap-3">
            {!current ? (
              <div className="grid flex-1 place-items-center rounded-lg border border-dashed border-line text-muted">{t("logs.pick")}</div>
            ) : (
              <>
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="min-w-0">
                    <div className="truncate font-medium">{current.name}</div>
                    <div className="text-xs text-muted">
                      {start > 0
                        ? t("logs.partial", { shown: formatBytes(size - start), total: formatBytes(size) })
                        : end < size
                          ? t("logs.partialHead", { shown: formatBytes(end), total: formatBytes(size) })
                          : formatBytes(size)}{" "}
                      · {t("logs.lines", { count: lines.length })}
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <button title={t("logs.refresh")} aria-label={t("logs.refresh")} onClick={() => void refresh()} className="btn btn-icon">
                      <RefreshCw size={15} />
                    </button>
                    <button title={t("logs.copy")} aria-label={t("logs.copy")} onClick={() => void copy()} className="btn btn-icon">
                      <Copy size={15} />
                    </button>
                    <button title={t("logs.openFile")} aria-label={t("logs.openFile")} onClick={() => void ipc.logsOpen(current.id)} className="btn btn-icon">
                      <ExternalLink size={15} />
                    </button>
                    <button title={t("logs.reveal")} aria-label={t("logs.reveal")} onClick={() => void ipc.logsReveal(current.id)} className="btn btn-icon">
                      <FolderOpen size={15} />
                    </button>
                    {copied && <span className="text-xs text-ok">{t("logs.copied")}</span>}
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-2">
                  {(
                    [
                      ["all", t("logs.filterAll")],
                      ["error", `${t("logs.filterErrors")} (${counts.error})`],
                      ["warn", `${t("logs.filterWarnings")} (${counts.warn})`],
                    ] as [Filter, string][]
                  ).map(([value, label]) => (
                    <button
                      key={value}
                      onClick={() => setFilter(value)}
                      aria-pressed={filter === value}
                      className={clsx(chip, filter === value ? "border-accent bg-accent text-accent-ink" : "border-line hover:bg-surface-2")}
                    >
                      {label}
                    </button>
                  ))}
                  <input
                    value={query}
                    onChange={(e) => setQuery(e.target.value)}
                    placeholder={t("logs.search")}
                    className="min-w-40 flex-1 rounded border border-line bg-surface px-3 py-1.5"
                  />
                  <button onClick={goNext} disabled={counts.error === 0} className={clsx(chip, "border-line hover:bg-surface-2")}>
                    {t("logs.nextError")}
                  </button>
                  <button
                    onClick={() => setFollow(!follow)}
                    aria-pressed={follow}
                    className={clsx(chip, follow ? "border-accent bg-accent text-accent-ink" : "border-line hover:bg-surface-2")}
                  >
                    {t("logs.follow")}
                  </button>
                </div>

                <div ref={scroller} className="min-h-0 flex-1 select-text overflow-auto rounded border border-line bg-surface font-mono text-xs">
                  {start > 0 && (
                    <button disabled={loading} onClick={() => void earlier()} className="w-full border-b border-line px-3 py-2 text-left text-muted hover:bg-surface-2">
                      {t("logs.earlier")}
                    </button>
                  )}
                  {filtered.length > MAX_RENDERED && (
                    <p className="border-b border-line px-3 py-2 text-muted">{t("logs.capped", { count: MAX_RENDERED })}</p>
                  )}
                  {shown.length === 0 && !loading && lines.length > 0 && <p className="px-3 py-4 text-muted">{t("logs.noMatches")}</p>}
                  {shown.map((l, i) => (
                    <div
                      key={l.n}
                      data-i={i}
                      className={clsx(
                        "whitespace-pre-wrap break-words px-3 py-0.5",
                        l.level === "error" && "bg-danger/15 text-danger",
                        l.level === "warn" && "text-accent",
                        i === active && "outline outline-1 outline-accent",
                      )}
                    >
                      {l.text || " "}
                    </div>
                  ))}
                  {start === 0 && end < size && (
                    <button disabled={loading} onClick={() => void poll()} className="w-full border-t border-line px-3 py-2 text-left text-muted hover:bg-surface-2">
                      {t("logs.later")}
                    </button>
                  )}
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
}
