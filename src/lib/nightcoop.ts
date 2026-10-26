import type { Detected, ModRecord, NcData } from "./ipc";

export const NC_SOURCE = "nightcoop";

export function ncLang(locale: string): "ru" | "en" {
  return locale === "ru" ? "ru" : "en";
}

export function activeNightcoop(mods: readonly ModRecord[]): ModRecord | null {
  return mods.find((m) => m.source === NC_SOURCE && m.enabled) ?? null;
}

export interface Installed {
  version: string | null;
  build: string | null;
  byHand: boolean;
}

export function installedNightcoop(mods: readonly ModRecord[], detected: Detected | null): Installed | null {
  const active = activeNightcoop(mods);
  if (active) return { version: active.version || null, build: null, byHand: false };
  if (detected && (detected.version || detected.build)) {
    return { version: detected.version, build: detected.build, byHand: true };
  }
  return null;
}

export function updateAvailable(
  data: NcData | null,
  mods: readonly ModRecord[],
  detected: Detected | null = null,
): boolean {
  const installed = installedNightcoop(mods, detected);
  return Boolean(installed && installed.version && data && data.latest && installed.version !== data.latest);
}

export function formatSize(bytes: number): string {
  return `${(bytes / 1048576).toFixed(1)} MB`;
}

export function shortHash(hash: string): string {
  return hash.length > 16 ? `${hash.slice(0, 16)}…` : hash;
}

export function percent(done: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(100, Math.floor((done * 100) / total));
}
