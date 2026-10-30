import { useEffect, useMemo, useRef, useState, type CSSProperties } from "react";
import clsx from "clsx";
import type { ReactNode } from "react";
import { ChevronRight, Clock, Download, ExternalLink, Eye, EyeOff, Flame, Heart, RotateCw, Search, Settings, Sparkles, Target, X } from "lucide-react";
import { Dropdown } from "../../components/Dropdown";
import { CoverPlaceholder } from "../../brand/CoverPlaceholder";
import { activeLocale, useLocale, useT, type MessageKey } from "../../i18n";
import { categoryName, categoryOptions, compactNumber, formatDate, SORTS, type Sort } from "../../lib/browse";
import { ipc, type NexusCategory, type NexusMod } from "../../lib/ipc";
import { useIncoming } from "../../stores/incoming";
import { useNexus } from "../../stores/nexus";
import { useTranslated, useTranslateState, useTranslations } from "../../lib/translate";
import { useSettings } from "../../stores/settings";
import { useNexusBrowse } from "../../stores/nexusBrowse";
import { percent } from "../../lib/nightcoop";

const SORT_ICON: Record<Sort, ReactNode> = {
  downloads: <Flame size={15} />,
  endorsements: <Heart size={15} />,
  createdAt: <Sparkles size={15} />,
  updatedAt: <Clock size={15} />,
  relevance: <Target size={15} />,
};

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
  empty,
  canInstall,
  busy,
  progress,
  selected,
  onInstall,
  onOpen,
  locale,
  step,
}: {
  mod: NexusMod;
  categories: NexusCategory[];
  index: number;
  installed: boolean;
  empty: boolean;
  canInstall: boolean;
  busy: boolean;
  progress: number | null;
  selected: boolean;
  onInstall: () => void;
  onOpen: () => void;
  locale: string;
  step: { done: number; total: number } | null;
}) {
  const t = useT();
  const [broken, setBroken] = useState(false);
  const category = categoryName(categories, mod.category_id);
  const summary = useTranslated(mod.summary);

  return (
    <li
      onClick={onOpen}
      className={clsx(
        "anim-rise flex cursor-pointer flex-col overflow-hidden rounded-lg border bg-surface hover:border-accent/60",
        selected ? "border-accent" : "border-line",
      )}
      style={{ "--i": Math.min(index, 8) } as CSSProperties}
    >
      <div className="relative h-28 shrink-0 overflow-hidden bg-surface-2">
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
          <CoverPlaceholder name={mod.name ?? String(mod.mod_id)} />
        )}
        {mod.contains_adult_content && (
          <span className="absolute left-2 top-2 rounded bg-danger px-1.5 py-0.5 text-xs font-medium text-white">
            {t("nexusPage.adult")}
          </span>
        )}
      </div>
      <div className="flex flex-1 flex-col gap-1.5 p-3">
        <div className="flex items-start justify-between gap-2">
          <h3 className="line-clamp-1 font-semibold">{mod.name ?? `#${mod.mod_id}`}</h3>
          {mod.version && (
            <span className="shrink-0 rounded border border-line px-1.5 py-0.5 text-xs text-muted">
              {t("nexusPage.version", { version: mod.version })}
            </span>
          )}
        </div>
        <p className="line-clamp-1 text-xs text-muted">
          {[mod.author ? t("nexusPage.by", { author: mod.author }) : "", category].filter(Boolean).join(" · ")}
        </p>
        {mod.updated_timestamp > 0 && (
          <p className="text-xs text-muted">{t("nexusPage.updatedOn", { date: formatDate(mod.updated_timestamp, locale) })}</p>
        )}
        <p className="line-clamp-2 text-muted">{summary}</p>
        <div className="mt-auto flex items-center justify-between gap-2 pt-1">
          <span className="text-xs text-muted">
            {t("nexusPage.downloads", { count: compactNumber(mod.mod_downloads, locale) })}
          </span>
          {canInstall && (
            <button
              disabled={busy || installed || empty}
              onClick={(e) => {
                e.stopPropagation();
                onInstall();
              }}
              className={clsx("btn btn-sm", !installed && "btn-primary")}
            >
              {!installed && <Download size={14} />}
              {installed
                ? t("nexusPage.installed")
                : empty
                  ? t("nexusPage.nothingToInstall")
                  : progress !== null
                  ? step
                    ? `${Math.min(step.done + 1, step.total)}/${step.total} · ${progress}%`
                    : t("nexusPage.installingPercent", { percent: progress })
                  : t("nexusPage.install")}
            </button>
          )}
        </div>
      </div>
    </li>
  );
}

function DetailPanel({
  mod,
  categories,
  installed,
  empty,
  premium,
  busy,
  progress,
  step,
  onInstall,
  onClose,
  locale,
}: {
  mod: NexusMod;
  categories: NexusCategory[];
  installed: boolean;
  empty: boolean;
  premium: boolean;
  busy: boolean;
  progress: number | null;
  step: { done: number; total: number } | null;
  onInstall: () => void;
  onClose: () => void;
  locale: string;
}) {
  const t = useT();
  const [broken, setBroken] = useState(false);
  const summary = useTranslated(mod.summary);
  const category = categoryName(categories, mod.category_id);

  useEffect(() => setBroken(false), [mod.mod_id]);

  return (
    <aside
      key={mod.mod_id}
      className="anim-rise fixed bottom-0 right-0 top-10 z-30 flex w-[26rem] max-w-full flex-col gap-3 overflow-auto border-l border-line bg-surface p-4 shadow-2xl"
    >
      <div className="flex items-start justify-between gap-2">
        <h2 className="text-lg font-semibold">{mod.name ?? `#${mod.mod_id}`}</h2>
        <button onClick={onClose} aria-label={t("action.close")} className="rounded p-1 text-muted hover:bg-surface-2 hover:text-ink">
          <X size={18} />
        </button>
      </div>
      <div className="relative h-44 shrink-0 overflow-hidden rounded-md bg-surface-2">
        {mod.picture_url && !broken ? (
          <img src={mod.picture_url} alt="" draggable={false} onError={() => setBroken(true)} className="h-full w-full object-cover" />
        ) : (
          <CoverPlaceholder name={mod.name ?? String(mod.mod_id)} size="lg" />
        )}
      </div>
      <p className="text-xs text-muted">
        {[
          mod.version ? t("nexusPage.version", { version: mod.version }) : "",
          mod.author ? t("nexusPage.by", { author: mod.author }) : "",
          category,
        ]
          .filter(Boolean)
          .join(" · ")}
      </p>
      <p className="text-xs text-muted">
        {t("nexusPage.downloads", { count: compactNumber(mod.mod_downloads, locale) })} ·{" "}
        {t("nexusPage.endorsements", { count: compactNumber(mod.endorsement_count, locale) })}
        {mod.updated_timestamp > 0 && ` · ${t("nexusPage.updatedOn", { date: formatDate(mod.updated_timestamp, locale) })}`}
      </p>
      <p className="whitespace-pre-line text-muted">{summary}</p>
      {mod.tags.length > 0 && (
        <div className="flex flex-wrap gap-1">
          {mod.tags.map((tag) => (
            <span key={tag} className="rounded-full border border-line px-2 py-0.5 text-xs text-muted">
              {tag}
            </span>
          ))}
        </div>
      )}
      <div className="flex flex-wrap gap-2">
        {premium && (
          <button
            disabled={busy || installed || empty}
            onClick={onInstall}
            className={clsx("btn", !installed && "btn-primary")}
          >
            {!installed && <Download size={14} />}
            {installed
              ? t("nexusPage.installed")
              : empty
                ? t("nexusPage.nothingToInstall")
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
          className="btn"
        >
          <ExternalLink size={15} />
          {t("nexusPage.open")}
        </button>
      </div>
      {empty && <p className="rounded-md border border-line bg-surface-2 p-3 text-xs text-muted">{t("err.no_files")}</p>}
      <Requirements mod={mod} premium={premium} />
      <Versions mod={mod} premium={premium} />
    </aside>
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
        <button onClick={() => setMode("on")} className="btn btn-sm btn-primary">{t("translate.yes")}</button>
        <button onClick={() => setMode("off")} className="btn btn-sm">{t("translate.no")}</button>
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

function Versions({ mod, premium }: { mod: NexusMod; premium: boolean }) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const entry = useNexusBrowse((s) => s.files[mod.mod_id]);
  const installing = useNexusBrowse((s) => s.installing);
  const loadFiles = useNexusBrowse((s) => s.loadFiles);
  const installFile = useNexusBrowse((s) => s.installFile);

  function toggle() {
    setOpen((v) => !v);
    if (!open) void loadFiles(mod.mod_id);
  }

  return (
    <div className="flex flex-col gap-2 border-t border-line pt-2">
      <button onClick={toggle} aria-expanded={open} className="flex items-center gap-2 text-left text-muted hover:text-ink">
        <ChevronRight size={14} className={clsx("transition-transform", open && "rotate-90")} />
        {t("nexusPage.versions")}
      </button>
      {open && (
        <div className="anim-rise flex flex-col gap-2">
          {entry?.loading && <span className="text-xs text-muted">{t("nexusPage.loading")}</span>}
          {entry?.failed && <span className="text-xs text-danger">{t("nexusPage.versionsFailed")}</span>}
          {entry && !entry.loading && !entry.failed && entry.items.length === 0 && (
            <span className="text-xs text-muted">{t("nexusPage.noVersions")}</span>
          )}
          {entry?.items.map((f) => (
            <div key={f.file_id} className="flex items-center justify-between gap-2 rounded bg-surface-2 px-2 py-1.5">
              <span className="min-w-0">
                <span className="block truncate">
                  {f.name || f.file_name} {f.version && <span className="font-medium">{f.version}</span>}
                </span>
                <span className="text-xs text-muted">
                  {f.category_name.toUpperCase() === "OLD_VERSION" ? t("nexusPage.oldVersion") : f.category_name.toLowerCase()} · {(f.size_kb / 1024).toFixed(1)} MB
                </span>
              </span>
              {premium && (
                <button
                  disabled={installing !== null}
                  onClick={() => void installFile(mod.mod_id, f.file_id)}
                  className="btn shrink-0 btn-sm disabled:opacity-60"
                >
                  <Download size={14} />
                  {t("nexusPage.installThis")}
                </button>
              )}
            </div>
          ))}
        </div>
      )}
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
                    className="btn shrink-0 btn-sm"
                  >
                    <ExternalLink size={14} />
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
              className="self-start btn btn-primary disabled:opacity-60"
            >
              <Download size={15} />
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
  const emptyIds = useNexusBrowse((s) => s.empty);
  const loading = useNexusBrowse((s) => s.loading);
  const loadingMore = useNexusBrowse((s) => s.loadingMore);
  const failed = useNexusBrowse((s) => s.failed);
  const installing = useNexusBrowse((s) => s.installing);
  const progress = useNexusBrowse((s) => s.progress);
  const chain = useNexusBrowse((s) => s.chain);
  const chainRoot = useNexusBrowse((s) => s.chainRoot);
  const detailId = useNexusBrowse((s) => s.detail);
  const openDetail = useNexusBrowse((s) => s.openDetail);
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
  const detailMod = items.find((m) => m.mod_id === detailId) ?? null;
  const progressFor = (id: number) =>
    installing !== null && (chainRoot !== null ? chainRoot === id : installing === id) ? percentDone : null;
  const stepFor = (id: number) => (installing !== null && chainRoot === id ? chain : null);

  useEffect(() => {
    if (detailId === null) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && openDetail(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [detailId, openDetail]);
  const chip = "chip";

  return (
    <div className={clsx("flex h-full flex-col gap-4", connected && detailMod && "mr-[25rem]")}>
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
                className={clsx(chip, sort === k ? "chip-on" : "")}
              >
                {SORT_ICON[k]}
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
            <button onClick={onSettings} className="btn btn-primary">
              <Settings size={15} />
              {t("nexusPage.goSettings")}
            </button>
          </div>
        </div>
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-2">
            <label className="field flex min-w-48 flex-1 items-center gap-2">
              <Search size={16} className="shrink-0 text-muted" />
              <input
                value={typed}
                onChange={(e) => setTyped(e.target.value)}
                placeholder={t("nexusPage.search")}
                aria-label={t("nexusPage.search")}
                className="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted"
              />
              {typed !== "" && (
                <button type="button" onClick={() => setTyped("")} aria-label={t("action.close")} className="shrink-0 rounded p-0.5 text-muted transition-colors hover:text-ink">
                  <X size={15} />
                </button>
              )}
            </label>
            <Dropdown
              className="w-56"
              value={category}
              onChange={setCategory}
              ariaLabel={t("nexusPage.allCategories")}
              searchPlaceholder={t("nexusPage.search")}
              options={[{ value: 0, label: t("nexusPage.allCategories") }, ...options.map((o) => ({ value: o.id, label: o.name, depth: o.depth }))]}
            />
            <button type="button" onClick={() => setHideAdult(!hideAdult)} aria-pressed={hideAdult} className={clsx("chip py-2", hideAdult && "chip-on")}>
              {hideAdult ? <EyeOff size={16} /> : <Eye size={16} />}
              {t("nexusPage.hideAdult")}
            </button>
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
                <button onClick={() => void reload()} className="btn">
                  <RotateCw size={15} />
                  {t("nightcoop.retry")}
                </button>
              </div>
            )}
            {!loading && !failed && items.length === 0 && <p className="text-muted">{t("nexusPage.noResults")}</p>}
            {items.length > 0 && (
              <ul className={clsx("grid grid-cols-[repeat(auto-fill,minmax(15rem,1fr))] gap-3", loading && "opacity-60 transition-opacity")}>
                {items.map((m, i) => (
                  <Card
                    key={m.mod_id}
                    mod={m}
                    categories={categories}
                    index={i % 24}
                    installed={installedIds.includes(m.mod_id)}
                    empty={emptyIds.includes(m.mod_id)}
                    canInstall={premium}
                    busy={installing !== null}
                    progress={progressFor(m.mod_id)}
                    step={stepFor(m.mod_id)}
                    selected={detailId === m.mod_id}
                    onInstall={() => void installWithRequirements(m.mod_id)}
                    onOpen={() => openDetail(m.mod_id)}
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
      {connected && detailMod && (
        <DetailPanel
          mod={detailMod}
          categories={categories}
          installed={installedIds.includes(detailMod.mod_id)}
          empty={emptyIds.includes(detailMod.mod_id)}
          premium={premium}
          busy={installing !== null}
          progress={progressFor(detailMod.mod_id)}
          step={stepFor(detailMod.mod_id)}
          onInstall={() => void installWithRequirements(detailMod.mod_id)}
          onClose={() => openDetail(null)}
          locale={locale}
        />
      )}
    </div>
  );
}
