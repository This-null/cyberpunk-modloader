import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import clsx from "clsx";
import { Download } from "lucide-react";
import { Logo } from "../../brand/Logo";
import { activeLocale, useLocale, useT, type MessageKey } from "../../i18n";
import { categoryName, categoryOptions, compactNumber, SORTS, type Sort } from "../../lib/browse";
import { ipc, type NexusCategory, type NexusMod } from "../../lib/ipc";
import { useIncoming } from "../../stores/incoming";
import { useNexus } from "../../stores/nexus";
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
}) {
  const t = useT();
  const [broken, setBroken] = useState(false);
  const category = categoryName(categories, mod.category_id);

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
        <p className="line-clamp-3 text-muted">{mod.summary}</p>
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
                  ? t("nexusPage.installingPercent", { percent: progress })
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
      </div>
    </li>
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
  const premiumNeeded = useNexusBrowse((s) => s.premiumNeeded);
  const sort = useNexusBrowse((s) => s.sort);
  const query = useNexusBrowse((s) => s.query);
  const category = useNexusBrowse((s) => s.category);
  const hideAdult = useNexusBrowse((s) => s.hideAdult);
  const tag = useNexusBrowse((s) => s.tag);
  const start = useNexusBrowse((s) => s.start);
  const reload = useNexusBrowse((s) => s.reload);
  const loadMore = useNexusBrowse((s) => s.loadMore);
  const install = useNexusBrowse((s) => s.install);
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
                    progress={installing === m.mod_id ? percentDone : null}
                    onInstall={() => void install(m.mod_id)}
                    onTag={setTag}
                    locale={locale}
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
