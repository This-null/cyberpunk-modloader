import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import clsx from "clsx";
import { useLocale, useT, type LocaleChoice, type MessageKey } from "../../i18n";
import { LOCALES, LOCALE_NAMES } from "../../i18n/locale";
import { Logo } from "../../brand/Logo";
import { ipc, type AppInfo } from "../../lib/ipc";
import { useSettings, type AutoTranslate, type Motion, type Theme } from "../../stores/settings";
import { NexusSection } from "./NexusSection";
import { NxmSection } from "./NxmSection";
import { UpdateSection } from "./UpdateSection";

const translates: { value: AutoTranslate; label: MessageKey }[] = [
  { value: "ask", label: "translate.optAsk" },
  { value: "on", label: "translate.optOn" },
  { value: "off", label: "translate.optOff" },
];

const motions: { value: Motion; label: MessageKey }[] = [
  { value: "system", label: "motion.system" },
  { value: "full", label: "motion.full" },
  { value: "reduced", label: "motion.reduced" },
];

const themes: { value: Theme; label: MessageKey }[] = [
  { value: "system", label: "theme.system" },
  { value: "dark", label: "theme.dark" },
  { value: "light", label: "theme.light" },
];

export function SettingsScreen() {
  const t = useT();
  const motion = useSettings((s) => s.motion);
  const setMotion = useSettings((s) => s.setMotion);
  const translate = useSettings((s) => s.translate);
  const setTranslate = useSettings((s) => s.setTranslate);
  const translateLang = useSettings((s) => s.translateLang);
  const setTranslateLang = useSettings((s) => s.setTranslateLang);
  const gameDir = useSettings((s) => s.gameDir);
  const detected = useSettings((s) => s.detected);
  const invalid = useSettings((s) => s.invalid);
  const chooseGame = useSettings((s) => s.chooseGame);
  const theme = useSettings((s) => s.theme);
  const setTheme = useSettings((s) => s.setTheme);
  const choice = useLocale((s) => s.choice);
  const setChoice = useLocale((s) => s.setChoice);

  const [info, setInfo] = useState<AppInfo | null>(null);

  useEffect(() => {
    ipc.appInfo().then(setInfo).catch(() => {});
  }, []);

  async function browse() {
    const chosen = await open({ directory: true });
    if (typeof chosen === "string") await chooseGame(chosen);
  }

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-8 pb-10">
    <div className="flex flex-col gap-5">
      <header className="flex flex-col items-center gap-1 pb-2 text-center">
        <h1 className="text-2xl font-semibold tracking-tight">{t("settings.title")}</h1>
        <p className="text-sm text-muted">{t("settings.subtitle")}</p>
      </header>

      <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
        <h2 className="text-base font-semibold">{t("settings.gameFolder")}</h2>
        <div className="flex items-center gap-3">
          <code className="min-w-0 flex-1 truncate rounded border border-line bg-surface px-3 py-2">
            {gameDir ?? t("settings.notSet")}
          </code>
          <button onClick={browse} className="rounded border border-line px-4 py-2 hover:bg-surface-2">
            {t("settings.browse")}
          </button>
        </div>
        {invalid && <p className="text-danger">{t("settings.invalid")}</p>}
        {detected.length > 0 && (
          <div className="flex flex-col gap-2">
            <span className="text-muted">{t("settings.detected")}</span>
            {detected.map((d) => (
              <button
                key={d}
                onClick={() => chooseGame(d)}
                className={clsx(
                  "truncate rounded border px-3 py-2 text-left hover:bg-surface-2",
                  d === gameDir ? "border-accent" : "border-line",
                )}
              >
                {d}
              </button>
            ))}
          </div>
        )}
      </section>

      <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
        <h2 className="text-base font-semibold">{t("settings.language")}</h2>
        <select
          value={choice}
          onChange={(e) => setChoice(e.target.value as LocaleChoice)}
          className="w-72 rounded-md border border-line bg-surface-2 px-3 py-2"
        >
          <option value="auto">{t("settings.languageAuto")}</option>
          {LOCALES.map((l) => (
            <option key={l} value={l}>
              {LOCALE_NAMES[l]}
            </option>
          ))}
        </select>
      </section>

      <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
        <h2 className="text-base font-semibold">{t("settings.theme")}</h2>
        <div className="flex w-fit overflow-hidden rounded border border-line">
          {themes.map(({ value, label }) => (
            <button
              key={value}
              onClick={() => setTheme(value)}
              aria-pressed={theme === value}
              className={clsx(
                "px-4 py-2",
                theme === value ? "bg-accent font-medium text-accent-ink" : "bg-surface hover:bg-surface-2",
              )}
            >
              {t(label)}
            </button>
          ))}
        </div>
      </section>

      <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
        <h2 className="text-base font-semibold">{t("settings.motion")}</h2>
        <div className="flex w-fit overflow-hidden rounded border border-line">
          {motions.map(({ value, label }) => (
            <button
              key={value}
              onClick={() => setMotion(value)}
              aria-pressed={motion === value}
              className={clsx(
                "px-4 py-2",
                motion === value ? "bg-accent font-medium text-accent-ink" : "bg-surface hover:bg-surface-2",
              )}
            >
              {t(label)}
            </button>
          ))}
        </div>
        <p className="text-xs text-muted">{t("settings.motionHint")}</p>
      </section>

      <section className="flex flex-col gap-3 rounded-xl border border-line bg-surface p-5">
        <h2 className="text-base font-semibold">{t("translate.title")}</h2>
        <div className="flex w-fit overflow-hidden rounded border border-line">
          {translates.map(({ value, label }) => (
            <button
              key={value}
              onClick={() => setTranslate(value)}
              aria-pressed={translate === value}
              className={clsx(
                "px-4 py-2",
                translate === value ? "bg-accent font-medium text-accent-ink" : "bg-surface hover:bg-surface-2",
              )}
            >
              {t(label)}
            </button>
          ))}
        </div>
        <label className="flex flex-col gap-1">
          <span className="text-xs text-muted">{t("translate.into")}</span>
          <select
            value={translateLang}
            onChange={(e) => setTranslateLang(e.target.value)}
            className="w-72 rounded-md border border-line bg-surface-2 px-3 py-2"
          >
            <option value="app">{t("translate.sameAsApp")}</option>
            {LOCALES.map((l) => (
              <option key={l} value={l}>
                {LOCALE_NAMES[l]}
              </option>
            ))}
          </select>
        </label>
        <p className="text-xs text-muted">{t("translate.hint")}</p>
        <p className="text-xs text-muted">{t("translate.byGoogle")}</p>
      </section>

      <NexusSection />
      <NxmSection />
      <UpdateSection />

    </div>

      {info && (
        <footer className="mx-auto flex w-full max-w-xl flex-col items-center gap-3 border-t border-line pt-8 text-center">
          <Logo size={30} className="text-accent" />
          <div className="flex items-center gap-2">
            <span className="text-base font-semibold">CP2077 Mod Launcher</span>
            <span className="rounded bg-accent px-1.5 py-0.5 text-xs font-medium text-accent-ink">{t("settings.beta")}</span>
          </div>
          <span className="text-xs text-muted">
            {t("settings.appVersion")} {info.version.split("-")[0]}
          </span>
          <p className="max-w-md text-xs leading-relaxed text-muted">{t("settings.betaNote")}</p>
          <p className="max-w-md rounded-lg border border-line bg-surface px-4 py-3 text-xs leading-relaxed text-muted">
            {t("settings.unofficial")}
          </p>
          <button
            onClick={() => void ipc.openUrl("https://github.com/This-null/cyberpunk-modloader")}
            className="text-xs text-accent hover:underline"
          >
            GitHub · {t("nav.openSource")}
          </button>
          <span className="break-all text-[11px] text-muted/70">
            {t("settings.dataFolder")}: {info.data_dir}
          </span>
        </footer>
      )}
    </div>
  );
}
