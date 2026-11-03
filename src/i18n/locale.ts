export const LOCALES = ["en", "ru", "es", "pt-BR", "fr", "tr", "pl", "de"] as const;

export type Locale = (typeof LOCALES)[number];

export const LOCALE_NAMES: Record<Locale, string> = {
  en: "English",
  ru: "Русский",
  es: "Español",
  "pt-BR": "Português (Brasil)",
  fr: "Français",
  tr: "Türkçe",
  pl: "Polski",
  de: "Deutsch",
};

export function pickLocale(languages: readonly string[]): Locale {
  for (const raw of languages) {
    const base = raw.toLowerCase().split("-")[0];
    if (base === "pt") return "pt-BR";
    const hit = LOCALES.find((l) => l.toLowerCase() === base);
    if (hit) return hit;
  }
  return "en";
}
