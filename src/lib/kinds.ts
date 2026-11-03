import type { ExternalMod } from "./ipc";

export type Kind = ExternalMod["kind"];

const PREFIXES: [string, Kind][] = [
  ["archive/", "archive"],
  ["bin/x64/plugins/cyber_engine_tweaks/", "cet"],
  ["r6/scripts/", "redscript"],
  ["r6/tweaks/", "tweaks"],
  ["red4ext/plugins/", "red4ext"],
];

export function kindsOf(files: readonly string[]): Kind[] {
  const found = new Set<Kind>();
  for (const file of files) {
    const lower = file.toLowerCase();
    for (const [prefix, kind] of PREFIXES) {
      if (lower.startsWith(prefix)) found.add(kind);
    }
  }
  return [...found];
}
