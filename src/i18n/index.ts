import { create } from "zustand";
import { persist } from "zustand/middleware";
import { dictionaries } from "./dictionaries";
import { en, type MessageKey } from "./en";
import { format, type Vars } from "./format";
import { pickLocale, type Locale } from "./locale";

export type LocaleChoice = Locale | "auto";

interface LocaleState {
  choice: LocaleChoice;
  setChoice: (choice: LocaleChoice) => void;
}

export const useLocale = create<LocaleState>()(
  persist(
    (set) => ({
      choice: "auto",
      setChoice: (choice) => set({ choice }),
    }),
    { name: "cp2077-locale" },
  ),
);

export function systemLocale(): Locale {
  if (typeof navigator === "undefined") return "en";
  return pickLocale(navigator.languages?.length ? navigator.languages : [navigator.language]);
}

export function activeLocale(choice: LocaleChoice): Locale {
  return choice === "auto" ? systemLocale() : choice;
}

function lookup(locale: Locale, key: MessageKey, vars?: Vars): string {
  return format(dictionaries[locale][key] || en[key], vars);
}

export function translate(key: MessageKey, vars?: Vars): string {
  return lookup(activeLocale(useLocale.getState().choice), key, vars);
}

export function useT() {
  const choice = useLocale((s) => s.choice);
  const locale = activeLocale(choice);
  return (key: MessageKey, vars?: Vars) => lookup(locale, key, vars);
}

export type { MessageKey };
