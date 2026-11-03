import { useEffect, useState } from "react";
import { Check, Copy, FileInput, KeyRound, Settings2, Share2, X } from "lucide-react";
import { useT } from "../../i18n";
import { ipc, type SharedEntry } from "../../lib/ipc";
import { useMods } from "../../stores/mods";
import { useMyPresets } from "../../stores/presets";
import { useProfile } from "../../stores/profile";
import { FrameworkList } from "./FrameworkList";
import { PresetsDialog } from "../mods/PresetsDialog";
import { ProfileDialog } from "../mods/ProfileDialog";

export function timeLeft(expires: number, now: number): string {
  const s = Math.max(0, expires - now);
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const t = useT();
  const [done, setDone] = useState(false);
  return (
    <button
      onClick={() => {
        void navigator.clipboard.writeText(text).then(() => {
          setDone(true);
          setTimeout(() => setDone(false), 1500);
        });
      }}
      className="flex items-center gap-1.5 btn"
    >
      {done ? <Check size={14} className="text-ok" /> : <Copy size={14} />}
      {done ? t("pview.copied") : label}
    </button>
  );
}

export function ProfileScreen() {
  const t = useT();
  const mods = useMods((s) => s.mods);
  const presets = useMyPresets((s) => s.list);
  const [id, setId] = useState("");
  const [shares, setShares] = useState<SharedEntry[]>([]);
  const [confirm, setConfirm] = useState(false);
  const now = Math.floor(Date.now() / 1000);

  useEffect(() => {
    void ipc.getInstallId().then(setId).catch(() => {});
    void ipc.sharesList().then(setShares).catch(() => {});
    void useMyPresets.getState().load();
  }, []);

  const btn = "btn";
  const card = "flex flex-col gap-3 rounded-xl border border-line bg-surface p-5";

  return (
    <div className="mx-auto flex max-w-2xl flex-col gap-4 pb-10">
      <PresetsDialog />
      <ProfileDialog names={mods.map((m) => m.name)} />
      <div>
        <h1 className="text-xl font-semibold">{t("pview.title")}</h1>
        <p className="mt-1 text-xs text-muted">{t("pview.intro")}</p>
      </div>

      <FrameworkList />

      <section className={card}>
        <h2 className="text-base font-semibold">{t("pview.idTitle")}</h2>
        <code className="w-fit rounded bg-surface-2 px-3 py-1.5 text-sm">{id || "..."}</code>
        <div className="flex flex-wrap items-center gap-2">
          <CopyButton text={id} label={t("pview.copyId")} />
          {confirm ? (
            <>
              <button
                onClick={() => void ipc.resetInstallId().then((v) => { setId(v); setConfirm(false); })}
                className="btn btn-danger-solid"
              >
                <KeyRound size={15} />
                {t("pview.confirmReset")}
              </button>
              <button onClick={() => setConfirm(false)} className={btn}><X size={15} />{t("action.cancel")}</button>
            </>
          ) : (
            <button onClick={() => setConfirm(true)} className={btn}><KeyRound size={15} />{t("pview.resetId")}</button>
          )}
        </div>
        <p className="text-xs text-muted">{t("pview.idHint")}</p>
      </section>

      <section className={card}>
        <div className="flex items-center justify-between">
          <h2 className="text-base font-semibold">{t("pview.presets")}</h2>
          <button onClick={() => void useMyPresets.getState().show()} className={btn}><Settings2 size={15} />{t("pview.manage")}</button>
        </div>
        {presets.length === 0 ? (
          <p className="text-muted">{t("presets.empty")}</p>
        ) : (
          <ul className="flex flex-col gap-1.5">
            {presets.map((p) => (
              <li key={p.name} className="flex items-center justify-between rounded-md bg-surface-2 px-3 py-2">
                <span className="font-medium">{p.name}</span>
                <span className="text-xs text-muted">{t("pview.mods", { count: p.mods.length })}</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className={card}>
        <div className="flex items-center justify-between">
          <h2 className="text-base font-semibold">{t("pview.sharedTitle")}</h2>
          <div className="flex gap-2">
            <button
              disabled={mods.length === 0}
              onClick={() => void useProfile.getState().open("share", mods.map((m) => m.name))}
              className={btn}
            >
              <Share2 size={15} />
              {t("profile.share")}
            </button>
            <button onClick={() => void useProfile.getState().open("import", [])} className={btn}><FileInput size={15} />{t("profile.import")}</button>
          </div>
        </div>
        {shares.length === 0 ? (
          <p className="text-muted">{t("pview.sharedEmpty")}</p>
        ) : (
          <ul className="flex flex-col gap-1.5">
            {shares.map((s) => (
              <li key={s.code} className="flex flex-wrap items-center gap-3 rounded-md bg-surface-2 px-3 py-2">
                <code className="font-semibold tracking-wider">{s.code}</code>
                <span className="min-w-0 flex-1 truncate text-muted">{s.name} · {t("pview.mods", { count: s.count })}</span>
                <span className="text-xs text-muted">{t("pview.expiresIn", { time: timeLeft(s.expires, now) })}</span>
                <CopyButton text={s.code} label={t("pview.copyCode")} />
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
