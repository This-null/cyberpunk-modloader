import { de } from "./de";
import { en, type MessageKey } from "./en";
import { es } from "./es";
import { fr } from "./fr";
import type { Locale } from "./locale";
import { pl } from "./pl";
import { ptBR } from "./pt-BR";
import { ru } from "./ru";
import { tr } from "./tr";

export const dictionaries: Record<Locale, Record<MessageKey, string>> = {
  en,
  ru,
  es,
  "pt-BR": ptBR,
  fr,
  tr,
  pl,
  de,
};
