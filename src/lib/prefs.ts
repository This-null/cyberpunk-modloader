import { useLocale, type LocaleChoice } from "../i18n";
import { LOCALES } from "../i18n/locale";
import { ipc } from "./ipc";
import { useSettings, type AutoTranslate, type Motion, type Theme } from "../stores/settings";

const THEMES: Theme[] = ["system", "dark", "light", "midnight", "ocean", "mocha", "aurora", "sunset", "neon", "emerald"];
const MOTIONS: Motion[] = ["system", "full", "reduced"];

const TRANSLATES: AutoTranslate[] = ["ask", "on", "off"];

export function isAutoTranslate(v: unknown): v is AutoTranslate {
  return typeof v === "string" && (TRANSLATES as string[]).includes(v);
}

export function isUpdates(v: unknown): v is "on" | "off" {
  return v === "on" || v === "off";
}

export function isTranslateLang(v: unknown): v is string {
  return v === "app" || (typeof v === "string" && v !== "auto" && isLocaleChoice(v));
}

export function isModUpdates(v: unknown): v is "manual" | "auto" {
  return v === "manual" || v === "auto";
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
  if (isUpdates(saved?.updates)) useSettings.setState({ updates: saved.updates });
  if (isTranslateLang(saved?.translate_lang)) useSettings.setState({ translateLang: saved.translate_lang });
  if (isModUpdates(saved?.mod_updates)) useSettings.setState({ modUpdates: saved.mod_updates });
  const push = () => {
    const { theme, motion, translate, updates, translateLang, modUpdates } = useSettings.getState();
    void ipc.setPrefs(useLocale.getState().choice, theme, motion, translate, updates, translateLang, modUpdates).catch(() => {});
  };
  useLocale.subscribe(push);
  useSettings.subscribe((s, prev) => {
    if (s.theme !== prev.theme || s.motion !== prev.motion || s.translate !== prev.translate || s.updates !== prev.updates || s.translateLang !== prev.translateLang || s.modUpdates !== prev.modUpdates) push();
  });
  push();
}
