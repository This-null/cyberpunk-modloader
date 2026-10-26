import clsx from "clsx";
import { ChevronDown, Pin } from "lucide-react";
import { useEffect, useState } from "react";
import { activeLocale, useLocale } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { useSettings } from "../../stores/settings";
import { Logo } from "../../brand/Logo";
import { useT, type MessageKey } from "../../i18n";
import type { NcRelease } from "../../lib/ipc";
import { formatSize, installedNightcoop, percent, shortHash, updateAvailable } from "../../lib/nightcoop";
import { useMods } from "../../stores/mods";
import { useNightcoop } from "../../stores/nightcoop";

const OPEN_KEY = "nc-card-open";

function readOpen(): boolean {
  try {
    return localStorage.getItem(OPEN_KEY) !== "0";
  } catch {
    return true;
  }
}

function channelLabel(channel: string): MessageKey | null {
  return channel === "beta" ? "channel.beta" : channel === "stable" ? "channel.stable" : null;
}

export function NightCoopCard({ locked }: { locked: boolean }) {
  const t = useT();
  const data = useNightcoop((s) => s.data);
  const loading = useNightcoop((s) => s.loading);
  const failed = useNightcoop((s) => s.failed);
  const installing = useNightcoop((s) => s.installing);
  const progress = useNightcoop((s) => s.progress);
  const fetch = useNightcoop((s) => s.fetch);
  const install = useNightcoop((s) => s.install);
  const mods = useMods((s) => s.mods);
  const detected = useNightcoop((s) => s.detected);

  const [open, setOpen] = useState(readOpen);
  const choice = useLocale((s) => s.choice);
  const [translated, setTranslated] = useState<{ version: string; lang: string; lines: string[] } | null>(null);
  const [showOriginal, setShowOriginal] = useState(false);
  const [translating, setTranslating] = useState(false);
  const [translateFailed, setTranslateFailed] = useState(false);
  function toggle() {
    setOpen((o) => {
      try {
        localStorage.setItem(OPEN_KEY, o ? "0" : "1");
      } catch {}
      return !o;
    });
  }

  const installed = installedNightcoop(mods, detected);
  const hasUpdate = updateAvailable(data, mods, detected);
  const installedLabel = !installed
    ? t("nightcoop.notInstalled")
    : installed.byHand
      ? installed.version
        ? t("nightcoop.installedByHand", { version: installed.version })
        : t("nightcoop.installedBuild", { build: installed.build ?? "?" })
      : t("nightcoop.installed", { version: installed.version ?? "" });
  const latest: NcRelease | undefined =
    data?.releases.find((r) => r.version === data.latest && !r.yanked) ?? data?.releases.find((r) => !r.yanked);
  const busy = installing !== null || locked;
  const lang = activeLocale(choice);

  const autoMode = useSettings((s) => s.translate);
  const setAutoMode = useSettings((s) => s.setTranslate);
  const sameTranslation = translated && latest && translated.version === latest.version && translated.lang === lang;
  const wantAuto = autoMode === "on" && lang !== "en" && !!latest && latest.notes.length > 0 && !sameTranslation;

  useEffect(() => {
    if (wantAuto && !translating && !translateFailed) void translate();
  }, [wantAuto, latest?.version, lang]);

  async function translate() {
    if (!latest) return;
    setTranslating(true);
    setTranslateFailed(false);
    try {
      const lines = await ipc.translateNotes(latest.notes, lang);
      setTranslated({ version: latest.version, lang, lines });
      setShowOriginal(false);
    } catch {
      setTranslateFailed(true);
    } finally {
      setTranslating(false);
    }
  }

  function release(r: NcRelease, primary: boolean) {
    const label = channelLabel(r.channel);
    return (
      <div className={clsx("flex flex-col gap-2", primary && "min-w-0 flex-1")}>
        <div className="flex flex-wrap items-center gap-2">
          <span className={clsx("font-semibold", primary ? "text-lg" : "text-base")}>v{r.version}</span>
          {label && <span className="rounded border border-line px-1.5 py-0.5 text-xs text-muted">{t(label)}</span>}
          {installed?.version === r.version && (
            <span className="rounded bg-accent px-1.5 py-0.5 text-xs font-medium text-accent-ink">{t("nightcoop.active")}</span>
          )}
        </div>
        {r.title && <p>{r.title}</p>}
        <p className="text-xs text-muted">
          {[
            r.date,
            r.game.length > 0 ? t("nightcoop.game", { versions: r.game.join(" · ") }) : "",
            t("nightcoop.protocol", { n: r.protocol }),
            formatSize(r.size),
          ]
            .filter(Boolean)
            .join(" · ")}
        </p>
        <p className="font-mono text-xs text-muted/80">{t("nightcoop.checksum", { hash: shortHash(r.sha256) })}</p>
      </div>
    );
  }

  return (
    <section className="pinned flex shrink-0 flex-col gap-4 rounded-lg border p-5">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <button
          onClick={toggle}
          aria-expanded={open}
          title={open ? t("nightcoop.collapse") : t("nightcoop.expand")}
          className="flex items-center gap-1.5 rounded-full border border-accent/40 bg-accent/10 px-2.5 py-1 text-[11px] font-medium uppercase tracking-wider text-accent hover:bg-accent/20"
        >
          <Pin size={12} className="rotate-45" />
          {t("nightcoop.pinned")}
          <ChevronDown size={13} className={clsx("transition-transform", !open && "-rotate-90")} />
        </button>
        <span className="text-xs text-muted">{installedLabel}</span>
      </div>
      <div className="flex flex-wrap items-center gap-3">
        <h2 className="text-2xl font-semibold tracking-tight">{t("nightcoop.title")}</h2>
        {hasUpdate && (
          <span className="rounded bg-accent px-2 py-0.5 text-xs font-medium text-accent-ink">{t("nightcoop.updateAvailable")}</span>
        )}
      </div>

      {open && (
        <>
      {!data && loading && <p className="text-muted">{t("nightcoop.loading")}</p>}

      {!data && !loading && failed && (
        <div className="flex flex-wrap items-center gap-3">
          <p className="text-muted">{t("nightcoop.unreachable")}</p>
          <button onClick={fetch} className="rounded border border-line px-3 py-1.5 hover:bg-surface-2">
            {t("nightcoop.retry")}
          </button>
        </div>
      )}

      {data?.offline && (
        <div className="flex flex-wrap items-center gap-3">
          <p className="text-muted">{t("nightcoop.offline")}</p>
          <button onClick={fetch} className="rounded border border-line px-3 py-1.5 hover:bg-surface-2">
            {t("nightcoop.retry")}
          </button>
        </div>
      )}

      {data && latest && (
        <>
          <div className="flex flex-wrap items-start gap-4">
            {release(latest, true)}
            <div className="flex w-56 shrink-0 flex-col gap-2">
              {installed?.version === latest.version ? (
                <button disabled className="rounded-md border border-line px-4 py-2.5">
                  {t("nightcoop.upToDate")}
                </button>
              ) : (
                <button
                  disabled={busy || data.offline}
                  onClick={() => install(latest.version)}
                  className="rounded-md bg-accent px-4 py-2.5 font-medium text-accent-ink hover:brightness-110"
                >
                  {installed?.version
                    ? t("nightcoop.update", { version: latest.version })
                    : t("nightcoop.install", { version: latest.version })}
                </button>
              )}
              {installing && (
                <div className="flex flex-col gap-1">
                  <div className="h-2 overflow-hidden rounded bg-surface-2">
                    <div
                      className="h-full bg-accent transition-[width]"
                      style={{ width: `${progress ? percent(progress.done, progress.total) : 0}%` }}
                    />
                  </div>
                  <span className="text-xs text-muted">
                    {progress && percent(progress.done, progress.total) < 100
                      ? t("nightcoop.downloading", { percent: percent(progress.done, progress.total) })
                      : t("nightcoop.finishing")}
                  </span>
                </div>
              )}
            </div>
          </div>

          <p className="rounded-md bg-surface-2/70 px-3 py-2 text-xs text-muted">{t("nightcoop.protocolHint")}</p>

          {latest.notes.length > 0 && (
            <details className="text-muted">
              <summary className="cursor-pointer">{t("nightcoop.whatsNew")}</summary>
              <ul className="mt-2 flex max-h-56 list-disc flex-col gap-1 overflow-auto pl-5 pr-2 text-ink">
                {(translated && !showOriginal && translated.version === latest.version && translated.lang === lang
                  ? translated.lines
                  : latest.notes
                ).map((n, i) => (
                  <li key={i}>{n}</li>
                ))}
              </ul>
              <div className="mt-2 flex flex-wrap items-center gap-3 text-xs">
                {sameTranslation ? (
                  <button onClick={() => setShowOriginal((v) => !v)} className="text-accent hover:underline">
                    {showOriginal ? t("nightcoop.showTranslation") : t("nightcoop.showOriginal")}
                  </button>
                ) : (
                  <button
                    disabled={translating}
                    onClick={() => void translate()}
                    title={t("nightcoop.translateHint")}
                    className="rounded border border-line px-2 py-1 hover:bg-surface-2 disabled:opacity-60"
                  >
                    {translating ? t("nightcoop.translating") : t("nightcoop.translate")}
                  </button>
                )}
                {translateFailed && <span className="text-danger">{t("nightcoop.translateFailed")}</span>}
                <span className="text-muted">{t("nightcoop.translateHint")}</span>
              </div>
              {autoMode === "ask" && lang !== "en" && !sameTranslation && (
                <div className="anim-rise mt-3 flex flex-wrap items-center gap-3 rounded-md border border-accent/40 bg-accent/10 px-3 py-2 text-xs">
                  <span className="min-w-0 flex-1 text-ink">{t("translate.ask")}</span>
                  <button onClick={() => setAutoMode("on")} className="rounded bg-accent px-3 py-1 font-medium text-accent-ink">
                    {t("translate.yes")}
                  </button>
                  <button onClick={() => setAutoMode("off")} className="rounded border border-line px-3 py-1 hover:bg-surface-2">
                    {t("translate.no")}
                  </button>
                </div>
              )}
            </details>
          )}

          <details className="text-muted">
            <summary className="cursor-pointer">
              {t("nightcoop.allVersions")} ({data.releases.length})
            </summary>
            <ul className="mt-2 flex flex-col gap-2">
              {data.releases.map((r) => (
                <li key={r.version} className="flex flex-wrap items-center gap-3 rounded border border-line bg-surface-2 px-3 py-2 text-ink">
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-medium">v{r.version}</span>
                      {channelLabel(r.channel) && (
                        <span className="text-xs text-muted">{t(channelLabel(r.channel)!)}</span>
                      )}
                    </div>
                    <div className="text-xs text-muted">
                      {r.date} · {t("nightcoop.protocol", { n: r.protocol })} · {formatSize(r.size)}
                    </div>
                    {r.yanked && <div className="text-xs text-danger">{t("nightcoop.withdrawn", { reason: r.yanked })}</div>}
                  </div>
                  {installed?.version === r.version ? (
                    <span className="text-xs text-accent">{t("nightcoop.active")}</span>
                  ) : (
                    !r.yanked && (
                      <button
                        disabled={busy || data.offline}
                        onClick={() => install(r.version)}
                        className="rounded border border-line px-3 py-1.5 hover:bg-surface"
                      >
                        {t("nightcoop.use")}
                      </button>
                    )
                  )}
                </li>
              ))}
            </ul>
          </details>
        </>
      )}
      <p className="flex items-center gap-2 border-t border-accent/20 pt-3 text-xs text-muted">
        <Logo size={14} className="shrink-0 text-accent" />
        {t("nightcoop.madeFor")}
      </p>
        </>
      )}
    </section>
  );
}
