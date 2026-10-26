import { useLocale, type LocaleChoice } from "../i18n";
import { LOCALES } from "../i18n/locale";
import { ipc } from "./ipc";
import { useSettings, type AutoTranslate, type Motion, type Theme } from "../stores/settings";

const THEMES: Theme[] = ["system", "dark", "light"];
const MOTIONS: Motion[] = ["system", "full", "reduced"];

const TRANSLATES: AutoTranslate[] = ["ask", "on", "off"];

export function isAutoTranslate(v: unknown): v is AutoTranslate {
  return typeof v === "string" && (TRANSLATES as string[]).includes(v);
}

export function isMotion(v: unknown): v is Motion {
  return typeof v === "string" && (MOTIONS as string[]).includes(v);
}

export function isLocaleChoice(v: unknown): v is LocaleChoice {
  return v === "auto" || (typeof v === "string" && (LOCALES as readonly string[]).includes(v));
}

export function isTheme(v: unknown): v is Theme {
  return typeof v === "string" && (THEMES as string[]).includes(v);
}

export async function startPrefsSync(): Promise<void> {
  const saved = await ipc.getPrefs().catch(() => null);
  if (isLocaleChoice(saved?.locale)) useLocale.setState({ choice: saved.locale });
  if (isTheme(saved?.theme)) useSettings.setState({ theme: saved.theme });
  if (isMotion(saved?.motion)) useSettings.setState({ motion: saved.motion });
  if (isAutoTranslate(saved?.translate)) useSettings.setState({ translate: saved.translate });
  const push = () => {
    const { theme, motion, translate } = useSettings.getState();
    void ipc.setPrefs(useLocale.getState().choice, theme, motion, translate).catch(() => {});
  };
  useLocale.subscribe(push);
  useSettings.subscribe((s, prev) => {
    if (s.theme !== prev.theme || s.motion !== prev.motion || s.translate !== prev.translate) push();
  });
  push();
}
