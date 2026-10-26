import type { ModRecord } from "./ipc";

export interface Group {
  key: string;
  primary: ModRecord;
  others: ModRecord[];
}

export type Relation = "older" | "newer" | "same" | "unknown";

function parts(version: string): { numbers: number[]; suffix: string } | null {
  const m = /^\s*[vV]?(\d+(?:\.\d+)*)(.*)$/.exec(version);
  if (!m) return null;
  return { numbers: m[1].split(".").map(Number), suffix: m[2].trim() };
}

function isPre(suffix: string): boolean {
  return /alpha|beta|rc|pre|dev|snapshot/i.test(suffix);
}

export function compareVersions(a: string, b: string): number | null {
  const x = parts(a);
  const y = parts(b);
  if (!x || !y) return null;
  const len = Math.max(x.numbers.length, y.numbers.length);
  for (let i = 0; i < len; i++) {
    const d = (x.numbers[i] ?? 0) - (y.numbers[i] ?? 0);
    if (d !== 0) return d < 0 ? -1 : 1;
  }
  if (x.suffix === y.suffix) return 0;
  if (x.suffix === "" && isPre(y.suffix)) return 1;
  if (y.suffix === "" && isPre(x.suffix)) return -1;
  return null;
}

export function relation(version: string, reference: string): Relation {
  const c = compareVersions(version, reference);
  if (c === null) return "unknown";
  return c < 0 ? "older" : c > 0 ? "newer" : "same";
}

export function groupKey(mod: ModRecord): string {
  if (mod.source) return `s:${mod.source}`;
  if (mod.nexus_id) return `n:${mod.nexus_id}`;
  return `m:${mod.name}`;
}

function newestFirst(list: ModRecord[]): ModRecord[] {
  return list
    .map((mod, i) => ({ mod, i }))
    .sort((a, b) => {
      const c = compareVersions(b.mod.version, a.mod.version);
      return c !== null && c !== 0 ? c : b.i - a.i;
    })
    .map((x) => x.mod);
}

export function groupMods(mods: readonly ModRecord[]): Group[] {
  const buckets = new Map<string, ModRecord[]>();
  for (const mod of mods) {
    const key = groupKey(mod);
    buckets.set(key, [...(buckets.get(key) ?? []), mod]);
  }
  return [...buckets.entries()].map(([key, list]) => {
    const ordered = newestFirst(list);
    const primary = ordered.find((m) => m.enabled) ?? ordered[0];
    return { key, primary, others: ordered.filter((m) => m !== primary) };
  });
}
