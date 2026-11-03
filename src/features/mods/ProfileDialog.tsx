import { open as openFile, save } from "@tauri-apps/plugin-dialog";
import { useState } from "react";
import clsx from "clsx";
import { useT } from "../../i18n";
import { ipc } from "../../lib/ipc";
import { openNexusPage, useProfile } from "../../stores/profile";
import { useNexus } from "../../stores/nexus";
import { useErrors } from "../../stores/errors";

const btn = "btn disabled:opacity-60";
const primary = "btn btn-primary disabled:opacity-60";

export function ProfileDialog({ names }: { names: string[] }) {
  const t = useT();
  const mode = useProfile((s) => s.mode);
  const title = useProfile((s) => s.title);
  const built = useProfile((s) => s.built);
  const shared = useProfile((s) => s.shared);
  const text = useProfile((s) => s.text);
  const items = useProfile((s) => s.items);
  const profileName = useProfile((s) => s.name);
  const invalid = useProfile((s) => s.invalid);
  const working = useProfile((s) => s.working);
  const premium = useNexus((s) => s.status?.user?.is_premium ?? false);
  const { close, setTitle, setText, rebuild, share, check, installMissing } = useProfile.getState();
  const [note, setNote] = useState("");

  if (!mode) return null;
  const missing = items.filter((i) => !i.installed);

  async function copy(value: string) {
    await navigator.clipboard.writeText(value);
    setNote(t("profile.copied"));
    setTimeout(() => setNote(""), 1500);
  }

  async function saveFile() {
    if (!built) return;
    const dest = await save({ defaultPath: "my-setup.cp2077profile", filters: [{ name: "Profile", extensions: ["cp2077profile", "json"] }] });
    if (!dest) return;
    try {
      await ipc.profileSave(dest, built.json);
      setNote(t("profile.saved"));
    } catch (e) {
      useErrors.getState().report(e);
    }
  }

  async function loadFile() {
    const chosen = await openFile({ multiple: false, filters: [{ name: "Profile", extensions: ["cp2077profile", "json"] }] });
    if (typeof chosen !== "string") return;
    try {
      setText(await ipc.profileRead(chosen));
      await check();
    } catch (e) {
      useErrors.getState().report(e);
    }
  }

  return (
    <div className="fixed inset-0 z-40 grid place-items-center bg-black/55 p-6 anim-fade" role="dialog" aria-modal="true">
      <div className="anim-pop flex max-h-[calc(100vh-3rem)] w-full max-w-xl flex-col gap-4 overflow-auto rounded-lg border border-line bg-surface p-6">
        <div className="flex items-center justify-between">
          <h2 className="text-lg font-semibold">{mode === "share" ? t("profile.shareTitle") : t("profile.importTitle")}</h2>
          <button onClick={close} className="text-muted hover:text-ink">{t("action.close")}</button>
        </div>

        {mode === "share" && (
          <>
            <label className="flex flex-col gap-1">
              <span className="text-xs text-muted">{t("profile.name")}</span>
              <input
                value={title}
                onChange={(e) => setTitle(e.target.value)}
                onBlur={() => void rebuild(names)}
                maxLength={80}
                className="rounded border border-line bg-surface-2 px-3 py-2"
              />
            </label>
            {built ? (
              <>
                <p className="text-muted">{t("profile.count", { count: built.count })}</p>
                {built.skipped.length > 0 && (
                  <p className="text-xs text-muted">{t("profile.skipped", { names: built.skipped.slice(0, 5).join(", ") })}</p>
                )}
                <div className="flex flex-wrap items-center gap-2">
                  <button className={btn} onClick={() => void copy(built.code)}>{t("profile.copy")}</button>
                  <button className={btn} onClick={() => void saveFile()}>{t("profile.save")}</button>
                  <button className={primary} disabled={working} onClick={() => void share()}>{t("profile.online")}</button>
                  {note && <span className="text-xs text-ok">{note}</span>}
                </div>
                <p className="text-xs text-muted">{t("profile.onlineHint")}</p>
                {shared && (
                  <div className="anim-rise flex flex-col gap-2 rounded-md border border-accent/40 bg-accent/10 p-3">
                    <span className="text-xs text-muted">{t("profile.shortCode")}</span>
                    <div className="flex items-center gap-3">
                      <span className="font-mono text-2xl tracking-widest text-accent">{shared.code}</span>
                      <button className={btn} onClick={() => void copy(shared.code)}>{t("profile.copy")}</button>
                    </div>
                    <span className="text-xs text-muted">
                      {t("profile.expires", { time: new Date(shared.expires * 1000).toLocaleTimeString() })}
                    </span>
                  </div>
                )}
              </>
            ) : (
              <p className="text-muted">{t("profile.nothing")}</p>
            )}
          </>
        )}

        {mode === "import" && (
          <>
            <textarea
              value={text}
              onChange={(e) => setText(e.target.value)}
              placeholder={t("profile.paste")}
              rows={3}
              className="resize-none rounded border border-line bg-surface-2 px-3 py-2 font-mono text-xs"
            />
            <div className="flex flex-wrap items-center gap-2">
              <button className={primary} disabled={working || text.trim() === ""} onClick={() => void check()}>{t("profile.check")}</button>
              <button className={btn} disabled={working} onClick={() => void loadFile()}>{t("profile.loadFile")}</button>
              {invalid && <span className="text-xs text-danger">{t("profile.invalid")}</span>}
            </div>
            {items.length > 0 && (
              <>
                {profileName && <p className="font-medium">{profileName}</p>}
                <ul className="flex max-h-64 flex-col gap-1 overflow-auto">
                  {items.map((it) => (
                    <li key={`${it.source}-${it.mod_id}-${it.version}`} className="flex items-center justify-between gap-2 rounded bg-surface-2 px-3 py-1.5">
                      <span className="min-w-0 truncate">
                        {it.name}
                        {it.version && <span className="text-xs text-muted"> {it.version}</span>}
                      </span>
                      {it.installed ? (
                        <span className="shrink-0 text-xs text-ok">{t("profile.have")}</span>
                      ) : it.source === "nexus" && !premium ? (
                        <button className="btn shrink-0 btn-sm" onClick={() => openNexusPage(it.mod_id)}>
                          {t("nexusPage.open")}
                        </button>
                      ) : (
                        <span className={clsx("shrink-0 text-xs", "text-accent")}>{t("profile.missing")}</span>
                      )}
                    </li>
                  ))}
                </ul>
                {missing.length === 0 ? (
                  <p className="text-ok">{t("profile.allInstalled")}</p>
                ) : (
                  <>
                    <button className={clsx(primary, "self-start")} disabled={working} onClick={() => void installMissing()}>
                      {t("profile.installMissing", { count: missing.filter((m) => m.source === "nightcoop" || premium).length })}
                    </button>
                    {!premium && missing.some((m) => m.source === "nexus") && <p className="text-xs text-muted">{t("nexusPage.requirementsFree")}</p>}
                  </>
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}
