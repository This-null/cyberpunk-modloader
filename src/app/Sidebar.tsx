import { useEffect, useState } from "react";
import { FileText, Globe, Package, Play, Settings, User } from "lucide-react";
import clsx from "clsx";
import { useT, type MessageKey } from "../i18n";
import { ipc } from "../lib/ipc";
import { updateAvailable } from "../lib/nightcoop";
import { useMods } from "../stores/mods";
import { useNexus } from "../stores/nexus";
import { useNightcoop } from "../stores/nightcoop";
import { useSettings } from "../stores/settings";

export type Page = "mods" | "nexus" | "profile" | "logs" | "settings";

const REPO = "https://github.com/This-null/cyberpunk-modloader";
const DISCORD = "https://discord.gg/EaNcFyCEqP";

const items: { page: Page; label: MessageKey; icon: typeof Package }[] = [
  { page: "mods", label: "nav.mods", icon: Package },
  { page: "nexus", label: "nav.nexus", icon: Globe },
  { page: "profile", label: "nav.profile", icon: User },
  { page: "logs", label: "nav.logs", icon: FileText },
  { page: "settings", label: "nav.settings", icon: Settings },
];

function DiscordMark({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M20.32 4.37A19.8 19.8 0 0 0 15.4 2.85a.07.07 0 0 0-.08.04c-.21.38-.45.87-.61 1.26a18.3 18.3 0 0 0-5.42 0 12.6 12.6 0 0 0-.62-1.26.08.08 0 0 0-.08-.04c-1.7.3-3.35.81-4.92 1.52a.07.07 0 0 0-.03.03C.53 9.05-.32 13.6.1 18.09c0 .02.01.04.03.05a19.9 19.9 0 0 0 6 3.03.08.08 0 0 0 .08-.03c.46-.63.87-1.3 1.22-2a.08.08 0 0 0-.04-.1 13 13 0 0 1-1.87-.9.08.08 0 0 1 0-.13l.37-.29a.07.07 0 0 1 .08-.01c3.93 1.8 8.18 1.8 12.07 0a.07.07 0 0 1 .08.01l.37.29a.08.08 0 0 1 0 .13c-.6.35-1.22.65-1.87.9a.08.08 0 0 0-.04.1c.36.7.77 1.37 1.22 2a.08.08 0 0 0 .08.03 19.8 19.8 0 0 0 6.01-3.03.08.08 0 0 0 .03-.05c.5-5.18-.84-9.68-3.55-13.67a.06.06 0 0 0-.03-.03ZM8.02 15.33c-1.18 0-2.16-1.09-2.16-2.42s.96-2.42 2.16-2.42c1.21 0 2.18 1.1 2.16 2.42 0 1.33-.96 2.42-2.16 2.42Zm7.97 0c-1.18 0-2.16-1.09-2.16-2.42s.96-2.42 2.16-2.42c1.21 0 2.18 1.1 2.16 2.42 0 1.33-.95 2.42-2.16 2.42Z" />
    </svg>
  );
}

function GithubMark({ size = 18 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M12 .5a11.5 11.5 0 0 0-3.64 22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.04-.71.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.68 0-1.25.45-2.28 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.78 0c2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.84 1.19 3.09 0 4.41-2.69 5.39-5.25 5.67.41.36.78 1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z" />
    </svg>
  );
}

export function Sidebar({ page, onChange }: { page: Page; onChange: (p: Page) => void }) {
  const t = useT();
  const mods = useMods((s) => s.mods);
  const data = useNightcoop((s) => s.data);
  const detected = useNightcoop((s) => s.detected);
  const nexusUpdates = useNexus((s) => Object.values(s.items).filter((i) => i.update).length);
  const updateCount = nexusUpdates + (updateAvailable(data, mods, detected) ? 1 : 0);
  const gameDir = useSettings((s) => s.gameDir);
  const [running, setRunning] = useState(false);

  useEffect(() => {
    let alive = true;
    const tick = () =>
      ipc
        .gameRunning()
        .then((r) => alive && setRunning(r))
        .catch(() => {});
    tick();
    const id = setInterval(tick, 3000);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, []);

  return (
    <nav className="flex w-56 shrink-0 flex-col border-r border-line bg-surface px-3 pb-3 pt-4">
      <span className="px-3 pb-2 text-[11px] font-medium uppercase tracking-[0.14em] text-muted">{t("nav.menu")}</span>
      <div className="flex flex-col gap-0.5">
        {items.map(({ page: p, label, icon: Icon }, i) => {
          const active = page === p;
          return (
            <button
              key={p}
              onClick={() => onChange(p)}
              aria-current={active ? "page" : undefined}
              style={{ "--i": i } as React.CSSProperties}
              className={clsx(
                "anim-rise group flex h-11 items-center gap-3 rounded-xl px-2 text-left transition-colors",
                active ? "bg-accent/15 font-medium text-ink ring-1 ring-accent/35" : "text-muted hover:bg-surface-2 hover:text-ink",
              )}
            >
              <span
                className={clsx(
                  "grid h-7 w-7 shrink-0 place-items-center rounded-lg transition-colors",
                  active ? "bg-accent text-accent-ink" : "bg-surface-2 text-muted group-hover:text-ink",
                )}
              >
                <Icon size={16} />
              </span>
              <span>{t(label)}</span>
              {p === "mods" && updateCount > 0 && (
                <span
                  className="ml-auto grid h-5 min-w-5 place-items-center rounded-full bg-accent px-1.5 text-[11px] font-semibold text-accent-ink"
                  title={t("nightcoop.updateAvailable")}
                >
                  {updateCount}
                </span>
              )}
            </button>
          );
        })}
      </div>

      <div className="mt-auto flex flex-col gap-2 border-t border-line pt-3">
        <button
          disabled={running || !gameDir}
          onClick={() => void ipc.launchGame()}
          className="btn btn-ok btn-play h-11 w-full text-base"
        >
          <Play size={18} />
          {running ? t("mods.playing") : t("mods.play")}
        </button>
        <button
          onClick={() => void ipc.openUrl(REPO)}
          className="group flex items-center gap-3 rounded-xl border border-line bg-surface-2 p-3 text-left hover:border-accent/50"
        >
          <span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-bg text-ink group-hover:text-accent">
            <GithubMark />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="font-medium">GitHub</span>
            <span className="truncate text-xs text-muted">{t("nav.openSource")}</span>
          </span>
        </button>
        <button
          onClick={() => void ipc.openUrl(DISCORD)}
          className="group flex items-center gap-3 rounded-xl border border-line bg-surface-2 p-3 text-left hover:border-accent/50"
        >
          <span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-bg text-ink group-hover:text-accent">
            <DiscordMark />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="font-medium">Discord</span>
            <span className="truncate text-xs text-muted">{t("nav.community")}</span>
          </span>
        </button>
      </div>
    </nav>
  );
}
