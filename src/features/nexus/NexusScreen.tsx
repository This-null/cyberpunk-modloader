import { useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";
import clsx from "clsx";
import { ChevronRight, Download } from "lucide-react";
import { Logo } from "../../brand/Logo";
import { activeLocale, useLocale, useT, type MessageKey } from "../../i18n";
import { categoryName, categoryOptions, compactNumber, SORTS, type Sort } from "../../lib/browse";
import { ipc, type NexusCategory, type NexusMod } from "../../lib/ipc";
import { useIncoming } from "../../stores/incoming";
import { useNexus } from "../../stores/nexus";
import { useTranslated, useTranslateState, useTranslations } from "../../lib/translate";
import { useSettings } from "../../stores/settings";
import { useNexusBrowse } from "../../stores/nexusBrowse";
import { percent } from "../../lib/nightcoop";

const SORT_LABEL: Record<Sort, MessageKey> = {
  downloads: "nexusPage.sortPopular",
  endorsements: "nexusPage.sortEndorsed",
  createdAt: "nexusPage.sortNewest",
  updatedAt: "nexusPage.sortUpdated",
  relevance: "nexusPage.sortRelevance",
};

function Card({
  mod,
  categories,
  index,
  installed,
  canInstall,
  busy,
  progress,
  onInstall,
  onTag,
  locale,
  requirements,
  step,
}: {
  mod: NexusMod;
  categories: NexusCategory[];
  index: number;
  installed: boolean;
  canInstall: boolean;
  busy: boolean;
  progress: number | null;
  onInstall: () => void;
  onTag: (tag: string) => void;
  locale: string;
  requirements: ReactNode;
  step: { done: number; total: number } | null;
}) {
  const t = useT();
  const [broken, setBroken] = useState(false);
  const category = categoryName(categories, mod.category_id);
  const summary = useTranslated(mod.summary);

  return (
    <li
      className="anim-rise flex flex-col overflow-hidden rounded-lg border border-line bg-surface"
      style={{ "--i": Math.min(index, 8) } as CSSProperties}
    >
      <div className="relative h-36 shrink-0 overflow-hidden bg-surface-2">
        {mod.picture_url && !broken ? (
          <img
            src={mod.picture_url}
            alt=""
            loading="lazy"
            draggable={false}
            onError={() => setBroken(true)}
            className="absolute inset-0 h-full w-full object-cover"
          />
        ) : (
          <div className="grid h-full place-items-center">
            <Logo size={44} className="text-line" />
          </div>
        )}
        {mod.contains_adult_content && (
          <span className="absolute left-2 top-2 rounded bg-danger px-1.5 py-0.5 text-xs font-medium text-white">
            {t("nexusPage.adult")}
          </span>
        )}
      </div>
      <div className="flex flex-1 flex-col gap-2 p-4">
        <div className="flex items-start justify-between gap-2">
          <h3 className="line-clamp-2 font-semibold">{mod.name ?? `#${mod.mod_id}`}</h3>
          {mod.version && (
            <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">
              {t("nexusPage.version", { version: mod.version })}
            </span>
          )}
        </div>
        <p className="text-xs text-muted">
          {[mod.author ? t("nexusPage.by", { author: mod.author }) : "", category].filter(Boolean).join(" · ")}
        </p>
        <p className="line-clamp-3 text-muted">{summary}</p>
        {mod.tags.length > 0 && (
          <div className="flex flex-wrap gap-1">
            {mod.tags.slice(0, 4).map((tag) => (
              <button
                key={tag}
                onClick={() => onTag(tag)}
                className="rounded-full border border-line px-2 py-0.5 text-xs text-muted hover:border-accent hover:text-ink"
              >
                {tag}
              </button>
            ))}
          </div>
        )}
        <p className="mt-auto pt-1 text-xs text-muted">
          {t("nexusPage.downloads", { count: compactNumber(mod.mod_downloads, locale) })} ·{" "}
          {t("nexusPage.endorsements", { count: compactNumber(mod.endorsement_count, locale) })}
        </p>
        <div className="flex flex-wrap gap-2 pt-1">
          {canInstall && (
            <button
              disabled={busy || installed}
              onClick={onInstall}
              className={clsx(
                "flex items-center gap-2 rounded px-3 py-1.5",
                installed ? "border border-line text-muted" : "bg-accent font-medium text-accent-ink",
              )}
            >
              {!installed && <Download size={14} />}
              {installed
                ? t("nexusPage.installed")
                : progress !== null
                  ? step
                    ? t("nexusPage.step", { done: Math.min(step.done + 1, step.total), total: step.total, percent: progress })
                    : t("nexusPage.installingPercent", { percent: progress })
                  : t("nexusPage.install")}
            </button>
          )}
          <button
            onClick={() => {
              useIncoming.getState().startWatch();
              void ipc.openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.mod_id}`);
            }}
            className="rounded border border-line px-3 py-1.5 hover:bg-surface-2"
          >
            {t("nexusPage.open")}
          </button>
        </div>
        {requirements}
      </div>
    </li>
  );
}

function TranslateStrip() {
  const t = useT();
  const mode = useSettings((s) => s.translate);
  const setMode = useSettings((s) => s.setTranslate);
  const { active } = useTranslateState();
  const showOriginal = useTranslations((s) => s.showOriginal);
  const setShowOriginal = useTranslations((s) => s.setShowOriginal);
  const appLang = activeLocale(useLocale((s) => s.choice));
  const choice = useSettings((s) => s.translateLang);
  if (mode === "ask" && (choice !== "app" || appLang !== "en")) {
    return (
      <div className="anim-rise flex flex-wrap items-center gap-3 rounded-md border border-accent/40 bg-accent/10 px-3 py-2 text-xs">
        <span className="min-w-0 flex-1 text-ink">{t("translate.askMods")}</span>
        <button onClick={() => setMode("on")} className="rounded bg-accent px-3 py-1 font-medium text-accent-ink">{t("translate.yes")}</button>
        <button onClick={() => setMode("off")} className="rounded border border-line px-3 py-1 hover:bg-surface-2">{t("translate.no")}</button>
      </div>
    );
  }
  if (!active) return null;
  return (
    <div className="flex items-center gap-3 text-xs text-muted">
      <span>{t("translate.byGoogle")}</span>
      <button onClick={() => setShowOriginal(!showOriginal)} className="text-accent hover:underline">
        {showOriginal ? t("nightcoop.showTranslation") : t("nightcoop.showOriginal")}
      </button>
    </div>
  );
}

function Requirements({ mod, premium }: { mod: NexusMod; premium: boolean }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const entry = useNexusBrowse((s) => s.reqs[mod.mod_id]);
  const installed = useNexusBrowse((s) => s.installed);
  const installing = useNexusBrowse((s) => s.installing);
  const loadReqs = useNexusBrowse((s) => s.loadReqs);
  const installMissing = useNexusBrowse((s) => s.installMissing);
  const missing = (entry?.items ?? []).filter((r) => !r.external && r.mod_id > 0 && !installed.includes(r.mod_id));

  function toggle() {
    setOpen((v) => !v);
    if (!open) void loadReqs(mod.mod_id);
  }

  return (
    <div className="flex flex-col gap-2 border-t border-line pt-2">
      <button onClick={toggle} aria-expanded={open} className="flex items-center gap-2 text-left text-muted hover:text-ink">
        <ChevronRight size={14} className={clsx("transition-transform", open && "rotate-90")} />
        {t("nexusPage.requirements")}
      </button>
      {open && (
        <div className="anim-rise flex flex-col gap-2">
          {entry?.loading && <span className="text-xs text-muted">{t("nexusPage.loading")}</span>}
          {entry?.failed && <span className="text-xs text-danger">{t("nexusPage.requirementsFailed")}</span>}
          {entry && !entry.loading && !entry.failed && entry.items.length === 0 && (
            <span className="text-xs text-muted">{t("nexusPage.noRequirements")}</span>
          )}
          {entry?.items.map((r) => {
            const have = installed.includes(r.mod_id);
            return (
              <div key={`${r.mod_id}-${r.name}`} className="flex items-center justify-between gap-2 rounded bg-surface-2 px-2 py-1.5">
                <span className="min-w-0 truncate">
                  {r.name}
                  {r.notes && <span className="text-xs text-muted"> {r.notes}</span>}
                </span>
                {have ? (
                  <span className="shrink-0 text-xs text-ok">{t("nexusPage.installed")}</span>
                ) : r.external || r.mod_id === 0 ? (
                  <span className="shrink-0 text-xs text-muted">{t("nexusPage.external")}</span>
                ) : (
                  <button
                    onClick={() => {
                      useIncoming.getState().startWatch();
                      void ipc.openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${r.mod_id}`);
                    }}
                    className="shrink-0 rounded border border-line px-2 py-0.5 text-xs hover:bg-surface"
                  >
                    {t("nexusPage.open")}
                  </button>
                )}
              </div>
            );
          })}
          {premium && missing.length > 0 && (
            <button
              disabled={installing !== null}
              onClick={() => void installMissing(mod.mod_id)}
              className="self-start rounded bg-accent px-3 py-1.5 font-medium text-accent-ink disabled:opacity-60"
            >
              {t("nexusPage.installMissing", { count: missing.length })}
            </button>
          )}
          {!premium && missing.length > 0 && <span className="text-xs text-muted">{t("nexusPage.requirementsFree")}</span>}
        </div>
      )}
    </div>
  );
}

export function NexusScreen({ onSettings }: { onSettings: () => void }) {
  const t = useT();
  const locale = activeLocale(useLocale((s) => s.choice));
  const connected = useNexus((s) => s.status?.connected ?? false);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);
  const items = useNexusBrowse((s) => s.items);
  const total = useNexusBrowse((s) => s.total);
  const categories = useNexusBrowse((s) => s.categories);
  const installedIds = useNexusBrowse((s) => s.installed);
  const loading = useNexusBrowse((s) => s.loading);
  const loadingMore = useNexusBrowse((s) => s.loadingMore);
  const failed = useNexusBrowse((s) => s.failed);
  const installing = useNexusBrowse((s) => s.installing);
  const progress = useNexusBrowse((s) => s.progress);
  const chain = useNexusBrowse((s) => s.chain);
  const premiumNeeded = useNexusBrowse((s) => s.premiumNeeded);
  const sort = useNexusBrowse((s) => s.sort);
  const query = useNexusBrowse((s) => s.query);
  const category = useNexusBrowse((s) => s.category);
  const hideAdult = useNexusBrowse((s) => s.hideAdult);
  const tag = useNexusBrowse((s) => s.tag);
  const start = useNexusBrowse((s) => s.start);
  const reload = useNexusBrowse((s) => s.reload);
  const loadMore = useNexusBrowse((s) => s.loadMore);
  const installWithRequirements = useNexusBrowse((s) => s.installWithRequirements);
  const setSort = useNexusBrowse((s) => s.setSort);
  const setQuery = useNexusBrowse((s) => s.setQuery);
  const setCategory = useNexusBrowse((s) => s.setCategory);
  const setHideAdult = useNexusBrowse((s) => s.setHideAdult);
  const setTag = useNexusBrowse((s) => s.setTag);

  const [typed, setTyped] = useState(query);
  const scroller = useRef<HTMLDivElement>(null);
  const sentinel = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (connected) void start();
  }, [connected, start]);

  useEffect(() => {
    if (typed === query) return;
    const id = setTimeout(() => setQuery(typed), 400);
    return () => clearTimeout(id);
  }, [typed, query, setQuery]);

  useEffect(() => {
    const el = sentinel.current;
    if (!el || !scroller.current) return;
    const io = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && void loadMore(), {
      root: scroller.current,
      rootMargin: "400px",
    });
    io.observe(el);
    return () => io.disconnect();
  }, [loadMore, items.length, connected]);

  const options = useMemo(() => categoryOptions(categories), [categories]);
  const sorts: Sort[] = query.trim() === "" ? SORTS : ["relevance", ...SORTS];
  const percentDone = progress ? percent(progress.done, progress.total) : 0;
  const chip = "rounded border px-3 py-1.5";

  return (
    <div className="flex h-full flex-col gap-4">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="flex flex-col">
          <h1 className="text-xl font-semibold">{t("nexusPage.title")}</h1>
          {connected && !failed && (
            <span className="text-xs text-muted">{t("nexusPage.total", { count: total.toLocaleString(locale) })}</span>
          )}
        </div>
        {connected && (
          <div className="flex flex-wrap gap-2">
            {sorts.map((k) => (
              <button
                key={k}
                onClick={() => setSort(k)}
                aria-pressed={sort === k}
                className={clsx(chip, sort === k ? "border-accent bg-accent text-accent-ink" : "border-line hover:bg-surface-2")}
              >
                {t(SORT_LABEL[k])}
              </button>
            ))}
          </div>
        )}
      </div>

      {!connected ? (
        <div className="grid flex-1 place-items-center rounded-lg border border-dashed border-line p-8 text-center">
          <div className="flex max-w-md flex-col items-center gap-4">
            <p className="text-muted">{t("nexusPage.connectFirst")}</p>
            <button onClick={onSettings} className="rounded bg-accent px-4 py-2 font-medium text-accent-ink">
              {t("nexusPage.goSettings")}
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <input
              value={typed}
              onChange={(e) => setTyped(e.target.value)}
              placeholder={t("nexusPage.search")}
              aria-label={t("nexusPage.search")}
              className="min-w-48 flex-1 rounded border border-line bg-surface px-3 py-2"
            />
            <select
              value={category}
              onChange={(e) => setCategory(Number(e.target.value))}
              aria-label={t("nexusPage.allCategories")}
              className="max-w-56 rounded border border-line bg-surface px-3 py-2"
            >
              <option value={0}>{t("nexusPage.allCategories")}</option>
              {options.map((o) => (
                <option key={o.id} value={o.id}>
                  {`${"  ".repeat(o.depth)}${o.name}`}
                </option>
              ))}
            </select>
            <label className="flex cursor-pointer items-center gap-2 rounded border border-line px-3 py-2">
              <input
                type="checkbox"
                checked={hideAdult}
                onChange={(e) => setHideAdult(e.target.checked)}
                className="h-4 w-4 accent-[var(--accent)]"
              />
              {t("nexusPage.hideAdult")}
            </label>
            {tag && (
              <button
                onClick={() => setTag(null)}
                title={t("nexusPage.clearTag")}
                className="flex items-center gap-2 rounded-full border border-accent bg-accent/10 px-3 py-1.5 text-accent hover:bg-accent/20"
              >
                {t("nexusPage.tag", { tag })}
                <span aria-hidden>×</span>
              </button>
            )}
          </div>
          <TranslateStrip />
          {(!premium || premiumNeeded) && <p className="rounded border border-line bg-surface p-3 text-muted">{t("nexusPage.premiumOnly")}</p>}

          <div ref={scroller} className="flex min-h-0 flex-1 flex-col gap-4 overflow-auto pr-1">
            {loading && items.length === 0 && <p className="text-muted">{t("nexusPage.loading")}</p>}
            {failed && (
              <div className="flex flex-wrap items-center gap-3">
                <p className="text-danger">{t("nexusPage.failed")}</p>
                <button onClick={() => void reload()} className="rounded border border-line px-3 py-1.5 hover:bg-surface-2">
                  {t("nightcoop.retry")}
                </button>
              </div>
            )}
            {!loading && !failed && items.length === 0 && <p className="text-muted">{t("nexusPage.noResults")}</p>}
            {items.length > 0 && (
              <ul className={clsx("grid grid-cols-1 gap-4 md:grid-cols-2 xl:grid-cols-3", loading && "opacity-60 transition-opacity")}>
                {items.map((m, i) => (
                  <Card
                    key={m.mod_id}
                    mod={m}
                    categories={categories}
                    index={i % 24}
                    installed={installedIds.includes(m.mod_id)}
                    canInstall={premium}
                    busy={installing !== null}
                    progress={installing !== null && (installing === m.mod_id || chain !== null) ? percentDone : null}
                    step={installing !== null && chain !== null ? chain : null}
                    onInstall={() => void installWithRequirements(m.mod_id)}
                    onTag={setTag}
                    locale={locale}
                    requirements={<Requirements mod={m} premium={premium} />}
                  />
                ))}
              </ul>
            )}
            <div ref={sentinel} className="flex h-10 shrink-0 items-center justify-center text-xs text-muted">
              {loadingMore && t("nexusPage.loadingMore")}
              {!loadingMore && !loading && items.length > 0 && items.length >= total && t("nexusPage.end")}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
