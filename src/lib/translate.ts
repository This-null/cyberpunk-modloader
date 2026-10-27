import { useEffect } from "react";
import { create } from "zustand";
import { activeLocale, useLocale } from "../i18n";
import { ipc } from "./ipc";
import { useSettings } from "../stores/settings";

const MAX_LEN = 1400;
const cache = new Map<string, string>();
const failedAt = new Map<string, number>();
const pending = new Map<string, Set<string>>();
let timer: ReturnType<typeof setTimeout> | null = null;

interface View {
  version: number;
  showOriginal: boolean;
  bump: () => void;
  setShowOriginal: (v: boolean) => void;
}

export const useTranslations = create<View>((set) => ({
  version: 0,
  showOriginal: false,
  bump: () => set((s) => ({ version: s.version + 1 })),
  setShowOriginal: (showOriginal) => set({ showOriginal }),
}));

export function targetLang(translateLang: string, appLang: string): string {
  return translateLang === "app" ? appLang : translateLang;
}

export function shouldTranslate(mode: string, translateLang: string, appLang: string): boolean {
  return mode === "on" && (translateLang !== "app" || appLang !== "en");
}

const keyOf = (lang: string, text: string) => `${lang}|${text}`;

async function flush() {
  timer = null;
  const jobs = [...pending.entries()];
  pending.clear();
  for (const [lang, set] of jobs) {
    const texts = [...set];
    for (let i = 0; i < texts.length; i += 40) {
      const chunk = texts.slice(i, i + 40);
      try {
        const out = await ipc.translateNotes(chunk, lang);
        chunk.forEach((text, n) => cache.set(keyOf(lang, text), out[n] ?? text));
      } catch {
        chunk.forEach((text) => failedAt.set(keyOf(lang, text), Date.now()));
      }
      useTranslations.getState().bump();
    }
  }
}

function enqueue(text: string, lang: string) {
  const failed = failedAt.get(keyOf(lang, text));
  if (failed && Date.now() - failed < 60_000) return;
  const set = pending.get(lang) ?? new Set<string>();
  set.add(text);
  pending.set(lang, set);
  if (!timer) timer = setTimeout(() => void flush(), 150);
}

export function useTranslateState() {
  const mode = useSettings((s) => s.translate);
  const choice = useSettings((s) => s.translateLang);
  const appLang = activeLocale(useLocale((s) => s.choice));
  return { active: shouldTranslate(mode, choice, appLang), lang: targetLang(choice, appLang) };
}

export function useTranslated(text: string | null | undefined): string {
  const { active, lang } = useTranslateState();
  useTranslations((s) => s.version);
  const showOriginal = useTranslations((s) => s.showOriginal);
  const plain = (text ?? "").trim();
  const usable = active && plain !== "" && plain.length <= MAX_LEN;
  const hit = usable ? cache.get(keyOf(lang, plain)) : undefined;
  useEffect(() => {
    if (usable && hit === undefined) enqueue(plain, lang);
  }, [usable, hit, plain, lang]);
  if (!usable || showOriginal || hit === undefined) return text ?? "";
  return hit;
}

export function clearTranslationCache() {
  cache.clear();
  failedAt.clear();
}
